//! Public child discovery must not depend on the observing operator's journal.
use super::*;

#[tokio::test(flavor = "current_thread")]
async fn discovers_both_children_without_local_deployment_records() {
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
    let root = owner.client().create_canister().await;
    owner
        .client()
        .install_canister(root, artifact("root.wasm"), vec![], None)
        .await;
    owner.client().add_cycles(root, 10_000_000_000_000).await;
    let url = owner.client_mut().make_live(None).await.to_string();
    let pic = owner.client();
    let key: String = pic
        .root_key()
        .await
        .unwrap()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let directory = repo().join(format!(
        "target/root/discovery-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let config = directory.join("canic.json");
    let mut value = serde_json::json!({"apps":{"discovery":{
        "canisters":{"root":{"role":"root"}},
        "fleets":{"local":{
            "network":{"kind":"Local","url":url,"root_key":key,"root_subnet":pic.topology().await.get_nns().unwrap().to_text(),"routing_canister":root.to_text()},
            "journal":"journal","canisters":{"root":{"id":root.to_text(),"profile_manager":true}}
        }}
    }}});
    std::fs::write(&config, serde_json::to_vec(&value).unwrap()).unwrap();
    let show = |report: &std::path::Path| {
        cli(
            &config,
            &["fleet", "show", "--html", report.to_str().unwrap()],
        )
    };
    let stable = pic.get_stable_memory(root).await;
    let empty = show(&directory.join("empty.html"));
    assert_eq!(empty["canisters"].as_array().unwrap().len(), 1);
    assert_eq!(empty["managers"].as_array().unwrap().len(), 1);
    assert_eq!(empty["issues"], serde_json::json!([]));
    assert_eq!(pic.get_stable_memory(root).await, stable);

    // Admit remotely, in reverse order, without giving Host either child's ID.
    let mut discovered = Vec::new();
    for slot in [ChildId::SECOND, ChildId::FIRST] {
        pic.stop_progress().await;
        let stage = input(
            root,
            artifact("app.wasm"),
            pic.get_time().await.as_nanos_since_unix_epoch(),
        );
        admit(pic, root, slot, stage).await.unwrap();
        pic.advance_time(Duration::from_secs(11)).await;
        settle(pic).await;
        let custody = read(pic, root).await;
        let status = &custody[slot.index()].creation;
        assert_eq!(status.phase, Phase::Ready);
        discovered.push(status.worker.clone().unwrap());
        let stable = pic.get_stable_memory(root).await;
        pic.auto_progress().await;
        let report = directory.join(format!("after-slot-{}.html", slot.index() + 1));
        let observed = show(&report);
        assert_eq!(observed["operations"], serde_json::json!([]));
        assert_eq!(observed["issues"], serde_json::json!([]));
        assert_eq!(observed["managers"].as_array().unwrap().len(), 2);
        assert_eq!(observed["metrics"].as_array().unwrap().len(), 1);
        let canisters = observed["canisters"].as_array().unwrap();
        assert_eq!(canisters.len(), discovered.len() + 1);
        for child in &discovered {
            let row = canisters
                .iter()
                .find(|row| row["canister"] == *child)
                .unwrap();
            assert_eq!(row["controllers"], serde_json::json!([root.to_text()]));
            assert!(row["observation_error"].is_null());
        }
        let html = std::fs::read_to_string(&report).unwrap();
        for (index, child) in discovered.iter().enumerate() {
            assert!(html.contains(&format!(
                "Child: <strong>slot {} · {child}</strong>",
                2 - index
            )));
        }
        assert_eq!(pic.get_stable_memory(root).await, stable);
    }
    let stable = pic.get_stable_memory(root).await;
    let output = std::process::Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args(["fleet", "show", "--color", "never", "--config"])
        .arg(&config)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    for (index, child) in discovered.iter().enumerate() {
        assert!(
            text.contains(&format!(
                "root / slot {} · {child} — creation Ready",
                2 - index
            )),
            "{text}"
        );
    }
    assert_eq!(pic.get_stable_memory(root).await, stable);
    assert!(!directory.join("journal").exists());

    // An older executable's absent slot endpoint is unknown, not an empty slot.
    let old = pic.create_canister().await;
    pic.install_canister(
        old,
        std::fs::read(
            repo().join("qualification/canic-qualification/fixtures/root-one-child-history.wasm"),
        )
        .unwrap(),
        vec![],
        None,
    )
    .await;
    value["apps"]["discovery"]["fleets"]["local"]["canisters"]["root"]["id"] =
        serde_json::json!(old.to_text());
    std::fs::write(&config, serde_json::to_vec(&value).unwrap()).unwrap();
    let stable = pic.get_stable_memory(old).await;
    let unknown = show(&directory.join("older-root.html"));
    assert_eq!(unknown["canisters"].as_array().unwrap().len(), 1);
    assert_eq!(unknown["managers"].as_array().unwrap().len(), 1);
    assert!(unknown["managers"][0]["reported"].is_object());
    let issues = unknown["issues"].as_array().unwrap();
    assert_eq!(issues.len(), 1);
    assert!(
        issues[0]
            .as_str()
            .unwrap()
            .contains("second child slot unknown")
    );
    assert_eq!(pic.get_stable_memory(old).await, stable);
    assert!(!directory.join("journal").exists());
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
