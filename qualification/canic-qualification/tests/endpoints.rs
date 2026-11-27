//! Installed management adapter contracts and reply-failure transaction boundaries.
use canic_management_protocol::application::{
    RECONCILE_CHILD_CREATION as RECONCILE, STAGE_CHILD_CREATION as STAGE, STAGE_CHILD_UPGRADE,
};
use canic_management_protocol::{
    Approval, CREATION_CYCLES, INJECT, LIMITS, Phase, Refusal, STATUS, Status, children, digest,
    upgrade,
};
#[path = "../fixtures/child_module.rs"]
mod child_module;
#[path = "../fixtures/child_store.rs"]
mod child_store;
#[path = "../fixtures/local_store.rs"]
mod local_store;
use child_module::Stage;
use ic_testkit::{
    pocket_ic::{CanisterId, PocketIcBuilder, RejectCode, RejectResponse, nonblocking::PocketIc},
    server::{Limits, PreparedServer, Server},
};
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
        let root_status = async |slot: children::ChildId| {
            let encoding = |direction| ic_wire::Encoding::Frame {
                method: children::READ_STATUS,
                direction,
            };
            let request = ic_wire::encode(
                &children::StatusRequest::Creation(slot),
                encoding(ic_wire::Direction::Request),
                LIMITS,
            )
            .unwrap();
            let reply = pic
                .query_call(target, outsider, "child_status", request)
                .await
                .unwrap();
            let children::ObservedStatus::Creation(status) =
                ic_wire::decode(&reply, encoding(ic_wire::Direction::Response), LIMITS).unwrap()
            else {
                panic!("expected creation status")
            };
            status
        };
        if variant == "root" {
            for method in [
                "store_admission",
                "store_removal",
                "observe_child",
                "observe_child_upgrade",
                "stage_inline_child_creation",
                "stage_inline_child_upgrade",
                "reconcile_inline_child_creation",
                "reconcile_inline_child_upgrade",
            ] {
                let error = pic
                    .update_call(target, controller, method, vec![])
                    .await
                    .unwrap_err();
                assert_eq!(
                    error.error_code,
                    ic_testkit::pocket_ic::ErrorCode::CanisterMethodNotFound,
                    "{error:?}"
                );
            }
        }
        let initial = if variant == "root" {
            root_status(children::ChildId::FIRST).await
        } else {
            status(pic, target).await
        };
        assert_eq!(initial.phase, Phase::Empty);
        // Export name, mode, access, frame ID and request policy are deliberately
        // specified independently here: this tests the mounted public contract.
        let mut methods = if variant == "root" {
            vec![
                ("child_status", true, false, children::READ_STATUS, LIMITS),
                (
                    "observe_child_status",
                    false,
                    true,
                    children::OBSERVE_STATUS,
                    LIMITS,
                ),
                (
                    "observe_root_custody",
                    false,
                    true,
                    canic_management_protocol::application::CUSTODY,
                    LIMITS,
                ),
                (
                    "stage_child_creation",
                    false,
                    true,
                    canic_management_protocol::application::STAGE_CHILD_CREATION,
                    canic_management_protocol::stored::LIMITS,
                ),
                (
                    "reconcile_child_creation",
                    false,
                    true,
                    canic_management_protocol::application::RECONCILE_CHILD_CREATION,
                    canic_management_protocol::stored::LIMITS,
                ),
                (
                    "stage_child_upgrade",
                    false,
                    true,
                    canic_management_protocol::application::STAGE_CHILD_UPGRADE,
                    canic_management_protocol::stored::LIMITS,
                ),
                (
                    "reconcile_child_upgrade",
                    false,
                    true,
                    canic_management_protocol::application::RECONCILE_CHILD_UPGRADE,
                    canic_management_protocol::stored::LIMITS,
                ),
            ]
        } else {
            vec![
                ("status", true, false, STATUS, LIMITS),
                ("observe", false, true, STATUS, LIMITS),
                (
                    "observe_custody",
                    false,
                    true,
                    canic_management_protocol::custody::OBSERVE,
                    LIMITS,
                ),
                (
                    "stage_child_creation",
                    false,
                    true,
                    STAGE,
                    canic_management_protocol::stored::LIMITS,
                ),
                (
                    "reconcile_child_creation",
                    false,
                    true,
                    RECONCILE,
                    canic_management_protocol::stored::LIMITS,
                ),
                (
                    "stage_child_upgrade",
                    false,
                    true,
                    STAGE_CHILD_UPGRADE,
                    canic_management_protocol::stored::LIMITS,
                ),
                ("upgrade_status", true, false, upgrade::STATUS, LIMITS),
                ("observe_upgrade", false, true, upgrade::OBSERVE, LIMITS),
                (
                    "reconcile_child_upgrade",
                    false,
                    true,
                    canic_management_protocol::application::RECONCILE_CHILD_UPGRADE,
                    canic_management_protocol::stored::LIMITS,
                ),
            ]
        };
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
                            "management frame too large"
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
        if variant == "root" {
            // Root requires an explicit slot. The children suite exercises staging,
            // reconciliation and matching approvals across multiple installed slots.
            for slot in [children::ChildId::FIRST, children::ChildId::SECOND] {
                assert_eq!(root_status(slot).await, initial);
                let frame = |method, direction| ic_wire::Encoding::Frame { method, direction };
                let request = ic_wire::encode(
                    &children::ObserveStatus::Creation(slot),
                    frame(children::OBSERVE_STATUS, ic_wire::Direction::Request),
                    LIMITS,
                )
                .unwrap();
                let reply = pic
                    .update_call(target, controller, "observe_child_status", request)
                    .await
                    .unwrap();
                let observed = ic_wire::decode(
                    &reply,
                    frame(children::OBSERVE_STATUS, ic_wire::Direction::Response),
                    LIMITS,
                )
                .unwrap();
                assert!(
                    matches!(observed, children::ObservedStatus::Creation(status) if status == initial)
                );
                let old_frame = ic_wire::encode(
                    &slot,
                    frame(children::STATUS, ic_wire::Direction::Request),
                    LIMITS,
                )
                .unwrap();
                assert!(
                    pic.query_call(target, outsider, "child_status", old_frame.clone())
                        .await
                        .is_err()
                );
                assert!(
                    pic.update_call(target, controller, "observe_child_status", old_frame)
                        .await
                        .is_err()
                );
            }
            continue;
        }
        assert_eq!(status(pic, target).await, initial);
        let store = child_store::ChildStore::install(
            pic,
            target,
            std::fs::read(artifacts.join("store.wasm")).unwrap(),
        )
        .await;
        let now = pic.get_time().await.as_nanos_since_unix_epoch();
        let mut invalid = stage_input(target, &worker, now);
        invalid.approval.module_hash = [0; 32];
        let frame = store
            .frame(
                STAGE,
                children::ChildId::FIRST,
                &invalid.approval,
                &invalid.wasm,
            )
            .await;
        let reply = pic
            .update_call(target, controller, "stage_child_creation", frame)
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
        let invalid_upgrade = Stage {
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
                "stage_child_upgrade",
                store
                    .frame(
                        STAGE_CHILD_UPGRADE,
                        children::ChildId::FIRST,
                        &invalid_upgrade.approval,
                        &invalid_upgrade.wasm,
                    )
                    .await,
            )
            .await
            .unwrap();
        let refusal: Result<upgrade::Status, Refusal> = ic_wire::decode(
            &reply,
            ic_wire::Encoding::Frame {
                method: STAGE_CHILD_UPGRADE,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap();
        assert_eq!(refusal, Err(Refusal::Binding));
        assert_eq!(status(pic, target).await, initial);
    }

    let target = fault_target.unwrap();
    let store = child_store::ChildStore::install(
        pic,
        target,
        std::fs::read(artifacts.join("store.wasm")).unwrap(),
    )
    .await;
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
            "stage_child_creation",
            store
                .frame(
                    STAGE,
                    children::ChildId::FIRST,
                    &input.approval,
                    &input.wasm,
                )
                .await,
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
            "stage_child_creation",
            store
                .frame(
                    STAGE,
                    children::ChildId::FIRST,
                    &input.approval,
                    &input.wasm,
                )
                .await,
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
        store
            .frame(RECONCILE, children::ChildId::FIRST, &other, &input.wasm)
            .await,
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
            pic.update_call(target, controller, "reconcile_child_creation", frame)
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
            "reconcile_child_creation",
            store
                .frame(
                    RECONCILE,
                    children::ChildId::FIRST,
                    &input.approval,
                    &input.wasm,
                )
                .await,
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
    let candidate = std::fs::read(artifacts.join("maintenance.wasm")).unwrap();
    let candidate_hash = digest(&candidate).to_vec();
    canic_qualification::upgrade_stopped(pic, target, candidate, vec![], None)
        .await
        .unwrap();
    assert_eq!(
        pic.canister_status(target, None).await.unwrap().module_hash,
        Some(candidate_hash)
    );
    let maintained = ready;
    assert_eq!(status(pic, target).await, maintained);
    let reply = pic
        .update_call(
            target,
            controller,
            "reconcile_child_creation",
            store
                .frame(
                    RECONCILE,
                    children::ChildId::FIRST,
                    &input.approval,
                    &input.wasm,
                )
                .await,
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
            "stage_child_creation",
            store
                .frame(
                    STAGE,
                    children::ChildId::FIRST,
                    &input.approval,
                    &input.wasm,
                )
                .await,
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
