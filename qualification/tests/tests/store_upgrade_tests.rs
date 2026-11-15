//! Real fleet planner, isolated signer and Root-owned Store replacement on a local replica.
use super::{cli, hex, run};
use canic_management_protocol::{children, store_maintenance as wire};
use ic_testkit::{
    pic::{PocketIcStartupConfig, server_assets},
    pocket_ic::{CanisterId, PocketIcBuilder},
};
use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn quoted(path: &Path) -> String {
    format!("'{}'", path.to_str().unwrap().replace('\'', "'\\''"))
}
fn call(dir: &Path, method: &str, id: &str, signer: &Path) -> Value {
    cli(
        dir,
        &[
            "fleet",
            method,
            id,
            "--identity",
            "store-fixture",
            "--icp-bin",
            signer.to_str().unwrap(),
        ],
    )
}
fn write_config(dir: &Path, config: &Value) {
    std::fs::write(dir.join("canic.json"), serde_json::to_vec(config).unwrap()).unwrap();
}
async fn retained(
    pic: &ic_testkit::pocket_ic::nonblocking::PocketIc,
    root: CanisterId,
    operator: CanisterId,
) -> wire::Status {
    let reply = pic
        .update_call(
            root,
            operator,
            "observe_store_maintenance",
            ic_wire::native::encode_request(wire::OBSERVE, &(), wire::LIMITS).unwrap(),
        )
        .await
        .unwrap();
    ic_wire::native::decode_response::<Option<wire::Status>>(wire::OBSERVE, &reply, wire::LIMITS)
        .unwrap()
        .unwrap()
}
async fn inject(
    pic: &ic_testkit::pocket_ic::nonblocking::PocketIc,
    root: CanisterId,
    operator: CanisterId,
    point: u8,
) {
    pic.update_call(
        root,
        operator,
        "inject_child_fault",
        ic_wire::native::encode_request(
            children::INJECT,
            &children::Request {
                slot: children::Slot::First,
                value: point,
            },
            canic_management_protocol::LIMITS,
        )
        .unwrap(),
    )
    .await
    .unwrap();
}
fn pause(dir: &Path, signer: &Path, method: &str) -> PathBuf {
    let path = dir.join(format!("pause-{method}"));
    std::fs::write(&path, format!("#!/bin/sh\nfor arg in \"$@\"; do\n if [ \"$arg\" = '{method}' ]; then exit 73; fi\ndone\nexec {} \"$@\"\n", quoted(signer))).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    path
}

