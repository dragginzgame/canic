//! Public child discovery must not depend on the observing operator's journal.
use super::*;

#[tokio::test(flavor = "current_thread")]
async fn discovers_nine_children_without_local_deployment_records() {
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
        .install_canister(
            root,
            artifact("root.wasm"),
            ic_wire::encode(
                &canic_management_protocol::stored::RootBootstrap {
                    services: 9,
                    max_children: 9,
                    allow_large_subnet: false,
                },
                ic_wire::Encoding::Payload,
                LIMITS,
            )
            .unwrap(),
            None,
        )
        .await;
    owner.client().add_cycles(root, 20_000_000_000_000).await;
    let url = owner.client_mut().make_live(None).await.to_string();
    let pic = owner.client();
    let store = ChildStore::install(pic, root, artifact("store.wasm")).await;
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
    let config = directory.join("canic.toml");
    let mut value = serde_json::json!({"apps":{"discovery":{
        "canisters":{"root":{"role":"root"}},
        "fleets":{"local":{
            "network":{"kind":"Local","url":url,"root_key":key,"root_subnet":pic.topology().await.get_nns().unwrap().to_text(),"routing_canister":root.to_text()},
            "journal":"journal","canisters":{"root":{"id":root.to_text()}}
        }}
    }}});
    std::fs::write(&config, toml::to_string(&value).unwrap()).unwrap();
    let show = |manager: &str, report: &std::path::Path| {
        cli(
            &config,
            &[
                "fleet",
                "show",
                "--manager",
                manager,
                "--html",
                report.to_str().unwrap(),
            ],
        )
    };
    let stable = pic.get_stable_memory(root).await;
    let empty = show(&root.to_text(), &directory.join("empty.html"));
    assert_eq!(empty["canisters"].as_array().unwrap().len(), 1);
    assert_eq!(empty["managers"].as_array().unwrap().len(), 0);
    assert_eq!(empty["issues"], serde_json::json!([]));
    assert_eq!(pic.get_stable_memory(root).await, stable);

    // A gap cannot reserve or hide a service from journal-free discovery.
    let gap = input(
        root,
        artifact("app.wasm"),
        pic.get_time().await.as_nanos_since_unix_epoch(),
    );
    assert_eq!(
        admit(pic, root, &store, ChildId::SECOND, gap).await,
        Err(Refusal::Binding)
    );
    let mut discovered = Vec::new();
    for index in 0..9 {
        let slot = ChildId::service(index).unwrap();
        pic.stop_progress().await;
        let stage = input(
            root,
            artifact("app.wasm"),
            pic.get_time().await.as_nanos_since_unix_epoch(),
        );
        admit(pic, root, &store, slot, stage.clone()).await.unwrap();
        pic.advance_time(Duration::from_secs(11)).await;
        settle(pic).await;
        if index == 8 {
            let original = read(pic, root).await[index].creation.clone();
            assert_eq!(original.phase, Phase::Ready);
            pic.upgrade_canister(root, artifact("root.wasm"), vec![], None)
                .await
                .unwrap();
            let retried = admit(pic, root, &store, slot, stage).await.unwrap();
            assert_eq!(retried.phase, Phase::Ready);
            assert_eq!(retried.worker, original.worker);
            assert_eq!(retried.attempts, original.attempts);
        }
        let custody = read(pic, root).await;
        let status = &custody[slot.service_index().unwrap()].creation;
        assert_eq!(status.phase, Phase::Ready);
        discovered.push((index + 1, status.worker.clone().unwrap()));
        let stable = pic.get_stable_memory(root).await;
        pic.auto_progress().await;
        let report = directory.join(format!(
            "after-slot-{}.html",
            slot.service_index().unwrap() + 1
        ));
        let observed = show(&root.to_text(), &report);
        assert_eq!(observed["operations"], serde_json::json!([]));
        assert_eq!(observed["issues"], serde_json::json!([]));
        assert_eq!(
            observed["managers"].as_array().unwrap().len(),
            discovered.len()
        );
        assert_eq!(observed["metrics"].as_array().unwrap().len(), 1);
        let canisters = observed["canisters"].as_array().unwrap();
        assert_eq!(canisters.len(), discovered.len() + 1);
        for (_, child) in &discovered {
            let row = canisters
                .iter()
                .find(|row| row["canister"] == *child)
                .unwrap();
            assert_eq!(row["controllers"], serde_json::json!([root.to_text()]));
            assert!(row["observation_error"].is_null());
        }
        let html = std::fs::read_to_string(&report).unwrap();
        for (ordinal, child) in &discovered {
            assert!(html.contains(&format!("Child: <strong>slot {ordinal} · {child}</strong>")));
        }
        assert_eq!(pic.get_stable_memory(root).await, stable);
    }
    let stable = pic.get_stable_memory(root).await;
    let tenth = input(
        root,
        artifact("app.wasm"),
        pic.get_time().await.as_nanos_since_unix_epoch(),
    );
    assert_eq!(
        admit(pic, root, &store, ChildId::service(9).unwrap(), tenth).await,
        Err(Refusal::Occupied)
    );
    let output = std::process::Command::new(std::env::var_os("CANIC_CLI").unwrap())
        .args([
            "fleet",
            "show",
            "--manager",
            &root.to_text(),
            "--color",
            "never",
            "--config",
        ])
        .arg(&config)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    for (ordinal, child) in &discovered {
        assert!(
            text.contains(&format!("root / slot {ordinal} · {child} — creation Ready")),
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
    std::fs::write(&config, toml::to_string(&value).unwrap()).unwrap();
    let stable = pic.get_stable_memory(old).await;
    let unknown = show(&old.to_text(), &directory.join("older-root.html"));
    assert_eq!(unknown["canisters"].as_array().unwrap().len(), 1);
    assert!(unknown["managers"].as_array().unwrap().is_empty());
    let issues = unknown["issues"].as_array().unwrap();
    assert_eq!(issues.len(), 1);
    assert!(
        issues[0]
            .as_str()
            .unwrap()
            .contains("permanent service slot 1 unknown")
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
