//! Three-canister fleet through the shipped CLI, including custody and operator views.
use super::*;

fn rejected(config: &std::path::Path, args: &[&str]) -> serde_json::Value {
    let output = std::process::Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(args)
        .args(["--config", config.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    // The external signer may log before Canic's final structured error.
    serde_json::from_slice(
        output
            .stderr
            .trim_ascii_end()
            .rsplit(|b| *b == b'\n')
            .next()
            .unwrap(),
    )
    .unwrap_or_else(|error| {
        panic!(
            "{args:?}: {error}; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

async fn advance(pic: &PocketIc) {
    pic.stop_progress().await;
    pic.advance_time(Duration::from_secs(65)).await;
    settle(pic).await;
    pic.auto_progress().await;
}

#[tokio::test(flavor = "current_thread")]
async fn multi_child_cli_creation_upgrades_maintenance_and_views() {
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
    let workspace = repo().join(format!(
        "target/root/multi-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let output = {
        let created = std::process::Command::new(std::env::var_os("CANIC_CLI").unwrap())
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
        std::process::Command::new(std::env::var_os("CANIC_CLI").unwrap())
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
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let config = workspace.join("canic.toml");
    inline_fixture(&config, "starter");
    cli(
        &config,
        &[
            "app",
            "canister",
            "add",
            "worker",
            "--parent",
            "root",
            "--role",
            "worker",
            "--wasm",
            "artifacts/worker.wasm",
            "--manifest",
            "Cargo.toml",
            "--package",
            "starter-app",
        ],
    );
    cli(&config, &["app", "build"]);
    let artifacts = repo().join("target/root");
    for (source, target) in [
        ("root.wasm", "root.wasm"),
        ("app.wasm", "app.wasm"),
        ("app.wasm", "worker.wasm"),
    ] {
        std::fs::copy(
            artifacts.join(source),
            workspace.join("artifacts").join(target),
        )
        .unwrap();
    }
    let config_bytes = std::fs::read(&config).unwrap();
    let dispatch = (owner.client().get_time().await.as_nanos_since_unix_epoch() / 1_000_000_000
        + 60)
        .to_string();
    let planned = cli(
        &config,
        &[
            "fleet",
            "plan",
            "--identity",
            "anonymous",
            "--execute-at",
            &dispatch,
        ],
    );
    let id = planned["operation"].as_str().unwrap();
    assert_eq!(planned["fleet_plan"]["version"], 2);
    assert_eq!(
        planned["fleet_plan"]["child"]["deployment"]["canister"],
        "app"
    );
    assert_eq!(
        planned["fleet_plan"]["sibling"]["deployment"]["canister"],
        "worker"
    );
    let journal = PathBuf::from(planned["state_dir"].as_str().unwrap());
    assert_eq!(
        cli(&config, &["fleet", "plan", "--identity", "anonymous"]),
        planned
    );
    let apply = ["fleet", "apply", id, "--identity", "anonymous"];
    let pending = cli(&config, &apply);
    assert_eq!(
        pending["children"][0]["outcome"]["status"]["phase"],
        "Scheduled"
    );
    assert_eq!(
        pending["children"][1]["outcome"]["status"]["phase"],
        "Scheduled"
    );
    let summary = status_text(&config, id, "local");
    assert!(summary.contains("\nChild: app\nRetained Root phase: Scheduled"));
    assert!(summary.contains("\nChild: worker\nRetained Root phase: Scheduled"));
    let path = journal.join(id);
    let first_signed = std::fs::read(path.join("child/stage/signed.json")).unwrap();
    let second_signed = std::fs::read(path.join("sibling/stage/signed.json")).unwrap();
    assert_ne!(first_signed, second_signed); // Same Wasm/approval, different authenticated slot.
    // A declaration cannot enlarge an already admitted fleet or remap its children.
    owner.client().stop_progress().await;
    let root = CanisterId::from_text(pending["root"]["canister"].as_str().unwrap()).unwrap();
    let stable = owner.client().get_stable_memory(root).await;
    let approval_bytes = std::fs::read(path.join("fleet.json")).unwrap();
    let operations = std::fs::read_dir(&journal).unwrap().count();
    let mut expanded: serde_json::Value =
        toml::from_str(std::str::from_utf8(&config_bytes).unwrap()).unwrap();
    expanded["apps"]["starter"]["canisters"]["aaa"] = serde_json::json!({
        "parent": "root", "role": "worker", "wasm": "missing-third.wasm"
    });
    std::fs::write(&config, toml::to_string(&expanded).unwrap()).unwrap();
    let error = rejected(&config, &["fleet", "plan", "--identity", "anonymous"]);
    assert!(error["message"].as_str().unwrap().contains(
        "fleet declares 3 direct application children under root; this Root path supports 1 to 2"
    ), "{error}");
    assert_eq!(std::fs::read_dir(&journal).unwrap().count(), operations);
    assert_eq!(
        std::fs::read(path.join("fleet.json")).unwrap(),
        approval_bytes
    );
    assert_eq!(
        std::fs::read(path.join("child/stage/signed.json")).unwrap(),
        first_signed
    );
    assert_eq!(
        std::fs::read(path.join("sibling/stage/signed.json")).unwrap(),
        second_signed
    );
    assert_eq!(owner.client().get_stable_memory(root).await, stable);
    std::fs::write(&config, &config_bytes).unwrap();
    assert_eq!(
        cli(&config, &["fleet", "plan", "--identity", "anonymous"]),
        planned
    );
    owner.client().auto_progress().await;
    std::fs::remove_file(path.join("sibling/stage/reply.bin")).unwrap();
    let recovered = cli(&config, &apply);
    assert_eq!(
        recovered["children"][1]["outcome"]["status"]["phase"],
        "Scheduled"
    );
    assert_eq!(
        std::fs::read(path.join("sibling/stage/signed.json")).unwrap(),
        second_signed
    );
    assert_eq!(
        rejected(&config, &["fleet", "cancel", id])["error"],
        "blocked"
    );
    cli(
        &config,
        &["fleet", "reconcile", id, "--identity", "anonymous"],
    );
    advance(owner.client()).await;
    let report = workspace.join("fleet.html");
    let before_receipts = cli(
        &config,
        &["fleet", "show", "--html", report.to_str().unwrap()],
    );
    assert_eq!(before_receipts["canisters"].as_array().unwrap().len(), 3);
    assert_eq!(before_receipts["managers"].as_array().unwrap().len(), 2);
    assert!(
        before_receipts["managers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["reported"]["phase"] == "Ready" && m["observation_error"].is_null())
    );
    assert!(!path.join("child/receipt.json").exists());
    assert!(!path.join("sibling/receipt.json").exists());
    let html = std::fs::read_to_string(&report).unwrap();
    assert!(html.contains("Child: <strong>slot 1 · app</strong>"));
    assert!(html.contains("Child: <strong>slot 2 · worker</strong>"));
    let complete = cli(&config, &apply);
    let root = CanisterId::from_text(complete["root"]["canister"].as_str().unwrap()).unwrap();
    let app = complete["children"][0]["outcome"]["canister"]
        .as_str()
        .unwrap();
    let worker = complete["children"][1]["outcome"]["canister"]
        .as_str()
        .unwrap();
    assert_ne!(app, worker);
    for child in [app, worker] {
        assert_eq!(
            owner
                .client()
                .canister_status(CanisterId::from_text(child).unwrap(), Some(root))
                .await
                .unwrap()
                .settings
                .controllers,
            [root]
        );
    }
    assert_eq!(
        cli(&config, &["fleet", "plan", "--identity", "anonymous"])["action"],
        "unchanged"
    );
    for child in [app, worker] {
        assert_eq!(
            greeting::call(
                owner.client(),
                CanisterId::from_text(child).unwrap(),
                "team"
            )
            .await,
            "Hello from Canic, team!"
        );
    }
    let terminal = std::process::Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(["fleet", "show", "--config", config.to_str().unwrap()])
        .output()
        .unwrap();
    let terminal = String::from_utf8(terminal.stdout).unwrap();
    assert!(terminal.contains("root / slot 1 · app"));
    assert!(terminal.contains("root / slot 2 · worker"));
    // Configuration cannot remove/reassign a retained child or redirect its slot.
    let mut renamed: serde_json::Value =
        toml::from_str(std::str::from_utf8(&config_bytes).unwrap()).unwrap();
    let definition = renamed["apps"]["starter"]["canisters"]
        .as_object_mut()
        .unwrap()
        .remove("worker")
        .unwrap();
    renamed["apps"]["starter"]["canisters"]["aaa"] = definition;
    std::fs::write(&config, toml::to_string(&renamed).unwrap()).unwrap();
    assert_eq!(
        rejected(&config, &["fleet", "plan", "--identity", "anonymous"])["error"],
        "blocked"
    );
    std::fs::write(&config, &config_bytes).unwrap();
    let creation_receipts = [
        std::fs::read(path.join("child/receipt.json")).unwrap(),
        std::fs::read(path.join("sibling/receipt.json")).unwrap(),
    ];
    for (name, child, slot) in [("app", app, "First"), ("worker", worker, "Second")] {
        std::fs::copy(
            artifacts.join("app-v2.wasm"),
            workspace.join(format!("artifacts/{name}.wasm")),
        )
        .unwrap();
        let dispatch =
            (owner.client().get_time().await.as_nanos_since_unix_epoch() / 1_000_000_000 + 60)
                .to_string();
        let upgrade = if name == "worker" {
            // Named planning must recover Second from creation custody, not
            // silently use the original first-slot endpoint.
            let root_module = complete["root"]["module_hash"].as_str().unwrap();
            let previous_module = complete["children"][1]["outcome"]["module_hash"]
                .as_str()
                .unwrap();
            cli(
                &config,
                &[
                    "fleet",
                    "plan",
                    "--canister",
                    name,
                    "--root-module",
                    root_module,
                    "--target",
                    child,
                    "--previous-module",
                    previous_module,
                    "--identity",
                    "anonymous",
                    "--execute-at",
                    &dispatch,
                ],
            )
        } else {
            cli(
                &config,
                &[
                    "fleet",
                    "plan",
                    "--identity",
                    "anonymous",
                    "--execute-at",
                    &dispatch,
                ],
            )
        };
        if name == "app" {
            assert_eq!(upgrade["action"], "upgrade");
        }
        assert_eq!(upgrade["plan"]["management"]["slot"], slot);
        assert_eq!(upgrade["plan"]["target"], child);
        let operation = upgrade["operation"].as_str().unwrap();
        let apply = ["fleet", "apply", operation, "--identity", "anonymous"];
        assert_eq!(cli(&config, &apply)["upgrade"]["phase"], "Scheduled");
        let upgrade_dir = journal.join(operation);
        let signed = std::fs::read(upgrade_dir.join("stage/signed.json")).unwrap();
        std::fs::remove_file(upgrade_dir.join("stage/reply.bin")).unwrap();
        assert_eq!(cli(&config, &apply)["upgrade"]["phase"], "Scheduled");
        cli(
            &config,
            &["fleet", "reconcile", operation, "--identity", "anonymous"],
        );
        advance(owner.client()).await;
        let view = cli(&config, &["fleet", "show"]);
        let selected = view["managers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["slot"] == slot)
            .unwrap();
        assert_eq!(selected["upgrade"]["reported"]["phase"], "Ready");
        assert!(!upgrade_dir.join("receipt.json").exists());
        assert_eq!(cli(&config, &apply)["canister"], child);
        assert_eq!(
            std::fs::read(upgrade_dir.join("stage/signed.json")).unwrap(),
            signed
        );
        assert_eq!(
            cli(&config, &["fleet", "plan", "--identity", "anonymous"])["action"],
            "unchanged"
        );
    }
    // Maintenance covers both full journals and verifies both children afterwards.
    std::fs::copy(
        artifacts.join("maintenance.wasm"),
        workspace.join("artifacts/root.wasm"),
    )
    .unwrap();
    let maintenance = cli(&config, &["fleet", "plan", "--identity", "anonymous"]);
    assert_eq!(maintenance["action"], "maintenance");
    assert_eq!(
        maintenance["plan"]["maintenance"]["upgrades"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        maintenance["plan"]["maintenance_sibling"]["upgrades"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let operation = maintenance["operation"].as_str().unwrap();
    let maintained = cli(
        &config,
        &["fleet", "apply", operation, "--identity", "anonymous"],
    );
    assert_eq!(maintained["canister"], root.to_text());
    assert!(
        journal
            .join(operation)
            .join("custody-before/reply.bin")
            .exists()
    );
    assert!(
        journal
            .join(operation)
            .join("custody-after/reply.bin")
            .exists()
    );
    assert_eq!(
        cli(&config, &["fleet", "plan", "--identity", "anonymous"])["action"],
        "unchanged"
    );
    assert_eq!(
        std::fs::read(path.join("child/receipt.json")).unwrap(),
        creation_receipts[0]
    );
    assert_eq!(
        std::fs::read(path.join("sibling/receipt.json")).unwrap(),
        creation_receipts[1]
    );
    assert_eq!(std::fs::read(&config).unwrap(), config_bytes);
    // A second-child change outside the retained workspace must block Root
    // maintenance before an install attempt, just like a first-child change.
    std::fs::copy(
        artifacts.join("root.wasm"),
        workspace.join("artifacts/root.wasm"),
    )
    .unwrap();
    let blocked = cli(&config, &["fleet", "plan", "--identity", "anonymous"]);
    assert_eq!(blocked["action"], "maintenance");
    let blocked_id = blocked["operation"].as_str().unwrap();
    let wasm = std::fs::read(artifacts.join("app.wasm")).unwrap();
    let now = owner.client().get_time().await.as_nanos_since_unix_epoch();
    let change = canic_management_protocol::upgrade::Stage {
        approval: canic_management_protocol::upgrade::Approval {
            manager: root.to_text(),
            child: worker.into(),
            previous_module: digest(&std::fs::read(artifacts.join("app-v2.wasm")).unwrap()),
            module_hash: digest(&wasm),
            execute_at_ns: now + 60_000_000_000,
            expires_at_ns: now + 660_000_000_000,
        },
        wasm,
    };
    let reply = owner
        .client()
        .update_call(
            root,
            CanisterId::anonymous(),
            "stage_child_upgrade",
            ic_wire::encode(
                &canic_management_protocol::children::Request {
                    slot: canic_management_protocol::children::ChildId::SECOND,
                    value: change,
                },
                ic_wire::Encoding::Frame {
                    method: canic_management_protocol::children::STAGE_UPGRADE,
                    direction: ic_wire::Direction::Request,
                },
                STAGE_LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let admitted: Result<canic_management_protocol::upgrade::Status, Refusal> = ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: canic_management_protocol::children::STAGE_UPGRADE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap();
    admitted.unwrap();
    assert_eq!(
        rejected(
            &config,
            &["fleet", "apply", blocked_id, "--identity", "anonymous"]
        )["error"],
        "blocked"
    );
    assert!(
        !journal
            .join(blocked_id)
            .join("install/signed.json")
            .exists()
    );
    assert!(!journal.join(blocked_id).join("receipt.json").exists());
    assert_eq!(
        owner
            .client()
            .canister_status(root, None)
            .await
            .unwrap()
            .module_hash,
        Some(digest(&std::fs::read(artifacts.join("maintenance.wasm")).unwrap()).to_vec())
    );
    let subnet = owner.client().get_subnet(root).await.unwrap();
    assert_eq!(
        owner
            .client()
            .get_subnet_metrics(subnet)
            .await
            .unwrap()
            .num_canisters,
        4
    ); // Includes local routing fixture.
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
