//! Exercise the shipped CLI process, with PocketIC confined to its own graph.

mod demo_actor;

use ic_testkit::{
    pocket_ic::{CanisterId, PocketIcBuilder},
    server::{Limits, PreparedServer, Server},
};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

fn cli(root: &Path, args: &[&str]) -> Value {
    let output = Command::new(std::env::var_os("CANIC_CLI").expect("run make journey-test"))
        .args(args)
        .arg("--config")
        .arg(root.join("canic.toml"))
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("CLI stdout is one JSON value")
}

#[tokio::test(flavor = "current_thread")]
async fn deploy_call_resume_and_upgrade() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();
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
    let mut owner = server
        .instance(
            PocketIcBuilder::new()
                .with_nns_subnet()
                .with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    let url = owner.client_mut().make_live(None).await.to_string();
    let root_key: String = owner
        .client()
        .root_key()
        .await
        .unwrap()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let routing_canister = owner.client().create_canister().await.to_text();
    let root_subnet = owner.client().topology().await.get_nns().unwrap().to_text();
    let root = repo
        .join("target/journey-tests")
        .join(format!("journey-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let wasm = std::env::var("CANIC_HELLO_V1").unwrap();
    let wasm_v2 = std::env::var("CANIC_HELLO_V2").unwrap();
    let artifacts = root.join("artifacts");
    std::fs::create_dir_all(&artifacts).unwrap();
    for (name, file) in [("root", "root.wasm"), ("store", "wasm_store.wasm")] {
        std::fs::copy(
            repo.join(format!("target/journey/{name}.wasm")),
            artifacts.join(file),
        )
        .unwrap();
    }
    std::fs::copy(wasm, artifacts.join("backend.wasm")).unwrap();
    let config = serde_json::json!({"apps":{"hello":{"canisters":{"backend":{"role":"greeting","parent":"root","wasm":"artifacts/backend.wasm"}},
        "fleets":{"local":{"network":{"kind":"Local","url":url,"root_key":root_key,"root_subnet":root_subnet,"routing_canister":routing_canister}}}}}});
    std::fs::write(root.join("canic.toml"), toml::to_string(&config).unwrap()).unwrap();
    let plan = || cli(&root, &["fleet", "plan", "--identity", "anonymous"]);
    let apply = |id: &str| cli(&root, &["fleet", "apply", id, "--identity", "anonymous"]);
    let creation = plan();
    let id = creation["operation"].as_str().unwrap();
    let first = apply(id);
    let path = PathBuf::from(creation["state_dir"].as_str().unwrap())
        .join(id)
        .join("child/stage");
    let signed = std::fs::read(path.join("signed.json")).unwrap();
    std::fs::remove_file(path.join("reply.bin")).unwrap();
    for _ in 0..60 {
        owner.client().tick().await;
    }
    let created = apply(id);
    assert_eq!(first["root"], created["root"]);
    assert_eq!(std::fs::read(path.join("signed.json")).unwrap(), signed);
    let target = CanisterId::from_text(
        created["children"][0]["outcome"]["canister"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    demo_actor::check(owner.client(), target, "Hello from Canic").await;
    std::fs::copy(wasm_v2, artifacts.join("backend.wasm")).unwrap();
    let publication = plan();
    assert_eq!(publication["action"], "publication");
    apply(publication["operation"].as_str().unwrap());
    let upgrade = plan();
    assert_eq!(upgrade["action"], "upgrade");
    let upgrade_id = upgrade["operation"].as_str().unwrap();
    apply(upgrade_id);
    for _ in 0..60 {
        owner.client().tick().await;
    }
    let receipt = apply(upgrade_id);
    assert_eq!(receipt["canister"], target.to_text());
    assert_eq!(apply(upgrade_id), receipt);
    demo_actor::check(owner.client(), target, "Hello again from Canic").await;
    let report = root.join("fleet.html");
    let view = cli(
        &root,
        &["fleet", "show", "--html", report.to_str().unwrap()],
    );
    assert_eq!(view["canisters"].as_array().unwrap().len(), 3);
    assert!(
        std::fs::read_to_string(report)
            .unwrap()
            .contains(&target.to_text())
    );
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
