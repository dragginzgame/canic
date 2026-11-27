//! One operator-absent worker, real caller wiring, and irreversible effect boundaries.
#[path = "../fixtures/child_module.rs"]
mod child_module;
use canic_management_protocol::{
    Approval, CREATION_CYCLES, INJECT, LIMITS, Phase, Refusal, STATUS, Status, digest,
};
use canic_profile_protocol::{PUT, ProfileResult, Put};
use child_module::Stage;
use ic_testkit::{
    pocket_ic::{CanisterId, CreateCanisterParams, PocketIcBuilder, nonblocking::PocketIc},
    server::{Limits, PreparedServer, Server},
};

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

#[path = "../fixtures/child_store.rs"]
mod child_store;
#[path = "../fixtures/local_store.rs"]
mod local_store;
use canic_management_protocol::{application, children::ChildId};
use child_store::ChildStore;

async fn store(pic: &PocketIc, manager: CanisterId) -> ChildStore<'_> {
    ChildStore::install(
        pic,
        manager,
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/growth/store.wasm"))
            .unwrap(),
    )
    .await
}

async fn managed_call<A: serde::Serialize, R: serde::de::DeserializeOwned>(
    pic: &PocketIc,
    manager: CanisterId,
    store: &ChildStore<'_>,
    method: &str,
    id: u64,
    approval: &A,
    wasm: &[u8],
) -> R {
    let frame = store.frame(id, ChildId::FIRST, approval, wasm).await;
    let reply = pic
        .update_call(manager, CanisterId::anonymous(), method, frame)
        .await
        .unwrap();
    ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: id,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
}

