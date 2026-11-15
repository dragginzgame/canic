//! Real CLI, signer, Root and Store; publication has no deployment side effects.
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
    let payload = 600_000_u32;
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
async fn publish_resume_observe_and_retain_versions_without_deploying() {
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
    let store = pic.create_canister().await;
    for (id, bytes) in [(root, &root_bytes), (store, &store_bytes)] {
        pic.add_cycles(id, 10_000_000_000_000).await;
        pic.install_canister(id, bytes.clone(), vec![], Some(operator))
            .await;
    }
    pic.update_canister_settings(
        store,
        Some(operator),
        CanisterSettings {
            controllers: Some(vec![root]),
            ..Default::default()
        },
    )
    .await
    .unwrap();
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
        "canisters": {"root":{"role":"root"}, "store":{"role":"wasm-store","parent":"root"},
            "app":{"parent":"root","wasm":wasm}},
        "fleets":{"local":{"network":{"kind":"Local","url":url,"root_key":root_key,"routing_canister":root.to_text()},
            "journal":"journal", "canisters":{"root":{"id":root.to_text()},"store":{"id":store.to_text()}}}}
    }}});
    std::fs::write(dir.join("canic.json"), serde_json::to_vec(&config).unwrap()).unwrap();
    let root_hash = hex(&canic_wasm_store_protocol::digest(&root_bytes));
    let store_hash = hex(&canic_wasm_store_protocol::digest(&store_bytes));
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
    let id = first["operation"].as_str().unwrap();
    let retained = dir.join("journal").join(id);
    let signer = repo.join(".tools/ic/bin/icp");
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
    assert!(view["operations"].as_array().unwrap().is_empty());
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
