//! Installed predecessor migration, serial Store upgrades and immutable replay.
use super::{
    Client, artifacts, inject, maintain, maintenance_tests as first, quiescence_tests,
    readiness_tests, recover_store, store, support,
};
use canic_management_protocol::{Refusal, custody, publication, store_maintenance as wire};

async fn history(root: &Client<'_>) -> custody::RootHistory {
    root.update(
        "observe_root_history",
        custody::ROOT_HISTORY,
        &(),
        custody::ROOT_HISTORY_LIMITS,
    )
    .await
}

async fn stage(root: &Client<'_>, approval: &wire::Approval) -> Result<wire::Status, Refusal> {
    root.update(
        "stage_store_maintenance",
        wire::STAGE,
        approval,
        wire::LIMITS,
    )
    .await
}

async fn finish(root: &Client<'_>, approval: &wire::Approval, wasm: &[u8]) -> wire::Status {
    first::install(
        root,
        &wire::Install {
            approval: approval.clone(),
            wasm: wasm.to_vec(),
        },
    )
    .await
    .unwrap();
    for _ in 0..24 {
        let status = first::advance(root, approval).await;
        assert!(status.failure.is_none(), "{status:?}");
        if status.phase == wire::Phase::Verified {
            first::activate(root, approval).await;
            let ready = recover_store(root, approval).await;
            assert_eq!(ready.phase, wire::Phase::Ready);
            return ready;
        }
    }
    panic!("bounded Store replacement");
}

#[tokio::test(flavor = "current_thread")]
async fn refused_root_upgrade_preserves_the_original_issued_store_install() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let (root, supplier) = readiness_tests::fixture_with_root(
        pic,
        include_bytes!("../../fixtures/root-store-single-maintenance.wasm").to_vec(),
    )
    .await;
    let wasm = std::fs::read(artifacts().join("candidate.wasm")).unwrap();
    let approval = first::stopped(&root, &supplier, &wasm).await;
    first::install(
        &root,
        &wire::Install {
            approval: approval.clone(),
            wasm: wasm.clone(),
        },
    )
    .await
    .unwrap();
    let issued = first::advance(&root, &approval).await;
    assert_eq!(issued.phase, wire::Phase::InstallIssued);
    assert!(issued.installation.as_ref().unwrap().request.is_some());
    super::refuse_old_backing(&root).await;
    assert_eq!(first::observe(&root).await, issued);
    first::check_predecessor_custody(&root, &issued).await;
    let ready = finish(&root, &approval, &wasm).await;
    assert_eq!(ready.approval, issued.approval);
    assert_eq!(
        ready.installation.as_ref().unwrap().request,
        issued.installation.as_ref().unwrap().request
    );
    assert_eq!(
        ready.installation.as_ref().unwrap().sender_version,
        issued.installation.as_ref().unwrap().sender_version
    );
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

