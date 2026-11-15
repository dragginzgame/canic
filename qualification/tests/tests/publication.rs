//! Real CLI Store bootstrap and publication through Root with retained recovery.
use ic_testkit::{
    pic::{PocketIcStartupConfig, server_assets},
    pocket_ic::{CanisterId, CanisterSettings, PocketIcBuilder},
};
use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::Duration,
};

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(std::env::var_os("CANIC_CLI").expect("make publication-test"))
        .args(args)
        .args(["--json", "--config"])
        .arg(dir.join("canic.json"))
        .output()
        .unwrap()
}
fn cli(dir: &Path, args: &[&str]) -> Value {
    let output = run(dir, args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn padded(mut bytes: Vec<u8>) -> Vec<u8> {
    // Valid custom section: more than one transfer, without changing executable behavior.
    let payload = 2_000_000_u32;
    bytes.push(0);
    let mut length = payload;
    loop {
        let byte = (length & 127) as u8;
        length >>= 7;
        bytes.push(byte | if length == 0 { 0 } else { 128 });
        if length == 0 {
            break;
        }
    }
    bytes.push(0);
    bytes.resize(bytes.len() + payload as usize - 1, 42);
    bytes
}

#[tokio::test(flavor = "current_thread")]
async fn fleet_bootstrap_gates_both_application_slots_on_store_readiness() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_owned();
    let assets = PathBuf::from(std::env::var_os("CANIC_PUBLICATION_ARTIFACTS").unwrap());
    let mut owner = PocketIcStartupConfig::Spawn {
        server_binary: server_assets::check(&canic_qualification::server_directory()).unwrap(),
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
    let routing = owner.instance().create_canister().await;
    let subnet = owner.instance().get_subnet(routing).await.unwrap();
    let url = owner.instance_mut().make_live(None).await.to_string();
    let root_key = hex(&owner.instance().root_key().await.unwrap());
    let dir = repo
        .join("target/publication")
        .join(format!("fleet-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let config = json!({"apps":{"demo":{
        "canisters":{"root":{"role":"root","wasm":assets.join("root.wasm")},
            "store":{"role":"wasm-store","parent":"root","wasm":assets.join("store.wasm")},
            "app":{"role":"app","parent":"root","wasm":assets.join("app.wasm")},
            "worker":{"role":"worker","parent":"root","wasm":assets.join("app.wasm")}},
        "fleets":{"local":{"network":{"kind":"Local","url":url,"root_key":root_key,"routing_canister":routing.to_text()},"journal":"journal","canisters":{}}}
    }}});
    std::fs::write(dir.join("canic.json"), serde_json::to_vec(&config).unwrap()).unwrap();
    let dispatch = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 90)
        .to_string();
    let planned = cli(
        &dir,
        &[
            "fleet",
            "plan",
            "--controller",
            "2vxsx-fae",
            "--execute-at",
            &dispatch,
        ],
    );
    let id = planned["operation"].as_str().unwrap();
    let path = dir.join("journal").join(id);
    assert_eq!(
        planned["fleet_plan"]["root"]["creation_cycles"],
        4_000_000_000_000_u64
    );
    assert_eq!(
        planned["fleet_plan"]["store"]["creation_cycles"],
        1_000_000_000_000_u64
    );
    let human = Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(["fleet", "status", id, "--config"])
        .arg(dir.join("canic.json"))
        .output()
        .unwrap();
    assert!(human.status.success());
    let text = String::from_utf8(human.stdout).unwrap();
    assert!(text.contains("store [WASM STORE]"));
    assert!(text.contains("Total operator debit ceiling: 5T"));
    let signer = repo.join(".tools/ic/bin/icp");
    let paused = dir.join("pause-store-ready");
    std::fs::write(&paused, "#!/bin/sh\nfor arg in \"$@\"; do\n case \"$arg\" in */store/store-observe/0/arguments.bin) exit 71;; esac\ndone\nexec \"$CANIC_REAL_SIGNER\" \"$@\"\n").unwrap();
    std::fs::set_permissions(&paused, std::fs::Permissions::from_mode(0o700)).unwrap();
    let stopped = Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(["fleet", "apply", id, "--json", "--config"])
        .arg(dir.join("canic.json"))
        .args(["--identity", "anonymous", "--icp-bin"])
        .arg(&paused)
        .env("CANIC_REAL_SIGNER", &signer)
        .output()
        .unwrap();
    assert!(!stopped.status.success());
    assert!(
        path.join("store/handover/reply.bin").exists(),
        "{}",
        String::from_utf8_lossy(&stopped.stderr)
    );
    for leaf in ["child", "sibling"] {
        assert!(!path.join(leaf).join("plan.json").exists());
    }
    assert_eq!(
        owner
            .instance()
            .get_subnet_metrics(subnet)
            .await
            .unwrap()
            .num_canisters,
        3
    );
    assert!(!run(&dir, &["fleet", "cancel", id]).status.success());
    assert_eq!(
        cli(&dir, &["fleet", "plan", "--controller", "2vxsx-fae"])["operation"],
        id
    );
    let root = CanisterId::from_text(
        cli(&dir, &["fleet", "status", id])["root"]["receipt"]["canister"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let root_before = owner.instance().get_stable_memory(root).await;
    let signed = std::fs::read(path.join("store/handover/signed.json")).unwrap();
    std::fs::remove_file(path.join("store/handover/reply.bin")).unwrap();
    // Reconcile may complete Store, but cannot admit either application child.
    let reconciled = run(
        &dir,
        &[
            "fleet",
            "reconcile",
            id,
            "--identity",
            "anonymous",
            "--icp-bin",
            signer.to_str().unwrap(),
        ],
    );
    assert!(!reconciled.status.success());
    let state = cli(&dir, &["fleet", "status", id]);
    assert!(!state["store"]["state"]["receipt"].is_null());
    assert!(
        state["children"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["state"].is_null())
    );
    assert_eq!(owner.instance().get_stable_memory(root).await, root_before);
    assert_eq!(
        std::fs::read(path.join("store/handover/signed.json")).unwrap(),
        signed
    );
    let apply = || {
        cli(
            &dir,
            &[
                "fleet",
                "apply",
                id,
                "--identity",
                "anonymous",
                "--icp-bin",
                signer.to_str().unwrap(),
            ],
        )
    };
    let admitted = apply();
    assert_eq!(admitted["children"].as_array().unwrap().len(), 2);
    let store = CanisterId::from_text(admitted["store"]["canister"].as_str().unwrap()).unwrap();
    assert_eq!(owner.instance().get_subnet(store).await, Some(subnet));
    owner.instance().stop_progress().await;
    owner.instance().advance_time(Duration::from_secs(95)).await;
    for _ in 0..40 {
        owner.instance().tick().await;
    }
    owner.instance().auto_progress().await;
    let complete = apply();
    for child in complete["children"].as_array().unwrap() {
        assert_eq!(child["outcome"]["controller"], root.to_text(), "{complete}");
        assert!(child["outcome"]["canister"].is_string());
    }
    assert_eq!(
        owner
            .instance()
            .get_subnet_metrics(subnet)
            .await
            .unwrap()
            .num_canisters,
        5
    );
    assert_eq!(apply(), complete);
    assert_eq!(
        cli(&dir, &["fleet", "plan", "--controller", "2vxsx-fae"])["action"],
        "unchanged"
    );
    let view = cli(&dir, &["fleet", "show"]);
    assert_eq!(view["canisters"].as_array().unwrap().len(), 4);
    assert!(view["issues"].as_array().unwrap().is_empty(), "{view}");
    // Readiness custody is part of fleet admission, not a disposable display summary.
    let proof = path.join("store/store-observe/0/reply.bin");
    let bytes = std::fs::read(&proof).unwrap();
    std::fs::remove_file(&proof).unwrap();
    assert!(
        !run(&dir, &["fleet", "plan", "--controller", "2vxsx-fae"])
            .status
            .success()
    );
    std::fs::write(proof, bytes).unwrap();
    owner.instance_mut().stop_live().await;
}

#[tokio::test(flavor = "current_thread")]
async fn bootstrap_store_then_publish_resume_and_retain_versions() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_owned();
    let assets = PathBuf::from(std::env::var_os("CANIC_PUBLICATION_ARTIFACTS").unwrap());
    let binary = server_assets::check(&canic_qualification::server_directory())
        .expect("make qualification-setup");
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
    let operator = CanisterId::anonymous();
    let root_bytes = std::fs::read(assets.join("root.wasm")).unwrap();
    let store_bytes = std::fs::read(assets.join("store.wasm")).unwrap();
    let pic = owner.instance();
    let root = pic.create_canister().await;
    pic.add_cycles(root, 10_000_000_000_000).await;
    pic.install_canister(root, root_bytes.clone(), vec![], Some(operator))
        .await;
    let root_before = pic.get_stable_memory(root).await;
    let url = owner.instance_mut().make_live(None).await.to_string();
    let root_key = hex(&owner.instance().root_key().await.unwrap());
    let dir = repo
        .join("target/publication")
        .join(format!("run-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let wasm = dir.join("app.wasm");
    std::fs::write(
        &wasm,
        padded(std::fs::read(assets.join("app.wasm")).unwrap()),
    )
    .unwrap();
    let config = json!({"apps":{"assets":{
        "canisters": {"root":{"role":"root"}, "store":{"role":"wasm-store","parent":"root","wasm":assets.join("store.wasm")},
            "app":{"parent":"root","wasm":wasm}},
        "fleets":{"local":{"network":{"kind":"Local","url":url,"root_key":root_key,"routing_canister":root.to_text()},
            "journal":"journal", "canisters":{"root":{"id":root.to_text()}}}}
    }}});
    std::fs::write(dir.join("canic.json"), serde_json::to_vec(&config).unwrap()).unwrap();
    let inspected = Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(["artifact", "inspect", "--json"])
        .arg(&wasm)
        .output()
        .unwrap();
    assert!(
        inspected.status.success(),
        "{}",
        String::from_utf8_lossy(&inspected.stderr)
    );
    let inspected: Value = serde_json::from_slice(&inspected.stdout).unwrap();
    assert!(inspected["bytes"].as_u64().unwrap() > 1_500_000);
    let root_hash = hex(&canic_wasm_store_protocol::digest(&root_bytes));
    let store_hash = hex(&canic_wasm_store_protocol::digest(&store_bytes));
    let signer = repo.join(".tools/ic/bin/icp");
    let bootstrap = cli(
        &dir,
        &[
            "fleet",
            "plan",
            "--canister",
            "store",
            "--root-module",
            &root_hash,
            "--controller",
            "2vxsx-fae",
        ],
    );
    let bootstrap_id = bootstrap["operation"].as_str().unwrap();
    let bootstrap_dir = dir.join("journal").join(bootstrap_id);
    assert_eq!(bootstrap["plan"]["creation_cycles"], 1_000_000_000_000_u64);
    assert_eq!(bootstrap["plan"]["store_root"]["canister"], root.to_text());
    // Changed Root control must refuse before any creation signing.
    owner
        .instance()
        .update_canister_settings(
            root,
            Some(operator),
            CanisterSettings {
                controllers: Some(vec![operator, CanisterId::self_authenticating(b"other")]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(
        !run(
            &dir,
            &[
                "fleet",
                "apply",
                bootstrap_id,
                "--identity",
                "anonymous",
                "--icp-bin",
                signer.to_str().unwrap()
            ]
        )
        .status
        .success()
    );
    assert!(!bootstrap_dir.join("create/signed.json").exists());
    owner
        .instance()
        .update_canister_settings(
            root,
            Some(operator),
            CanisterSettings {
                controllers: Some(vec![operator]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let subnet = owner.instance().get_subnet(root).await.unwrap();
    let before_creation = owner
        .instance()
        .get_subnet_metrics(subnet)
        .await
        .unwrap()
        .num_canisters;
    // Stop at the real signer boundary after controller handover has committed.
    let paused = dir.join("pause-readiness-icp");
    std::fs::write(&paused, "#!/bin/sh\nfor arg in \"$@\"; do\n case \"$arg\" in */store-observe/0/arguments.bin) exit 71;; esac\ndone\nexec \"$CANIC_REAL_SIGNER\" \"$@\"\n").unwrap();
    std::fs::set_permissions(&paused, std::fs::Permissions::from_mode(0o700)).unwrap();
    let stopped = Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(["fleet", "apply", bootstrap_id, "--json", "--config"])
        .arg(dir.join("canic.json"))
        .args(["--identity", "anonymous", "--icp-bin"])
        .arg(&paused)
        .env("CANIC_REAL_SIGNER", &signer)
        .output()
        .unwrap();
    assert!(!stopped.status.success());
    assert!(
        bootstrap_dir.join("handover/reply.bin").exists(),
        "{}",
        String::from_utf8_lossy(&stopped.stderr)
    );
    assert!(!bootstrap_dir.join("receipt.json").exists());
    let attempts: Vec<_> = ["create", "install", "handover"]
        .iter()
        .map(|step| std::fs::read(bootstrap_dir.join(step).join("signed.json")).unwrap())
        .collect();
    // Simulate lost local handover reply; certified sole control reconstructs its effect.
    std::fs::remove_file(bootstrap_dir.join("handover/reply.bin")).unwrap();
    let completed_store = cli(
        &dir,
        &[
            "fleet",
            "reconcile",
            bootstrap_id,
            "--identity",
            "anonymous",
            "--icp-bin",
            signer.to_str().unwrap(),
        ],
    );
    let store_text = completed_store["canister"].as_str().unwrap();
    let store = CanisterId::from_text(store_text).unwrap();
    assert_eq!(owner.instance().get_subnet(store).await, Some(subnet));
    assert_eq!(
        serde_json::from_slice::<String>(
            &std::fs::read(bootstrap_dir.join("store-subnet.json")).unwrap()
        )
        .unwrap(),
        subnet.to_text()
    );
    assert_eq!(completed_store["controller"], root.to_text());
    assert_eq!(completed_store["module_hash"], store_hash);
    let after_creation = owner
        .instance()
        .get_subnet_metrics(subnet)
        .await
        .unwrap()
        .num_canisters;
    assert_eq!(after_creation, before_creation + 1);
    assert_eq!(
        cli(
            &dir,
            &[
                "fleet",
                "apply",
                bootstrap_id,
                "--identity",
                "anonymous",
                "--icp-bin",
                signer.to_str().unwrap()
            ]
        ),
        completed_store
    );
    assert_eq!(
        owner
            .instance()
            .get_subnet_metrics(subnet)
            .await
            .unwrap()
            .num_canisters,
        after_creation
    );
    for (step, bytes) in ["create", "install", "handover"].iter().zip(attempts) {
        assert_eq!(
            std::fs::read(bootstrap_dir.join(step).join("signed.json")).unwrap(),
            bytes
        );
    }
    assert!(!bootstrap_dir.join("handover/reply.bin").exists());
    assert_eq!(owner.instance().get_stable_memory(root).await, root_before);
    let readiness_path = bootstrap_dir.join("store-observe/0/reply.bin");
    let readiness = std::fs::read(&readiness_path).unwrap();
    std::fs::remove_file(&readiness_path).unwrap();
    assert!(
        !run(&dir, &["fleet", "status", bootstrap_id])
            .status
            .success()
    );
    std::fs::write(readiness_path, readiness).unwrap();
    cli(
        &dir,
        &["fleet", "canister", "bind", "store", "--id", store_text],
    );
    let plan = |version: &str, module: &str| {
        cli(
            &dir,
            &[
                "fleet",
                "plan",
                "--canister",
                "app",
                "--publish-version",
                version,
                "--store",
                "store",
                "--root-module",
                &root_hash,
                "--store-module",
                module,
                "--controller",
                "2vxsx-fae",
            ],
        )
    };
    let first = plan("v1", &store_hash);
    assert_eq!(first["plan"]["artifact"], inspected);
    let id = first["operation"].as_str().unwrap();
    let retained = dir.join("journal").join(id);
    // Fail before signing the first chunk. Prepare has really committed through Root.
    let interrupted = dir.join("interrupted-icp");
    std::fs::write(&interrupted, "#!/bin/sh\nfor arg in \"$@\"; do\n case \"$arg\" in */publication/chunk-0/arguments.bin) exit 71;; esac\ndone\nexec \"$CANIC_REAL_SIGNER\" \"$@\"\n").unwrap();
    std::fs::set_permissions(&interrupted, std::fs::Permissions::from_mode(0o700)).unwrap();
    let failed = Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(["fleet", "apply", id, "--json", "--config"])
        .arg(dir.join("canic.json"))
        .args(["--identity", "anonymous", "--icp-bin"])
        .arg(&interrupted)
        .env("CANIC_REAL_SIGNER", &signer)
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(
        String::from_utf8_lossy(&failed.stderr).contains("signing failed"),
        "{}",
        String::from_utf8_lossy(&failed.stderr)
    );
    let prepare = retained.join("publication/prepare/signed.json");
    let signed = std::fs::read(&prepare).unwrap();
    assert!(retained.join("publication/prepare/reply.bin").exists());
    assert!(!retained.join("publication/chunk-0/signed.json").exists());
    let store_before = owner.instance().get_stable_memory(store).await;
    let apply = |action: &str, operation: &str| {
        cli(
            &dir,
            &[
                "fleet",
                action,
                operation,
                "--identity",
                "anonymous",
                "--icp-bin",
                signer.to_str().unwrap(),
            ],
        )
    };
    let observed = apply("reconcile", id);
    assert_eq!(observed["publication"]["status"]["published"], false);
    assert!(
        !observed["publication"]["status"]["missing"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        owner.instance().get_stable_memory(store).await,
        store_before
    );
    let completed = apply("apply", id);
    assert_eq!(completed["publication"]["status"]["published"], true);
    assert_eq!(std::fs::read(&prepare).unwrap(), signed);
    assert!(!retained.join("receipt.json").exists());
    let status = cli(&dir, &["fleet", "status", id]);
    assert!(status["receipt"].is_null());
    assert_eq!(status["publication"], completed["publication"]);
    let human = Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(["fleet", "status", id, "--config"])
        .arg(dir.join("canic.json"))
        .output()
        .unwrap();
    assert!(human.status.success());
    let text = String::from_utf8(human.stdout).unwrap();
    assert!(text.contains("Published receipt retained: true"));
    assert!(!text.contains("deployment completion is not established"));
    let view = cli(&dir, &["fleet", "show"]);
    assert_eq!(view["operations"].as_array().unwrap().len(), 1);
    assert_eq!(view["operations"][0]["operation"], bootstrap_id);
    assert!(view["issues"].as_array().unwrap().is_empty());
    assert_eq!(view["canisters"].as_array().unwrap().len(), 2);
    assert_eq!(plan("v1", &store_hash)["operation"], id);
    let published = owner.instance().get_stable_memory(store).await;
    assert_eq!(apply("apply", id), completed);
    assert_eq!(owner.instance().get_stable_memory(store).await, published);
    // A new immutable label can reuse existing chunks; it gets no new chunk signatures.
    let second = plan("v2", &store_hash);
    let second_id = second["operation"].as_str().unwrap();
    assert_eq!(
        apply("apply", second_id)["publication"]["status"]["published"],
        true
    );
    assert!(
        !dir.join("journal")
            .join(second_id)
            .join("publication/chunk-0")
            .exists()
    );
    assert_eq!(apply("reconcile", id), completed);
    assert_eq!(owner.instance().get_stable_memory(root).await, root_before);
    let bad = plan("v3", &"00".repeat(32));
    let refused = run(
        &dir,
        &[
            "fleet",
            "apply",
            bad["operation"].as_str().unwrap(),
            "--identity",
            "anonymous",
            "--icp-bin",
            signer.to_str().unwrap(),
        ],
    );
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("code differs"));
    // Removing the signed proof invalidates a saved completion summary.
    let index = completed["publication"]["observation"].as_u64().unwrap();
    let proof = retained.join(format!("publication/observe/{index}/signed.json"));
    let evidence = std::fs::read(&proof).unwrap();
    std::fs::remove_file(&proof).unwrap();
    assert!(!run(&dir, &["fleet", "status", id]).status.success());
    std::fs::write(&proof, evidence).unwrap();
    assert_eq!(
        cli(&dir, &["fleet", "status", id])["publication"],
        completed["publication"]
    );
    owner.instance_mut().stop_live().await;
}
