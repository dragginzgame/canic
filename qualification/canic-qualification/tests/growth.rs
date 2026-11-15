//! One operator-absent worker, real caller wiring, and irreversible effect boundaries.
use canic_management_protocol::{
    Approval, CREATION_CYCLES, INJECT, LIMITS, Phase, RECONCILE, Refusal, STAGE, STAGE_LIMITS,
    STATUS, Stage, Status, digest,
};
use canic_profile_protocol::{PUT, ProfileResult, Put};
use ic_testkit::{
    pic::{PocketIcStartupConfig, server_assets},
    pocket_ic::{CanisterId, CreateCanisterParams, PocketIcBuilder, nonblocking::PocketIc},
};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

fn cli(root: &Path, args: &[&str]) -> Value {
    let output = Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(args)
        .arg("--json")
        .arg("--state-dir")
        .arg(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn client(args: &[&str]) -> Value {
    let output = Command::new(std::env::var_os("CANIC_PROFILE_CLIENT").unwrap())
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
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
async fn reconcile(pic: &PocketIc, manager: CanisterId) -> Status {
    let reply = pic
        .update_call(
            manager,
            CanisterId::anonymous(),
            "reconcile",
            ic_wire::encode(
                &status(pic, manager).await.approval.unwrap(),
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
    ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: RECONCILE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
}
async fn stage(pic: &PocketIc, manager: CanisterId, input: &Stage) -> Result<Status, Refusal> {
    let reply = pic
        .update_call(
            manager,
            CanisterId::anonymous(),
            "stage",
            ic_wire::encode(
                input,
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
    ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: STAGE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
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
    let facts = ic_host_artifacts::wasm::inspect(
        &manager_wasm,
        ic_host_artifacts::wasm::InspectionLimits {
            module_bytes: 1_500_000,
            sections: 64,
            exports: 64,
            custom_sections: 32,
        },
    )
    .unwrap();
    let binary = server_assets::check(&canic_qualification::server_directory())
        .expect("run make qualification-setup");
    let mut owner = PocketIcStartupConfig::Spawn {
        server_binary: binary,
        timeout: Duration::from_secs(30),
        server_hard_ttl: Some(Duration::from_secs(300)),
        server_idle_ttl: None,
        server_output_files: None,
    }
    .build_instance(
        PocketIcBuilder::new()
            .with_nns_subnet()
            .with_application_subnet(),
    )
    .await
    .unwrap();
    let url = owner.instance_mut().make_live(None).await.to_string();
    let key: String = owner
        .instance()
        .root_key()
        .await
        .unwrap()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let routing = owner.instance().create_canister().await.to_text();
    let root = artifacts.join(format!("journey-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let icp = repo.join(".tools/ic/bin/icp");
    let plan = cli(
        &root,
        &[
            "fleet",
            "plan",
            "--wasm",
            artifacts.join("manager.wasm").to_str().unwrap(),
            "--network",
            "local",
            "--url",
            &url,
            "--root-key",
            &key,
            "--routing-canister",
            &routing,
            "--controller",
            "2vxsx-fae",
            "--cycles",
            "2500000000000",
        ],
    );
    let receipt = cli(
        &root,
        &[
            "fleet",
            "apply",
            plan["operation"].as_str().unwrap(),
            "--identity",
            "anonymous",
            "--icp-bin",
            icp.to_str().unwrap(),
        ],
    );
    let manager_text = receipt["canister"].as_str().unwrap();
    let manager = CanisterId::from_text(manager_text).unwrap();
    let pic = owner.instance();
    let original_count = count(pic, manager).await;
    let file = root.join("approval.json");
    let base = [
        "--manager",
        manager_text,
        "--identity",
        "anonymous",
        "--network",
        &url,
        "--root-key",
        &key,
        "--icp-bin",
        icp.to_str().unwrap(),
    ];
    let mut args = base.to_vec();
    let worker_path = artifacts.join("worker.wasm");
    args.extend([
        "growth-plan",
        "--wasm",
        worker_path.to_str().unwrap(),
        "--output",
        file.to_str().unwrap(),
        "--delay-seconds",
        "60",
    ]);
    let planned = client(&args);
    assert_eq!(planned["approval"]["creation_cycles"], CREATION_CYCLES);
    // Equal principal text on another network must not reuse the retained authority.
    let refused = Command::new(std::env::var_os("CANIC_PROFILE_CLIENT").unwrap())
        .args([
            "--manager",
            manager_text,
            "--network",
            "ic",
            "growth-approve",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("network/root key differs"));
    let mut args = base.to_vec();
    args.extend(["growth-approve", file.to_str().unwrap()]);
    let accepted = client(&args);
    assert_eq!(accepted["result"]["phase"], "Scheduled");
    assert_eq!(count(pic, manager).await, original_count);
    // No mutating ingress follows admission: only IC time/ticks and read-only observations.
    pic.stop_progress().await;
    pic.advance_time(Duration::from_secs(61)).await;
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
    pic.upgrade_canister(manager, maintenance_wasm.clone(), vec![], None)
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
    assert!(preserved.maintenance_candidate);
    assert_eq!(count(pic, manager).await, original_count + 1);
    pic.auto_progress().await;
    let replay = client(&args);
    assert_eq!(replay["result"]["phase"], "Ready");
    let report = root.join("fleet.html");
    let fleet = cli(
        &root,
        &[
            "fleet",
            "show",
            "--manager",
            manager_text,
            "--html",
            report.to_str().unwrap(),
        ],
    );
    assert_eq!(fleet["canisters"].as_array().unwrap().len(), 2);
    assert_eq!(fleet["managers"][0]["reported"]["phase"], "Ready");
    assert!(
        std::fs::read_to_string(report)
            .unwrap()
            .contains("Manager report (uncertified)")
    );
    pic.stop_progress().await;

    for fault in 1_u8..=5 {
        let manager = pic.create_canister().await;
        pic.add_cycles(manager, 3_000_000_000_000).await;
        pic.install_canister(manager, faults_wasm.clone(), vec![], None)
            .await;
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
                "stage",
                ic_wire::encode(
                    &input,
                    ic_wire::Encoding::Frame {
                        method: STAGE,
                        direction: ic_wire::Direction::Request
                    },
                    STAGE_LIMITS
                )
                .unwrap()
            )
            .await
            .is_err()
        );
        let mut invalid = input.clone();
        invalid.approval.module_hash[0] ^= 1;
        assert_eq!(stage(pic, manager, &invalid).await, Err(Refusal::Artifact));
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
        stage(pic, manager, &input).await.unwrap();
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
        assert_eq!(stage(pic, manager, &invalid).await, Err(Refusal::Occupied));
        if fault >= 4 {
            // An abandoned timer stays blocked even after another scheduling
            // opportunity; only the domain reconciliation below may resume it.
            pic.advance_time(Duration::from_secs(3)).await;
            settle(pic).await;
            assert_eq!(status(pic, manager).await, blocked);
            // Callback cleanup must release heap exclusion without replacing
            // the owner. Preserve the issued upload/install evidence throughout.
            reconcile(pic, manager).await;
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
        pic.upgrade_canister(manager, maintenance_wasm.clone(), vec![], None)
            .await
            .unwrap();
        if fault == 2 {
            assert_eq!(reconcile(pic, manager).await.phase, Phase::CreateIssued);
            settle(pic).await;
            let still = status(pic, manager).await;
            assert_eq!(still.worker, None);
            assert_eq!(still.creation_refund, None);
            assert_eq!(still.attempts, [1, 0, 0]);
        } else {
            if fault >= 4 {
                reconcile(pic, manager).await;
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
        stage(pic, manager, &input).await.unwrap();
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
    let input = approval(
        manager,
        &worker_wasm,
        pic.get_time().await.as_nanos_since_unix_epoch(),
    );
    let before = count(pic, manager).await;
    stage(pic, manager, &input).await.unwrap();
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
    let input = approval(
        underfunded,
        &worker_wasm,
        pic.get_time().await.as_nanos_since_unix_epoch(),
    );
    let before = count(pic, underfunded).await;
    assert_eq!(stage(pic, underfunded, &input).await, Err(Refusal::Reserve));
    pic.advance_time(Duration::from_secs(11)).await;
    settle(pic).await;
    assert_eq!(status(pic, underfunded).await.phase, Phase::Empty);
    assert_eq!(count(pic, underfunded).await, before);
    println!(
        "manager footprint: raw={}, code={}, functions={}, stable_pages={}; normal/5 interruption windows/expiry/reserve passed",
        facts.raw_bytes, facts.code_section_bytes, facts.defined_functions, ready.stable_pages
    );
    owner.close().await;
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
    input: &canic_management_protocol::upgrade::Stage,
) -> Result<canic_management_protocol::upgrade::Status, Refusal> {
    let reply = pic
        .update_call(
            manager,
            CanisterId::anonymous(),
            "stage_upgrade",
            ic_wire::encode(
                input,
                ic_wire::Encoding::Frame {
                    method: canic_management_protocol::upgrade::STAGE,
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
            method: canic_management_protocol::upgrade::STAGE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
}
async fn reconcile_upgrade(pic: &PocketIc, manager: CanisterId) {
    let retained = upgrade_status(pic, manager).await;
    let mut other = retained.approval.clone().unwrap();
    other.expires_at_ns += 1;
    for frame in [
        ic_wire::encode(
            &other,
            ic_wire::Encoding::Frame {
                method: canic_management_protocol::upgrade::RECONCILE,
                direction: ic_wire::Direction::Request,
            },
            LIMITS,
        )
        .unwrap(),
        ic_wire::encode(
            &(),
            ic_wire::Encoding::Frame {
                method: 26,
                direction: ic_wire::Direction::Request,
            },
            LIMITS,
        )
        .unwrap(),
    ] {
        assert!(
            pic.update_call(manager, CanisterId::anonymous(), "reconcile_upgrade", frame)
                .await
                .is_err()
        );
        // Existing autonomous timers may advance while the refused call runs.
        assert_eq!(
            upgrade_status(pic, manager).await.approval,
            retained.approval
        );
    }
    pic.update_call(
        manager,
        CanisterId::anonymous(),
        "reconcile_upgrade",
        ic_wire::encode(
            &upgrade_status(pic, manager).await.approval.unwrap(),
            ic_wire::Encoding::Frame {
                method: canic_management_protocol::upgrade::RECONCILE,
                direction: ic_wire::Direction::Request,
            },
            LIMITS,
        )
        .unwrap(),
    )
    .await
    .unwrap();
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
    qualify_child_upgrades(None, false).await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires CANIC_PREVIOUS_UPGRADE_MANAGER naming a retained v2-journal manager"]
async fn previous_single_upgrade_manager_migrates_to_retained_history() {
    let previous =
        std::fs::read(std::env::var_os("CANIC_PREVIOUS_UPGRADE_MANAGER").unwrap()).unwrap();
    qualify_child_upgrades(Some(previous.clone()), false).await;
    qualify_child_upgrades(Some(previous), true).await;
}

async fn qualify_child_upgrades(previous: Option<Vec<u8>>, migrate_pending: bool) {
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
    let binary = server_assets::check(&canic_qualification::server_directory()).unwrap();
    let owner = PocketIcStartupConfig::Spawn {
        server_binary: binary,
        timeout: Duration::from_secs(30),
        server_hard_ttl: Some(Duration::from_secs(300)),
        server_idle_ttl: None,
        server_output_files: None,
    }
    .build_instance(PocketIcBuilder::new().with_application_subnet())
    .await
    .unwrap();
    let pic = owner.instance();
    let alice = CanisterId::self_authenticating(b"upgrade profile owner");
    let bob = CanisterId::self_authenticating(b"upgrade outsider");
    // Normal, upload callback loss, pre-dispatch trap, install callback loss,
    // and a real application post_upgrade rejection use the same admitted path.
    let cases: &[u8] = if previous.is_some() {
        &[0]
    } else {
        &[0, 6, 7, 8, 9]
    };
    for &case in cases {
        let manager = pic.create_canister().await;
        pic.add_cycles(manager, 3_000_000_000_000).await;
        pic.install_canister(manager, manager_wasm.clone(), vec![], None)
            .await;
        let creation = approval(
            manager,
            &original,
            pic.get_time().await.as_nanos_since_unix_epoch(),
        );
        stage(pic, manager, &creation).await.unwrap();
        if migrate_pending {
            pic.upgrade_canister(manager, maintenance.clone(), vec![], None)
                .await
                .unwrap();
        }
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
        let input = upgrade::Stage {
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
                "stage_upgrade",
                ic_wire::encode(
                    &input,
                    ic_wire::Encoding::Frame {
                        method: upgrade::STAGE,
                        direction: ic_wire::Direction::Request
                    },
                    STAGE_LIMITS
                )
                .unwrap()
            )
            .await
            .is_err()
        );
        let mut invalid = input.clone();
        invalid.approval.child = manager.to_text();
        assert_eq!(
            stage_upgrade(pic, manager, &invalid).await,
            Err(Refusal::Binding)
        );
        invalid = input.clone();
        invalid.approval.previous_module[0] ^= 1;
        assert_eq!(
            stage_upgrade(pic, manager, &invalid).await,
            Err(Refusal::Binding)
        );
        if (6..=8).contains(&case) {
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
            stage_upgrade(pic, manager, &input).await.unwrap().phase,
            upgrade::Phase::Scheduled
        );
        pic.advance_time(Duration::from_secs(11)).await;
        settle(pic).await;
        let interrupted = upgrade_status(pic, manager).await;
        let expected = match case {
            6 => upgrade::Phase::UploadIssued,
            7 => upgrade::Phase::Uploaded,
            8 | 9 => upgrade::Phase::InstallIssued,
            _ => upgrade::Phase::Ready,
        };
        assert_eq!(interrupted.phase, expected, "case {case}: {interrupted:?}");
        assert_eq!(
            interrupted.attempts,
            if case == 6 || case == 7 {
                [1, 0]
            } else {
                [1, 1]
            }
        );
        invalid = input.clone();
        invalid.approval.expires_at_ns += 1;
        assert_eq!(
            stage_upgrade(pic, manager, &invalid).await,
            Err(Refusal::Occupied)
        );
        pic.upgrade_canister(manager, maintenance.clone(), vec![], None)
            .await
            .unwrap();
        reconcile_upgrade(pic, manager).await;
        settle(pic).await;
        let complete = upgrade_status(pic, manager).await;
        assert_eq!(complete.approval, Some(input.approval.clone()));
        assert_eq!(complete.attempts, [1, 1]);
        assert_eq!(complete.requests[0], interrupted.requests[0]);
        if interrupted.requests[1].is_some() {
            assert_eq!(complete.requests[1], interrupted.requests[1]);
        }
        assert_eq!(complete.previous_version, interrupted.previous_version);
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
        assert_eq!(stage_upgrade(pic, manager, &input).await.unwrap(), complete);
        reconcile_upgrade(pic, manager).await;
        settle(pic).await;
        assert_eq!(upgrade_status(pic, manager).await, complete);
        assert_eq!(count(pic, manager).await, before);
        if case == 0 {
            repeated_upgrades(
                pic,
                manager,
                &original,
                &replacement,
                &maintenance,
                input.clone(),
                complete.clone(),
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
                stage_upgrade(pic, manager, &successor).await,
                Err(Refusal::Occupied)
            );
        }
        println!(
            "child upgrade case {case}: {:?}, profile/private ownership and one-install custody retained",
            complete.phase
        );
    }
    owner.close().await;
}

async fn repeated_upgrades(
    pic: &PocketIc,
    manager: CanisterId,
    original: &[u8],
    replacement: &[u8],
    maintenance: &[u8],
    first: canic_management_protocol::upgrade::Stage,
    first_status: canic_management_protocol::upgrade::Status,
) {
    use canic_management_protocol::upgrade;
    let worker = CanisterId::from_text(&first.approval.child).unwrap();
    let mut retained = vec![(first, first_status)];
    for index in 1..upgrade::MAX_UPGRADES {
        let latest = upgrade_status(pic, manager).await;
        let now = pic.get_time().await.as_nanos_since_unix_epoch();
        let wasm = if index % 2 == 1 {
            original
        } else {
            replacement
        };
        let next = upgrade::Stage {
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
            stage_upgrade(pic, manager, &next).await.unwrap().phase,
            upgrade::Phase::Scheduled
        );
        // A second approval cannot replace admitted work, even before effects.
        let mut competing = next.clone();
        competing.approval.expires_at_ns += 1;
        assert_eq!(
            stage_upgrade(pic, manager, &competing).await,
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
            assert_eq!(stage_upgrade(pic, manager, input).await.unwrap(), *expected);
            for (method, id) in [
                ("observe_upgrade", upgrade::OBSERVE),
                ("reconcile_upgrade", upgrade::RECONCILE),
            ] {
                let reply = pic
                    .update_call(
                        manager,
                        CanisterId::anonymous(),
                        method,
                        ic_wire::encode(
                            &input.approval,
                            ic_wire::Encoding::Frame {
                                method: id,
                                direction: ic_wire::Direction::Request,
                            },
                            LIMITS,
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
                    *expected
                );
            }
        }
        assert_eq!(upgrade_status(pic, manager).await, complete);
    }
    // Reopen the full journal: same original two allocations and retained history.
    pic.upgrade_canister(manager, maintenance.to_vec(), vec![], None)
        .await
        .unwrap();
    let (last, expected) = retained.last().unwrap();
    assert_eq!(upgrade_status(pic, manager).await, *expected);
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let exhausted = upgrade::Stage {
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
        stage_upgrade(pic, manager, &exhausted).await,
        Err(Refusal::Occupied)
    );
    assert_eq!(
        stage_upgrade(pic, manager, &retained[0].0).await.unwrap(),
        retained[0].1
    );
    assert_eq!(pic.get_controllers(worker).await, [manager]);
}
