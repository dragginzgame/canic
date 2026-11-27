//! Installed Root ownership and preservation, using synthetic cycles only.
#[path = "../fixtures/child_store.rs"]
mod child_store;
#[path = "../fixtures/local_store.rs"]
mod local_store;
use canic_management_protocol::application;
use child_store::ChildStore;
mod children;
mod fleet_children;
mod greeting;
mod metrics;
mod predecessor;
mod profile_store;
mod snapshot;
mod storage_reopen;

use canic_management_protocol::{
    Approval, CREATION_CYCLES, LIMITS, Phase, Refusal, STAGE, STAGE_LIMITS, STATUS, Stage, Status,
    digest,
};
use ic_testkit::{
    pocket_ic::{CanisterId, PocketIcBuilder, nonblocking::PocketIc},
    server::{Limits, PreparedServer, Server},
};
use std::{path::PathBuf, time::Duration};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn supply_store_artifact(config: &std::path::Path) {
    let directory = config.parent().unwrap().join("artifacts");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::copy(
        repo().join("target/root/store.wasm"),
        directory.join("wasm_store.wasm"),
    )
    .unwrap();
}

async fn status(pic: &PocketIc, root: CanisterId) -> Status {
    use canic_management_protocol::children;
    let reply = pic
        .query_call(
            root,
            CanisterId::anonymous(),
            "child_status",
            ic_wire::encode(
                &children::StatusRequest::Creation(children::ChildId::FIRST),
                ic_wire::Encoding::Frame {
                    method: children::READ_STATUS,
                    direction: ic_wire::Direction::Request,
                },
                LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let children::ObservedStatus::Creation(status) = ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: children::READ_STATUS,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap() else {
        panic!("expected Creation status")
    };
    status
}

async fn stage(
    pic: &PocketIc,
    root: CanisterId,
    store: &ChildStore<'_>,
    input: &Stage,
) -> Result<Status, Refusal> {
    let reply = pic
        .update_call(
            root,
            CanisterId::anonymous(),
            "stage_child_creation",
            store
                .frame(
                    application::STAGE_CHILD_CREATION,
                    canic_management_protocol::children::ChildId::FIRST,
                    &input.approval,
                    &input.wasm,
                )
                .await,
        )
        .await
        .unwrap();
    ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: application::STAGE_CHILD_CREATION,
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

// Both a fresh Root and a previously shipped manager must preserve the same
// immutable creation authority. The readiness codec is owned by each wrapper.
async fn qualify(
    original: Vec<u8>,
    replacement: Vec<u8>,
    child: Vec<u8>,
    greeting: bool,
    before_dispatch: bool,
) {
    assert_ne!(original, replacement);
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
    let root = pic.create_canister().await;
    let store = ChildStore::install(pic, root, children::artifact("store.wasm")).await;
    pic.install_canister(root, original, vec![], None).await;
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let input = Stage {
        approval: Approval {
            manager: root.to_text(),
            module_hash: digest(&child),
            creation_cycles: CREATION_CYCLES,
            execute_at_ns: now + 10_000_000_000,
            expires_at_ns: now + 120_000_000_000,
        },
        wasm: child,
    };
    let outsider = CanisterId::self_authenticating(b"not-the-controller");
    assert!(
        pic.update_call(
            root,
            outsider,
            "stage_child_creation",
            store
                .frame(
                    application::STAGE_CHILD_CREATION,
                    canic_management_protocol::children::ChildId::FIRST,
                    &input.approval,
                    &input.wasm
                )
                .await,
        )
        .await
        .is_err()
    );
    assert_eq!(status(pic, root).await.phase, Phase::Empty);
    let admitted = stage(pic, root, &store, &input).await.unwrap();
    assert_eq!(admitted.phase, Phase::Scheduled);
    // Scheduled migration must resume original bytes and authority.
    if before_dispatch {
        canic_qualification::upgrade_stopped(pic, root, replacement.clone(), vec![], None)
            .await
            .unwrap();
    }
    let maintained = status(pic, root).await;
    assert_eq!(maintained.approval, admitted.approval);
    assert_eq!(maintained.phase, Phase::Scheduled);
    assert_eq!(maintained.attempts, [0; 3]);
    pic.advance_time(Duration::from_secs(11)).await;
    settle(pic).await;
    let ready = status(pic, root).await;
    assert_eq!(ready.phase, Phase::Ready, "{ready:?}");
    assert_eq!(ready.attempts, [1; 3]);
    assert_eq!(ready.approval, admitted.approval);
    if !before_dispatch {
        // Completed migration must retain the irreplaceable principal and evidence.
        canic_qualification::upgrade_stopped(pic, root, replacement, vec![], None)
            .await
            .unwrap();
        let preserved = status(pic, root).await;
        assert_eq!(preserved.approval, ready.approval);
        assert_eq!(preserved.worker, ready.worker);
        assert_eq!(preserved.phase, ready.phase);
        assert_eq!(preserved.attempts, ready.attempts);
        assert_eq!(preserved.requests, ready.requests);
        assert_eq!(preserved.creation_refund, ready.creation_refund);
        assert_eq!(preserved.observed_module, ready.observed_module);
    }
    assert_eq!(
        ready.observed_module,
        Some(input.approval.module_hash.to_vec())
    );
    let child = CanisterId::from_text(ready.worker.as_ref().unwrap()).unwrap();
    let child_status = pic.canister_status(child, Some(root)).await.unwrap();
    assert_eq!(child_status.settings.controllers, [root]);
    let root_status = pic.canister_status(root, None).await.unwrap();
    assert_eq!(root_status.settings.controllers, [CanisterId::anonymous()]);
    assert!(pic.canister_status(child, Some(outsider)).await.is_err());
    if greeting {
        let reply = pic
            .query_call(
                child,
                outsider,
                "greet",
                ic_wire::encode(
                    &"team",
                    ic_wire::Encoding::Frame {
                        method: canic_greeting_witness::GREET,
                        direction: ic_wire::Direction::Request,
                    },
                    canic_greeting_witness::LIMITS,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let reply: canic_greeting_witness::Greeting = ic_wire::decode(
            &reply,
            ic_wire::Encoding::Frame {
                method: canic_greeting_witness::GREET,
                direction: ic_wire::Direction::Response,
            },
            canic_greeting_witness::LIMITS,
        )
        .unwrap();
        assert_eq!(reply.message, "Hello from Canic, team!");
    }
    let subnet = pic.get_subnet(root).await.unwrap();
    let count = pic.get_subnet_metrics(subnet).await.unwrap().num_canisters;
    assert_eq!(count, 3);
    assert_eq!(stage(pic, root, &store, &input).await.unwrap(), ready);
    let mut changed = input.clone();
    changed.approval.expires_at_ns += 1;
    assert_eq!(
        stage(pic, root, &store, &changed).await,
        Err(Refusal::Occupied)
    );
    settle(pic).await;
    assert_eq!(
        pic.get_subnet_metrics(subnet).await.unwrap().num_canisters,
        count
    );
    assert_eq!(status(pic, root).await.worker, ready.worker);
    println!("Root {root}: one child {child}, exact replay and maintenance preserved authority");
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

#[tokio::test(flavor = "current_thread")]
async fn root_controls_one_greeting_application() {
    let artifacts = repo().join("target/root");
    qualify(
        std::fs::read(artifacts.join("root.wasm")).unwrap(),
        std::fs::read(artifacts.join("maintenance.wasm")).unwrap(),
        std::fs::read(artifacts.join("app.wasm")).unwrap(),
        true,
        true,
    )
    .await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "historical old-Storage migration; requires a compatible old build and CANIC_PREVIOUS_MANAGER"]
async fn previous_profile_manager_preserves_admitted_work() {
    let artifacts = repo().join("target/growth");
    qualify(
        std::fs::read(std::env::var_os("CANIC_PREVIOUS_MANAGER").unwrap()).unwrap(),
        std::fs::read(artifacts.join("manager.wasm")).unwrap(),
        std::fs::read(artifacts.join("worker.wasm")).unwrap(),
        false,
        true,
    )
    .await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "historical old-Storage migration; requires a compatible old build and CANIC_PREVIOUS_ROOT"]
async fn previous_root_migrates_payloads_without_replacing_creation() {
    let artifacts = repo().join("target/root");
    qualify(
        std::fs::read(std::env::var_os("CANIC_PREVIOUS_ROOT").unwrap()).unwrap(),
        std::fs::read(artifacts.join("root.wasm")).unwrap(),
        std::fs::read(artifacts.join("app.wasm")).unwrap(),
        true,
        false,
    )
    .await;
}

fn cli(config: &std::path::Path, args: &[&str]) -> serde_json::Value {
    let output = std::process::Command::new(std::env::var_os("CANIC_CLI").expect("make root-test"))
        .args(args)
        .arg("--config")
        .arg(config)
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn status_text(config: &std::path::Path, operation: &str, fleet: &str) -> String {
    let args = ["fleet", "status", operation, "--fleet", fleet];
    let before = cli(config, &args);
    let output = std::process::Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(args)
        .arg("--config")
        .arg(config)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(cli(config, &args), before);
    String::from_utf8(output.stdout).unwrap()
}

fn deploy_root(config: &std::path::Path, fleet: &str) -> serde_json::Value {
    let plan_args = [
        "fleet",
        "plan",
        "--fleet",
        fleet,
        "--canister",
        "root",
        "--controller",
        "2vxsx-fae",
        "--cycles",
        "3000000000000",
    ];
    let plan = cli(config, &plan_args);
    let id = plan["operation"].as_str().unwrap();
    let signer = repo().join(".tools/ic/bin/icp");
    let receipt = cli(
        config,
        &[
            "fleet",
            "apply",
            id,
            "--fleet",
            fleet,
            "--identity",
            "anonymous",
            "--icp-bin",
            signer.to_str().unwrap(),
        ],
    );
    assert_eq!(cli(config, &plan_args)["operation"], id);
    assert_eq!(
        cli(
            config,
            &[
                "fleet",
                "apply",
                id,
                "--fleet",
                fleet,
                "--identity",
                "anonymous",
                "--icp-bin",
                "must-not-run",
            ]
        ),
        receipt
    );
    receipt
}

#[tokio::test(flavor = "current_thread")]
async fn named_child_requires_publication_even_after_root_bootstrap() {
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
    let mut owner = server
        .instance(
            PocketIcBuilder::new()
                .with_nns_subnet()
                .with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    // Only local subnet routing is fixture setup. Both fleet Roots and apps are
    // created through the shipped CLI and its retained operation owners.
    let routing = owner.client().create_canister().await;
    let artifacts = repo().join("target/root");
    let root_wasm = std::fs::read(artifacts.join("root.wasm")).unwrap();
    let root_hash: String = digest(&root_wasm)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let url = owner.client_mut().make_live(None).await.to_string();
    let key: String = owner
        .client()
        .root_key()
        .await
        .unwrap()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let workspace = artifacts.join(format!(
        "cli-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config = workspace.join("canic.toml");
    let executable = std::env::var_os("CANIC_CLI").unwrap();
    let generated = {
        let created = std::process::Command::new(&executable)
            .args([
                "app",
                "init",
                "starter",
                "--source",
                workspace.to_str().unwrap(),
                "--canic-source",
                repo().to_str().unwrap(),
                "--json",
            ])
            .output()
            .unwrap();
        assert!(
            created.status.success(),
            "{}",
            String::from_utf8_lossy(&created.stderr)
        );
        std::process::Command::new(&executable)
            .args([
                "fleet",
                "init",
                "local",
                "--app",
                "starter",
                "--network",
                "local",
                "--url",
                &url,
                "--root-key",
                &key,
                "--root-subnet",
                &owner.client().topology().await.get_nns().unwrap().to_text(),
                "--routing-canister",
                &routing.to_text(),
                "--config",
                workspace.join("canic.toml").to_str().unwrap(),
                "--json",
            ])
            .output()
            .unwrap()
    };
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    supply_store_artifact(&config);
    std::fs::create_dir_all(workspace.join("artifacts")).unwrap();
    std::fs::copy(
        artifacts.join("app.wasm"),
        workspace.join("artifacts/app.wasm"),
    )
    .unwrap();
    std::fs::copy(
        artifacts.join("root.wasm"),
        workspace.join("artifacts/root.wasm"),
    )
    .unwrap();
    let config_bytes = std::fs::read(&config).unwrap();
    let root_receipt = deploy_root(&config, "local");
    let root = CanisterId::from_text(root_receipt["canister"].as_str().unwrap()).unwrap();
    assert_eq!(std::fs::read(&config).unwrap(), config_bytes);
    assert_eq!(root_receipt["controller"], "2vxsx-fae");
    let refused = std::process::Command::new(executable)
        .args([
            "fleet",
            "plan",
            "--canister",
            "app",
            "--controller",
            "2vxsx-fae",
            "--root-module",
            &root_hash,
            "--config",
            config.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    let error: serde_json::Value = serde_json::from_slice(&refused.stderr).unwrap();
    assert!(
        error["message"].as_str().unwrap().contains("--release"),
        "{error}"
    );
    assert_eq!(status(owner.client(), root).await.phase, Phase::Empty);
    assert_eq!(std::fs::read(&config).unwrap(), config_bytes);
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn whole_fleet_cli_binds_future_root_and_replays_without_new_effects() {
    qualify_whole_fleet(false, false).await;
}

#[tokio::test]
async fn whole_fleet_upgrade_preserves_private_profile_state() {
    qualify_whole_fleet(true, false).await;
}

#[tokio::test(flavor = "current_thread")]
async fn whole_fleet_maintenance_refuses_untracked_child_work() {
    qualify_whole_fleet(false, true).await;
}

async fn qualify_whole_fleet(profile: bool, untracked: bool) {
    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(300),
            stdout_bytes: 1024 * 1024,
            stderr_bytes: 1024 * 1024,
        },
    )
    .unwrap();
    let mut owner = server
        .instance(
            PocketIcBuilder::new()
                .with_nns_subnet()
                .with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    let routing = owner.client().create_canister().await;
    let url = owner.client_mut().make_live(None).await.to_string();
    let key: String = owner
        .client()
        .root_key()
        .await
        .unwrap()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let artifacts = repo().join("target/root");
    let workspace = artifacts.join(format!(
        "whole-{profile}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let executable = std::env::var_os("CANIC_CLI").unwrap();
    let initialized = {
        let created = std::process::Command::new(&executable)
            .args([
                "app",
                "init",
                "starter",
                "--source",
                workspace.to_str().unwrap(),
                "--json",
            ])
            .output()
            .unwrap();
        assert!(
            created.status.success(),
            "{}",
            String::from_utf8_lossy(&created.stderr)
        );
        std::process::Command::new(&executable)
            .args([
                "fleet",
                "init",
                "first",
                "--app",
                "starter",
                "--network",
                "local",
                "--url",
                &url,
                "--root-key",
                &key,
                "--root-subnet",
                &owner.client().topology().await.get_nns().unwrap().to_text(),
                "--routing-canister",
                &routing.to_text(),
                "--config",
                workspace.join("canic.toml").to_str().unwrap(),
                "--json",
            ])
            .output()
            .unwrap()
    };
    assert!(
        initialized.status.success(),
        "{}",
        String::from_utf8_lossy(&initialized.stderr)
    );
    let config = workspace.join("canic.toml");
    supply_store_artifact(&config);
    cli(&config, &["app", "build"]);
    let lock = std::fs::read(workspace.join("Cargo.lock")).unwrap();
    cli(&config, &["app", "build"]);
    assert_eq!(lock, std::fs::read(workspace.join("Cargo.lock")).unwrap());
    cli(
        &config,
        &[
            "fleet",
            "init",
            "second",
            "--network",
            "local",
            "--url",
            &url,
            "--root-key",
            &key,
            "--root-subnet",
            &owner.client().topology().await.get_nns().unwrap().to_text(),
            "--routing-canister",
            &routing.to_text(),
        ],
    );
    if profile {
        std::fs::copy(
            artifacts.join("profile-manager.wasm"),
            workspace.join("artifacts/root.wasm"),
        )
        .unwrap();
        std::fs::copy(
            artifacts.join("profile-worker.wasm"),
            workspace.join("artifacts/app.wasm"),
        )
        .unwrap();
    }
    let config_bytes = std::fs::read(&config).unwrap();
    let alice = CanisterId::self_authenticating(b"whole fleet profile owner");
    let mut retained_profile = None;
    let mut identities = Vec::new();
    for fleet in ["first", "second"] {
        let dispatch =
            (owner.client().get_time().await.as_nanos_since_unix_epoch() / 1_000_000_000 + 40)
                .to_string();
        let plan_args = [
            "fleet",
            "plan",
            "--fleet",
            fleet,
            "--identity",
            "anonymous",
            "--execute-at",
            &dispatch,
        ];
        let planned = cli(&config, &plan_args);
        let id = planned["operation"].as_str().unwrap();
        let journal = PathBuf::from(planned["state_dir"].as_str().unwrap());
        assert_eq!(cli(&config, &plan_args), planned);
        assert!(
            cli(&config, &["fleet", "status", id, "--fleet", fleet])["children"][0]["state"]
                .is_null()
        );
        assert!(status_text(&config, id, fleet).contains("Not yet admitted"));
        // Cross-fleet selection refuses before invoking an absent signer.
        let other = if fleet == "first" { "second" } else { "first" };
        let rejected = std::process::Command::new(&executable)
            .args([
                "fleet",
                "apply",
                id,
                "--config",
                config.to_str().unwrap(),
                "--fleet",
                other,
                "--state-dir",
                journal.to_str().unwrap(),
                "--identity",
                "anonymous",
                "--icp-bin",
                "must-not-run",
            ])
            .output()
            .unwrap();
        assert!(!rejected.status.success());
        assert!(String::from_utf8_lossy(&rejected.stderr).contains("does not belong"));
        let apply = [
            "fleet",
            "apply",
            id,
            "--fleet",
            fleet,
            "--identity",
            "anonymous",
        ];
        let pending = cli(&config, &apply);
        assert_eq!(pending["operation"], id);
        assert_eq!(
            pending["children"][0]["outcome"]["status"]["phase"],
            "Scheduled"
        );
        let mut recovery = apply;
        recovery[1] = "reconcile";
        assert_eq!(
            cli(&config, &recovery)["children"][0]["outcome"]["status"]["phase"],
            "Scheduled"
        );
        let root = CanisterId::from_text(pending["root"]["canister"].as_str().unwrap()).unwrap();
        let child_dir = journal.join(id).join("child");
        let signed = std::fs::read(child_dir.join("stage/signed.json")).unwrap();
        // Crash after admission, before retaining its reply: same aggregate approval resumes.
        std::fs::remove_file(child_dir.join("stage/reply.bin")).unwrap();
        assert_eq!(
            cli(&config, &apply)["children"][0]["outcome"]["status"]["phase"],
            "Scheduled"
        );
        assert_eq!(
            std::fs::read(child_dir.join("stage/signed.json")).unwrap(),
            signed
        );
        owner.client().stop_progress().await;
        owner.client().advance_time(Duration::from_secs(45)).await;
        settle(owner.client()).await;
        owner.client().auto_progress().await;
        let view = cli(&config, &["fleet", "show", "--fleet", fleet]);
        assert_eq!(view["issues"], serde_json::json!([]));
        assert_eq!(view["canisters"].as_array().unwrap().len(), 3);
        assert_eq!(view["managers"][0]["reported"]["phase"], "Ready");
        assert!(!child_dir.join("receipt.json").exists());
        let completed = cli(&config, &apply);
        let child = CanisterId::from_text(
            completed["children"][0]["outcome"]["canister"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            completed["children"][0]["outcome"]["controller"],
            root.to_text()
        );
        let calls = status(owner.client(), root).await.attempts;
        assert_eq!(calls, [1; 3]);
        let repeat = cli(
            &config,
            &[
                "fleet",
                "apply",
                id,
                "--fleet",
                fleet,
                "--identity",
                "anonymous",
                "--icp-bin",
                "must-not-run",
            ],
        );
        assert_eq!(repeat, completed);
        assert_eq!(status(owner.client(), root).await.attempts, calls);
        let unchanged = cli(&config, &plan_args);
        assert_eq!(unchanged["action"], "unchanged");
        assert_eq!(unchanged["operation"], id);
        if !profile {
            assert_eq!(
                greeting::call(owner.client(), child, "fleet").await,
                "Hello from Canic, fleet!"
            );
        } else if fleet == "first" {
            let reply = owner
                .client()
                .update_call(
                    child,
                    alice,
                    "put",
                    ic_wire::encode(
                        &canic_profile_protocol::Put {
                            user_id: 7,
                            expected_revision: 0,
                            display_name: "Keep this profile".into(),
                        },
                        ic_wire::Encoding::Frame {
                            method: canic_profile_protocol::PUT,
                            direction: ic_wire::Direction::Request,
                        },
                        canic_profile_protocol::LIMITS,
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
            retained_profile = Some(
                ic_wire::decode::<canic_profile_protocol::ProfileResult>(
                    &reply,
                    ic_wire::Encoding::Frame {
                        method: canic_profile_protocol::PUT,
                        direction: ic_wire::Direction::Response,
                    },
                    canic_profile_protocol::LIMITS,
                )
                .unwrap()
                .unwrap(),
            );
        }
        identities.push((
            root,
            child,
            CanisterId::from_text(completed["store"]["canister"].as_str().unwrap()).unwrap(),
        ));
    }
    // Rebuild only the app input. The planner derives Root, child and old code from custody.
    let replacement = artifacts.join(if profile {
        "profile-worker-v2.wasm"
    } else {
        "app-v2.wasm"
    });
    std::fs::copy(&replacement, workspace.join("artifacts/app.wasm")).unwrap();
    let dispatch = (owner.client().get_time().await.as_nanos_since_unix_epoch() / 1_000_000_000
        + 40)
        .to_string();
    let plan_args = [
        "fleet",
        "plan",
        "--fleet",
        "first",
        "--identity",
        "anonymous",
        "--execute-at",
        &dispatch,
    ];
    let publication = cli(&config, &plan_args);
    assert_eq!(publication["action"], "publication");
    cli(
        &config,
        &[
            "fleet",
            "apply",
            publication["operation"].as_str().unwrap(),
            "--fleet",
            "first",
            "--identity",
            "anonymous",
        ],
    );
    let upgraded = cli(&config, &plan_args);
    assert_eq!(upgraded["action"], "upgrade");
    assert_eq!(
        upgraded["plan"]["action"]["approval"]["target"]["canister"],
        identities[0].1.to_text()
    );
    assert_eq!(
        upgraded["plan"]["action"]["approval"]["owner"]["root"],
        identities[0].0.to_text()
    );
    assert_eq!(
        upgraded["plan"]["action"]["contract"],
        "canic:child-upgrade.v1"
    );
    assert_eq!(cli(&config, &plan_args), upgraded);
    let id = upgraded["operation"].as_str().unwrap();
    let operation_dir = PathBuf::from(upgraded["state_dir"].as_str().unwrap()).join(id);
    let apply = [
        "fleet",
        "apply",
        id,
        "--fleet",
        "first",
        "--identity",
        "anonymous",
    ];
    assert_eq!(cli(&config, &apply)["upgrade"]["phase"], "Scheduled");
    let signed = std::fs::read(operation_dir.join("stage/signed.json")).unwrap();
    std::fs::remove_file(operation_dir.join("stage/reply.bin")).unwrap();
    // First recover authenticated admission through a read; recovery cannot assume it.
    assert_eq!(cli(&config, &apply)["upgrade"]["phase"], "Scheduled");
    let mut reconcile = apply;
    reconcile[1] = "reconcile";
    assert_eq!(cli(&config, &reconcile)["upgrade"]["phase"], "Scheduled");
    assert_eq!(cli(&config, &plan_args), upgraded);
    owner.client().stop_progress().await;
    owner.client().advance_time(Duration::from_secs(45)).await;
    settle(owner.client()).await;
    owner.client().auto_progress().await;
    let receipt = cli(&config, &apply);
    assert_eq!(receipt["canister"], identities[0].1.to_text());
    assert_eq!(receipt["controller"], identities[0].0.to_text());
    assert_eq!(
        std::fs::read(operation_dir.join("stage/signed.json")).unwrap(),
        signed
    );
    assert_eq!(cli(&config, &apply), receipt);
    let unchanged = cli(&config, &plan_args);
    assert_eq!(unchanged["action"], "unchanged");
    assert_eq!(unchanged["operation"], id);
    let first_code = owner
        .client()
        .canister_status(identities[0].1, Some(identities[0].0))
        .await
        .unwrap()
        .module_hash;
    let second_code = owner
        .client()
        .canister_status(identities[1].1, Some(identities[1].0))
        .await
        .unwrap()
        .module_hash;
    assert_eq!(
        first_code,
        Some(digest(&std::fs::read(&replacement).unwrap()).to_vec())
    );
    assert_ne!(first_code, second_code);
    // Root maintenance uses the ordinary plan/apply workflow and preserves the complete journal.
    let original_root = std::fs::read(workspace.join("artifacts/root.wasm")).unwrap();
    std::fs::copy(
        artifacts.join(if profile {
            "profile-maintenance.wasm"
        } else {
            "maintenance.wasm"
        }),
        workspace.join("artifacts/root.wasm"),
    )
    .unwrap();
    let maintenance = cli(&config, &plan_args);
    assert_eq!(maintenance["action"], "maintenance");
    assert_eq!(
        maintenance["plan"]["action"]["approval"]["target"]["canister"],
        identities[0].0.to_text()
    );
    assert_eq!(
        maintenance["plan"]["action"]["approval"]["children"][0]["upgrades"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(cli(&config, &plan_args), maintenance);
    let maintenance_id = maintenance["operation"].as_str().unwrap();
    let maintenance_dir =
        PathBuf::from(maintenance["state_dir"].as_str().unwrap()).join(maintenance_id);
    let saved_child = std::fs::read(workspace.join("artifacts/app.wasm")).unwrap();
    std::fs::copy(
        artifacts.join(if profile {
            "profile-worker.wasm"
        } else {
            "app.wasm"
        }),
        workspace.join("artifacts/app.wasm"),
    )
    .unwrap();
    let competing = std::process::Command::new(&executable)
        .args(plan_args)
        .args(["--config", config.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert!(!competing.status.success());
    let error: serde_json::Value = serde_json::from_slice(&competing.stderr).unwrap();
    assert_eq!(error["error"], "blocked");
    std::fs::write(workspace.join("artifacts/app.wasm"), saved_child).unwrap();
    assert_eq!(cli(&config, &plan_args), maintenance);
    let maintenance_apply = [
        "fleet",
        "apply",
        maintenance_id,
        "--fleet",
        "first",
        "--identity",
        "anonymous",
    ];
    if untracked {
        let wasm = std::fs::read(artifacts.join("app.wasm")).unwrap();
        let input = canic_management_protocol::upgrade::Stage {
            approval: canic_management_protocol::upgrade::Approval {
                manager: identities[0].0.to_text(),
                child: identities[0].1.to_text(),
                previous_module: digest(&std::fs::read(&replacement).unwrap()),
                module_hash: digest(&wasm),
                execute_at_ns: owner.client().get_time().await.as_nanos_since_unix_epoch()
                    + 40_000_000_000,
                expires_at_ns: owner.client().get_time().await.as_nanos_since_unix_epoch()
                    + 640_000_000_000,
            },
            wasm,
        };
        let store = ChildStore::bound(owner.client(), identities[0].0, identities[0].2);
        let response = owner
            .client()
            .update_call(
                identities[0].0,
                CanisterId::anonymous(),
                "stage_child_upgrade",
                store
                    .frame(
                        application::STAGE_CHILD_UPGRADE,
                        canic_management_protocol::children::ChildId::FIRST,
                        &input.approval,
                        &input.wasm,
                    )
                    .await,
            )
            .await
            .unwrap();
        let admitted: Result<canic_management_protocol::upgrade::Status, Refusal> =
            ic_wire::decode(
                &response,
                ic_wire::Encoding::Frame {
                    method: application::STAGE_CHILD_UPGRADE,
                    direction: ic_wire::Direction::Response,
                },
                LIMITS,
            )
            .unwrap();
        assert_eq!(
            admitted.unwrap().phase,
            canic_management_protocol::upgrade::Phase::Scheduled
        );
        let refused = std::process::Command::new(&executable)
            .args(maintenance_apply)
            .args(["--config", config.to_str().unwrap(), "--json"])
            .output()
            .unwrap();
        assert!(!refused.status.success());
        let error: serde_json::Value = serde_json::from_str(
            String::from_utf8(refused.stderr)
                .unwrap()
                .lines()
                .last()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(error["error"], "blocked");
        assert!(!maintenance_dir.join("install/signed.json").exists());
        assert!(!maintenance_dir.join("receipt.json").exists());
        assert!(maintenance_dir.join("custody-before/reply.bin").exists());
        assert_eq!(
            owner
                .client()
                .canister_status(identities[0].0, None)
                .await
                .unwrap()
                .module_hash,
            Some(digest(&original_root).to_vec())
        );
        assert_eq!(
            owner.close(std::time::Duration::from_secs(10)).await,
            ic_testkit::server::Deletion::Attempted
        );
        let stopped = server.stop().unwrap();
        assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
        return;
    }
    let maintained = cli(&config, &maintenance_apply);
    assert_eq!(maintained["canister"], identities[0].0.to_text());
    assert_eq!(maintained["controller"], "2vxsx-fae");
    // A receipt without the retained post-install journal cannot unlock future work.
    let after_reply = maintenance_dir.join("custody-after/reply.bin");
    let saved_reply = maintenance_dir.join("custody-after/fixture-held-reply.bin");
    std::fs::rename(&after_reply, &saved_reply).unwrap();
    let missing = std::process::Command::new(&executable)
        .args(plan_args)
        .args(["--config", config.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert!(!missing.status.success());
    let error: serde_json::Value = serde_json::from_slice(&missing.stderr).unwrap();
    assert_eq!(error["error"], "invalid");
    std::fs::rename(saved_reply, after_reply).unwrap();
    let install_attempt = std::fs::read(maintenance_dir.join("install/signed.json")).unwrap();
    let root_version = owner
        .client()
        .canister_status(identities[0].0, None)
        .await
        .unwrap()
        .version;
    // Lost host completion after an accepted install resumes the original signed attempt.
    std::fs::remove_file(maintenance_dir.join("receipt.json")).unwrap();
    std::fs::remove_file(maintenance_dir.join("install/reply.bin")).unwrap();
    assert_eq!(cli(&config, &maintenance_apply), maintained);
    assert_eq!(
        std::fs::read(maintenance_dir.join("install/signed.json")).unwrap(),
        install_attempt
    );
    assert_eq!(
        owner
            .client()
            .canister_status(identities[0].0, None)
            .await
            .unwrap()
            .version,
        root_version
    );
    assert_eq!(cli(&config, &plan_args)["action"], "unchanged");
    assert_eq!(
        owner
            .client()
            .canister_status(identities[1].0, None)
            .await
            .unwrap()
            .module_hash,
        Some(digest(&original_root).to_vec())
    );
    // Two more approvals exercise a code-hash cycle without replaying old work.
    let old_receipt = std::fs::read(operation_dir.join("receipt.json")).unwrap();
    for input_file in [
        if profile {
            "profile-worker.wasm"
        } else {
            "app.wasm"
        },
        if profile {
            "profile-worker-v2.wasm"
        } else {
            "app-v2.wasm"
        },
    ] {
        std::fs::copy(
            artifacts.join(input_file),
            workspace.join("artifacts/app.wasm"),
        )
        .unwrap();
        let dispatch =
            (owner.client().get_time().await.as_nanos_since_unix_epoch() / 1_000_000_000 + 40)
                .to_string();
        let args = [
            "fleet",
            "plan",
            "--fleet",
            "first",
            "--identity",
            "anonymous",
            "--execute-at",
            &dispatch,
        ];
        let mut next = cli(&config, &args);
        if next["action"] == "publication" {
            cli(
                &config,
                &[
                    "fleet",
                    "apply",
                    next["operation"].as_str().unwrap(),
                    "--fleet",
                    "first",
                    "--identity",
                    "anonymous",
                ],
            );
            next = cli(&config, &args);
        }
        assert_eq!(next["action"], "upgrade");
        assert_eq!(
            next["plan"]["action"]["approval"]["owner"]["root_module"],
            maintained["module_hash"]
        );
        let next_id = next["operation"].as_str().unwrap();
        assert_ne!(next_id, id);
        assert_eq!(cli(&config, &args), next);
        let apply = [
            "fleet",
            "apply",
            next_id,
            "--fleet",
            "first",
            "--identity",
            "anonymous",
        ];
        assert_eq!(cli(&config, &apply)["upgrade"]["phase"], "Scheduled");
        owner.client().stop_progress().await;
        owner.client().advance_time(Duration::from_secs(45)).await;
        settle(owner.client()).await;
        owner.client().auto_progress().await;
        let completed = cli(&config, &apply);
        assert_eq!(completed["canister"], identities[0].1.to_text());
        assert_eq!(cli(&config, &args)["action"], "unchanged");
        let view = cli(&config, &["fleet", "show", "--fleet", "first"]);
        assert_eq!(view["issues"], serde_json::json!([]));
        assert_eq!(view["managers"][0]["upgrade"]["operation"], next_id);
        assert_eq!(view["managers"][0]["upgrade"]["reported"]["phase"], "Ready");
        assert_eq!(
            std::fs::read(operation_dir.join("receipt.json")).unwrap(),
            old_receipt
        );
    }
    if let Some(expected) = retained_profile {
        for (caller, expected) in [
            (alice, Ok(expected)),
            (
                CanisterId::self_authenticating(b"outsider"),
                Err(canic_profile_protocol::ProfileError::Forbidden),
            ),
        ] {
            let reply = owner
                .client()
                .query_call(
                    identities[0].1,
                    caller,
                    "get",
                    ic_wire::encode(
                        &7_u64,
                        ic_wire::Encoding::Frame {
                            method: canic_profile_protocol::GET,
                            direction: ic_wire::Direction::Request,
                        },
                        canic_profile_protocol::LIMITS,
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                ic_wire::decode::<canic_profile_protocol::ProfileResult>(
                    &reply,
                    ic_wire::Encoding::Frame {
                        method: canic_profile_protocol::GET,
                        direction: ic_wire::Direction::Response
                    },
                    canic_profile_protocol::LIMITS
                )
                .unwrap(),
                expected
            );
        }
    } else {
        assert_eq!(
            greeting::call(owner.client(), identities[0].1, "fleet").await,
            "Hello again from Canic, fleet!"
        );
    }
    assert_ne!(identities[0].0, identities[1].0);
    assert_ne!(identities[0].1, identities[1].1);
    let subnet = owner.client().get_subnet(routing).await.unwrap();
    assert_eq!(
        owner
            .client()
            .get_subnet_metrics(subnet)
            .await
            .unwrap()
            .num_canisters,
        7
    );
    assert_eq!(std::fs::read(config).unwrap(), config_bytes);
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
