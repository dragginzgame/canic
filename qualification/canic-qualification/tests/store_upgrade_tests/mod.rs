//! Real fleet planner, isolated signer and Root-owned Store replacement on a local replica.
use super::{cli, hex, run};
use canic_management_protocol::{children, store_maintenance as wire};
use ic_testkit::{
    pocket_ic::{CanisterId, PocketIcBuilder},
    server::{Limits, PreparedServer, Server},
};
use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
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
    std::fs::write(dir.join("canic.toml"), toml::to_string(config).unwrap()).unwrap();
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
            ic_wire::encode(
                &(),
                ic_wire::Encoding::Frame {
                    method: wire::OBSERVE,
                    direction: ic_wire::Direction::Request,
                },
                wire::LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    ic_wire::decode::<Option<wire::Status>>(
        &reply,
        ic_wire::Encoding::Frame {
            method: wire::OBSERVE,
            direction: ic_wire::Direction::Response,
        },
        wire::LIMITS,
    )
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
        ic_wire::encode(
            &children::Request {
                slot: children::ChildId::FIRST,
                value: point,
            },
            ic_wire::Encoding::Frame {
                method: children::INJECT,
                direction: ic_wire::Direction::Request,
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
    let (signer, controller) = canic_qualification::isolated_signer(&dir, "store-fixture");
    let operator = CanisterId::from_text(&controller).unwrap();
    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(600),
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
    let pic = owner.client();
    let dir = dir.join("project");
    let initialized = {
        let created = Command::new(std::env::var_os("CANIC_CLI").unwrap())
            .args([
                "app",
                "init",
                "demo",
                "--source",
                dir.to_str().unwrap(),
                "--json",
            ])
            .output()
            .unwrap();
        assert!(
            created.status.success(),
            "{}",
            String::from_utf8_lossy(&created.stderr)
        );
        Command::new(std::env::var_os("CANIC_CLI").unwrap())
            .args([
                "fleet",
                "init",
                "local",
                "--app",
                "demo",
                "--network",
                "local",
                "--url",
                &url,
                "--root-key",
                &hex(&pic.root_key().await.unwrap()),
                "--root-subnet",
                &pic.topology().await.get_nns().unwrap().to_text(),
                "--routing-canister",
                &routing.to_text(),
                "--config",
                dir.join("canic.toml").to_str().unwrap(),
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
    let built = cli(&dir, &["app", "build"]);
    assert_eq!(built["canisters"].as_array().unwrap().len(), 3);
    let lock = std::fs::read(dir.join("Cargo.lock")).unwrap();
    let mut config: Value =
        toml::from_str(&std::fs::read_to_string(dir.join("canic.toml")).unwrap()).unwrap();
    assert!(
        config["apps"]["demo"]["fleets"]["local"]["canisters"]
            .as_object()
            .unwrap()
            .is_empty()
    );
    let started = Instant::now();
    let initial = cli(&dir, &["fleet", "plan", "--controller", &controller]);
    assert_eq!(
        initial["fleet_plan"]["store"]["creation_cycles"],
        1_000_000_000_000_u64
    );
    assert_eq!(
        initial["fleet_plan"]["root"]["creation_cycles"],
        3_000_000_000_000_u64
    );
    assert_eq!(
        initial["fleet_plan"]["child"]["execute_at_ns"],
        initial["fleet_plan"]["root"]["created_at_ns"]
    );
    let creation = initial["operation"].as_str().unwrap();
    let unpublished = cli(&dir, &["fleet", "artifact", "list"]);
    assert!(unpublished["publications"].as_array().unwrap().is_empty());
    assert!(unpublished["issues"].as_array().unwrap().is_empty());
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
    let application = deployed["children"][0]["outcome"]["canister"].clone();
    let artifacts = cli(&dir, &["fleet", "artifact", "list"]);
    let entries = artifacts["publications"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert!(artifacts["issues"].as_array().unwrap().is_empty());
    assert_eq!(entries[0]["operation"], creation);
    assert_ne!(entries[0]["publication_operation"], creation);
    assert_eq!(
        entries[0]["publication"]["store"]["canister"],
        store.to_text()
    );
    assert_eq!(
        entries[0]["artifact"]["sha256"],
        initial["fleet_plan"]["child"]["artifact"]["sha256"]
    );
    assert_eq!(entries[0]["publication_retained"], true);
    println!(
        "Ready-dispatch fleet bootstrap: {} ms (local wall time, no schedule override)",
        started.elapsed().as_millis()
    );
    assert_eq!(
        crate::greeting::call(
            owner.client(),
            CanisterId::from_text(application.as_str().unwrap()).unwrap(),
            "team"
        )
        .await,
        "Hello from Canic, team!"
    );
    // A warm build must preserve the selected artifacts and lock, leaving no new
    // deployment approval. Every CLI invocation is a separate host process.
    assert_eq!(cli(&dir, &["app", "build"]), built);
    assert_eq!(std::fs::read(dir.join("Cargo.lock")).unwrap(), lock);
    assert_eq!(
        cli(&dir, &["fleet", "plan", "--controller", &controller])["action"],
        "unchanged"
    );
    // The generated normal Root completed bootstrap. Select the qualification-only
    // fault executable through reviewed maintenance before the interruption cases.
    config["apps"]["demo"]["canisters"]["root"]["wasm"] = json!(assets.join("root-faults.wasm"));
    write_config(&dir, &config);
    let fault_owner = cli(&dir, &["fleet", "plan", "--controller", &controller]);
    assert_eq!(fault_owner["action"], "maintenance");
    call(
        &dir,
        "apply",
        fault_owner["operation"].as_str().unwrap(),
        &signer,
    );
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
    let operation = PathBuf::from(planned["state_dir"].as_str().unwrap()).join(id);
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

    // The second approved replacement traverses the same effect owner and recovers its own upload.
    let first_outcome = retained(pic, root, operator).await;
    config["apps"]["demo"]["canisters"]["store"]["wasm"] = json!(assets.join("store.wasm"));
    write_config(&dir, &config);
    let second = cli(&dir, &["fleet", "plan", "--controller", &controller]);
    assert_eq!(second["plan"]["store_upgrade_sequence"], 2);
    let second_id = second["operation"].as_str().unwrap();
    assert!(
        !run(
            &dir,
            &[
                "fleet",
                "apply",
                second_id,
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
        cli(&dir, &["fleet", "plan", "--controller", &controller]),
        second
    );
    let before_second = pic.get_stable_memory(store).await;
    inject(pic, root, operator, 15).await;
    assert!(
        !run(
            &dir,
            &[
                "fleet",
                "apply",
                second_id,
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
        call(&dir, "reconcile", second_id, &signer)["store_maintenance"]["phase"],
        "Uploaded"
    );
    assert!(
        !run(
            &dir,
            &[
                "fleet",
                "apply",
                second_id,
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
    assert_eq!(pic.get_stable_memory(store).await, before_second);
    let completed = call(&dir, "apply", second_id, &signer);
    assert_eq!(completed["canister"], store.to_text());
    assert_eq!(call(&dir, "reconcile", second_id, &signer), completed);
    assert_eq!(
        cli(&dir, &["fleet", "status", id])["store_maintenance"]["phase"],
        "Ready"
    );

    config["apps"]["demo"]["canisters"]["root"]["wasm"] =
        json!(assets.join("root-maintenance.wasm"));
    write_config(&dir, &config);
    let maintenance = cli(&dir, &["fleet", "plan", "--controller", &controller]);
    assert_eq!(maintenance["action"], "maintenance");
    assert_eq!(
        maintenance["plan"]["maintenance_store"]["history"],
        serde_json::to_value(vec![first_outcome]).unwrap()
    );
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

    // Rebuild only the generated app through its declared recipe. Infrastructure
    // now selects feature artifacts and must not be replaced by default builds.
    let infrastructure = ["root", "store"].map(|name| {
        let path = PathBuf::from(
            config["apps"]["demo"]["canisters"][name]["wasm"]
                .as_str()
                .unwrap(),
        );
        let bytes = std::fs::read(&path).unwrap();
        (path, bytes)
    });
    let source = dir.join("starter-app/src/endpoints/mod.rs");
    let original = std::fs::read_to_string(&source).unwrap();
    let changed = original.replace("Hello from Canic", "Hello from rebuilt starter");
    assert_ne!(changed, original);
    std::fs::write(&source, changed).unwrap();
    let rebuilt = cli(&dir, &["app", "build", "--canister", "app"]);
    let selected = rebuilt["canisters"].as_array().unwrap();
    assert_eq!(selected.len(), 1);
    let replacement = &selected[0];
    assert_eq!(replacement["canister"], "app");
    for (path, bytes) in infrastructure {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    assert_eq!(std::fs::read(dir.join("Cargo.lock")).unwrap(), lock);
    let publication = cli(&dir, &["fleet", "plan", "--controller", &controller]);
    assert_eq!(publication["action"], "publication");
    let artifacts = cli(&dir, &["fleet", "artifact", "list"]);
    assert_eq!(artifacts["publications"].as_array().unwrap().len(), 2);
    let pending = artifacts["publications"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["operation"] == publication["operation"])
        .unwrap();
    assert_eq!(pending["publication_retained"], false);
    call(
        &dir,
        "apply",
        publication["operation"].as_str().unwrap(),
        &signer,
    );
    let artifacts = cli(&dir, &["fleet", "artifact", "list"]);
    assert!(artifacts["issues"].as_array().unwrap().is_empty());
    assert!(
        artifacts["publications"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["publication_retained"] == true)
    );
    let started = Instant::now();
    let child = cli(&dir, &["fleet", "plan", "--controller", &controller]);
    assert_eq!(
        child["plan"]["management"]["execute_at_ns"],
        child["plan"]["created_at_ns"]
    );
    assert_eq!(child["action"], "upgrade");
    assert_eq!(
        child["plan"]["management"]["source"]["store"]["module"],
        completed["module_hash"]
    );
    assert_eq!(
        child["plan"]["management"]["source"]["maintenance"]["sequence"],
        2
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
    assert_eq!(ready["canister"], application);
    assert_eq!(ready["module_hash"], replacement["artifact"]["sha256"]);
    assert_eq!(
        crate::greeting::call(
            owner.client(),
            CanisterId::from_text(application.as_str().unwrap()).unwrap(),
            "team"
        )
        .await,
        "Hello from rebuilt starter, team!"
    );
    println!(
        "Ready-dispatch child upgrade: {} ms (local wall time, publication already retained)",
        started.elapsed().as_millis()
    );
    assert_eq!(
        cli(&dir, &["fleet", "plan", "--controller", &controller])["action"],
        "unchanged"
    );
    config["apps"]["demo"]["canisters"]["store"]["wasm"] =
        json!(assets.join("store-candidate.wasm"));
    write_config(&dir, &config);
    let third = cli(&dir, &["fleet", "plan", "--controller", &controller]);
    assert_eq!(third["plan"]["store_upgrade_sequence"], 3);
    owner.client_mut().stop_live().await;
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
