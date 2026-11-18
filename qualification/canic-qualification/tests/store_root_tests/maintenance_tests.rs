//! Real Root-owned Store replacement, including callback loss and explicit activation.
use super::{
    Client, artifacts, inject, maintain, quiescence_tests, readiness_tests, recover_store, store,
    support,
};
use canic_management_protocol::{
    self as management, publication,
    store_maintenance::{self as wire, Approval, Phase, Status},
};
use ic_testkit::pocket_ic::{CanisterId, CanisterSettings};
use std::time::Duration;

pub(super) async fn observe(root: &Client<'_>) -> Status {
    let retained = root
        .update::<_, Option<Status>>(
            "observe_store_maintenance",
            wire::OBSERVE,
            &(),
            wire::LIMITS,
        )
        .await
        .unwrap();
    let reply = root
        .pic
        .query_call(
            root.id,
            CanisterId::anonymous(),
            "store_maintenance_status",
            ic_wire::encode(
                &(),
                ic_wire::Encoding::Frame {
                    method: wire::STATUS,
                    direction: ic_wire::Direction::Request,
                },
                wire::LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        ic_wire::decode::<Option<Status>>(
            &reply,
            ic_wire::Encoding::Frame {
                method: wire::STATUS,
                direction: ic_wire::Direction::Response
            },
            wire::LIMITS
        )
        .unwrap(),
        Some(retained.clone())
    );
    let complete: management::custody::Root = root
        .update(
            "observe_root_custody",
            management::custody::ROOT,
            &(),
            management::custody::ROOT_LIMITS,
        )
        .await;
    assert_eq!(complete.store_operation.as_ref(), Some(&retained));
    retained
}
pub(super) async fn advance(root: &Client<'_>, approval: &Approval) -> Status {
    root.update(
        "advance_store_maintenance",
        wire::ADVANCE,
        approval,
        wire::LIMITS,
    )
    .await
}
pub(super) async fn activate(root: &Client<'_>, approval: &Approval) -> Status {
    root.update(
        "activate_store_maintenance",
        wire::ACTIVATE,
        approval,
        wire::LIMITS,
    )
    .await
}
pub(super) async fn install(
    root: &Client<'_>,
    input: &wire::Install,
) -> Result<Status, management::Refusal> {
    root.update(
        "install_store_maintenance",
        wire::INSTALL,
        input,
        wire::INSTALL_LIMITS,
    )
    .await
}
pub(super) async fn stopped(
    root: &Client<'_>,
    supplier: &Client<'_>,
    replacement: &[u8],
) -> Approval {
    // Capture the installed predecessor while the fixture operator still controls
    // Store; it need not be the executable built by the current checkout.
    let previous_module = root
        .pic
        .canister_status(supplier.id, Some(root.operator))
        .await
        .unwrap()
        .module_hash
        .unwrap()
        .try_into()
        .unwrap();
    root.pic
        .update_canister_settings(
            supplier.id,
            Some(root.operator),
            CanisterSettings {
                controllers: Some(vec![root.id]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let mut approval = quiescence_tests::approval(root, supplier).await;
    approval.previous_module = previous_module;
    approval.module_hash = store::digest(replacement);
    approval.expires_at_ns =
        root.pic.get_time().await.as_nanos_since_unix_epoch() + 1_800_000_000_000;
    root.update::<_, Result<Status, management::Refusal>>(
        "stage_store_maintenance",
        wire::STAGE,
        &approval,
        wire::LIMITS,
    )
    .await
    .unwrap();
    for _ in 0..16 {
        if advance(root, &approval).await.phase == Phase::Stopped {
            return approval;
        }
    }
    panic!("bounded closure/scan/stop: {:?}", observe(root).await);
}
async fn lost_advance(root: &Client<'_>, approval: &Approval, point: u8, phase: Phase) {
    inject(root, point).await;
    assert!(
        root.pic
            .update_call(
                root.id,
                root.operator,
                "advance_store_maintenance",
                ic_wire::encode(
                    approval,
                    ic_wire::Encoding::Frame {
                        method: wire::ADVANCE,
                        direction: ic_wire::Direction::Request
                    },
                    wire::LIMITS
                )
                .unwrap()
            )
            .await
            .is_err()
    );
    inject(root, 0).await;
    assert_eq!(observe(root).await.phase, phase);
}
async fn restore_faults(root: &Client<'_>) {
    canic_qualification::upgrade_stopped(
        root.pic,
        root.id,
        std::fs::read(artifacts().join("root-faults.wasm")).unwrap(),
        vec![],
        Some(root.operator),
    )
    .await
    .unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn root_replaces_store_preserves_all_stable_bytes_and_recovers_each_callback() {
    replacement_preserves_state(
        std::fs::read(artifacts().join("original.wasm")).unwrap(),
        false,
    )
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn refused_pre_registry_root_upgrade_preserves_original_upload_and_store() {
    replacement_preserves_state(
        include_bytes!("../../fixtures/store-0.1.0.wasm").to_vec(),
        true,
    )
    .await;
}

async fn replacement_preserves_state(store_wasm: Vec<u8>, historical_scan: bool) {
    let previous_module = store::digest(&store_wasm);
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let (root, supplier) = readiness_tests::fixture_with_code(
        pic,
        if historical_scan {
            include_bytes!("../../fixtures/root-release-scan.wasm").to_vec()
        } else {
            std::fs::read(artifacts().join("root-faults.wasm")).unwrap()
        },
        store_wasm,
    )
    .await;
    let mut payload = vec![42; store::CHUNK_BYTES + 17];
    payload[..8].copy_from_slice(b"\0asm\x01\0\0\0");
    let first = supplier.publish_bytes("first", &payload).await;
    let alias = store::Manifest {
        release: store::Release {
            version: "alias".into(),
            ..first.release.clone()
        },
        ..first.clone()
    };
    supplier
        .update::<_, Result<store::Status, store::Error>>(
            "prepare",
            store::PREPARE,
            &alias,
            store::LIMITS,
        )
        .await
        .unwrap();
    supplier
        .pin(&store::Pin {
            operation: [81; 32],
            manifest: first.clone(),
        })
        .await;
    supplier
        .pin(&store::Pin {
            operation: [82; 32],
            manifest: alias,
        })
        .await;
    let pending = store::Manifest {
        release: store::Release {
            name: "pending".into(),
            version: "v1".into(),
        },
        bytes: 8,
        hash: store::digest(b"\0asm\x01\0\0\0"),
        chunks: vec![store::digest(b"\0asm\x01\0\0\0")],
    };
    supplier
        .update::<_, Result<store::Status, store::Error>>(
            "prepare",
            store::PREPARE,
            &pending,
            store::LIMITS,
        )
        .await
        .unwrap();
    let replacement = std::fs::read(artifacts().join("candidate.wasm")).unwrap();
    let approval = stopped(&root, &supplier, &replacement).await;
    assert_eq!(approval.previous_module, previous_module);
    let closed = observe(&root).await;
    assert_eq!(recover_store(&root, &approval).await, closed);
    let before_store = pic.get_stable_memory(supplier.id).await;
    let before_root = pic.get_stable_memory(root.id).await.len();
    let input = wire::Install {
        approval: approval.clone(),
        wasm: replacement,
    };
    assert_eq!(activate(&root, &approval).await, closed);
    inject(&root, 15).await;
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
    assert_eq!(observe(&root).await.phase, Phase::UploadIssued);
    let pending_upload = observe(&root).await;
    if historical_scan {
        super::refuse_old_backing(&root).await;
        assert_eq!(observe(&root).await, pending_upload);
        assert_eq!(pic.get_stable_memory(supplier.id).await, before_store);
        assert_eq!(
            owner.close(Duration::from_secs(10)).await,
            ic_testkit::server::Deletion::Attempted
        );
        assert!(server.stop().unwrap().process.cleanup.is_empty());
        return;
    }
    maintain(&root).await;
    assert_eq!(observe(&root).await, pending_upload); // Preserve the original upload commitment.
    restore_faults(&root).await;
    let (first, second) = tokio::join!(
        recover_store(&root, &approval),
        recover_store(&root, &approval)
    );
    assert_eq!(first.phase, Phase::Uploaded);
    assert_eq!(second.phase, Phase::Uploaded);
    let uploaded = observe(&root).await;
    assert_eq!(recover_store(&root, &approval).await, uploaded); // Recovery never starts installation.
    lost_advance(&root, &approval, 19, Phase::Uploaded).await;
    assert_eq!(observe(&root).await, uploaded); // Dispatch trap commits neither install intent nor effect.
    assert_eq!(pic.get_stable_memory(supplier.id).await, before_store);
    lost_advance(&root, &approval, 16, Phase::InstallIssued).await;
    let issued = observe(&root).await;
    assert!(issued.installation.as_ref().unwrap().request.is_some());
    maintain(&root).await;
    assert_eq!(observe(&root).await, issued);
    assert_eq!(install(&root, &input).await.unwrap(), issued); // No second upload or installation.
    let installed = recover_store(&root, &approval).await;
    assert_eq!(installed.phase, Phase::Installed);
    assert_eq!(recover_store(&root, &approval).await, installed); // No fresh start while authority is live.
    restore_faults(&root).await;
    lost_advance(&root, &approval, 17, Phase::StartIssued).await;
    assert_eq!(
        recover_store(&root, &approval).await.phase,
        Phase::Verifying
    );
    for _ in 0..8 {
        if recover_store(&root, &approval).await.phase == Phase::Verified {
            break;
        }
    }
    let verified = observe(&root).await;
    assert_eq!(verified.phase, Phase::Verified);
    assert_eq!(
        verified.installation.as_ref().unwrap().verification,
        verified.preservation
    );
    assert_eq!(advance(&root, &approval).await, verified); // Advance never implicitly activates.
    assert_eq!(recover_store(&root, &approval).await, verified);
    assert_eq!(pic.get_stable_memory(supplier.id).await, before_store); // Includes all chunks, pins and pending upload.
    assert_eq!(pic.get_stable_memory(root.id).await.len(), before_root); // No stable Wasm allocation.
    let raw = publication::AdmissionRequest {
        store: supplier.id.to_text(),
        expires_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 240_000_000_000,
        command: store::AdmissionCommand::Activate(approval.attempt.clone()),
    };
    assert_eq!(
        root.update::<_, publication::AdmissionReply>(
            "store_admission",
            publication::ADMISSION,
            &raw,
            store::LIMITS
        )
        .await,
        Err(publication::Error::Maintenance)
    );
    inject(&root, 18).await;
    assert!(
        pic.update_call(
            root.id,
            root.operator,
            "activate_store_maintenance",
            ic_wire::encode(
                &approval,
                ic_wire::Encoding::Frame {
                    method: wire::ACTIVATE,
                    direction: ic_wire::Direction::Request
                },
                wire::LIMITS
            )
            .unwrap()
        )
        .await
        .is_err()
    );
    inject(&root, 0).await;
    let activating = observe(&root).await;
    assert_eq!(activating.phase, Phase::ActivateIssued);
    maintain(&root).await;
    assert_eq!(observe(&root).await, activating);
    pic.advance_time(Duration::from_secs(3601)).await;
    let ready = recover_store(&root, &approval).await;
    assert_eq!(ready.phase, Phase::Ready); // Read-only recovery may finish after dispatch expiry.
    assert_eq!(advance(&root, &approval).await, ready);
    assert_eq!(activate(&root, &approval).await, ready);
    assert_eq!(install(&root, &input).await.unwrap(), ready);
    assert_eq!(recover_store(&root, &approval).await, ready);
    let expires = pic.get_time().await.as_nanos_since_unix_epoch() + 240_000_000_000;
    for command in [
        publication::Command::Put(store::Put {
            release: pending.release.clone(),
            index: 0,
            bytes: b"\0asm\x01\0\0\0".to_vec(),
        }),
        publication::Command::Publish(pending.release),
    ] {
        root.update::<_, publication::Reply>(
            "store_publication",
            publication::EXECUTE,
            &publication::Request {
                store: supplier.id.to_text(),
                expires_at_ns: expires,
                command,
            },
            publication::REQUEST_LIMITS,
        )
        .await
        .unwrap();
    }
    quiescence_tests::child_stage(&root).await.unwrap(); // Completion releases the mutation fence.
    maintain(&root).await; // Later child work does not invalidate completed maintenance custody.
    assert_eq!(observe(&root).await, ready);
    assert!(
        pic.update_call(
            root.id,
            root.operator,
            "observe_stored_children",
            ic_wire::encode(
                &(),
                ic_wire::Encoding::Frame {
                    method: management::stored::CUSTODY,
                    direction: ic_wire::Direction::Request
                },
                management::stored::LIMITS
            )
            .unwrap()
        )
        .await
        .is_err()
    );
    let next = store::MaintenanceAttempt {
        sequence: 2,
        operation: [91; 32],
    };
    let mut raw = publication::AdmissionRequest {
        store: supplier.id.to_text(),
        expires_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 240_000_000_000,
        command: store::AdmissionCommand::Close(next.clone()),
    };
    let closed = root
        .update::<_, publication::AdmissionReply>(
            "store_admission",
            publication::ADMISSION,
            &raw,
            store::LIMITS,
        )
        .await
        .unwrap()
        .unwrap();
    assert!(closed.closed);
    assert_eq!(closed.attempt, next);
    assert_eq!(activate(&root, &approval).await, ready); // A completed old operation cannot reopen a newer gate.
    assert_eq!(recover_store(&root, &approval).await, ready);
    raw.command = store::AdmissionCommand::Observe;
    assert_eq!(
        root.update::<_, publication::AdmissionReply>(
            "store_admission",
            publication::ADMISSION,
            &raw,
            store::LIMITS
        )
        .await
        .unwrap(),
        Some(closed)
    );
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

#[tokio::test(flavor = "current_thread")]
async fn store_install_refuses_wrong_bytes_callers_frames_and_expired_dispatch() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let (root, supplier) = readiness_tests::fixture(pic).await;
    let wasm = std::fs::read(artifacts().join("candidate.wasm")).unwrap();
    let approval = stopped(&root, &supplier, &wasm).await;
    let input = wire::Install {
        approval: approval.clone(),
        wasm,
    };
    let before = observe(&root).await;
    let mut foreign = approval.clone();
    foreign.attempt.operation[0] ^= 1;
    for (caller, frame) in [
        (
            CanisterId::anonymous(),
            ic_wire::encode(
                &approval,
                ic_wire::Encoding::Frame {
                    method: wire::RECOVER,
                    direction: ic_wire::Direction::Request,
                },
                wire::LIMITS,
            )
            .unwrap(),
        ),
        (
            root.operator,
            ic_wire::encode(
                &foreign,
                ic_wire::Encoding::Frame {
                    method: wire::RECOVER,
                    direction: ic_wire::Direction::Request,
                },
                wire::LIMITS,
            )
            .unwrap(),
        ),
        (root.operator, vec![0, 1, 2]),
    ] {
        assert!(
            pic.update_call(root.id, caller, "recover_store_maintenance", frame)
                .await
                .is_err()
        );
    }
    assert!(
        pic.query_call(
            root.id,
            CanisterId::anonymous(),
            "store_maintenance_status",
            vec![0, 1, 2]
        )
        .await
        .is_err()
    );
    assert!(
        pic.update_call(
            root.id,
            CanisterId::anonymous(),
            "observe_store_maintenance",
            ic_wire::encode(
                &(),
                ic_wire::Encoding::Frame {
                    method: wire::OBSERVE,
                    direction: ic_wire::Direction::Request
                },
                wire::LIMITS
            )
            .unwrap()
        )
        .await
        .is_err()
    );
    assert_eq!(observe(&root).await, before);
    for (caller, bytes) in [
        (
            CanisterId::anonymous(),
            ic_wire::encode(
                &input,
                ic_wire::Encoding::Frame {
                    method: wire::INSTALL,
                    direction: ic_wire::Direction::Request,
                },
                wire::INSTALL_LIMITS,
            )
            .unwrap(),
        ),
        (root.operator, vec![0, 1, 2]),
    ] {
        assert!(
            pic.update_call(root.id, caller, "install_store_maintenance", bytes)
                .await
                .is_err()
        );
    }
    let wrong = wire::Install {
        approval: approval.clone(),
        wasm: b"\0asm\x01\0\0\0".to_vec(),
    };
    assert_eq!(
        install(&root, &wrong).await,
        Err(management::Refusal::Artifact)
    );
    assert_eq!(observe(&root).await, before);
    let store_before = pic.get_stable_memory(supplier.id).await;
    pic.advance_time(Duration::from_secs(3601)).await;
    let expired = install(&root, &input).await.unwrap();
    assert_eq!(expired.phase, Phase::Stopped);
    assert!(expired.failure.is_some());
    assert!(expired.installation.is_none());
    assert_eq!(pic.get_stable_memory(supplier.id).await, store_before);
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

#[tokio::test(flavor = "current_thread")]
async fn installed_store_outcome_recovers_after_expiry_without_fresh_start_authority() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let (root, supplier) = readiness_tests::fixture(pic).await;
    let wasm = std::fs::read(artifacts().join("candidate.wasm")).unwrap();
    let approval = stopped(&root, &supplier, &wasm).await;
    let input = wire::Install {
        approval: approval.clone(),
        wasm,
    };
    assert_eq!(install(&root, &input).await.unwrap().phase, Phase::Uploaded);
    lost_advance(&root, &approval, 16, Phase::InstallIssued).await;
    let issued = observe(&root).await;
    maintain(&root).await;
    pic.advance_time(Duration::from_secs(3601)).await;
    let installed = recover_store(&root, &approval).await;
    assert_eq!(installed.phase, Phase::Installed);
    assert_eq!(installed.installation, issued.installation);
    assert_eq!(recover_store(&root, &approval).await, installed);
    let blocked = advance(&root, &approval).await;
    assert_eq!(blocked.phase, Phase::Installed);
    assert!(blocked.failure.is_some());
    assert!(
        blocked
            .installation
            .as_ref()
            .unwrap()
            .start_request
            .is_none()
    );
    assert_eq!(install(&root, &input).await.unwrap(), blocked);
    assert_eq!(activate(&root, &approval).await, blocked);
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

#[tokio::test(flavor = "current_thread")]
async fn rejected_or_incompatible_store_replacement_never_activates_or_reissues() {
    let (_, binary) = support::inputs();
    for rejected in [true, false] {
        let (mut server, owner) = support::instance(&binary).await;
        let pic = owner.client();
        let (root, supplier) = readiness_tests::fixture(pic).await;
        let mut wasm = b"\0asm\x01\0\0\0".to_vec();
        if rejected {
            wasm.push(255);
        }
        let approval = stopped(&root, &supplier, &wasm).await;
        let input = wire::Install {
            approval: approval.clone(),
            wasm,
        };
        assert_eq!(install(&root, &input).await.unwrap().phase, Phase::Uploaded);
        assert_eq!(advance(&root, &approval).await.phase, Phase::InstallIssued);
        if !rejected {
            assert_eq!(advance(&root, &approval).await.phase, Phase::Installed);
            assert_eq!(advance(&root, &approval).await.phase, Phase::StartIssued);
        }
        let blocked = advance(&root, &approval).await;
        assert_eq!(
            blocked.phase,
            if rejected {
                Phase::InstallIssued
            } else {
                Phase::StartIssued
            }
        );
        assert!(blocked.failure.is_some());
        assert_eq!(install(&root, &input).await.unwrap(), blocked);
        assert_eq!(activate(&root, &approval).await, blocked);
        assert!(
            blocked
                .installation
                .as_ref()
                .unwrap()
                .activation_request
                .is_none()
        );
        assert_eq!(
            owner.close(std::time::Duration::from_secs(10)).await,
            ic_testkit::server::Deletion::Attempted
        );
        let stopped = server.stop().unwrap();
        assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
    }
}
