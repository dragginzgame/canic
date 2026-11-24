//! Independent siblings, aggregate reservations and preservation in Storage records.
mod directory;
mod discovery;
mod identity;

use super::*;
use canic_management_protocol::{
    children::{self, ChildId, Request},
    custody, upgrade,
};

async fn read(pic: &PocketIc, root: CanisterId) -> children::Custody {
    let reply = pic
        .update_call(
            root,
            CanisterId::anonymous(),
            "observe_children",
            ic_wire::encode(
                &(),
                ic_wire::Encoding::Frame {
                    method: children::CUSTODY,
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
            method: children::CUSTODY,
            direction: ic_wire::Direction::Response,
        },
        children::LIMITS,
    )
    .unwrap()
}

pub(super) async fn admit(
    pic: &PocketIc,
    root: CanisterId,
    slot: ChildId,
    input: Stage,
) -> Result<Status, Refusal> {
    let reply = pic
        .update_call(
            root,
            CanisterId::anonymous(),
            "stage_child",
            ic_wire::encode(
                &Request { slot, value: input },
                ic_wire::Encoding::Frame {
                    method: children::STAGE,
                    direction: ic_wire::Direction::Request,
                },
                STAGE_LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: children::STAGE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
}

pub(super) fn input(root: CanisterId, wasm: Vec<u8>, now: u64) -> Stage {
    Stage {
        approval: Approval {
            manager: root.to_text(),
            module_hash: digest(&wasm),
            creation_cycles: CREATION_CYCLES,
            execute_at_ns: now + 10_000_000_000,
            expires_at_ns: now + 120_000_000_000,
        },
        wasm,
    }
}

pub(super) fn artifact(name: &str) -> Vec<u8> {
    std::fs::read(repo().join("target/root").join(name)).unwrap()
}

#[tokio::test]
async fn two_children_preserve_history_and_upgrade_independently() {
    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
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
    let root = pic.create_canister().await;
    // Old allocator images are qualified separately as refused upgrades.
    pic.install_canister(root, artifact("root.wasm"), vec![], None)
        .await;
    let first = input(
        root,
        padded_artifact("app.wasm"),
        pic.get_time().await.as_nanos_since_unix_epoch(),
    );
    stage(pic, root, &first).await.unwrap();
    pic.advance_time(Duration::from_secs(11)).await;
    settle(pic).await;
    let first_ready = status(pic, root).await;
    assert_eq!(first_ready.phase, Phase::Ready);
    // Populate history before reopening, including a return to the
    // original module hash. Neither completion may be mistaken for new work.
    let mut previous = first.approval.module_hash;
    for name in ["app-v2.wasm", "app.wasm"] {
        let wasm = padded_artifact(name);
        let now = pic.get_time().await.as_nanos_since_unix_epoch();
        let change = upgrade::Stage {
            approval: upgrade::Approval {
                manager: root.to_text(),
                child: first_ready.worker.clone().unwrap(),
                previous_module: previous,
                module_hash: digest(&wasm),
                execute_at_ns: now + 10_000_000_000,
                expires_at_ns: now + 120_000_000_000,
            },
            wasm,
        };
        let reply = pic
            .update_call(
                root,
                CanisterId::anonymous(),
                "stage_child_upgrade",
                ic_wire::encode(
                    &canic_management_protocol::children::Request {
                        slot: canic_management_protocol::children::ChildId::FIRST,
                        value: &change,
                    },
                    ic_wire::Encoding::Frame {
                        method: canic_management_protocol::children::STAGE_UPGRADE,
                        direction: ic_wire::Direction::Request,
                    },
                    STAGE_LIMITS,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let admitted: Result<upgrade::Status, Refusal> = ic_wire::decode(
            &reply,
            ic_wire::Encoding::Frame {
                method: canic_management_protocol::children::STAGE_UPGRADE,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap();
        admitted.unwrap();
        pic.advance_time(Duration::from_secs(11)).await;
        settle(pic).await;
        previous = change.approval.module_hash;
    }
    let reply = pic
        .update_call(
            root,
            CanisterId::anonymous(),
            "observe_custody",
            ic_wire::encode(
                &(),
                ic_wire::Encoding::Frame {
                    method: custody::OBSERVE,
                    direction: ic_wire::Direction::Request,
                },
                LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let old_custody: custody::Custody = ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: custody::OBSERVE,
            direction: ic_wire::Direction::Response,
        },
        custody::LIMITS,
    )
    .unwrap();
    assert_eq!(old_custody.upgrades.len(), 2);
    // Latest and historical reads must still distinguish two completed approvals.
    for (selector, expected) in [
        (None, &old_custody.upgrades[1]),
        (
            old_custody.upgrades[0].approval.as_ref(),
            &old_custody.upgrades[0],
        ),
    ] {
        let frame = |direction| ic_wire::Encoding::Frame {
            method: children::UPGRADE_STATUS,
            direction,
        };
        let args = ic_wire::encode(
            &Request {
                slot: ChildId::FIRST,
                value: selector,
            },
            frame(ic_wire::Direction::Request),
            LIMITS,
        )
        .unwrap();
        let bytes = pic
            .query_call(root, CanisterId::anonymous(), "child_upgrade_status", args)
            .await
            .unwrap();
        let status: upgrade::Status =
            ic_wire::decode(&bytes, frame(ic_wire::Direction::Response), LIMITS).unwrap();
        assert_eq!(&status, expected);
    }

    canic_qualification::upgrade_stopped(pic, root, artifact("root.wasm"), vec![], None)
        .await
        .unwrap();
    let migrated = read(pic, root).await;
    assert_eq!(migrated[0], old_custody);
    assert_eq!(migrated[0].creation.approval, first_ready.approval);
    assert_eq!(migrated[0].creation.worker, first_ready.worker);
    assert_eq!(migrated[0].creation.requests, first_ready.requests);
    assert_eq!(migrated[1].creation, Status::default());
    let second = input(
        root,
        padded_artifact("app-v2.wasm"),
        pic.get_time().await.as_nanos_since_unix_epoch(),
    );
    let too_large = input(
        root,
        pad_artifact("app-v2.wasm", canic_management_protocol::MAX_WASM),
        pic.get_time().await.as_nanos_since_unix_epoch(),
    );
    assert_eq!(
        admit(pic, root, ChildId::SECOND, too_large).await,
        Err(Refusal::Artifact)
    );
    assert_eq!(read(pic, root).await, migrated);
    let outsider = CanisterId::self_authenticating(b"not-root-operator");
    for (method, frame) in [
        (
            "observe_children",
            ic_wire::encode(
                &(),
                ic_wire::Encoding::Frame {
                    method: children::CUSTODY,
                    direction: ic_wire::Direction::Request,
                },
                LIMITS,
            )
            .unwrap(),
        ),
        (
            "stage_child",
            ic_wire::encode(
                &Request {
                    slot: ChildId::SECOND,
                    value: second.clone(),
                },
                ic_wire::Encoding::Frame {
                    method: children::STAGE,
                    direction: ic_wire::Direction::Request,
                },
                STAGE_LIMITS,
            )
            .unwrap(),
        ),
    ] {
        assert!(
            pic.update_call(root, outsider, method, frame)
                .await
                .is_err()
        );
    }
    assert_eq!(read(pic, root).await, migrated);
    admit(pic, root, ChildId::SECOND, second.clone())
        .await
        .unwrap();
    // An old maintenance client must fail closed, never omit the second slot.
    assert!(
        pic.update_call(
            root,
            CanisterId::anonymous(),
            "observe_custody",
            ic_wire::encode(
                &(),
                ic_wire::Encoding::Frame {
                    method: custody::OBSERVE,
                    direction: ic_wire::Direction::Request
                },
                LIMITS
            )
            .unwrap()
        )
        .await
        .is_err()
    );
    // Scheduled work survives Root replacement without the operator resubmitting it.
    let scheduled = read(pic, root).await;
    canic_qualification::upgrade_stopped(pic, root, artifact("maintenance.wasm"), vec![], None)
        .await
        .unwrap();
    assert_eq!(read(pic, root).await, scheduled);
    pic.advance_time(Duration::from_secs(11)).await;
    settle(pic).await;
    let both = read(pic, root).await;
    assert_eq!(both[0], migrated[0]);
    assert_eq!(both[1].creation.phase, Phase::Ready);
    assert_eq!(both[1].creation.attempts, [1; 3]);
    let ids = [&both[0], &both[1]]
        .map(|c| CanisterId::from_text(c.creation.worker.as_ref().unwrap()).unwrap());
    assert_ne!(ids[0], ids[1]);
    for child in ids {
        assert_eq!(
            pic.canister_status(child, Some(root))
                .await
                .unwrap()
                .settings
                .controllers,
            [root]
        );
    }
    let wasm = padded_artifact("app-v2.wasm");
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let change = upgrade::Stage {
        approval: upgrade::Approval {
            manager: root.to_text(),
            child: ids[0].to_text(),
            previous_module: first.approval.module_hash,
            module_hash: digest(&wasm),
            execute_at_ns: now + 10_000_000_000,
            expires_at_ns: now + 120_000_000_000,
        },
        wasm,
    };
    let mut too_large = change.clone();
    too_large.wasm = pad_artifact("app-v2.wasm", canic_management_protocol::MAX_WASM);
    too_large.approval.module_hash = digest(&too_large.wasm);
    let reply = pic
        .update_call(
            root,
            CanisterId::anonymous(),
            "stage_child_upgrade",
            ic_wire::encode(
                &Request {
                    slot: ChildId::FIRST,
                    value: too_large,
                },
                ic_wire::Encoding::Frame {
                    method: children::STAGE_UPGRADE,
                    direction: ic_wire::Direction::Request,
                },
                STAGE_LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let refused: Result<upgrade::Status, Refusal> = ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: children::STAGE_UPGRADE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap();
    assert_eq!(refused, Err(Refusal::Artifact));
    assert_eq!(read(pic, root).await, both);
    // Wrong-slot upgrade authority must not cross into its sibling's journal.
    for (slot, expected) in [(ChildId::SECOND, false), (ChildId::FIRST, true)] {
        let reply = pic
            .update_call(
                root,
                CanisterId::anonymous(),
                "stage_child_upgrade",
                ic_wire::encode(
                    &Request {
                        slot,
                        value: change.clone(),
                    },
                    ic_wire::Encoding::Frame {
                        method: children::STAGE_UPGRADE,
                        direction: ic_wire::Direction::Request,
                    },
                    STAGE_LIMITS,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let admitted: Result<upgrade::Status, Refusal> = ic_wire::decode(
            &reply,
            ic_wire::Encoding::Frame {
                method: children::STAGE_UPGRADE,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap();
        assert_eq!(admitted.is_ok(), expected);
    }
    pic.advance_time(Duration::from_secs(11)).await;
    settle(pic).await;
    let upgraded = read(pic, root).await;
    assert_eq!(upgraded[0].creation, both[0].creation);
    assert_eq!(upgraded[0].upgrades[0].phase, upgrade::Phase::Ready);
    assert_eq!(upgraded[1], both[1]);
    assert_eq!(
        pic.canister_status(ids[0], Some(root))
            .await
            .unwrap()
            .module_hash,
        Some(change.approval.module_hash.to_vec())
    );
    // Fill both histories and the combined artifact budget. Every successor
    // must leave the sibling's full retained evidence unchanged.
    for round in 0..upgrade::MAX_UPGRADES {
        for slot in [ChildId::FIRST, ChildId::SECOND] {
            let before = read(pic, root).await;
            let record = &before[slot.index()];
            if record.upgrades.len() > round {
                continue;
            }
            let previous = record.upgrades.last().map_or_else(
                || record.creation.approval.as_ref().unwrap().module_hash,
                |u| u.approval.as_ref().unwrap().module_hash,
            );
            let wasm = padded_artifact(if previous == first.approval.module_hash {
                "app-v2.wasm"
            } else {
                "app.wasm"
            });
            let now = pic.get_time().await.as_nanos_since_unix_epoch();
            let change = upgrade::Stage {
                approval: upgrade::Approval {
                    manager: root.to_text(),
                    child: ids[slot.index()].to_text(),
                    previous_module: previous,
                    module_hash: digest(&wasm),
                    execute_at_ns: now + 10_000_000_000,
                    expires_at_ns: now + 120_000_000_000,
                },
                wasm,
            };
            let reply = pic
                .update_call(
                    root,
                    CanisterId::anonymous(),
                    "stage_child_upgrade",
                    ic_wire::encode(
                        &Request {
                            slot,
                            value: change,
                        },
                        ic_wire::Encoding::Frame {
                            method: children::STAGE_UPGRADE,
                            direction: ic_wire::Direction::Request,
                        },
                        STAGE_LIMITS,
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
            let admitted: Result<upgrade::Status, Refusal> = ic_wire::decode(
                &reply,
                ic_wire::Encoding::Frame {
                    method: children::STAGE_UPGRADE,
                    direction: ic_wire::Direction::Response,
                },
                LIMITS,
            )
            .unwrap();
            admitted.unwrap();
            pic.advance_time(Duration::from_secs(11)).await;
            settle(pic).await;
            let after = read(pic, root).await;
            assert_eq!(after[1 - slot.index()], before[1 - slot.index()]);
            assert_eq!(
                after[slot.index()].upgrades.last().unwrap().phase,
                upgrade::Phase::Ready
            );
        }
    }
    let upgraded = read(pic, root).await;
    assert!(
        upgraded[..2]
            .iter()
            .all(|c| c.upgrades.len() == upgrade::MAX_UPGRADES)
    );
    let replay = admit(pic, root, ChildId::SECOND, second.clone())
        .await
        .unwrap();
    assert_eq!(replay.worker, both[1].creation.worker);
    let mut changed = second;
    changed.approval.expires_at_ns += 1;
    assert_eq!(
        admit(pic, root, ChildId::SECOND, changed).await,
        Err(Refusal::Occupied)
    );
    canic_qualification::upgrade_stopped(pic, root, artifact("root.wasm"), vec![], None)
        .await
        .unwrap();
    assert_eq!(read(pic, root).await, upgraded);
    assert!(status(pic, root).await.stable_pages <= 64);
    let subnet = pic.get_subnet(root).await.unwrap();
    assert_eq!(
        pic.get_subnet_metrics(subnet).await.unwrap().num_canisters,
        3
    );
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

#[tokio::test]
async fn lost_creation_callback_does_not_block_sibling() {
    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
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
    let root = pic.create_canister().await;
    pic.install_canister(root, artifact("faults.wasm"), vec![], None)
        .await;
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let first = input(root, artifact("app.wasm"), now);
    let second = input(root, artifact("app-v2.wasm"), now);
    pic.update_call(
        root,
        CanisterId::anonymous(),
        "inject_child_fault",
        ic_wire::encode(
            &Request {
                slot: ChildId::FIRST,
                value: 2_u8,
            },
            ic_wire::Encoding::Frame {
                method: children::INJECT,
                direction: ic_wire::Direction::Request,
            },
            LIMITS,
        )
        .unwrap(),
    )
    .await
    .unwrap();
    admit(pic, root, ChildId::FIRST, first).await.unwrap();
    admit(pic, root, ChildId::SECOND, second).await.unwrap();
    pic.advance_time(Duration::from_secs(11)).await;
    settle(pic).await;
    let evidence = read(pic, root).await;
    assert_eq!(evidence[0].creation.phase, Phase::CreateIssued);
    assert_eq!(evidence[0].creation.attempts, [1, 0, 0]);
    assert!(evidence[0].creation.worker.is_none());
    assert_eq!(evidence[1].creation.phase, Phase::Ready);
    assert_eq!(evidence[1].creation.attempts, [1; 3]);
    // The surviving sibling also owns its own uncertain-upgrade recovery path.
    pic.update_call(
        root,
        CanisterId::anonymous(),
        "inject_child_fault",
        ic_wire::encode(
            &Request {
                slot: ChildId::SECOND,
                value: 8_u8,
            },
            ic_wire::Encoding::Frame {
                method: children::INJECT,
                direction: ic_wire::Direction::Request,
            },
            LIMITS,
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let wasm = artifact("app.wasm");
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let change = upgrade::Stage {
        approval: upgrade::Approval {
            manager: root.to_text(),
            child: evidence[1].creation.worker.clone().unwrap(),
            previous_module: evidence[1].creation.approval.as_ref().unwrap().module_hash,
            module_hash: digest(&wasm),
            execute_at_ns: now + 10_000_000_000,
            expires_at_ns: now + 120_000_000_000,
        },
        wasm,
    };
    let reply = pic
        .update_call(
            root,
            CanisterId::anonymous(),
            "stage_child_upgrade",
            ic_wire::encode(
                &Request {
                    slot: ChildId::SECOND,
                    value: change.clone(),
                },
                ic_wire::Encoding::Frame {
                    method: children::STAGE_UPGRADE,
                    direction: ic_wire::Direction::Request,
                },
                STAGE_LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let admitted: Result<upgrade::Status, Refusal> = ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: children::STAGE_UPGRADE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap();
    admitted.unwrap();
    pic.advance_time(Duration::from_secs(11)).await;
    settle(pic).await;
    let interrupted = read(pic, root).await;
    assert_eq!(
        interrupted[1].upgrades[0].phase,
        upgrade::Phase::InstallIssued
    );
    assert_eq!(interrupted[0], evidence[0]);
    canic_qualification::upgrade_stopped(pic, root, artifact("root.wasm"), vec![], None)
        .await
        .unwrap();
    settle(pic).await;
    assert_eq!(read(pic, root).await, interrupted);
    for slot in [ChildId::FIRST, ChildId::SECOND] {
        let result = pic
            .update_call(
                root,
                CanisterId::anonymous(),
                "reconcile_child_upgrade",
                ic_wire::encode(
                    &Request {
                        slot,
                        value: change.approval.clone(),
                    },
                    ic_wire::Encoding::Frame {
                        method: children::RECONCILE_UPGRADE,
                        direction: ic_wire::Direction::Request,
                    },
                    LIMITS,
                )
                .unwrap(),
            )
            .await;
        assert_eq!(result.is_ok(), slot == ChildId::SECOND);
    }
    settle(pic).await;
    let recovered = read(pic, root).await;
    assert_eq!(recovered[0], evidence[0]);
    assert_eq!(recovered[1].creation, evidence[1].creation);
    assert_eq!(recovered[1].upgrades[0].phase, upgrade::Phase::Ready);
    assert_eq!(recovered[1].upgrades[0].attempts, [1; 2]);
    let subnet = pic.get_subnet(root).await.unwrap();
    assert_eq!(
        pic.get_subnet_metrics(subnet).await.unwrap().num_canisters,
        3
    );
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

#[tokio::test]
async fn child_admission_reserves_siblings_unissued_attachment() {
    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
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
    let root = pic
        .create_canister_with_params(
            None,
            ic_testkit::pocket_ic::CreateCanisterParams {
                cycles: Some(2_000_000_000_000),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    pic.install_canister(root, artifact("root.wasm"), vec![], None)
        .await;
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let first = input(root, artifact("app.wasm"), now);
    let second = input(root, artifact("app-v2.wasm"), now);
    admit(pic, root, ChildId::FIRST, first).await.unwrap();
    assert_eq!(
        admit(pic, root, ChildId::SECOND, second.clone()).await,
        Err(Refusal::Reserve)
    );
    assert_eq!(read(pic, root).await[1].creation, Status::default());
    pic.add_cycles(root, 1_000_000_000_000).await;
    admit(pic, root, ChildId::SECOND, second).await.unwrap();
    pic.advance_time(Duration::from_secs(11)).await;
    settle(pic).await;
    let evidence = read(pic, root).await;
    assert!(
        evidence[..2]
            .iter()
            .all(|c| c.creation.phase == Phase::Ready)
    );
    assert!(
        evidence[2..]
            .iter()
            .all(|c| c.creation == Status::default())
    );
    assert!(pic.cycle_balance(root).await > u128::from(canic_management_protocol::RESERVE_CYCLES));
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

// A custom section changes the exact module bytes without changing application
// semantics. Its payload uses a three-byte ULEB length and an empty section name.
fn padded_artifact(name: &str) -> Vec<u8> {
    pad_artifact(name, children::MAX_ARTIFACT_BYTES / 4)
}

fn pad_artifact(name: &str, maximum: usize) -> Vec<u8> {
    let mut bytes = artifact(name);
    let mut length = maximum - bytes.len() - 4;
    bytes.push(0);
    let start = bytes.len();
    loop {
        let byte = (length & 0x7f) as u8;
        length >>= 7;
        bytes.push(byte | if length == 0 { 0 } else { 0x80 });
        if length == 0 {
            break;
        }
    }
    assert_eq!(bytes.len() - start, 3);
    bytes.resize(maximum, 0);
    bytes
}
