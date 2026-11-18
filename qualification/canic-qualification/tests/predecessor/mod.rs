//! The old allocator Root is preserved when a new records executable refuses its image.
use super::*;
use canic_management_protocol::upgrade;

#[tokio::test]
async fn retained_predecessor_maintenance_refuses_old_storage_without_reset() {
    for completed in [false, true] {
        qualify_predecessor(completed, false).await;
    }
}

#[tokio::test]
async fn retained_predecessor_maintenance_refuses_untracked_scheduled_work() {
    qualify_predecessor(false, true).await;
}

async fn qualify_predecessor(completed: bool, untracked: bool) {
    let original = std::fs::read(
        repo().join("qualification/canic-qualification/fixtures/root-single-upgrade.wasm"),
    )
    .unwrap();
    let hash: String = digest(&original)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        hash,
        "e4a4127abd525f59378503dcdb98fcb13f8eb9895673d4d8a4dea41d85e24296"
    );
    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(240),
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
        "target/root/predecessor-{}-{completed}-{untracked}",
        std::process::id()
    ));
    let initialized = {
        let created = std::process::Command::new(std::env::var_os("CANIC_CLI").unwrap())
            .args([
                "app",
                "init",
                "transition",
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
                "transition",
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
    let artifacts = repo().join("target/root");
    std::fs::create_dir_all(workspace.join("artifacts")).unwrap();
    std::fs::write(workspace.join("artifacts/root.wasm"), &original).unwrap();
    std::fs::copy(
        artifacts.join("app.wasm"),
        workspace.join("artifacts/app.wasm"),
    )
    .unwrap();
    let config = workspace.join("canic.toml");
    inline_fixture(&config, "transition");
    let dispatch = || {
        (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 40)
            .to_string()
    };
    let creation = cli(
        &config,
        &[
            "fleet",
            "plan",
            "--identity",
            "anonymous",
            "--execute-at",
            &dispatch(),
        ],
    );
    let creation_id = creation["operation"].as_str().unwrap();
    let pending = cli(
        &config,
        &["fleet", "apply", creation_id, "--identity", "anonymous"],
    );
    let root = CanisterId::from_text(pending["root"]["canister"].as_str().unwrap()).unwrap();
    owner.client().stop_progress().await;
    owner.client().advance_time(Duration::from_secs(45)).await;
    settle(owner.client()).await;
    owner.client().auto_progress().await;
    let created = cli(
        &config,
        &["fleet", "apply", creation_id, "--identity", "anonymous"],
    );
    let child = CanisterId::from_text(
        created["children"][0]["outcome"]["canister"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let journal = PathBuf::from(creation["state_dir"].as_str().unwrap());
    let creation_bytes = std::fs::read(journal.join(creation_id).join("fleet.json")).unwrap();
    let before = status(owner.client(), root).await;
    let child_code = std::fs::read(artifacts.join("app-v2.wasm")).unwrap();
    let mut old_upgrade_record = None;
    if completed {
        std::fs::write(workspace.join("artifacts/app.wasm"), &child_code).unwrap();
        let at = (owner.client().get_time().await.as_nanos_since_unix_epoch() / 1_000_000_000 + 40)
            .to_string();
        let planned = cli(
            &config,
            &[
                "fleet",
                "plan",
                "--identity",
                "anonymous",
                "--execute-at",
                &at,
            ],
        );
        let id = planned["operation"].as_str().unwrap();
        assert_eq!(
            cli(&config, &["fleet", "apply", id, "--identity", "anonymous"])["upgrade"]["phase"],
            "Scheduled"
        );
        owner.client().stop_progress().await;
        owner.client().advance_time(Duration::from_secs(45)).await;
        settle(owner.client()).await;
        owner.client().auto_progress().await;
        // Model custody already retained by the previous CLI: an authenticated
        // ID-25 reply remains readable, without adding live fallback to child apply.
        let reply = owner
            .client()
            .update_call(
                root,
                CanisterId::anonymous(),
                "observe_upgrade",
                ic_wire::encode(
                    &(),
                    ic_wire::Encoding::Frame {
                        method: upgrade::STATUS,
                        direction: ic_wire::Direction::Request,
                    },
                    LIMITS,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let old: upgrade::Status = ic_wire::decode(
            &reply,
            ic_wire::Encoding::Frame {
                method: upgrade::STATUS,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap();
        assert_eq!(old.phase, upgrade::Phase::Ready);
        let dir = journal.join(id).join("observe/0");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("reply.bin"), &reply).unwrap();
        assert_eq!(
            cli(&config, &["fleet", "apply", id, "--identity", "anonymous"])["canister"],
            child.to_text()
        );
        old_upgrade_record = Some((dir.join("reply.bin"), reply));
    }
    std::fs::copy(
        artifacts.join("root.wasm"),
        workspace.join("artifacts/root.wasm"),
    )
    .unwrap();
    let maintenance = cli(&config, &["fleet", "plan", "--identity", "anonymous"]);
    assert_eq!(maintenance["action"], "maintenance");
    let id = maintenance["operation"].as_str().unwrap();
    let dir = journal.join(id);
    let apply = ["fleet", "apply", id, "--identity", "anonymous"];
    if untracked {
        let now = owner.client().get_time().await.as_nanos_since_unix_epoch();
        let input = upgrade::Stage {
            approval: upgrade::Approval {
                manager: root.to_text(),
                child: child.to_text(),
                previous_module: digest(&std::fs::read(artifacts.join("app.wasm")).unwrap()),
                module_hash: digest(&child_code),
                execute_at_ns: now + 40_000_000_000,
                expires_at_ns: now + 640_000_000_000,
            },
            wasm: child_code,
        };
        let reply = owner
            .client()
            .update_call(
                root,
                CanisterId::anonymous(),
                "stage_upgrade",
                ic_wire::encode(
                    &input,
                    ic_wire::Encoding::Frame {
                        method: upgrade::STAGE,
                        direction: ic_wire::Direction::Request,
                    },
                    STAGE_LIMITS,
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let admitted: Result<upgrade::Status, Refusal> = ic_wire::decode(
            &reply,
            ic_wire::Encoding::Frame {
                method: upgrade::STAGE,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .unwrap();
        assert_eq!(admitted.unwrap().phase, upgrade::Phase::Scheduled);
        let refused = std::process::Command::new(std::env::var_os("CANIC_CLI").unwrap())
            .args(apply)
            .args(["--config", config.to_str().unwrap(), "--json"])
            .output()
            .unwrap();
        assert!(!refused.status.success());
        assert!(!dir.join("install/signed.json").exists());
        assert!(!dir.join("receipt.json").exists());
        assert_eq!(
            owner
                .client()
                .canister_status(root, None)
                .await
                .unwrap()
                .module_hash,
            Some(digest(&original).to_vec())
        );
        assert_eq!(
            owner.close(std::time::Duration::from_secs(10)).await,
            ic_testkit::server::Deletion::Attempted
        );
        let stopped = server.stop().unwrap();
        assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
        return;
    }
    let stable = owner.client().get_stable_memory(root).await;
    let refused = std::process::Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(apply)
        .args(["--config", config.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(!dir.join("receipt.json").exists());
    assert_eq!(owner.client().get_stable_memory(root).await, stable);
    let install = std::fs::read(dir.join("install/signed.json")).unwrap();
    let original_status = owner.client().canister_status(root, None).await.unwrap();
    assert_eq!(
        original_status.module_hash,
        Some(digest(&original).to_vec())
    );
    assert_eq!(format!("{:?}", original_status.status), "Stopped");
    let replay = std::process::Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(apply)
        .args(["--config", config.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert!(!replay.status.success());
    assert_eq!(
        std::fs::read(dir.join("install/signed.json")).unwrap(),
        install
    );
    assert_eq!(
        owner
            .client()
            .canister_status(root, None)
            .await
            .unwrap()
            .version,
        original_status.version
    );
    assert_eq!(
        std::fs::read(journal.join(creation_id).join("fleet.json")).unwrap(),
        creation_bytes
    );
    if let Some((path, bytes)) = old_upgrade_record {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    // Explicit fixture recovery restarts the retained old executable, not a replacement.
    owner.client().start_canister(root, None).await.unwrap();
    assert_eq!(status(owner.client(), root).await, before);
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