#[tokio::test(flavor = "current_thread")]
async fn root_retains_store_successors_across_upgrade_and_refuses_stale_authority() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let predecessor = include_bytes!("../../fixtures/root-store-single-maintenance.wasm");
    let (root, supplier) = readiness_tests::fixture(pic).await;
    let original = std::fs::read(artifacts().join("original.wasm")).unwrap();
    let candidate = std::fs::read(artifacts().join("candidate.wasm")).unwrap();
    let manifest = supplier.publish_bytes("retained", b"\0asm\x01\0\0\0").await;
    supplier
        .pin(&store::Pin {
            operation: [93; 32],
            manifest,
        })
        .await;
    let approval = first::stopped(&root, &supplier, &candidate).await;
    let old = finish(&root, &approval, &candidate).await;
    assert_eq!(first::observe(&root).await, old);
    maintain(&root).await;
    let complete = history(&root).await;
    assert_eq!(complete.current.store_operation, Some(old.clone()));
    assert!(complete.store_history.is_empty());
    let mut records = vec![old.clone()];

    for sequence in 2..=wire::MAX_UPGRADES as u64 {
        let wasm = if sequence % 2 == 0 {
            &original
        } else {
            &candidate
        };
        let previous = records.last().unwrap();
        let next = wire::Approval {
            previous_module: previous.approval.module_hash,
            module_hash: store::digest(wasm),
            attempt: store::MaintenanceAttempt {
                sequence,
                operation: [sequence as u8; 32],
            },
            expires_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 1_800_000_000_000,
            ..approval.clone()
        };
        let before = history(&root).await;
        for invalid in [
            wire::Approval {
                attempt: store::MaintenanceAttempt {
                    sequence: sequence + 1,
                    ..next.attempt.clone()
                },
                ..next.clone()
            },
            wire::Approval {
                attempt: store::MaintenanceAttempt {
                    operation: old.approval.attempt.operation,
                    ..next.attempt.clone()
                },
                ..next.clone()
            },
            wire::Approval {
                previous_module: [44; 32],
                ..next.clone()
            },
        ] {
            assert!(stage(&root, &invalid).await.is_err());
            assert_eq!(history(&root).await, before);
        }
        stage(&root, &next).await.unwrap();
        assert_eq!(
            first::advance(&root, &next).await.phase,
            wire::Phase::Closed
        );
        let pending = history(&root).await;
        assert_eq!(pending.store_history, records);
        // Old authorities return their own terminal evidence and cannot reopen the new gate.
        assert_eq!(stage(&root, &approval).await.unwrap(), old);
        assert_eq!(first::activate(&root, &approval).await, old);
        assert_eq!(first::advance(&root, &approval).await, old);
        assert_eq!(recover_store(&root, &approval).await, old);
        assert_eq!(
            root.update::<_, Option<wire::Status>>(
                "observe_store_attempt",
                wire::OBSERVE_ATTEMPT,
                &approval,
                wire::LIMITS
            )
            .await,
            Some(old.clone())
        );
        assert_eq!(history(&root).await, pending);
        assert_eq!(
            stage(
                &root,
                &wire::Approval {
                    expires_at_ns: next.expires_at_ns + 1,
                    ..next.clone()
                }
            )
            .await,
            Err(Refusal::Occupied)
        );
        // The old full-custody contract cannot silently omit completed predecessors.
        assert!(
            pic.update_call(
                root.id,
                root.operator,
                "observe_root_custody",
                ic_wire::encode(
                    &(),
                    ic_wire::Encoding::Frame {
                        method: custody::ROOT,
                        direction: ic_wire::Direction::Request
                    },
                    custody::ROOT_LIMITS
                )
                .unwrap()
            )
            .await
            .is_err()
        );
        for _ in 0..16 {
            if first::advance(&root, &next).await.phase == wire::Phase::Stopped {
                break;
            }
        }
        assert_eq!(
            history(&root).await.current.store_operation.unwrap().phase,
            wire::Phase::Stopped
        );
        let closed_bytes = pic.get_stable_memory(supplier.id).await;
        if sequence == 2 {
            // Upgrade Root with the successor pending, then lose its upload callback.
            maintain(&root).await;
            canic_qualification::upgrade_stopped(
                pic,
                root.id,
                std::fs::read(artifacts().join("root-faults.wasm")).unwrap(),
                vec![],
                Some(root.operator),
            )
            .await
            .unwrap();
            inject(&root, 15).await;
            let input = wire::Install {
                approval: next.clone(),
                wasm: wasm.clone(),
            };
            assert!(
                pic.update_call(
                    root.id,
                    root.operator,
                    "install_store_maintenance",
                    ic_wire::encode(
                        &input,
                        ic_wire::Encoding::Frame {
                            method: wire::INSTALL,
                            direction: ic_wire::Direction::Request
                        },
                        wire::INSTALL_LIMITS
                    )
                    .unwrap()
                )
                .await
                .is_err()
            );
            inject(&root, 0).await;
            assert_eq!(
                recover_store(&root, &next).await.phase,
                wire::Phase::Uploaded
            );
        }
        // No implicit activation: inspect every stable Store byte after installation/reopen.
        first::install(
            &root,
            &wire::Install {
                approval: next.clone(),
                wasm: wasm.clone(),
            },
        )
        .await
        .unwrap();
        for _ in 0..24 {
            if first::advance(&root, &next).await.phase == wire::Phase::Verified {
                break;
            }
        }
        assert_eq!(
            history(&root).await.current.store_operation.unwrap().phase,
            wire::Phase::Verified
        );
        assert_eq!(pic.get_stable_memory(supplier.id).await, closed_bytes);
        records.push(finish(&root, &next, wasm).await);
        maintain(&root).await;
        let retained = history(&root).await;
        assert_eq!(retained.current.store_operation.as_ref(), records.last());
        assert_eq!(retained.store_history, records[..records.len() - 1]);
    }
    let before = history(&root).await;
    let last = records.last().unwrap();
    let ninth = wire::Approval {
        previous_module: last.approval.module_hash,
        module_hash: last.approval.previous_module,
        attempt: store::MaintenanceAttempt {
            sequence: 9,
            operation: [9; 32],
        },
        ..last.approval.clone()
    };
    assert_eq!(stage(&root, &ninth).await, Err(Refusal::Occupied));
    assert_eq!(history(&root).await, before);
    assert!(
        canic_qualification::upgrade_stopped(
            pic,
            root.id,
            predecessor.to_vec(),
            vec![],
            Some(root.operator)
        )
        .await
        .is_err()
    );
    assert_eq!(history(&root).await, before);
    // Publication and child admission still work once the final operation has activated.
    root.update::<_, publication::Reply>(
        "store_publication",
        publication::EXECUTE,
        &publication::Request {
            store: supplier.id.to_text(),
            expires_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 240_000_000_000,
            command: publication::Command::Prepare(store::Manifest {
                release: store::Release {
                    name: "after".into(),
                    version: "v1".into(),
                },
                hash: store::digest(b"\0asm\x01\0\0\0"),
                bytes: 8,
                chunks: vec![store::digest(b"\0asm\x01\0\0\0")],
            }),
        },
        publication::REQUEST_LIMITS,
    )
    .await
    .unwrap();
    quiescence_tests::child_stage(&root).await.unwrap();
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
