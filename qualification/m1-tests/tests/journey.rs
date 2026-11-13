//! Exercise the shipped CLI process, with PocketIC confined to its own graph.

use ic_testkit::{
    pic::{PocketIcStartupConfig, server_assets},
    pocket_ic::{CanisterId, PocketIcBuilder},
};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

fn cli(root: &Path, args: &[&str]) -> Value {
    let output = Command::new(std::env::var_os("CANIC_CLI").expect("run make m1-test"))
        .arg("--state-dir")
        .arg(root)
        .arg("--json")
        .args(args)
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
    let binary = server_assets::check(&repo.join("target/m0/server")).expect("run make m0-setup");
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
    let url = owner.instance_mut().make_live(None).await.to_string();
    let root_key: String = owner
        .instance()
        .root_key()
        .await
        .unwrap()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let routing_canister = owner.instance().create_canister().await.to_text();
    let root = repo
        .join("target/m1-tests")
        .join(format!("journey-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let wasm = std::env::var("CANIC_HELLO_V1").unwrap();
    let wasm_v2 = std::env::var("CANIC_HELLO_V2").unwrap();
    let icp = repo.join(".tools/ic/bin/icp");
    let plan = cli(
        &root,
        &[
            "plan",
            "--wasm",
            &wasm,
            "--network",
            "local",
            "--url",
            &url,
            "--root-key",
            &root_key,
            "--routing-canister",
            &routing_canister,
            "--controller",
            "2vxsx-fae",
        ],
    );
    let id = plan["operation"].as_str().unwrap();
    let receipt = cli(
        &root,
        &[
            "apply",
            id,
            "--identity",
            "anonymous",
            "--icp-bin",
            icp.to_str().unwrap(),
        ],
    );
    let target = receipt["canister"].as_str().unwrap();
    let target_id = CanisterId::from_text(target).unwrap();
    let subnet = owner.instance().get_subnet(target_id).await.unwrap();
    let count = owner
        .instance()
        .get_subnet_metrics(subnet)
        .await
        .unwrap()
        .num_canisters;
    assert_eq!(
        cli(&root, &["call", id, "--name", "team"])["greeting"],
        "Hello from Canic, team!"
    );
    // Completed replay neither signs nor creates another canister.
    assert_eq!(
        cli(
            &root,
            &[
                "apply",
                id,
                "--identity",
                "anonymous",
                "--icp-bin",
                "/missing-signer"
            ]
        ),
        receipt
    );
    assert_eq!(
        owner
            .instance()
            .get_subnet_metrics(subnet)
            .await
            .unwrap()
            .num_canisters,
        count
    );
    // Simulate interruption after committed effect replies but before final
    // receipt publication. Recovery must use those replies without a signer.
    std::fs::remove_file(root.join(id).join("receipt.json")).unwrap();
    assert_eq!(
        cli(
            &root,
            &[
                "apply",
                id,
                "--identity",
                "anonymous",
                "--icp-bin",
                "/missing-signer"
            ]
        ),
        receipt
    );
    // Model process loss after network completion but before saving the creation
    // reply. Only the original signed request may be resubmitted; no signer exists.
    let signed_path = root.join(id).join("create/signed.json");
    let original_signed = std::fs::read(&signed_path).unwrap();
    std::fs::remove_file(root.join(id).join("receipt.json")).unwrap();
    std::fs::remove_file(root.join(id).join("create/reply.bin")).unwrap();
    assert_eq!(
        cli(
            &root,
            &[
                "apply",
                id,
                "--identity",
                "anonymous",
                "--icp-bin",
                "/missing-signer"
            ]
        ),
        receipt
    );
    assert_eq!(std::fs::read(&signed_path).unwrap(), original_signed);
    assert_eq!(
        owner
            .instance()
            .get_subnet_metrics(subnet)
            .await
            .unwrap()
            .num_canisters,
        count
    );
    let upgrade = cli(
        &root,
        &[
            "plan",
            "--wasm",
            &wasm_v2,
            "--network",
            "local",
            "--url",
            &url,
            "--root-key",
            &root_key,
            "--routing-canister",
            &routing_canister,
            "--controller",
            "2vxsx-fae",
            "--target",
            target,
            "--previous-module",
            receipt["module_hash"].as_str().unwrap(),
        ],
    );
    let upgrade_id = upgrade["operation"].as_str().unwrap();
    let upgraded = cli(
        &root,
        &[
            "apply",
            upgrade_id,
            "--identity",
            "anonymous",
            "--icp-bin",
            icp.to_str().unwrap(),
        ],
    );
    assert_eq!(upgraded["canister"], target);
    assert_ne!(upgraded["module_hash"], receipt["module_hash"]);
    assert_eq!(
        cli(&root, &["call", upgrade_id, "--name", "team"])["greeting"],
        "Hello again from Canic, team!"
    );
    let signed_path = root.join(upgrade_id).join("install/signed.json");
    let original_signed = std::fs::read(&signed_path).unwrap();
    std::fs::remove_file(root.join(upgrade_id).join("receipt.json")).unwrap();
    std::fs::remove_file(root.join(upgrade_id).join("install/reply.bin")).unwrap();
    assert_eq!(
        cli(
            &root,
            &[
                "apply",
                upgrade_id,
                "--identity",
                "anonymous",
                "--icp-bin",
                "/missing-signer"
            ]
        ),
        upgraded
    );
    assert_eq!(std::fs::read(&signed_path).unwrap(), original_signed);
    assert_eq!(
        owner
            .instance()
            .get_subnet_metrics(subnet)
            .await
            .unwrap()
            .num_canisters,
        count
    );
    println!(
        "deployed {target}, retained replay and receipt recovery passed, upgraded same canister to greeting v2"
    );
    owner.close().await;
}
