//! Installed management adapter contracts and reply-failure transaction boundaries.
use canic_management_protocol::{
    Approval, CREATION_CYCLES, INJECT, LIMITS, Phase, RECONCILE, Refusal, STAGE, STAGE_LIMITS,
    STATUS, Stage, Status, digest, upgrade,
};
use ic_testkit::pocket_ic::{
    CanisterId, PocketIcBuilder, RejectCode, RejectResponse, nonblocking::PocketIc,
};
use ic_testkit::server::{Limits, PreparedServer, Server};
use std::{path::PathBuf, time::Duration};

async fn call(
    pic: &PocketIc,
    target: CanisterId,
    caller: CanisterId,
    method: &str,
    query: bool,
    bytes: Vec<u8>,
) -> Result<Vec<u8>, RejectResponse> {
    if query {
        pic.query_call(target, caller, method, bytes).await
    } else {
        pic.update_call(target, caller, method, bytes).await
    }
}

async fn status(pic: &PocketIc, target: CanisterId) -> Status {
    let reply = pic
        .query_call(
            target,
            CanisterId::anonymous(),
            "status",
            ic_wire::encode(
                &(),
                ic_wire::Encoding::Frame {
                    method: STATUS,
                    direction: ic_wire::Direction::Request,
                },
                LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: STATUS,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
}

async fn inject(pic: &PocketIc, target: CanisterId, point: u8) {
    let reply = pic
        .update_call(
            target,
            CanisterId::anonymous(),
            "inject_fault",
            ic_wire::encode(
                &point,
                ic_wire::Encoding::Frame {
                    method: INJECT,
                    direction: ic_wire::Direction::Request,
                },
                LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    ic_wire::decode::<()>(
        &reply,
        ic_wire::Encoding::Frame {
            method: INJECT,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap();
}

fn stage_input(target: CanisterId, wasm: &[u8], now: u64) -> Stage {
    Stage {
        approval: Approval {
            manager: target.to_text(),
            module_hash: digest(wasm),
            creation_cycles: CREATION_CYCLES,
            execute_at_ns: now + 10_000_000_000,
            expires_at_ns: now + 120_000_000_000,
        },
        wasm: wasm.to_vec(),
    }
}

async fn settle(pic: &PocketIc) {
    for _ in 0..30 {
        pic.advance_time(Duration::from_millis(100)).await;
        pic.tick().await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn management_mounts_preserve_contracts_authority_and_reply_rollback() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();
    let artifacts = repo.join("target/endpoints");
    let binary = PreparedServer::check(&canic_qualification::server_directory())
        .expect("make qualification-setup");
    let mut server = Server::start(
        binary,
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(180),
            stdout_bytes: 1024 * 1024,
            stderr_bytes: 1024 * 1024,
        },
    )
    .unwrap();
    let owner = server
        .instance(
            PocketIcBuilder::new().with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    let pic = owner.client();
    let controller = CanisterId::anonymous();
    let outsider = CanisterId::self_authenticating(b"endpoint-outsider");
    let worker = std::fs::read(artifacts.join("worker.wasm")).unwrap();
    let mut fault_target = None;

    for variant in ["root", "manager", "faults"] {
        let target = pic.create_canister().await;
        pic.install_canister(
            target,
            std::fs::read(artifacts.join(format!("{variant}.wasm"))).unwrap(),
            vec![],
            None,
        )
        .await;
        let initial = status(pic, target).await;
        assert_eq!(initial.phase, Phase::Empty);
        // Export name, mode, access, frame ID and request policy are deliberately
        // specified independently here: this tests the mounted public contract.
        let mut methods = vec![
            ("status", true, false, STATUS, LIMITS),
            ("observe", false, true, STATUS, LIMITS),
            (
                "observe_custody",
                false,
                true,
                canic_management_protocol::custody::OBSERVE,
                LIMITS,
            ),
            ("stage", false, true, STAGE, STAGE_LIMITS),
            ("reconcile", false, true, RECONCILE, LIMITS),
            ("stage_upgrade", false, true, upgrade::STAGE, STAGE_LIMITS),
            ("upgrade_status", true, false, upgrade::STATUS, LIMITS),
            ("observe_upgrade", false, true, upgrade::OBSERVE, LIMITS),
            ("reconcile_upgrade", false, true, upgrade::RECONCILE, LIMITS),
        ];
        if variant == "faults" {
            methods.push(("inject_fault", false, true, INJECT, LIMITS));
            fault_target = Some(target);
        } else {
            let absent = pic
                .update_call(target, controller, "inject_fault", vec![])
                .await
                .unwrap_err();
            assert!(absent.reject_message.contains("inject_fault"));
        }
        for (method, query, private, id, limits) in methods {
            for frame in [
                vec![],
                ic_wire::encode(
                    &(),
                    ic_wire::Encoding::Frame {
                        method: u64::MAX,
                        direction: ic_wire::Direction::Request,
                    },
                    LIMITS,
                )
                .unwrap(),
                ic_wire::encode(
                    &(),
                    ic_wire::Encoding::Frame {
                        method: id,
                        direction: ic_wire::Direction::Response,
                    },
                    LIMITS,
                )
                .unwrap(),
                vec![0; limits.bytes],
            ] {
                if private {
                    let denied = call(pic, target, outsider, method, query, frame.clone())
                        .await
                        .unwrap_err();
                    assert_eq!(
                        denied.reject_code,
                        RejectCode::CanisterError,
                        "{variant}/{method}"
                    );
                    assert!(denied.reject_message.contains("controller required"));
                }
                let malformed = call(pic, target, controller, method, query, frame)
                    .await
                    .unwrap_err();
                assert_eq!(
                    malformed.reject_code,
                    RejectCode::CanisterError,
                    "{variant}/{method}: {malformed:?}"
                );
            }
            for caller in [controller, outsider] {
                let oversized = call(
                    pic,
                    target,
                    caller,
                    method,
                    query,
                    vec![0; limits.bytes + 1],
                )
                .await
                .unwrap_err();
                assert_eq!(oversized.reject_code, RejectCode::CanisterError);
                assert!(
                    oversized
                        .reject_message
                        .contains(if private && caller == outsider {
                            "controller required"
                        } else {
                            "growth frame too large"
                        })
                );
            }
            // Creation shares an ID; upgrade observations bind an exact approval.
            let reader = if private { controller } else { outsider };
            if id == canic_management_protocol::custody::OBSERVE {
                let reply = call(
                    pic,
                    target,
                    reader,
                    method,
                    query,
                    ic_wire::encode(
                        &(),
                        ic_wire::Encoding::Frame {
                            method: id,
                            direction: ic_wire::Direction::Request,
                        },
                        limits,
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
                let value: canic_management_protocol::custody::Custody = ic_wire::decode(
                    &reply,
                    ic_wire::Encoding::Frame {
                        method: id,
                        direction: ic_wire::Direction::Response,
                    },
                    canic_management_protocol::custody::LIMITS,
                )
                .unwrap();
                assert_eq!(value.creation, Status::default());
                assert!(value.upgrades.is_empty());
            } else if id == STATUS {
                let reply = call(
                    pic,
                    target,
                    reader,
                    method,
                    query,
                    ic_wire::encode(
                        &(),
                        ic_wire::Encoding::Frame {
                            method: id,
                            direction: ic_wire::Direction::Request,
                        },
                        limits,
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
                let value: Status = ic_wire::decode(
                    &reply,
                    ic_wire::Encoding::Frame {
                        method: id,
                        direction: ic_wire::Direction::Response,
                    },
                    LIMITS,
                )
                .unwrap();
                assert_eq!(value, initial);
            } else if id == upgrade::STATUS {
                let reply = call(
                    pic,
                    target,
                    reader,
                    method,
                    query,
                    ic_wire::encode(
                        &(),
                        ic_wire::Encoding::Frame {
                            method: id,
                            direction: ic_wire::Direction::Request,
                        },
                        limits,
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
                let value: upgrade::Status = ic_wire::decode(
                    &reply,
                    ic_wire::Encoding::Frame {
                        method: id,
                        direction: ic_wire::Direction::Response,
                    },
                    LIMITS,
                )
                .unwrap();
                assert_eq!(value, upgrade::Status::default());
            } else if id == upgrade::OBSERVE {
                let expected = upgrade::Approval {
                    manager: target.to_text(),
                    child: controller.to_text(),
                    previous_module: [1; 32],
                    module_hash: [2; 32],
                    execute_at_ns: 1,
                    expires_at_ns: 2,
                };
                let reply = call(
                    pic,
                    target,
                    reader,
                    method,
                    query,
                    ic_wire::encode(
                        &expected,
                        ic_wire::Encoding::Frame {
                            method: id,
                            direction: ic_wire::Direction::Request,
                        },
                        limits,
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
                assert_eq!(
                    ic_wire::decode::<upgrade::Status>(
                        &reply,
                        ic_wire::Encoding::Frame {
                            method: id,
                            direction: ic_wire::Direction::Response
                        },
                        LIMITS
                    )
                    .unwrap(),
                    upgrade::Status::default()
                );
                // A legacy unbound read cannot silently select the latest upgrade.
                assert!(
                    call(
                        pic,
                        target,
                        reader,
                        method,
                        query,
                        ic_wire::encode(
                            &(),
                            ic_wire::Encoding::Frame {
                                method: upgrade::STATUS,
                                direction: ic_wire::Direction::Request
                            },
                            limits
                        )
                        .unwrap()
                    )
                    .await
                    .is_err()
                );
            }
        }
        assert_eq!(status(pic, target).await, initial);
        let now = pic.get_time().await.as_nanos_since_unix_epoch();
        let mut invalid = stage_input(target, &worker, now);
        invalid.approval.module_hash = [0; 32];
        let frame = ic_wire::encode(
            &invalid,
            ic_wire::Encoding::Frame {
                method: STAGE,
                direction: ic_wire::Direction::Request,
            },
            STAGE_LIMITS,
        )
        .unwrap();
        assert!(
            frame.len() > LIMITS.bytes,
            "exercise the asymmetric staging budget"
        );
        let reply = pic
            .update_call(target, controller, "stage", frame)
            .await
            .unwrap();
        let refusal: Result<Status, Refusal> = ic_wire::decode(
            &reply,
            ic_wire::Encoding::Frame {
                method: STAGE,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap();
        assert_eq!(refusal, Err(Refusal::Artifact));
        let invalid_upgrade = upgrade::Stage {
            approval: upgrade::Approval {
                manager: target.to_text(),
                child: target.to_text(),
                previous_module: [0; 32],
                module_hash: digest(&worker),
                execute_at_ns: now + 10_000_000_000,
                expires_at_ns: now + 120_000_000_000,
            },
            wasm: worker.clone(),
        };
        let reply = pic
            .update_call(
                target,
                controller,
                "stage_upgrade",
                ic_wire::encode(
                    &invalid_upgrade,
                    ic_wire::Encoding::Frame {
                        method: upgrade::STAGE,
                        direction: ic_wire::Direction::Request,
                    },
                    STAGE_LIMITS,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let refusal: Result<upgrade::Status, Refusal> = ic_wire::decode(
            &reply,
            ic_wire::Encoding::Frame {
                method: upgrade::STAGE,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap();
        assert_eq!(refusal, Err(Refusal::Binding));
        assert_eq!(status(pic, target).await, initial);
    }

    let target = fault_target.unwrap();
    let subnet = pic.get_subnet(target).await.unwrap();
    let count = pic.get_subnet_metrics(subnet).await.unwrap().num_canisters;
    let original = status(pic, target).await;
    let input = stage_input(
        target,
        &worker,
        pic.get_time().await.as_nanos_since_unix_epoch(),
    );
    inject(pic, target, 9).await;
    let failed = pic
        .update_call(
            target,
            controller,
            "stage",
            ic_wire::encode(
                &input,
                ic_wire::Encoding::Frame {
                    method: STAGE,
                    direction: ic_wire::Direction::Request,
                },
                STAGE_LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(failed.reject_code, RejectCode::CanisterError);
    assert!(
        failed
            .reject_message
            .contains("management endpoint: Encode(")
    );
    assert_eq!(
        status(pic, target).await,
        original,
        "synchronous journal/artifact writes must roll back"
    );
    pic.advance_time(Duration::from_secs(11)).await;
    settle(pic).await;
    assert_eq!(
        pic.get_subnet_metrics(subnet).await.unwrap().num_canisters,
        count,
        "the failed reply must also roll back the scheduled creation"
    );

    inject(pic, target, 0).await;
    let input = stage_input(
        target,
        &worker,
        pic.get_time().await.as_nanos_since_unix_epoch(),
    );
    let reply = pic
        .update_call(
            target,
            controller,
            "stage",
            ic_wire::encode(
                &input,
                ic_wire::Encoding::Frame {
                    method: STAGE,
                    direction: ic_wire::Direction::Request,
                },
                STAGE_LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let admitted: Result<Status, Refusal> = ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: STAGE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap();
    assert_eq!(admitted.unwrap().phase, Phase::Scheduled);
    let retained = status(pic, target).await;
    let mut other = input.approval.clone();
    other.expires_at_ns += 1;
    for frame in [
        ic_wire::encode(
            &other,
            ic_wire::Encoding::Frame {
                method: RECONCILE,
                direction: ic_wire::Direction::Request,
            },
            LIMITS,
        )
        .unwrap(),
        ic_wire::encode(
            &(),
            ic_wire::Encoding::Frame {
                method: 22,
                direction: ic_wire::Direction::Request,
            },
            LIMITS,
        )
        .unwrap(),
        ic_wire::encode(
            &(),
            ic_wire::Encoding::Frame {
                method: RECONCILE,
                direction: ic_wire::Direction::Request,
            },
            LIMITS,
        )
        .unwrap(),
    ] {
        assert!(
            pic.update_call(target, controller, "reconcile", frame)
                .await
                .is_err()
        );
        assert_eq!(status(pic, target).await, retained);
    }
    pic.advance_time(Duration::from_secs(11)).await;
    settle(pic).await;
    let ready = status(pic, target).await;
    assert_eq!(ready.phase, Phase::Ready);
    assert_eq!(ready.attempts, [1; 3]);
    assert_eq!(
        pic.get_subnet_metrics(subnet).await.unwrap().num_canisters,
        count + 1
    );

    inject(pic, target, 10).await;
    // Ready reconciliation performs a real inter-canister readiness call before
    // its reply encoding fails. Earlier committed creation evidence must survive.
    let failed = pic
        .update_call(
            target,
            controller,
            "reconcile",
            ic_wire::encode(
                &input.approval,
                ic_wire::Encoding::Frame {
                    method: RECONCILE,
                    direction: ic_wire::Direction::Request,
                },
                LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap_err();
    assert_eq!(failed.reject_code, RejectCode::CanisterError);
    assert!(
        failed
            .reject_message
            .contains("management endpoint: Encode(")
    );
    assert_eq!(status(pic, target).await, ready);
    assert_eq!(
        pic.get_subnet_metrics(subnet).await.unwrap().num_canisters,
        count + 1
    );

    // An infrastructure upgrade clears volatile callback fencing but preserves
    // the committed principal and spent slot. No reinstall or fresh effect.
    canic_qualification::upgrade_stopped(
        pic,
        target,
        std::fs::read(artifacts.join("maintenance.wasm")).unwrap(),
        vec![],
        None,
    )
    .await
    .unwrap();
    let mut maintained = ready;
    maintained.maintenance_candidate = true;
    assert_eq!(status(pic, target).await, maintained);
    let reply = pic
        .update_call(
            target,
            controller,
            "reconcile",
            ic_wire::encode(
                &input.approval,
                ic_wire::Encoding::Frame {
                    method: RECONCILE,
                    direction: ic_wire::Direction::Request,
                },
                LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let resumed: Status = ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: RECONCILE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap();
    assert_eq!(resumed, maintained);
    let reply = pic
        .update_call(
            target,
            controller,
            "stage",
            ic_wire::encode(
                &input,
                ic_wire::Encoding::Frame {
                    method: STAGE,
                    direction: ic_wire::Direction::Request,
                },
                STAGE_LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let replay: Result<Status, Refusal> = ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: STAGE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap();
    assert_eq!(replay.unwrap(), maintained);
    assert_eq!(
        pic.get_subnet_metrics(subnet).await.unwrap().num_canisters,
        count + 1
    );
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