async fn status(pic: &PocketIc, manager: CanisterId) -> Status {
    let reply = pic
        .query_call(
            manager,
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

async fn reconcile(
    pic: &PocketIc,
    manager: CanisterId,
    store: &ChildStore<'_>,
    input: &Stage,
) -> Status {
    managed_call(
        pic,
        manager,
        store,
        "reconcile_child_creation",
        application::RECONCILE_CHILD_CREATION,
        &input.approval,
        &input.wasm,
    )
    .await
}
async fn stage(
    pic: &PocketIc,
    manager: CanisterId,
    store: &ChildStore<'_>,
    input: &Stage,
) -> Result<Status, Refusal> {
    managed_call(
        pic,
        manager,
        store,
        "stage_child_creation",
        application::STAGE_CHILD_CREATION,
        &input.approval,
        &input.wasm,
    )
    .await
}

async fn settle(pic: &PocketIc) {
    for _ in 0..30 {
        pic.advance_time(Duration::from_millis(100)).await;
        pic.tick().await;
    }
}

async fn count(pic: &PocketIc, manager: CanisterId) -> u64 {
    pic.get_subnet_metrics(pic.get_subnet(manager).await.unwrap())
        .await
        .unwrap()
        .num_canisters
}

fn approval(manager: CanisterId, wasm: &[u8], now: u64) -> Stage {
    Stage {
        approval: Approval {
            manager: manager.to_text(),
            module_hash: digest(wasm),
            creation_cycles: CREATION_CYCLES,
            execute_at_ns: now + 10_000_000_000,
            expires_at_ns: now + 120_000_000_000,
        },
        wasm: wasm.to_vec(),
    }
}

#[tokio::test(flavor = "current_thread")]
async fn autonomous_growth_and_commit_boundaries() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();
    let artifacts = repo.join("target/growth");
    let worker_wasm = std::fs::read(artifacts.join("worker.wasm")).unwrap();
    let manager_wasm = std::fs::read(artifacts.join("manager.wasm")).unwrap();
    let faults_wasm = std::fs::read(artifacts.join("faults.wasm")).unwrap();
    let maintenance_wasm = std::fs::read(artifacts.join("maintenance.wasm")).unwrap();
    assert_ne!(manager_wasm, maintenance_wasm);
    let facts = canic_qualification::footprint::measure(&manager_wasm).unwrap();
    let binary = PreparedServer::check(&canic_qualification::server_directory())
        .expect("run make qualification-setup");
    let mut server = Server::start(
        binary,
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(300),
            stdout_bytes: 1024 * 1024,
            stderr_bytes: 1024 * 1024,
        },
    )
    .unwrap();
    let owner = server
        .instance(
            PocketIcBuilder::new()
                .with_nns_subnet()
                .with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    let pic = owner.client();
    let manager = pic
        .create_canister_with_params(
            None,
            CreateCanisterParams {
                cycles: Some(2_500_000_000_000),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    pic.install_canister(manager, manager_wasm.clone(), vec![], None)
        .await;
    let supplier = store(pic, manager).await;
    let original_count = count(pic, manager).await;
    let input = approval(
        manager,
        &worker_wasm,
        pic.get_time().await.as_nanos_since_unix_epoch(),
    );
    assert_eq!(
        stage(pic, manager, &supplier, &input).await.unwrap().phase,
        Phase::Scheduled
    );
    assert_eq!(count(pic, manager).await, original_count);
    pic.advance_time(Duration::from_secs(11)).await;
    settle(pic).await;
    let ready = status(pic, manager).await;
    assert_eq!(ready.phase, Phase::Ready, "{ready:?}");
    assert_eq!(ready.attempts, [1, 1, 1]);
    assert_eq!(count(pic, manager).await, original_count + 1);
    assert!(ready.stable_pages <= 64);
    let worker = CanisterId::from_text(ready.worker.as_ref().unwrap()).unwrap();
    assert_eq!(pic.get_controllers(worker).await, vec![manager]);
    assert_eq!(pic.get_subnet(worker).await, pic.get_subnet(manager).await);
    let alice = CanisterId::self_authenticating(b"managed profile alice");
    let request = ic_wire::encode(
        &Put {
            user_id: 7,
            expected_revision: 0,
            display_name: "Created without host".into(),
        },
        ic_wire::Encoding::Frame {
            method: PUT,
            direction: ic_wire::Direction::Request,
        },
        LIMITS,
    )
    .unwrap();
    let reply = pic
        .update_call(worker, alice, "put", request)
        .await
        .unwrap();
    let profile = ic_wire::decode::<ProfileResult>(
        &reply,
        ic_wire::Encoding::Frame {
            method: PUT,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
    .unwrap();
    assert_eq!(profile.owner, alice.as_slice());
    canic_qualification::upgrade_stopped(pic, manager, maintenance_wasm.clone(), vec![], None)
        .await
        .unwrap();
    settle(pic).await;
    let preserved = status(pic, manager).await;
    assert_eq!(preserved.approval, ready.approval);
    assert_eq!(preserved.worker, ready.worker);
    assert_eq!(preserved.attempts, ready.attempts);
    assert_eq!(preserved.requests, ready.requests);
    assert_eq!(preserved.creation_refund, ready.creation_refund);
    assert_eq!(preserved.observed_module, ready.observed_module);
    assert_eq!(
        pic.canister_status(manager, None)
            .await
            .unwrap()
            .module_hash,
        Some(digest(&maintenance_wasm).to_vec())
    );
    assert_eq!(count(pic, manager).await, original_count + 1);
    assert_eq!(
        stage(pic, manager, &supplier, &input).await.unwrap().phase,
        Phase::Ready
    );

    for fault in 1_u8..=5 {
        let manager = pic.create_canister().await;
        pic.add_cycles(manager, 3_000_000_000_000).await;
        pic.install_canister(manager, faults_wasm.clone(), vec![], None)
            .await;
        let supplier = store(pic, manager).await;
        let input = approval(
            manager,
            &worker_wasm,
            pic.get_time().await.as_nanos_since_unix_epoch(),
        );
        let outsider = CanisterId::self_authenticating(b"not the controller");
        assert!(
            pic.update_call(
                manager,
                outsider,
                "stage_child_creation",
                supplier
                    .frame(
                        application::STAGE_CHILD_CREATION,
                        ChildId::FIRST,
                        &input.approval,
                        &input.wasm
                    )
                    .await
            )
            .await
            .is_err()
        );
        let mut invalid = input.clone();
        invalid.approval.module_hash[0] ^= 1;
        assert_eq!(
            stage(pic, manager, &supplier, &invalid).await,
            Err(Refusal::Artifact)
        );
        pic.update_call(
            manager,
            CanisterId::anonymous(),
            "inject_fault",
            ic_wire::encode(
                &fault,
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
        let before = count(pic, manager).await;
        stage(pic, manager, &supplier, &input).await.unwrap();
        pic.advance_time(Duration::from_secs(11)).await;
        settle(pic).await;
        let blocked = status(pic, manager).await;
        let expected = [
            Phase::Scheduled,
            Phase::CreateIssued,
            Phase::Created,
            Phase::UploadIssued,
            Phase::InstallIssued,
        ][usize::from(fault - 1)];
        assert_eq!(blocked.phase, expected, "fault {fault}: {blocked:?}");
        assert_eq!(count(pic, manager).await, before + u64::from(fault != 1));
        invalid = input.clone();
        invalid.approval.expires_at_ns += 1;
        assert_eq!(
            stage(pic, manager, &supplier, &invalid).await,
            Err(Refusal::Occupied)
        );
        if fault >= 4 {
            // An abandoned timer stays blocked even after another scheduling
            // opportunity; only the domain reconciliation below may resume it.
            pic.advance_time(Duration::from_secs(3)).await;
            settle(pic).await;
            assert_eq!(status(pic, manager).await, blocked);
            // Callback cleanup must release heap exclusion without replacing
            // the owner. Preserve the issued upload/install evidence throughout.
            reconcile(pic, manager, &supplier, &input).await;
            settle(pic).await;
            let recovered = status(pic, manager).await;
            assert_eq!(
                recovered.phase,
                Phase::Ready,
                "fault {fault}: {recovered:?}"
            );
            assert_eq!(recovered.approval, blocked.approval);
            for index in 0..3 {
                if blocked.attempts[index] != 0 {
                    assert_eq!(recovered.attempts[index], blocked.attempts[index]);
                    assert_eq!(recovered.requests[index], blocked.requests[index]);
                }
            }
        }
        canic_qualification::upgrade_stopped(pic, manager, maintenance_wasm.clone(), vec![], None)
            .await
            .unwrap();
        if fault == 2 {
            assert_eq!(
                reconcile(pic, manager, &supplier, &input).await.phase,
                Phase::CreateIssued
            );
            settle(pic).await;
            let still = status(pic, manager).await;
            assert_eq!(still.worker, None);
            assert_eq!(still.creation_refund, None);
            assert_eq!(still.attempts, [1, 0, 0]);
        } else {
            if fault >= 4 {
                reconcile(pic, manager, &supplier, &input).await;
            }
            settle(pic).await;
            let recovered = status(pic, manager).await;
            assert_eq!(
                recovered.phase,
                Phase::Ready,
                "fault {fault}: {recovered:?}"
            );
            assert_eq!(recovered.attempts, [1, 1, 1]);
            if fault >= 3 {
                assert_eq!(recovered.worker, blocked.worker);
            }
        }
        assert_eq!(count(pic, manager).await, before + 1);
        stage(pic, manager, &supplier, &input).await.unwrap();
        settle(pic).await;
        assert_eq!(count(pic, manager).await, before + 1);
        println!(
            "fault {fault}: {:?}, maintenance/replay retained one-worker authority",
            blocked.phase
        );
    }
    let manager = pic.create_canister().await;
    pic.install_canister(manager, manager_wasm.clone(), vec![], None)
        .await;
    let supplier = store(pic, manager).await;
    let input = approval(
        manager,
        &worker_wasm,
        pic.get_time().await.as_nanos_since_unix_epoch(),
    );
    let before = count(pic, manager).await;
    stage(pic, manager, &supplier, &input).await.unwrap();
    pic.advance_time(Duration::from_secs(121)).await;
    settle(pic).await;
    assert_eq!(status(pic, manager).await.phase, Phase::Expired);
    assert_eq!(count(pic, manager).await, before);
    let underfunded = pic
        .create_canister_with_params(
            None,
            CreateCanisterParams {
                cycles: Some(u128::from(CREATION_CYCLES)),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    pic.install_canister(underfunded, manager_wasm.clone(), vec![], None)
        .await;
    let supplier = store(pic, underfunded).await;
    let input = approval(
        underfunded,
        &worker_wasm,
        pic.get_time().await.as_nanos_since_unix_epoch(),
    );
    let before = count(pic, underfunded).await;
    assert_eq!(
        stage(pic, underfunded, &supplier, &input).await,
        Err(Refusal::Reserve)
    );
    pic.advance_time(Duration::from_secs(11)).await;
    settle(pic).await;
    assert_eq!(status(pic, underfunded).await.phase, Phase::Empty);
    assert_eq!(count(pic, underfunded).await, before);
    println!(
        "manager footprint: raw={}, code={}, functions={}, stable_pages={}; normal/5 interruption windows/expiry/reserve passed",
        facts.raw_bytes, facts.code_section_bytes, facts.defined_functions, ready.stable_pages
    );
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

async fn upgrade_status(
    pic: &PocketIc,
    manager: CanisterId,
) -> canic_management_protocol::upgrade::Status {
    let reply = pic
        .query_call(
            manager,
            CanisterId::anonymous(),
            "upgrade_status",
            ic_wire::encode(
                &(),
                ic_wire::Encoding::Frame {
                    method: canic_management_protocol::upgrade::STATUS,
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
            method: canic_management_protocol::upgrade::STATUS,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
}

async fn stage_upgrade(
    pic: &PocketIc,
    manager: CanisterId,
    store: &ChildStore<'_>,
    input: &Stage<canic_management_protocol::upgrade::Approval>,
) -> Result<canic_management_protocol::upgrade::Status, Refusal> {
    managed_call(
        pic,
        manager,
        store,
        "stage_child_upgrade",
        application::STAGE_CHILD_UPGRADE,
        &input.approval,
        &input.wasm,
    )
    .await
}
async fn reconcile_upgrade(
    pic: &PocketIc,
    manager: CanisterId,
    store: &ChildStore<'_>,
    input: &Stage<canic_management_protocol::upgrade::Approval>,
) -> canic_management_protocol::upgrade::Status {
    managed_call(
        pic,
        manager,
        store,
        "reconcile_child_upgrade",
        application::RECONCILE_CHILD_UPGRADE,
        &input.approval,
        &input.wasm,
    )
    .await
}
async fn profile_get(pic: &PocketIc, worker: CanisterId, caller: CanisterId) -> ProfileResult {
    let reply = pic
        .query_call(
            worker,
            caller,
            "get",
            ic_wire::encode(
                &7_u64,
                ic_wire::Encoding::Frame {
                    method: canic_profile_protocol::GET,
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
            method: canic_profile_protocol::GET,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn child_upgrade_preserves_state_and_never_reissues_uncertain_install() {
    use canic_management_protocol::upgrade;
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();
    let artifacts = repo.join("target/growth");
    let original = std::fs::read(artifacts.join("worker.wasm")).unwrap();
    let replacement = std::fs::read(artifacts.join("worker-v2.wasm")).unwrap();
    let rejected = std::fs::read(artifacts.join("worker-reject.wasm")).unwrap();
    let manager_wasm = std::fs::read(artifacts.join("faults.wasm")).unwrap();
    let maintenance = std::fs::read(artifacts.join("maintenance.wasm")).unwrap();
    assert_ne!(original, replacement);
    let binary = PreparedServer::check(&canic_qualification::server_directory()).unwrap();
    let mut server = Server::start(
        binary,
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(300),
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
    let alice = CanisterId::self_authenticating(b"upgrade profile owner");
    let bob = CanisterId::self_authenticating(b"upgrade outsider");
    // Normal, upload callback loss, pre-dispatch trap, install callback loss,
    // and a real application post_upgrade rejection use the same admitted path.
    for case in [0_u8, 6, 7, 8, 9, 24, 25] {
        let manager = pic.create_canister().await;
        pic.add_cycles(manager, 3_000_000_000_000).await;
        pic.install_canister(manager, manager_wasm.clone(), vec![], None)
            .await;
        let supplier = store(pic, manager).await;
        let creation = approval(
            manager,
            &original,
            pic.get_time().await.as_nanos_since_unix_epoch(),
        );
        stage(pic, manager, &supplier, &creation).await.unwrap();
        pic.advance_time(Duration::from_secs(11)).await;
        settle(pic).await;
        let created = status(pic, manager).await;
        assert_eq!(created.phase, Phase::Ready, "{created:?}");
        let worker = CanisterId::from_text(created.worker.as_ref().unwrap()).unwrap();
        let put = Put {
            user_id: 7,
            expected_revision: 0,
            display_name: "Retained by the application".into(),
        };
        let reply = pic
            .update_call(
                worker,
                alice,
                "put",
                ic_wire::encode(
                    &put,
                    ic_wire::Encoding::Frame {
                        method: PUT,
                        direction: ic_wire::Direction::Request,
                    },
                    LIMITS,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let profile = ic_wire::decode::<ProfileResult>(
            &reply,
            ic_wire::Encoding::Frame {
                method: PUT,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap()
        .unwrap();
        let before = count(pic, manager).await;
        let wasm = if case == 9 { &rejected } else { &replacement };
        let now = pic.get_time().await.as_nanos_since_unix_epoch();
        let input = Stage {
            approval: upgrade::Approval {
                manager: manager.to_text(),
                child: worker.to_text(),
                previous_module: digest(&original),
                module_hash: digest(wasm),
                execute_at_ns: now + 10_000_000_000,
                expires_at_ns: now + 120_000_000_000,
            },
            wasm: wasm.clone(),
        };
        assert!(
            pic.update_call(
                manager,
                bob,
                "stage_child_upgrade",
                supplier
                    .frame(
                        application::STAGE_CHILD_UPGRADE,
                        ChildId::FIRST,
                        &input.approval,
                        &input.wasm
                    )
                    .await
            )
            .await
            .is_err()
        );
        let mut invalid = input.clone();
        invalid.approval.child = manager.to_text();
        assert_eq!(
            stage_upgrade(pic, manager, &supplier, &invalid).await,
            Err(Refusal::Binding)
        );
        invalid = input.clone();
        invalid.approval.previous_module[0] ^= 1;
        assert_eq!(
            stage_upgrade(pic, manager, &supplier, &invalid).await,
            Err(Refusal::Binding)
        );
        if (6..=8).contains(&case) || case >= 24 {
            pic.update_call(
                manager,
                CanisterId::anonymous(),
                "inject_fault",
                ic_wire::encode(
                    &case,
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
        }
        assert_eq!(
            stage_upgrade(pic, manager, &supplier, &input)
                .await
                .unwrap()
                .phase,
            upgrade::Phase::Scheduled
        );
        pic.advance_time(Duration::from_secs(11)).await;
        settle(pic).await;
        let interrupted = upgrade_status(pic, manager).await;
        let expected = match case {
            24 => upgrade::Phase::StopIssued,
            25 => upgrade::Phase::StartIssued,
            6 => upgrade::Phase::UploadIssued,
            7 => upgrade::Phase::Uploaded,
            8 | 9 => upgrade::Phase::InstallIssued,
            _ => upgrade::Phase::Ready,
        };
        assert_eq!(interrupted.phase, expected, "case {case}: {interrupted:?}");
        assert_eq!(
            interrupted.attempts,
            if case == 24 {
                [0, 0]
            } else if case == 6 || case == 7 {
                [1, 0]
            } else {
                [1, 1]
            }
        );
        invalid = input.clone();
        invalid.approval.expires_at_ns += 1;
        assert_eq!(
            stage_upgrade(pic, manager, &supplier, &invalid).await,
            Err(Refusal::Occupied)
        );
        canic_qualification::upgrade_stopped(pic, manager, maintenance.clone(), vec![], None)
            .await
            .unwrap();
        reconcile_upgrade(pic, manager, &supplier, &input).await;
        settle(pic).await;
        let complete = upgrade_status(pic, manager).await;
        assert_eq!(complete.approval, Some(input.approval.clone()));
        assert_eq!(complete.attempts, [1, 1]);
        if interrupted.requests[0].is_some() {
            assert_eq!(complete.requests[0], interrupted.requests[0]);
        }
        if interrupted.requests[1].is_some() {
            assert_eq!(complete.requests[1], interrupted.requests[1]);
        }
        if interrupted.previous_version.is_some() {
            assert_eq!(complete.previous_version, interrupted.previous_version);
        }
        if case == 9 {
            assert_eq!(complete.phase, upgrade::Phase::InstallIssued);
            assert!(complete.failure.is_some());
            assert_eq!(
                pic.canister_status(worker, Some(manager))
                    .await
                    .unwrap()
                    .module_hash,
                Some(digest(&original).to_vec())
            );
        } else {
            assert_eq!(
                complete.phase,
                upgrade::Phase::Ready,
                "case {case}: {complete:?}"
            );
            assert_eq!(
                complete.observed_version,
                complete.previous_version.map(|v| v + 1)
            );
            assert_eq!(
                pic.canister_status(worker, Some(manager))
                    .await
                    .unwrap()
                    .module_hash,
                Some(digest(&replacement).to_vec())
            );
        }
        if case == 9 {
            assert!(pic.query_call(worker, alice, "get", vec![]).await.is_err());
            // Explicit fixture operator recovery; rejected installation remains fenced.
            pic.start_canister(worker, Some(manager)).await.unwrap();
        }
        assert_eq!(profile_get(pic, worker, alice).await.unwrap(), profile);
        assert_eq!(
            profile_get(pic, worker, bob).await,
            Err(canic_profile_protocol::ProfileError::Forbidden)
        );
        assert_eq!(pic.get_controllers(worker).await, [manager]);
        let preserved = status(pic, manager).await;
        assert_eq!(preserved.approval, created.approval);
        assert_eq!(preserved.worker, created.worker);
        assert_eq!(preserved.requests, created.requests);
        assert_eq!(preserved.attempts, created.attempts);
        assert_eq!(preserved.observed_module, created.observed_module);
        // Replaying authority or reconciliation cannot reset a consumed upgrade slot.
        assert_eq!(
            stage_upgrade(pic, manager, &supplier, &input)
                .await
                .unwrap(),
            complete
        );
        reconcile_upgrade(pic, manager, &supplier, &input).await;
        settle(pic).await;
        assert_eq!(upgrade_status(pic, manager).await, complete);
        assert_eq!(count(pic, manager).await, before);
        if case == 0 {
            repeated_upgrades(
                pic,
                manager,
                &supplier,
                &original,
                &replacement,
                &maintenance,
                (input.clone(), complete.clone()),
            )
            .await;
            assert_eq!(profile_get(pic, worker, alice).await.unwrap(), profile);
            assert_eq!(
                profile_get(pic, worker, bob).await,
                Err(canic_profile_protocol::ProfileError::Forbidden)
            );
        } else if case == 9 {
            let mut successor = input.clone();
            successor.approval.execute_at_ns += 1;
            assert_eq!(
                stage_upgrade(pic, manager, &supplier, &successor).await,
                Err(Refusal::Occupied)
            );
        }
        println!(
            "child upgrade case {case}: {:?}, profile/private ownership and one-install custody retained",
            complete.phase
        );
    }
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

async fn repeated_upgrades(
    pic: &PocketIc,
    manager: CanisterId,
    supplier: &ChildStore<'_>,
    original: &[u8],
    replacement: &[u8],
    maintenance: &[u8],
    first: (
        Stage<canic_management_protocol::upgrade::Approval>,
        canic_management_protocol::upgrade::Status,
    ),
) {
    use canic_management_protocol::upgrade;
    let worker = CanisterId::from_text(&first.0.approval.child).unwrap();
    let mut retained = vec![first];
    for index in 1..upgrade::MAX_UPGRADES {
        let latest = upgrade_status(pic, manager).await;
        let now = pic.get_time().await.as_nanos_since_unix_epoch();
        let wasm = if index % 2 == 1 {
            original
        } else {
            replacement
        };
        let next = Stage {
            approval: upgrade::Approval {
                manager: manager.to_text(),
                child: worker.to_text(),
                previous_module: latest.approval.as_ref().unwrap().module_hash,
                module_hash: digest(wasm),
                execute_at_ns: now + 10_000_000_000,
                expires_at_ns: now + 120_000_000_000,
            },
            wasm: wasm.to_vec(),
        };
        assert_eq!(
            stage_upgrade(pic, manager, supplier, &next)
                .await
                .unwrap()
                .phase,
            upgrade::Phase::Scheduled
        );
        // A second approval cannot replace admitted work, even before effects.
        let mut competing = next.clone();
        competing.approval.expires_at_ns += 1;
        assert_eq!(
            stage_upgrade(pic, manager, supplier, &competing).await,
            Err(Refusal::Occupied)
        );
        pic.advance_time(Duration::from_secs(11)).await;
        settle(pic).await;
        let complete = upgrade_status(pic, manager).await;
        assert_eq!(complete.phase, upgrade::Phase::Ready, "{complete:?}");
        assert!(complete.previous_version.unwrap() >= latest.observed_version.unwrap());
        assert_eq!(complete.attempts, [1; 2]);
        retained.push((next, complete.clone()));
        // Historical replay and recovery must not mutate the newest operation.
        for (input, expected) in &retained {
            assert_eq!(
                stage_upgrade(pic, manager, supplier, input).await.unwrap(),
                *expected
            );
            assert_eq!(
                reconcile_upgrade(pic, manager, supplier, input).await,
                *expected
            );
        }
        assert_eq!(upgrade_status(pic, manager).await, complete);
    }
    // Reopen the full journal: same original two allocations and retained history.
    canic_qualification::upgrade_stopped(pic, manager, maintenance.to_vec(), vec![], None)
        .await
        .unwrap();
    let (last, expected) = retained.last().unwrap();
    assert_eq!(upgrade_status(pic, manager).await, *expected);
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let exhausted = Stage {
        approval: upgrade::Approval {
            previous_module: last.approval.module_hash,
            module_hash: digest(original),
            execute_at_ns: now + 10_000_000_000,
            expires_at_ns: now + 120_000_000_000,
            ..last.approval.clone()
        },
        wasm: original.to_vec(),
    };
    assert_eq!(
        stage_upgrade(pic, manager, supplier, &exhausted).await,
        Err(Refusal::Occupied)
    );
    assert_eq!(
        stage_upgrade(pic, manager, supplier, &retained[0].0)
            .await
            .unwrap(),
        retained[0].1
    );
    assert_eq!(pic.get_controllers(worker).await, [manager]);
}
