//! Operator CLI, separate Store and a Root-owned DB child on a disposable fleet.
use super::*;
use canic_management_protocol::{children, upgrade};
use serde_json::{Value, json};
use std::{path::Path, process::Command};

fn cli(config: &Path, args: &[&str]) -> Value {
    let output = Command::new(std::env::var_os("CANIC_CLI").expect("make db-test"))
        .args(args)
        .arg("--json")
        .arg("--config")
        .arg(config)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

async fn progress(pic: &PocketIc) {
    pic.stop_progress().await;
    pic.advance_time(Duration::from_secs(15)).await;
    for _ in 0..70 {
        pic.tick().await;
    }
    pic.auto_progress().await;
}

async fn status(pic: &PocketIc, root: Principal) -> upgrade::Status {
    use canic_management_protocol::children;
    let bytes = pic
        .query_call(
            root,
            Principal::anonymous(),
            "child_status",
            frame(
                children::READ_STATUS,
                &children::StatusRequest::Upgrade(children::Request {
                    slot: children::ChildId::FIRST,
                    value: None::<upgrade::Approval>,
                }),
                ic_wire::Direction::Request,
            ),
        )
        .await
        .unwrap();
    let children::ObservedStatus::Upgrade(status) = ic_wire::decode(
        &bytes,
        ic_wire::Encoding::Frame {
            method: children::READ_STATUS,
            direction: ic_wire::Direction::Response,
        },
        canic_management_protocol::LIMITS,
    )
    .unwrap() else {
        panic!("expected Upgrade status")
    };
    status
}

#[tokio::test]
async fn lost_install_callback_preserves_db_and_original_fleet_operation() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let assets = repo.join("target/db");
    let dir = assets.join(format!("fleet-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(240),
            stdout_bytes: 1_048_576,
            stderr_bytes: 1_048_576,
        },
    )
    .unwrap();
    let mut instance = server
        .instance(
            PocketIcBuilder::new()
                .with_nns_subnet()
                .with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    let routing = instance.client().create_canister().await;
    let url = instance.client_mut().make_live(None).await.to_string();
    let pic = instance.client();
    let root_key: String = pic
        .root_key()
        .await
        .unwrap()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let app = dir.join("app.wasm");
    std::fs::copy(assets.join("managed.wasm"), &app).unwrap();
    let config = dir.join("canic.toml");
    let fleet = json!({"apps":{"db":{"canisters":{
        "root":{"role":"root","wasm":assets.join("root.wasm")},
        "store":{"role":"wasm-store","parent":"root","wasm":assets.join("store.wasm")},
        "app":{"role":"app","parent":"root","wasm":app}},
        "fleets":{"local":{"network":{"kind":"Local","url":url,"root_key":root_key,
        "root_subnet":pic.topology().await.get_nns().unwrap().to_text(),"routing_canister":routing.to_text()},
        "journal":"journal","canisters":{}}}}}});
    std::fs::write(&config, toml::to_string(&fleet).unwrap()).unwrap();
    let signer = repo.join(".tools/ic/bin/icp");
    let plan = || cli(&config, &["fleet", "plan", "--controller", "2vxsx-fae"]);
    let act = |command: &str, id: &str| {
        cli(
            &config,
            &[
                "fleet",
                command,
                id,
                "--identity",
                "anonymous",
                "--icp-bin",
                signer.to_str().unwrap(),
            ],
        )
    };
    let initial = plan();
    let id = initial["operation"].as_str().unwrap();
    act("apply", id);
    progress(pic).await;
    let created = act("apply", id);
    let child = Principal::from_text(
        created["children"][0]["outcome"]["canister"]
            .as_str()
            .expect("deployed DB"),
    )
    .unwrap();
    let root = Principal::from_text(created["root"]["canister"].as_str().unwrap()).unwrap();
    assert_eq!(
        pic.canister_status(child, Some(root))
            .await
            .unwrap()
            .settings
            .controllers,
        [root]
    );
    let client = Client {
        pic,
        canister: child,
        owner: Principal::self_authenticating([11]),
    };
    let rows = [row(3, 10), row(4, 20)];
    let query = QueryRequest {
        minimum: 0,
        limit: 2,
        work: 2,
        after: None,
    };
    assert_eq!(client.write(&rows).await.unwrap().inserted, 2);
    let before = pic.get_stable_memory(child).await;
    pic.update_call(
        root,
        Principal::anonymous(),
        "inject_child_fault",
        frame(
            children::INJECT,
            &children::Request {
                slot: children::ChildId::FIRST,
                value: 8_u8,
            },
            ic_wire::Direction::Request,
        ),
    )
    .await
    .unwrap();
    std::fs::copy(assets.join("managed-next.wasm"), &app).unwrap();
    let publication = plan();
    assert_eq!(publication["action"], "publication");
    act("apply", publication["operation"].as_str().unwrap());
    let change = plan();
    assert_eq!(change["action"], "upgrade");
    let id = change["operation"].as_str().unwrap();
    act("apply", id);
    progress(pic).await;
    let pending = status(pic, root).await;
    assert_eq!(pending.phase, upgrade::Phase::InstallIssued);
    assert_eq!(pending.attempts[1], 1);
    assert!(pending.requests[1].is_some());
    let installed = pic
        .canister_status(child, Some(root))
        .await
        .unwrap()
        .module_hash
        .unwrap();
    assert_eq!(
        installed,
        canic_management_protocol::digest(&std::fs::read(&app).unwrap())
    );
    assert_eq!(plan(), change);
    let journal = dir.join("journal").join(id);
    assert!(!journal.join("receipt.json").exists());
    let signed = std::fs::read(journal.join("stage/signed.json")).unwrap();
    act("reconcile", id);
    progress(pic).await;
    assert_eq!(act("apply", id)["canister"], child.to_text());
    let settled = status(pic, root).await;
    assert_eq!(settled.phase, upgrade::Phase::Ready);
    assert_eq!(settled.approval, pending.approval);
    assert_eq!(settled.attempts, pending.attempts);
    assert_eq!(settled.requests, pending.requests);
    assert_eq!(
        settled.observed_version,
        Some(pending.previous_version.unwrap() + 1)
    );
    assert_eq!(
        std::fs::read(journal.join("stage/signed.json")).unwrap(),
        signed
    );
    assert_eq!(pic.get_stable_memory(child).await, before);
    assert_eq!(client.page(&query).await.unwrap().rows, rows);
    assert!(
        pic.query_call(
            child,
            root,
            "canic_db_query",
            frame(QUERY, &query, ic_wire::Direction::Request)
        )
        .await
        .is_err()
    );
    assert_eq!(client.write(&[row(5, 30)]).await.unwrap().inserted, 1);
    assert_eq!(plan()["action"], "unchanged");
    instance.client_mut().stop_live().await;
    assert_eq!(
        instance.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}
