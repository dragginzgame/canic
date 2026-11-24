//! Installed Root ownership and preservation, using synthetic cycles only.
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

// Keep the retained inline-custody witnesses explicit. The publication runner
// qualifies the generated default Root/Store/app topology through the CLI.
fn inline_fixture(config: &std::path::Path, app: &str) {
    let mut value = canic_host::load_config(config).unwrap();
    let slots = &mut value.apps.get_mut(app).unwrap().canisters;
    assert!(slots.remove("store").is_some());
    std::fs::write(config, toml::to_string(&value).unwrap()).unwrap();
}

async fn status(pic: &PocketIc, root: CanisterId) -> Status {
    let reply = pic
        .query_call(
            root,
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

async fn stage(pic: &PocketIc, root: CanisterId, input: &Stage) -> Result<Status, Refusal> {
    let reply = pic
        .update_call(
            root,
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
            "stage",
            ic_wire::encode(
                &input,
                ic_wire::Encoding::Frame {
                    method: STAGE,
                    direction: ic_wire::Direction::Request
                },
                STAGE_LIMITS
            )
            .unwrap(),
        )
        .await
        .is_err()
    );
    assert_eq!(status(pic, root).await.phase, Phase::Empty);
    let admitted = stage(pic, root, &input).await.unwrap();
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
    assert_eq!(count, 2);
    assert_eq!(stage(pic, root, &input).await.unwrap(), ready);
    let mut changed = input.clone();
    changed.approval.expires_at_ns += 1;
    assert_eq!(stage(pic, root, &changed).await, Err(Refusal::Occupied));
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
async fn managed_child_cli_retains_admission_and_recovers_lost_reply() {
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
    inline_fixture(&config, "starter");
    std::fs::create_dir(workspace.join("artifacts")).unwrap();
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
    // Also retain coverage of explicitly configured existing Roots. This separate
    // observation config is used only to test rejected authority, not deployment.
    let drift_config = workspace.join("authority-check.toml");
    let mut declaration: serde_json::Value =
        toml::from_str(std::str::from_utf8(&config_bytes).unwrap()).unwrap();
    declaration["apps"]["starter"]["fleets"]["local"]["canisters"]["root"] =
        serde_json::json!({"id":root.to_text()});
    std::fs::write(&drift_config, toml::to_string(&declaration).unwrap()).unwrap();
    let execute_at = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 30)
        .to_string();
    let planned = cli(
        &config,
        &[
            "fleet",
            "plan",
            "--canister",
            "app",
            "--controller",
            "2vxsx-fae",
            "--root-module",
            &root_hash,
            "--execute-at",
            &execute_at,
        ],
    );
    let id = planned["operation"].as_str().unwrap();
    let journal = std::path::PathBuf::from(planned["state_dir"].as_str().unwrap());
    let icp = repo().join(".tools/ic/bin/icp");
    // Authority drift is refused before the signer or staging path is touched.
    for (label, controller, module) in [
        ("code-drift", "2vxsx-fae".to_owned(), "00".repeat(32)),
        (
            "controller-drift",
            CanisterId::self_authenticating(b"outsider").to_text(),
            root_hash.clone(),
        ),
    ] {
        let scratch = workspace.join(label);
        let scratch = scratch.to_str().unwrap();
        let bad = cli(
            &drift_config,
            &[
                "fleet",
                "plan",
                "--canister",
                "app",
                "--controller",
                &controller,
                "--root-module",
                &module,
                "--state-dir",
                scratch,
            ],
        );
        let rejected = std::process::Command::new(&executable)
            .args([
                "fleet",
                "apply",
                bad["operation"].as_str().unwrap(),
                "--config",
                drift_config.to_str().unwrap(),
                "--state-dir",
                scratch,
                "--identity",
                "anonymous",
                "--icp-bin",
                "must-not-run",
                "--json",
            ])
            .output()
            .unwrap();
        assert!(!rejected.status.success());
        let error: serde_json::Value = serde_json::from_slice(&rejected.stderr).unwrap();
        assert_eq!(error["error"], "blocked");
        assert!(
            !std::path::Path::new(scratch)
                .join(bad["operation"].as_str().unwrap())
                .join("stage")
                .exists()
        );
    }
    let apply = [
        "fleet",
        "apply",
        id,
        "--identity",
        "anonymous",
        "--icp-bin",
        icp.to_str().unwrap(),
    ];
    let pending = cli(&config, &apply);
    assert_eq!(pending["status"]["phase"], "Scheduled");
    let scheduled_view = cli(&config, &["fleet", "show"]);
    assert_eq!(scheduled_view["managers"][0]["manager"], root.to_text());
    assert_eq!(
        scheduled_view["managers"][0]["reported"]["phase"],
        "Scheduled"
    );
    assert!(!journal.join(id).join("receipt.json").exists());
    let signed = std::fs::read(journal.join(id).join("stage/signed.json")).unwrap();
    // Model a crash after Root admitted work but before the host retained the response.
    std::fs::remove_file(journal.join(id).join("stage/reply.bin")).unwrap();
    let recovered = cli(&config, &apply);
    assert_eq!(recovered["status"]["phase"], "Scheduled");
    assert_eq!(
        std::fs::read(journal.join(id).join("stage/signed.json")).unwrap(),
        signed
    );
    let mut recovery = apply;
    recovery[1] = "reconcile";
    assert_eq!(cli(&config, &recovery)["status"]["phase"], "Scheduled");
    let retained_recovery = journal.join(id).join("observe/1/reconcile");
    assert!(retained_recovery.join("signed.json").exists());
    assert!(retained_recovery.join("reply.bin").exists());
    assert_eq!(
        cli(&config, &["fleet", "status", id])["effects"]["reconcile/1"],
        "reply_retained"
    );
    // Manual time changes must not race the live gateway's automatic clock.
    owner.client().stop_progress().await;
    owner.client().advance_time(Duration::from_secs(35)).await;
    settle(owner.client()).await;
    owner.client().auto_progress().await;
    let report = workspace.join("fleet.html");
    let ready_view = cli(
        &config,
        &["fleet", "show", "--html", report.to_str().unwrap()],
    );
    let html = std::fs::read_to_string(&report).unwrap();
    assert!(html.contains("Root report (uncertified)"));
    assert!(html.contains("reports child"));
    let terminal = std::process::Command::new(&executable)
        .args(["fleet", "show", "--config", config.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(terminal.status.success());
    assert!(
        std::str::from_utf8(&terminal.stdout)
            .unwrap()
            .contains("Root report (uncertified): root / slot 1 · app — creation Ready")
    );
    assert_eq!(ready_view["managers"][0]["reported"]["phase"], "Ready");
    assert_eq!(ready_view["canisters"].as_array().unwrap().len(), 2);
    let child_operation = ready_view["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|operation| operation["operation"] == id)
        .unwrap();
    assert!(child_operation["canister"].is_null());
    assert_eq!(child_operation["receipt_retained"], false);
    assert!(!journal.join(id).join("receipt.json").exists());
    assert!(!journal.join(id).join("observe/2").exists());
    assert_eq!(
        std::fs::read(journal.join(id).join("stage/signed.json")).unwrap(),
        signed
    );
    let receipt = cli(&config, &apply);
    assert_eq!(receipt["controller"], root.to_text());
    let child = CanisterId::from_text(receipt["canister"].as_str().unwrap()).unwrap();
    assert_eq!(
        owner
            .client()
            .canister_status(child, Some(root))
            .await
            .unwrap()
            .settings
            .controllers,
        [root]
    );
    assert_eq!(cli(&config, &apply), receipt);
    assert_eq!(
        greeting::call(owner.client(), child, "team").await,
        "Hello from Canic, team!"
    );
    let state = cli(&config, &["fleet", "status", id]);
    assert_eq!(state["management"]["phase"], "Ready");
    let snapshot = cli(&config, &["fleet", "show"]);
    assert_eq!(snapshot["issues"], serde_json::json!([]));
    assert_eq!(
        snapshot["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|operation| operation["operation"] == id)
            .unwrap()["approved_controller"],
        root.to_text()
    );
    // Completion recovery needs retained evidence and fresh metadata, not a new signature.
    std::fs::remove_file(journal.join(id).join("receipt.json")).unwrap();
    assert_eq!(
        cli(
            &config,
            &[
                "fleet",
                "apply",
                id,
                "--identity",
                "anonymous",
                "--icp-bin",
                "must-not-run"
            ]
        ),
        receipt
    );
    let ready = status(owner.client(), root).await;
    assert_eq!(ready.attempts, [1; 3]);
    let first_receipt_bytes = std::fs::read(journal.join(id).join("receipt.json")).unwrap();
    let initialized = cli(
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
    assert_eq!(initialized["deployed"], false);
    let second_journal = workspace.join(".canic/apps/starter/fleets/second");
    assert!(!second_journal.exists());
    let declarations: serde_json::Value =
        toml::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
    assert_eq!(
        declarations["apps"]["starter"]["canisters"],
        declaration["apps"]["starter"]["canisters"]
    );
    for fleet in ["local", "second"] {
        assert_eq!(
            declarations["apps"]["starter"]["fleets"][fleet]["canisters"],
            serde_json::json!({})
        );
    }
    let second_root_receipt = deploy_root(&config, "second");
    let second_root =
        CanisterId::from_text(second_root_receipt["canister"].as_str().unwrap()).unwrap();
    assert_ne!(root, second_root);
    let dispatch = (owner.client().get_time().await.as_nanos_since_unix_epoch() / 1_000_000_000
        + 30)
        .to_string();
    let second_plan_args = [
        "fleet",
        "plan",
        "--fleet",
        "second",
        "--canister",
        "app",
        "--controller",
        "2vxsx-fae",
        "--root-module",
        &root_hash,
        "--execute-at",
        &dispatch,
    ];
    let second_plan = cli(&config, &second_plan_args);
    let second_id = second_plan["operation"].as_str().unwrap();
    assert_ne!(second_id, id);
    assert_eq!(
        std::path::Path::new(second_plan["state_dir"].as_str().unwrap()),
        second_journal
    );
    let second_apply = [
        "fleet",
        "apply",
        second_id,
        "--fleet",
        "second",
        "--identity",
        "anonymous",
        "--icp-bin",
        icp.to_str().unwrap(),
    ];
    assert_eq!(cli(&config, &second_apply)["status"]["phase"], "Scheduled");
    owner.client().stop_progress().await;
    owner.client().advance_time(Duration::from_secs(35)).await;
    settle(owner.client()).await;
    owner.client().auto_progress().await;
    let second_receipt = cli(&config, &second_apply);
    let second_child = CanisterId::from_text(second_receipt["canister"].as_str().unwrap()).unwrap();
    assert_eq!(second_receipt["controller"], second_root.to_text());
    assert_eq!(
        std::collections::BTreeSet::from([root, child, second_root, second_child]).len(),
        4
    );
    assert_eq!(cli(&config, &second_plan_args)["operation"], second_id);
    assert_eq!(cli(&config, &second_apply), second_receipt);
    assert_eq!(
        greeting::call(owner.client(), second_child, "second").await,
        "Hello from Canic, second!"
    );
    for (fleet, fleet_root, fleet_child) in [
        ("local", root, child),
        ("second", second_root, second_child),
    ] {
        let view = cli(&config, &["fleet", "show", "--fleet", fleet]);
        assert_eq!(view["issues"], serde_json::json!([]));
        let identities: std::collections::BTreeSet<_> = view["canisters"]
            .as_array()
            .unwrap()
            .iter()
            .map(|canister| canister["canister"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(
            identities,
            std::collections::BTreeSet::from([fleet_root.to_text(), fleet_child.to_text()])
        );
        assert_eq!(view["operations"].as_array().unwrap().len(), 2);
        assert!(
            view["operations"]
                .as_array()
                .unwrap()
                .iter()
                .all(|operation| operation["deployment"]["fleet"] == fleet)
        );
    }
    // Selecting the second fleet must not replay the first fleet's operation,
    // even when its journal is explicitly supplied. Refuse before invoking a signer.
    let foreign = std::process::Command::new(&executable)
        .args([
            "fleet",
            "apply",
            id,
            "--config",
            config.to_str().unwrap(),
            "--fleet",
            "second",
            "--state-dir",
            journal.to_str().unwrap(),
            "--identity",
            "anonymous",
            "--icp-bin",
            "must-not-run",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!foreign.status.success());
    let error: serde_json::Value = serde_json::from_slice(&foreign.stderr).unwrap();
    assert_eq!(error["error"], "invalid");
    assert_eq!(
        std::fs::read(journal.join(id).join("receipt.json")).unwrap(),
        first_receipt_bytes
    );
    assert_eq!(status(owner.client(), root).await, ready);
    assert_eq!(status(owner.client(), second_root).await.attempts, [1; 3]);
    let subnet = owner.client().get_subnet(routing).await.unwrap();
    assert_eq!(
        owner
            .client()
            .get_subnet_metrics(subnet)
            .await
            .unwrap()
            .num_canisters,
        5
    );
    println!(
        "CLI fleets: Root {root} / child {child}, Root {second_root} / child {second_child}; bootstrap, recovery, replay and fleet isolation passed"
    );
    // The app definition selects replacement bytes; the fleet and retained
    // creation receipt select the existing child and its enduring Root.
    std::fs::copy(
        artifacts.join("app-v2.wasm"),
        workspace.join("artifacts/app.wasm"),
    )
    .unwrap();
    let dispatch = (owner.client().get_time().await.as_nanos_since_unix_epoch() / 1_000_000_000
        + 30)
        .to_string();
    let child_text = child.to_text();
    let upgrade_args = [
        "fleet",
        "plan",
        "--fleet",
        "local",
        "--canister",
        "app",
        "--target",
        &child_text,
        "--previous-module",
        receipt["module_hash"].as_str().unwrap(),
        "--root-module",
        &root_hash,
        "--controller",
        "2vxsx-fae",
        "--execute-at",
        &dispatch,
    ];
    let upgrade = cli(&config, &upgrade_args);
    let upgrade_id = upgrade["operation"].as_str().unwrap();
    assert_ne!(upgrade_id, id);
    assert_eq!(cli(&config, &upgrade_args), upgrade);
    let upgrade_dir = journal.join(upgrade_id);
    let unadmitted = cli(&config, &["fleet", "show", "--fleet", "local"]);
    assert_eq!(
        unadmitted["managers"][0]["upgrade"]["operation"],
        upgrade_id
    );
    assert_eq!(
        unadmitted["managers"][0]["upgrade"]["reported"]["phase"],
        "Empty"
    );
    assert!(!upgrade_dir.join("stage").exists());
    let upgrade_apply = [
        "fleet",
        "apply",
        upgrade_id,
        "--fleet",
        "local",
        "--identity",
        "anonymous",
        "--icp-bin",
        icp.to_str().unwrap(),
    ];
    assert_eq!(
        cli(&config, &upgrade_apply)["upgrade"]["phase"],
        "Scheduled"
    );
    let mut recovery = upgrade_apply;
    recovery[1] = "reconcile";
    assert_eq!(cli(&config, &recovery)["upgrade"]["phase"], "Scheduled");
    let pending_text = status_text(&config, upgrade_id, "local");
    assert!(pending_text.contains("Retained Root phase: Scheduled"));
    assert!(pending_text.contains("ROOT EFFECT"));
    assert!(pending_text.contains("observe Root with fleet show"));
    assert!(pending_text.contains("Child version: unknown → unknown"));
    let scheduled = cli(&config, &["fleet", "show", "--fleet", "local"]);
    assert_eq!(
        scheduled["managers"][0]["upgrade"]["reported"]["phase"],
        "Scheduled"
    );
    assert!(scheduled["managers"][0]["upgrade"]["observation_error"].is_null());
    let signed = std::fs::read(upgrade_dir.join("stage/signed.json")).unwrap();
    // Lost admission reply: exact original signed attempt stays in custody.
    std::fs::remove_file(upgrade_dir.join("stage/reply.bin")).unwrap();
    owner.client().stop_progress().await;
    owner.client().advance_time(Duration::from_secs(35)).await;
    settle(owner.client()).await;
    owner.client().auto_progress().await;
    let report = workspace.join("upgrade.html");
    let observed = cli(
        &config,
        &[
            "fleet",
            "show",
            "--fleet",
            "local",
            "--html",
            report.to_str().unwrap(),
        ],
    );
    assert_eq!(
        observed["managers"][0]["upgrade"]["reported"]["phase"],
        "Ready"
    );
    assert_eq!(observed["managers"][0]["reported"]["phase"], "Ready");
    assert!(
        std::fs::read_to_string(report)
            .unwrap()
            .contains("Upgrade phase: <strong>Ready</strong>")
    );
    assert!(!upgrade_dir.join("stage/reply.bin").exists());
    assert!(!upgrade_dir.join("observe/1").exists());
    assert!(!upgrade_dir.join("receipt.json").exists());
    let retained = cli(
        &config,
        &["fleet", "status", upgrade_id, "--fleet", "local"],
    );
    // Recovery evidence still establishes admission even without the staging reply.
    assert_eq!(retained["management"]["phase"], "Scheduled");
    assert!(retained["receipt"].is_null());
    assert!(status_text(&config, upgrade_id, "local").contains("Retained Root phase: Scheduled"));
    let other = cli(&config, &["fleet", "show", "--fleet", "second"]);
    assert!(other["managers"][0].get("upgrade").is_none());
    assert_eq!(other["operations"].as_array().unwrap().len(), 2);
    let upgraded = cli(&config, &upgrade_apply);
    assert_eq!(upgraded["canister"], child_text);
    assert_eq!(upgraded["controller"], root.to_text());
    assert_ne!(upgraded["module_hash"], receipt["module_hash"]);
    let completed_text = status_text(&config, upgrade_id, "local");
    assert!(completed_text.contains("Retained Root phase: Ready"));
    assert!(completed_text.contains("the retained receipt is historical evidence"));
    assert!(!completed_text.contains("Child version: unknown"));
    assert_eq!(
        std::fs::read(upgrade_dir.join("stage/signed.json")).unwrap(),
        signed
    );
    assert_eq!(
        cli(
            &config,
            &["fleet", "status", upgrade_id, "--fleet", "local"]
        )["management"]["phase"],
        "Ready"
    );
    assert_eq!(
        greeting::call(owner.client(), child, "team").await,
        "Hello again from Canic, team!"
    );
    let after_upgrade = status(owner.client(), root).await;
    // Recording the upgrade may grow Root's backing. Original creation evidence must survive.
    assert!(after_upgrade.stable_pages >= ready.stable_pages);
    assert_eq!(
        after_upgrade,
        Status {
            stable_pages: after_upgrade.stable_pages,
            ..ready
        }
    );
    assert_eq!(
        cli(
            &config,
            &[
                "fleet",
                "apply",
                upgrade_id,
                "--fleet",
                "local",
                "--identity",
                "anonymous",
                "--icp-bin",
                "must-not-run"
            ]
        ),
        upgraded
    );
    assert_eq!(
        std::fs::read(journal.join(id).join("receipt.json")).unwrap(),
        first_receipt_bytes
    );
    assert_eq!(status(owner.client(), root).await, after_upgrade);
    assert_eq!(
        greeting::call(owner.client(), second_child, "second").await,
        "Hello from Canic, second!"
    );
    let view = cli(&config, &["fleet", "show", "--fleet", "local"]);
    assert_eq!(view["issues"], serde_json::json!([]));
    assert_eq!(view["canisters"].as_array().unwrap().len(), 2);
    assert_eq!(view["operations"].as_array().unwrap().len(), 3);
    assert_eq!(
        owner
            .client()
            .get_subnet_metrics(subnet)
            .await
            .unwrap()
            .num_canisters,
        5
    );
    println!(
        "CLI Root upgrade: same child, replacement greeting, retained creation receipt, lost-reply recovery and second fleet isolation passed"
    );
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
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
    inline_fixture(&config, "starter");
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
        assert_eq!(view["canisters"].as_array().unwrap().len(), 2);
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
        identities.push((root, child));
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
    let upgraded = cli(&config, &plan_args);
    assert_eq!(upgraded["action"], "upgrade");
    assert_eq!(upgraded["plan"]["target"], identities[0].1.to_text());
    assert_eq!(
        upgraded["plan"]["management"]["root"],
        identities[0].0.to_text()
    );
    assert_eq!(upgraded["plan"]["creation_cycles"], 0);
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
    assert_eq!(maintenance["plan"]["target"], identities[0].0.to_text());
    assert_eq!(
        maintenance["plan"]["maintenance"]["upgrades"]
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
        let response = owner
            .client()
            .update_call(
                identities[0].0,
                CanisterId::anonymous(),
                "stage_upgrade",
                ic_wire::encode(
                    &input,
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
        let admitted: Result<canic_management_protocol::upgrade::Status, Refusal> =
            ic_wire::decode(
                &response,
                ic_wire::Encoding::Frame {
                    method: canic_management_protocol::upgrade::STAGE,
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
        let next = cli(&config, &args);
        assert_eq!(next["action"], "upgrade");
        assert_eq!(
            next["plan"]["management"]["root_module"],
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
        5
    );
    assert_eq!(std::fs::read(config).unwrap(), config_bytes);
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
