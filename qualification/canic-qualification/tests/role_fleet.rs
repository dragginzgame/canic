//! The fleet approval owns one authenticated key result through interrupted apply.
use super::*;

#[tokio::test(flavor = "current_thread")]
async fn pin_delivery_resumes_before_stage() {
    let assets = PathBuf::from(std::env::var_os("CANIC_PUBLICATION_ARTIFACTS").unwrap());
    let dir = assets.join(format!("role-fleet-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let (mut server, mut owner) = fleet_server().await;
    let routing = owner.client().create_canister().await;
    let url = owner.client_mut().make_live(None).await.to_string();
    let pic = owner.client();
    let config = json!({"apps":{"demo":{
        "canisters":{"root":{"role":"root","wasm":assets.join("role-root.wasm")},
            "store":{"role":"wasm-store","parent":"root","wasm":assets.join("store.wasm")},
            "hub":{"role":"accounts","parent":"root","wasm":assets.join("role-hub.wasm")}},
        "fleets":{"local":{"network":{"kind":"Local","url":url,"root_key":hex(&pic.root_key().await.unwrap()),"root_subnet":pic.topology().await.get_nns().unwrap().to_text(),"routing_canister":routing.to_text()},"journal":"journal","canisters":{},"role_receivers":["hub"]}}
    }}});
    std::fs::write(dir.join("canic.toml"), toml::to_string(&config).unwrap()).unwrap();
    let plan = cli(&dir, &["fleet", "plan", "--controller", "2vxsx-fae"]);
    let id = plan["operation"].as_str().unwrap();
    assert_eq!(plan["fleet_plan"]["child"]["receiver_role"], "accounts");
    let path = dir.join("journal").join(id);
    let signer = assets.ancestors().nth(2).unwrap().join(".tools/ic/bin/icp");
    let pause = dir.join("pause-stage");
    std::fs::write(&pause, format!("#!/bin/sh\nfor arg in \"$@\"; do\n case \"$arg\" in */child/stage/arguments.bin) exit 72;; esac\ndone\nexec '{}' \"$@\"\n",signer.display())).unwrap();
    std::fs::set_permissions(&pause, std::fs::Permissions::from_mode(0o700)).unwrap();
    let apply = |signer: &Path| {
        run(
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
    let paused = apply(&pause);
    assert!(!paused.status.success());
    assert!(
        path.join("role-key/reply.bin").exists(),
        "{}",
        String::from_utf8_lossy(&paused.stderr)
    );
    assert!(!path.join("child/stage/signed.json").exists());
    let signed = std::fs::read(path.join("role-key/signed.json")).unwrap();
    let arguments = std::fs::read(path.join("child/plan.json")).unwrap();
    let resumed = apply(&signer);
    assert!(
        resumed.status.success(),
        "{}",
        String::from_utf8_lossy(&resumed.stderr)
    );
    pic.stop_progress().await;
    pic.advance_time(Duration::from_secs(2)).await;
    for _ in 0..80 {
        pic.tick().await;
    }
    pic.auto_progress().await;
    let result = apply(&signer);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(
        result["children"][0]["outcome"]["canister"].is_string(),
        "{result}"
    );
    assert_eq!(
        std::fs::read(path.join("role-key/signed.json")).unwrap(),
        signed
    );
    assert_eq!(
        std::fs::read(path.join("child/plan.json")).unwrap(),
        arguments
    );
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}