#[tokio::test(flavor = "current_thread")]
async fn fleet_store_upgrade_retains_authority_recovers_and_feeds_root_and_child_plans() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let assets = PathBuf::from(std::env::var_os("CANIC_PUBLICATION_ARTIFACTS").unwrap());
    let dir = repo
        .join("target/publication")
        .join(format!("store-upgrade-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let signer = dir.join("signer");
    std::fs::write(&signer, format!("#!/bin/sh\nexport ICP_HOME={}\nexport XDG_CONFIG_HOME={}\nexport ICP_TELEMETRY_DISABLED=1\nexec {} \"$@\"\n", quoted(&dir.join("icp")), quoted(&dir.join("config")), quoted(&repo.join(".tools/ic/bin/icp")))).unwrap();
    std::fs::set_permissions(&signer, std::fs::Permissions::from_mode(0o700)).unwrap();
    // Generated fixture credentials stay under target; no operator wallet or seed is read.
    let created = Command::new(&signer)
        .args([
            "identity",
            "new",
            "store-fixture",
            "--storage",
            "plaintext",
            "--output-seed",
        ])
        .arg(dir.join("fixture-seed.txt"))
        .output()
        .unwrap();
    assert!(
        created.status.success(),
        "isolated identity creation failed"
    );
    let principal = Command::new(&signer)
        .args(["identity", "principal", "--identity", "store-fixture"])
        .output()
        .unwrap();
    assert!(principal.status.success());
    let controller = String::from_utf8(principal.stdout)
        .unwrap()
        .trim()
        .to_owned();
    let operator = CanisterId::from_text(&controller).unwrap();
    let mut owner = PocketIcStartupConfig::Spawn {
        server_binary: server_assets::check(&canic_qualification::server_directory()).unwrap(),
        timeout: Duration::from_secs(30),
        server_hard_ttl: Some(Duration::from_secs(600)),
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
    let url = owner.instance_mut().make_live(None).await.to_string();
    let pic = owner.instance();
    let mut config = json!({"apps":{"demo":{
        "canisters":{"root":{"role":"root","wasm":assets.join("root-faults.wasm")}, "store":{"role":"wasm-store","parent":"root","wasm":assets.join("store.wasm")}, "app":{"role":"app","parent":"root","wasm":assets.join("app.wasm")}},
        "fleets":{"local":{"network":{"kind":"Local","url":url,"root_key":hex(&pic.root_key().await.unwrap()),"routing_canister":routing.to_text()},"journal":"journal","canisters":{}}}
    }}});
    write_config(&dir, &config);
    let dispatch = (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 30)
        .to_string();
    let initial = cli(
        &dir,
        &[
            "fleet",
            "plan",
            "--controller",
            &controller,
            "--execute-at",
            &dispatch,
        ],
    );
    let creation = initial["operation"].as_str().unwrap();
    let mut deployed = Value::Null;
    for _ in 0..100 {
        deployed = call(&dir, "apply", creation, &signer);
        if deployed["children"][0]["outcome"]["canister"].is_string() {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    let root = CanisterId::from_text(deployed["root"]["canister"].as_str().unwrap()).unwrap();
    let store = CanisterId::from_text(deployed["store"]["canister"].as_str().unwrap()).unwrap();
    assert!(deployed["children"][0]["outcome"]["canister"].is_string());
    config["apps"]["demo"]["canisters"]["store"]["wasm"] =
        json!(assets.join("store-candidate.wasm"));
    write_config(&dir, &config);
    let planned = cli(&dir, &["fleet", "plan", "--controller", &controller]);
    assert_eq!(planned["action"], "upgrade");
    assert_eq!(
        planned["plan"]["store_upgrade_root"]["canister"],
        root.to_text()
    );
    let id = planned["operation"].as_str().unwrap();
    let operation = dir.join("journal").join(id);
    assert!(
        !run(
            &dir,
            &[
                "fleet",
                "reconcile",
                id,
                "--identity",
                "store-fixture",
                "--icp-bin",
                signer.to_str().unwrap()
            ]
        )
        .status
        .success()
    );
    assert!(!operation.join("stage/signed.json").exists());
    let stop_before_install = pause(&dir, &signer, "install_store_maintenance");
    assert!(
        !run(
            &dir,
            &[
                "fleet",
                "apply",
                id,
                "--identity",
                "store-fixture",
                "--icp-bin",
                stop_before_install.to_str().unwrap()
            ]
        )
        .status
        .success()
    );
    assert_eq!(
        retained(pic, root, operator).await.phase,
        wire::Phase::Stopped
    );
    assert_eq!(
        cli(&dir, &["fleet", "plan", "--controller", &controller]),
        planned
    );
    let before = pic.get_stable_memory(store).await;
    assert_eq!(
        call(&dir, "reconcile", id, &signer)["store_maintenance"]["phase"],
        "Stopped"
    );
    assert_eq!(pic.get_stable_memory(store).await, before);

    inject(pic, root, operator, 15).await;
    assert!(
        !run(
            &dir,
            &[
                "fleet",
                "apply",
                id,
                "--identity",
                "store-fixture",
                "--icp-bin",
                signer.to_str().unwrap()
            ]
        )
        .status
        .success()
    );
    inject(pic, root, operator, 0).await;
    assert_eq!(
        retained(pic, root, operator).await.phase,
        wire::Phase::UploadIssued
    );
    assert_eq!(
        call(&dir, "reconcile", id, &signer)["store_maintenance"]["phase"],
        "Uploaded"
    );
    let stop_before_activation = pause(&dir, &signer, "activate_store_maintenance");
    assert!(
        !run(
            &dir,
            &[
                "fleet",
                "apply",
                id,
                "--identity",
                "store-fixture",
                "--icp-bin",
                stop_before_activation.to_str().unwrap()
            ]
        )
        .status
        .success()
    );
    assert_eq!(
        retained(pic, root, operator).await.phase,
        wire::Phase::Verified
    );
    assert_eq!(pic.get_stable_memory(store).await, before);
    assert_eq!(
        call(&dir, "reconcile", id, &signer)["store_maintenance"]["phase"],
        "Verified"
    );
    assert_eq!(pic.get_stable_memory(store).await, before);
    let completed = call(&dir, "apply", id, &signer);
    assert_eq!(completed["canister"], store.to_text());
    assert_eq!(
        completed["module_hash"],
        planned["plan"]["artifact"]["sha256"]
    );
    assert_eq!(call(&dir, "reconcile", id, &signer), completed);
    assert_eq!(
        cli(&dir, &["fleet", "status", id])["store_maintenance"]["phase"],
        "Ready"
    );
    // A completion pointer cannot substitute for its exact signed observation.
    let completion = operation.join("store-completion.json");
    let original = std::fs::read(&completion).unwrap();
    std::fs::write(&completion, b"0").unwrap();
    assert!(!run(&dir, &["fleet", "status", id]).status.success());
    assert!(
        !run(&dir, &["fleet", "plan", "--controller", &controller])
            .status
            .success()
    );
    std::fs::write(&completion, &original).unwrap();
    let index: usize = serde_json::from_slice(&original).unwrap();
    let signed = operation.join(format!("store-upgrade/{index}/signed.json"));
    let bytes = std::fs::read(&signed).unwrap();
    std::fs::remove_file(&signed).unwrap();
    assert!(!run(&dir, &["fleet", "status", id]).status.success());
    std::fs::write(&signed, bytes).unwrap();
    assert_eq!(
        cli(&dir, &["fleet", "plan", "--controller", &controller])["action"],
        "unchanged"
    );

    config["apps"]["demo"]["canisters"]["root"]["wasm"] =
        json!(assets.join("root-maintenance.wasm"));
    write_config(&dir, &config);
    let maintenance = cli(&dir, &["fleet", "plan", "--controller", &controller]);
    assert_eq!(maintenance["action"], "maintenance");
    assert_eq!(
        maintenance["plan"]["maintenance_store"]["operation"]["phase"],
        "Ready"
    );
    call(
        &dir,
        "apply",
        maintenance["operation"].as_str().unwrap(),
        &signer,
    );
    assert_eq!(
        cli(&dir, &["fleet", "plan", "--controller", &controller])["action"],
        "unchanged"
    );

    config["apps"]["demo"]["canisters"]["app"]["wasm"] = json!(assets.join("app-v2.wasm"));
    write_config(&dir, &config);
    let publication = cli(&dir, &["fleet", "plan", "--controller", &controller]);
    assert_eq!(publication["action"], "publication");
    call(
        &dir,
        "apply",
        publication["operation"].as_str().unwrap(),
        &signer,
    );
    let dispatch = (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 10)
        .to_string();
    let child = cli(
        &dir,
        &[
            "fleet",
            "plan",
            "--controller",
            &controller,
            "--execute-at",
            &dispatch,
        ],
    );
    assert_eq!(child["action"], "upgrade");
    assert_eq!(
        child["plan"]["management"]["source"]["store"]["module"],
        completed["module_hash"]
    );
    assert_eq!(
        child["plan"]["management"]["source"]["maintenance"]["sequence"],
        1
    );
    let mut ready = Value::Null;
    for _ in 0..100 {
        ready = call(&dir, "apply", child["operation"].as_str().unwrap(), &signer);
        if ready["canister"].is_string() {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    assert!(ready["canister"].is_string());
    assert_eq!(
        cli(&dir, &["fleet", "plan", "--controller", &controller])["action"],
        "unchanged"
    );
    config["apps"]["demo"]["canisters"]["store"]["wasm"] = json!(assets.join("store.wasm"));
    write_config(&dir, &config);
    assert!(
        !run(&dir, &["fleet", "plan", "--controller", &controller])
            .status
            .success()
    );
    owner.instance_mut().stop_live().await;
}
