//! Actual CLI planning/signing and installed Root/receiver enrollment, using public fixture keys.
use super::*;
use ic_testkit::pocket_ic::CanisterSettings;
use serde_json::{Value, json};
use std::{
    path::Path,
    process::{Command, Output},
};

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(std::env::var_os("CANIC_CLI").expect("make auth-test"))
        .args(args)
        .args(["--json", "--config"])
        .arg(dir.join("canic.toml"))
        .output()
        .unwrap()
}
fn cli(dir: &Path, args: &[&str]) -> Value {
    let result = run(dir, args);
    assert!(
        result.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[tokio::test]
async fn cli_auth_approval_binds_real_code_and_recovers_only_the_original_call() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();
    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(180),
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
    let operator = CanisterId::anonymous();
    let root = owner
        .client()
        .create_canister_with_settings(Some(operator), None)
        .await;
    let receiver = owner
        .client()
        .create_canister_with_settings(Some(operator), None)
        .await;
    let root_wasm = repo.join("target/auth/root.wasm");
    let receiver_wasm = repo.join("target/auth/receiver.wasm");
    owner
        .client()
        .install_canister(
            root,
            std::fs::read(&root_wasm).unwrap(),
            vec![],
            Some(operator),
        )
        .await;
    owner
        .client()
        .install_canister(
            receiver,
            std::fs::read(&receiver_wasm).unwrap(),
            root.as_slice().to_vec(),
            Some(operator),
        )
        .await;
    let url = owner.client_mut().make_live(None).await.to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    let dir = repo
        .join("target/auth")
        .join(format!("host-{}-{now}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let mut config = json!({"apps":{"demo":{
    "canisters":{"root":{"wasm":root_wasm}, "receiver":{"wasm":receiver_wasm}},
    "fleets":{"local":{
        "network":{"kind":"Local", "url":url, "root_key":hex(&owner.client().root_key().await.unwrap()), "root_subnet":owner.client().topology().await.get_nns().unwrap().to_text(), "routing_canister":root.to_text()},
        "journal":"journal", "canisters":{"root":{"id":root.to_text()}, "receiver":{"id":receiver.to_text()}},
        "auth":{"root":"root", "receiver":"receiver", "issuer":Principal::from_slice(&[1]).to_text(),
            "key":hex(&KEY), "network_id":hex(&[0x11;32]), "application_id":hex(&[0x22;32]),
            "role":"user", "scopes":["read"], "not_before_ns":now - SECOND, "expires_at_ns":now + 600 * SECOND}
    }}}}});
    std::fs::write(dir.join("canic.toml"), toml::to_string(&config).unwrap()).unwrap();
    let mut wrong_code = config.clone();
    wrong_code["apps"]["demo"]["canisters"]["root"]["wasm"] =
        config["apps"]["demo"]["canisters"]["receiver"]["wasm"].clone();
    std::fs::write(
        dir.join("canic.toml"),
        toml::to_string(&wrong_code).unwrap(),
    )
    .unwrap();
    let wrong_journal = dir.join("wrong-code");
    let wrong_plan = cli(
        &dir,
        &[
            "fleet",
            "auth",
            "plan",
            "--controller",
            "2vxsx-fae",
            "--state-dir",
            wrong_journal.to_str().unwrap(),
        ],
    );
    let wrong_id = wrong_plan["operation"].as_str().unwrap();
    assert!(
        !run(
            &dir,
            &[
                "fleet",
                "apply",
                wrong_id,
                "--identity",
                "anonymous",
                "--state-dir",
                wrong_journal.to_str().unwrap()
            ]
        )
        .status
        .success()
    );
    assert!(
        !wrong_journal
            .join(wrong_id)
            .join("auth/signed.json")
            .exists()
    );
    std::fs::write(dir.join("canic.toml"), toml::to_string(&config).unwrap()).unwrap();
    let planned = cli(
        &dir,
        &["fleet", "auth", "plan", "--controller", "2vxsx-fae"],
    );
    let id = planned["operation"].as_str().unwrap();
    let effect = dir.join("journal").join(id).join("auth");
    assert!(!effect.exists());
    assert!(
        !run(&dir, &["fleet", "auth", "plan-revocation", id])
            .status
            .success()
    );
    assert!(
        !run(&dir, &["fleet", "reconcile", id, "--identity", "anonymous"])
            .status
            .success()
    );
    // A role/tree declaration cannot replace the actual sole-controller prerequisite.
    assert!(
        !run(&dir, &["fleet", "apply", id, "--identity", "anonymous"])
            .status
            .success()
    );
    assert!(!effect.join("signed.json").exists());
    owner
        .client()
        .update_canister_settings(
            receiver,
            Some(operator),
            CanisterSettings {
                controllers: Some(vec![root]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let configured = cli(&dir, &["fleet", "apply", id, "--identity", "anonymous"]);
    assert_eq!(configured["auth"]["revoked"], false);
    assert!(control(owner.client(), receiver, operator, 1).await);
    let installed = snapshot(owner.client(), receiver).await;
    assert!(!installed.revoked);
    assert_eq!(installed.grant.receiver.as_slice(), receiver.as_slice());
    assert_eq!(
        cli(
            &dir,
            &["fleet", "auth", "plan", "--controller", "2vxsx-fae"]
        ),
        planned
    );
    let signed = std::fs::read(effect.join("signed.json")).unwrap();
    // Edited config cannot widen retained authority or change the revocation's target grant.
    config["apps"]["demo"]["fleets"]["local"]["auth"]["scopes"] = json!(["read", "write"]);
    std::fs::write(dir.join("canic.toml"), toml::to_string(&config).unwrap()).unwrap();
    assert!(
        !run(
            &dir,
            &["fleet", "auth", "plan", "--controller", "2vxsx-fae"]
        )
        .status
        .success()
    );
    let revocation = cli(&dir, &["fleet", "auth", "plan-revocation", id]);
    assert_eq!(
        revocation["plan"]["auth"]["grant"],
        planned["plan"]["auth"]["grant"]
    );
    let revoke_id = revocation["operation"].as_str().unwrap();
    assert_eq!(
        cli(
            &dir,
            &["fleet", "apply", revoke_id, "--identity", "anonymous"]
        )["auth"]["revoked"],
        true
    );
    // Loss of the local reply is recovered under the original signed request, after revocation.
    std::fs::remove_file(effect.join("reply.bin")).unwrap();
    assert!(cli(&dir, &["fleet", "status", id])["auth"].is_null());
    assert_eq!(
        cli(&dir, &["fleet", "reconcile", id, "--identity", "anonymous"]),
        configured
    );
    assert_eq!(std::fs::read(effect.join("signed.json")).unwrap(), signed);
    assert!(control(owner.client(), receiver, operator, 1).await);
    assert!(snapshot(owner.client(), receiver).await.revoked);
    let mut wrong = installed.grant;
    wrong.key[0] ^= 1;
    let response = owner
        .client()
        .update_call(
            root,
            operator,
            "revoke_auth",
            enrollment_request(enrollment::REVOKE, &wrong),
        )
        .await
        .unwrap();
    assert_eq!(
        enrollment_response::<Result<(), enrollment::Error>>(enrollment::REVOKE, &response),
        Err(enrollment::Error::Conflict)
    );
    let status = cli(&dir, &["fleet", "status", id]);
    assert_eq!(status["auth"], configured["auth"]);
    assert!(status["receipt"].is_null());
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
