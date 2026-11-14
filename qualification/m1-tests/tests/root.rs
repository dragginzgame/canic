//! Installed Root ownership and preservation, using synthetic cycles only.
use canic_management_protocol::{
    Approval, CREATION_CYCLES, LIMITS, Phase, Refusal, STAGE, STAGE_LIMITS, STATUS, Stage, Status,
    digest,
};
use ic_testkit::{
    pic::{PocketIcStartupConfig, server_assets},
    pocket_ic::{CanisterId, PocketIcBuilder, nonblocking::PocketIc},
};
use std::{path::PathBuf, time::Duration};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

async fn status(pic: &PocketIc, root: CanisterId) -> Status {
    let reply = pic
        .query_call(
            root,
            CanisterId::anonymous(),
            "status",
            ic_wire::native::encode_request(STATUS, &(), LIMITS).unwrap(),
        )
        .await
        .unwrap();
    ic_wire::native::decode_response(STATUS, &reply, LIMITS).unwrap()
}

async fn stage(pic: &PocketIc, root: CanisterId, input: &Stage) -> Result<Status, Refusal> {
    let reply = pic
        .update_call(
            root,
            CanisterId::anonymous(),
            "stage",
            ic_wire::native::encode_request(STAGE, input, STAGE_LIMITS).unwrap(),
        )
        .await
        .unwrap();
    ic_wire::native::decode_response(STAGE, &reply, LIMITS).unwrap()
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
    let binary = server_assets::check(&repo().join("target/m0/server")).expect("make m0-setup");
    let owner = PocketIcStartupConfig::Spawn {
        server_binary: binary,
        timeout: Duration::from_secs(30),
        server_hard_ttl: Some(Duration::from_secs(180)),
        server_idle_ttl: None,
        server_output_files: None,
    }
    .build_instance(PocketIcBuilder::new().with_application_subnet())
    .await
    .unwrap();
    let pic = owner.instance();
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
            ic_wire::native::encode_request(STAGE, &input, STAGE_LIMITS).unwrap(),
        )
        .await
        .is_err()
    );
    assert_eq!(status(pic, root).await.phase, Phase::Empty);
    let admitted = stage(pic, root, &input).await.unwrap();
    assert_eq!(admitted.phase, Phase::Scheduled);
    // Scheduled migration must resume original bytes and authority.
    if before_dispatch {
        pic.upgrade_canister(root, replacement.clone(), vec![], None)
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
        pic.upgrade_canister(root, replacement, vec![], None)
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
                ic_wire::native::encode_request(canic::GREET, &"team", canic::LIMITS).unwrap(),
            )
            .await
            .unwrap();
        let reply: canic::Greeting =
            ic_wire::native::decode_response(canic::GREET, &reply, canic::LIMITS).unwrap();
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
    owner.close().await;
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
#[ignore = "requires CANIC_PREVIOUS_MANAGER naming the retained pre-extraction Wasm"]
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
#[ignore = "requires CANIC_PREVIOUS_ROOT naming the retained two-store Root Wasm"]
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
    let binary = server_assets::check(&repo().join("target/m0/server")).unwrap();
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
    // Only local subnet routing is fixture setup. Both fleet Roots and apps are
    // created through the shipped CLI and its retained operation owners.
    let routing = owner.instance().create_canister().await;
    let artifacts = repo().join("target/root");
    let root_wasm = std::fs::read(artifacts.join("root.wasm")).unwrap();
    let root_hash: String = digest(&root_wasm)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let url = owner.instance_mut().make_live(None).await.to_string();
    let key: String = owner
        .instance()
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
    let config = workspace.join("canic.json");
    let executable = std::env::var_os("CANIC_CLI").unwrap();
    let generated = std::process::Command::new(&executable)
        .args([
            "app",
            "init",
            "starter",
            "--fleet",
            "local",
            "--network",
            "local",
            "--url",
            &url,
            "--root-key",
            &key,
            "--routing-canister",
            &routing.to_text(),
            "--source",
            workspace.to_str().unwrap(),
            "--canic-source",
            repo().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
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
    let drift_config = workspace.join("authority-check.json");
    let mut declaration: serde_json::Value = serde_json::from_slice(&config_bytes).unwrap();
    declaration["apps"]["starter"]["fleets"]["local"]["canisters"]["root"] =
        serde_json::json!({"id":root.to_text()});
    std::fs::write(&drift_config, serde_json::to_vec(&declaration).unwrap()).unwrap();
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
    // Manual time changes must not race the live gateway's automatic clock.
    owner.instance().stop_progress().await;
    owner.instance().advance_time(Duration::from_secs(35)).await;
    settle(owner.instance()).await;
    owner.instance().auto_progress().await;
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
            .contains("Root report (uncertified): root — creation Ready")
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
    assert!(!journal.join(id).join("observe/1").exists());
    assert_eq!(
        std::fs::read(journal.join(id).join("stage/signed.json")).unwrap(),
        signed
    );
    let receipt = cli(&config, &apply);
    assert_eq!(receipt["controller"], root.to_text());
    let child = CanisterId::from_text(receipt["canister"].as_str().unwrap()).unwrap();
    assert_eq!(
        owner
            .instance()
            .canister_status(child, Some(root))
            .await
            .unwrap()
            .settings
            .controllers,
        [root]
    );
    assert_eq!(cli(&config, &apply), receipt);
    assert_eq!(
        cli(&config, &["fleet", "call", id, "--name", "team"])["greeting"],
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
    let ready = status(owner.instance(), root).await;
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
            "--routing-canister",
            &routing.to_text(),
        ],
    );
    assert_eq!(initialized["deployed"], false);
    let second_journal = workspace.join(".canic/apps/starter/fleets/second");
    assert!(!second_journal.exists());
    let declarations: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&config).unwrap()).unwrap();
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
    let dispatch = (owner
        .instance()
        .get_time()
        .await
        .as_nanos_since_unix_epoch()
        / 1_000_000_000
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
    owner.instance().stop_progress().await;
    owner.instance().advance_time(Duration::from_secs(35)).await;
    settle(owner.instance()).await;
    owner.instance().auto_progress().await;
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
        cli(
            &config,
            &[
                "fleet", "call", second_id, "--fleet", "second", "--name", "second"
            ]
        )["greeting"],
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
    assert_eq!(status(owner.instance(), root).await, ready);
    assert_eq!(status(owner.instance(), second_root).await.attempts, [1; 3]);
    let subnet = owner.instance().get_subnet(routing).await.unwrap();
    assert_eq!(
        owner
            .instance()
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
    let dispatch = (owner
        .instance()
        .get_time()
        .await
        .as_nanos_since_unix_epoch()
        / 1_000_000_000
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
    owner.instance().stop_progress().await;
    owner.instance().advance_time(Duration::from_secs(35)).await;
    settle(owner.instance()).await;
    owner.instance().auto_progress().await;
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
    assert!(!upgrade_dir.join("observe").exists());
    assert!(!upgrade_dir.join("receipt.json").exists());
    let retained = cli(
        &config,
        &["fleet", "status", upgrade_id, "--fleet", "local"],
    );
    assert!(retained["management"].is_null());
    assert!(retained["receipt"].is_null());
    assert!(
        status_text(&config, upgrade_id, "local").contains("admission and progress are unknown")
    );
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
        cli(
            &config,
            &[
                "fleet", "call", upgrade_id, "--fleet", "local", "--name", "team"
            ]
        )["greeting"],
        "Hello again from Canic, team!"
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
    assert_eq!(status(owner.instance(), root).await, ready);
    assert_eq!(
        cli(
            &config,
            &[
                "fleet", "call", second_id, "--fleet", "second", "--name", "second"
            ]
        )["greeting"],
        "Hello from Canic, second!"
    );
    let view = cli(&config, &["fleet", "show", "--fleet", "local"]);
    assert_eq!(view["issues"], serde_json::json!([]));
    assert_eq!(view["canisters"].as_array().unwrap().len(), 2);
    assert_eq!(view["operations"].as_array().unwrap().len(), 3);
    assert_eq!(
        owner
            .instance()
            .get_subnet_metrics(subnet)
            .await
            .unwrap()
            .num_canisters,
        5
    );
    println!(
        "CLI Root upgrade: same child, replacement greeting, retained creation receipt, lost-reply recovery and second fleet isolation passed"
    );
    owner.close().await;
}
