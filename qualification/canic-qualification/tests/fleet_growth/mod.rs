//! Add a child to a completed fleet without rewriting its original authority.
use super::*;

#[tokio::test(flavor = "current_thread")]
async fn existing_fleet_adds_child_preserving_identity_and_maintenance() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_owned();
    let assets = PathBuf::from(std::env::var_os("CANIC_PUBLICATION_ARTIFACTS").unwrap());
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
    let subnet = owner.client().get_subnet(routing).await.unwrap();
    let url = owner.client_mut().make_live(None).await.to_string();
    let dir = repo
        .join("target/publication")
        .join(format!("growth-store-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let (signer, controller) = canic_qualification::isolated_signer(&dir, "growth-fixture");
    let app = std::fs::read(assets.join("app.wasm")).unwrap();
    std::fs::write(dir.join("app.wasm"), &app).unwrap();
    let mut config = json!({"apps":{"demo":{
        "canisters":{"root":{"role":"root","wasm":assets.join("root.wasm")},
            "app":{"role":"app","parent":"root","wasm":dir.join("app.wasm")}},
        "fleets":{"local":{"network":{"kind":"Local","url":url,
            "root_key":hex(&owner.client().root_key().await.unwrap()), "root_subnet":owner.client().topology().await.get_nns().unwrap().to_text(),
            "routing_canister":routing.to_text()},"journal":"journal","canisters":{}}}
    }}});
    config["apps"]["demo"]["canisters"]["registry"] =
        json!({"parent":"root","role":"registry","wasm":dir.join("app.wasm")});
    config["apps"]["demo"]["canisters"]["store"] =
        json!({"role":"wasm-store","parent":"root","wasm":assets.join("store.wasm")});
    let save = |config: &Value| {
        std::fs::write(dir.join("canic.toml"), toml::to_string(config).unwrap()).unwrap()
    };
    save(&config);
    let plan = |dispatch: &str| {
        cli(
            &dir,
            &[
                "fleet",
                "plan",
                "--controller",
                &controller,
                "--execute-at",
                dispatch,
            ],
        )
    };
    let apply = |id: &str| {
        cli(
            &dir,
            &[
                "fleet",
                "apply",
                id,
                "--identity",
                "growth-fixture",
                "--icp-bin",
                signer.to_str().unwrap(),
            ],
        )
    };
    let dispatch = scheduled(owner.client()).await;
    let initial = plan(&dispatch);
    let id = initial["operation"].as_str().unwrap();
    let path = dir.join("journal").join(id);
    apply(id);
    advance(owner.client()).await;
    let original = apply(id);
    let root = CanisterId::from_text(original["root"]["canister"].as_str().unwrap()).unwrap();
    let first = original["children"][0]["outcome"]["canister"]
        .as_str()
        .unwrap()
        .to_owned();
    let envelope = std::fs::read(path.join("fleet.json")).unwrap();
    let first_signed = std::fs::read(path.join("child-1/stage/signed.json")).unwrap();
    assert_eq!(plan(&dispatch)["action"], "unchanged");

    // Growth must follow retained maintenance bindings, not the original creation code.
    config["apps"]["demo"]["canisters"]["root"]["wasm"] =
        json!(assets.join("root-maintenance.wasm"));
    save(&config);
    let maintenance = plan(&scheduled(owner.client()).await);
    assert_eq!(maintenance["action"], "maintenance");
    apply(maintenance["operation"].as_str().unwrap());
    config["apps"]["demo"]["canisters"]["store"]["wasm"] =
        json!(assets.join("store-candidate.wasm"));
    save(&config);
    let maintenance = plan(&scheduled(owner.client()).await);
    assert_eq!(maintenance["action"], "upgrade");
    let result = apply(maintenance["operation"].as_str().unwrap());
    assert!(result["canister"].is_string(), "{result}");

    // Sorting the config puts aaa first; its permanent allocation must still be Second.
    cli(
        &dir,
        &[
            "app",
            "canister",
            "add",
            "aaa",
            "--parent",
            "root",
            "--role",
            "worker",
            "--wasm",
            "worker.wasm",
        ],
    );
    config = toml::from_str(&std::fs::read_to_string(dir.join("canic.toml")).unwrap()).unwrap();
    std::fs::write(dir.join("worker.wasm"), &app).unwrap();
    let dispatch = scheduled(owner.client()).await;
    let mut next = app.clone();
    next.extend_from_slice(&[0, 2, 1, b'x']);
    std::fs::write(dir.join("app.wasm"), &next).unwrap();
    assert!(
        !run(&dir, &["fleet", "plan", "--controller", &controller])
            .status
            .success()
    );
    std::fs::write(dir.join("app.wasm"), &app).unwrap();
    let publication = plan(&dispatch);
    assert_eq!(publication["action"], "publication");
    assert_eq!(plan(&dispatch), publication);
    apply(publication["operation"].as_str().unwrap());
    let addition = plan(&dispatch);
    assert_eq!(addition["action"], "add_child", "{addition}");
    assert_eq!(
        addition["plan"]["action"]["approval"]["slot"],
        serde_json::to_value(canic_management_protocol::children::ChildId::service(2).unwrap(),)
            .unwrap(),
        "{addition}"
    );
    assert_eq!(addition["plan"]["deployment"]["canister"], "aaa");
    let added_id = addition["operation"].as_str().unwrap();
    let added = dir.join("journal").join(added_id);
    // Expired requested schedules cannot replace a pending approval.
    assert_eq!(plan("1"), addition);
    let stable = owner.client().get_stable_memory(root).await;
    let operations = std::fs::read_dir(dir.join("journal")).unwrap().count();
    for removed in ["app", "aaa"] {
        let mut changed = config.clone();
        changed["apps"]["demo"]["canisters"]
            .as_object_mut()
            .unwrap()
            .remove(removed);
        save(&changed);
        assert!(
            !run(&dir, &["fleet", "plan", "--controller", &controller])
                .status
                .success()
        );
    }
    save(&config);
    std::fs::write(dir.join("worker.wasm"), &next).unwrap();
    assert!(
        !run(&dir, &["fleet", "plan", "--controller", &controller])
            .status
            .success()
    );
    std::fs::write(dir.join("worker.wasm"), &app).unwrap();
    assert_eq!(
        std::fs::read_dir(dir.join("journal")).unwrap().count(),
        operations
    );
    assert_eq!(owner.client().get_stable_memory(root).await, stable);
    apply(added_id);
    let signed = std::fs::read(added.join("stage/signed.json")).unwrap();
    std::fs::remove_file(added.join("stage/reply.bin")).unwrap();
    assert_eq!(plan(&dispatch), addition);
    apply(added_id);
    assert_eq!(
        std::fs::read(added.join("stage/signed.json")).unwrap(),
        signed
    );
    advance(owner.client()).await;
    let created = apply(added_id);
    let second = created["canister"].as_str().unwrap().to_owned();
    assert_ne!(first, second);
    assert_eq!(created["controller"], root.to_text());
    assert_eq!(plan(&dispatch)["action"], "unchanged");
    // The original approval cannot bypass the changed Root code binding.
    assert!(
        !run(
            &dir,
            &[
                "fleet",
                "apply",
                id,
                "--identity",
                "growth-fixture",
                "--icp-bin",
                signer.to_str().unwrap()
            ]
        )
        .status
        .success()
    );
    let retained: Value =
        serde_json::from_slice(&std::fs::read(path.join("child-1/receipt.json")).unwrap()).unwrap();
    assert_eq!(retained["canister"], first);
    assert_eq!(std::fs::read(path.join("fleet.json")).unwrap(), envelope);
    assert_eq!(
        std::fs::read(path.join("child-1/stage/signed.json")).unwrap(),
        first_signed
    );

    // The added identity participates in the same upgrade chain and complete Root custody.
    let mut predecessor = created.clone();
    let mut predecessor_id = added_id.to_owned();
    let mut other = next.clone();
    other.extend_from_slice(&[0, 2, 1, b'y']);
    let mut releases = Vec::new();
    let mut approvals: Vec<(String, Value, Vec<u8>)> = Vec::new();
    for (round, artifact) in [&next, &other, &next, &other].into_iter().enumerate() {
        // B -> C -> B -> C repeats an exact code transition, but never its authority.
        std::fs::write(dir.join("worker.wasm"), artifact).unwrap();
        let dispatch = scheduled(owner.client()).await;

        if round < 2 {
            let publication = plan(&dispatch);
            assert_eq!(publication["action"], "publication");
            apply(publication["operation"].as_str().unwrap());
            releases.push(publication["operation"].as_str().unwrap().to_owned());
        }
        let publications = cli(&dir, &["fleet", "artifact", "list"]);
        // Named planning selects the added child through the same retained history.
        let mut named_config = config.clone();
        named_config["apps"]["demo"]["fleets"]["local"]["canisters"]["store"] =
            json!({"id": original["store"]["canister"]});
        save(&named_config);
        let named = |root_module: &str, target: &str, previous: &str| {
            let mut args = vec![
                "fleet",
                "plan",
                "--canister",
                "aaa",
                "--controller",
                &controller,
                "--root-module",
                root_module,
                "--target",
                target,
                "--previous-module",
                previous,
                "--execute-at",
                &dispatch,
            ];
            args.extend(["--release", &releases[round % 2]]);
            run(&dir, &args)
        };
        let root_module = addition["plan"]["action"]["approval"]["root_module"]
            .as_str()
            .unwrap();
        let previous_module = predecessor["module_hash"].as_str().unwrap();
        assert!(
            !named(&"ff".repeat(32), &second, previous_module)
                .status
                .success()
        );
        assert!(!named(root_module, &first, previous_module).status.success());
        if round > 0 {
            assert!(
                !named(
                    root_module,
                    &second,
                    created["module_hash"].as_str().unwrap()
                )
                .status
                .success()
            );
        }
        let receipt_path = dir
            .join("journal")
            .join(&predecessor_id)
            .join("receipt.json");
        let receipt_bytes = std::fs::read(&receipt_path).unwrap();
        std::fs::remove_file(&receipt_path).unwrap();
        assert!(
            !named(root_module, &second, previous_module)
                .status
                .success()
        );
        std::fs::write(receipt_path, receipt_bytes).unwrap();
        let output = named(root_module, &second, previous_module);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let named_upgrade: Value = serde_json::from_slice(&output.stdout).unwrap();
        let replay = named(root_module, &second, previous_module);
        assert!(
            replay.status.success(),
            "{}",
            String::from_utf8_lossy(&replay.stderr)
        );
        assert_eq!(
            serde_json::from_slice::<Value>(&replay.stdout).unwrap(),
            named_upgrade
        );
        assert_eq!(
            named_upgrade["plan"]["action"]["approval"]["owner"]["source"]["maintenance"]["sequence"],
            1
        );
        save(&config);
        let upgrade = plan(&dispatch);
        assert_eq!(upgrade["action"], "upgrade");
        assert_eq!(upgrade["operation"], named_upgrade["operation"]);
        assert_eq!(
            upgrade["plan"]["action"]["approval"]["target"]["canister"],
            second
        );
        assert_eq!(cli(&dir, &["fleet", "artifact", "list"]), publications);
        let upgrade_id = upgrade["operation"].as_str().unwrap();
        assert!(!approvals.iter().any(|(id, _, _)| id == upgrade_id));
        if round == 3 {
            // The code pair matches the second upgrade; the schedule/digest must be new.
            assert_eq!(
                upgrade["plan"]["action"]["approval"]["target"]["module"],
                approvals[1].1["plan"]["action"]["approval"]["target"]["module"]
            );
            assert_eq!(
                upgrade["plan"]["artifact"],
                approvals[1].1["plan"]["artifact"]
            );
        }
        apply(upgrade_id);
        let stage = dir.join("journal").join(upgrade_id).join("stage");
        let signed = std::fs::read(stage.join("signed.json")).unwrap();
        std::fs::remove_file(stage.join("reply.bin")).unwrap();
        save(&named_config);
        let replay = named(root_module, &second, previous_module);
        assert!(
            replay.status.success(),
            "{}",
            String::from_utf8_lossy(&replay.stderr)
        );
        assert_eq!(
            serde_json::from_slice::<Value>(&replay.stdout).unwrap(),
            named_upgrade
        );
        save(&config);
        apply(upgrade_id);
        assert_eq!(std::fs::read(stage.join("signed.json")).unwrap(), signed);
        advance(owner.client()).await;
        let completed = apply(upgrade_id);
        assert_eq!(completed["canister"], second);
        save(&named_config);
        let replay = named(root_module, &second, previous_module);
        assert!(
            replay.status.success(),
            "{}",
            String::from_utf8_lossy(&replay.stderr)
        );
        assert_eq!(
            serde_json::from_slice::<Value>(&replay.stdout).unwrap(),
            named_upgrade
        );
        save(&config);
        assert_eq!(plan(&dispatch)["action"], "unchanged");
        predecessor = completed;
        predecessor_id = upgrade_id.into();
        approvals.push((upgrade_id.to_owned(), upgrade.clone(), signed));
    }
    for (id, _, signed) in &approvals {
        assert_eq!(
            std::fs::read(dir.join("journal").join(id).join("stage/signed.json")).unwrap(),
            *signed
        );
    }

    config["apps"]["demo"]["canisters"]["root"]["wasm"] = json!(assets.join("root.wasm"));
    save(&config);
    let maintenance = plan(&scheduled(owner.client()).await);
    assert_eq!(maintenance["action"], "maintenance");
    assert_eq!(
        maintenance["plan"]["action"]["approval"]["children"][0]["creation"]["worker"],
        first
    );
    assert_eq!(
        maintenance["plan"]["action"]["approval"]["children"][1]["creation"]["worker"],
        original["children"][1]["outcome"]["canister"]
    );
    let added_custody = &maintenance["plan"]["action"]["approval"]["children"][2];
    assert_eq!(added_custody["creation"]["worker"], second);
    assert_eq!(added_custody["upgrades"].as_array().unwrap().len(), 4);
    let stable = owner.client().get_stable_memory(root).await;
    apply(maintenance["operation"].as_str().unwrap());
    assert_eq!(owner.client().get_stable_memory(root).await, stable);
    assert_eq!(
        plan(&scheduled(owner.client()).await)["action"],
        "unchanged"
    );
    let view = cli(&dir, &["fleet", "show"]);
    assert_eq!(view["canisters"].as_array().unwrap().len(), 5);
    assert!(view["issues"].as_array().unwrap().is_empty(), "{view}");
    assert_eq!(
        owner
            .client()
            .get_subnet_metrics(subnet)
            .await
            .unwrap()
            .num_canisters,
        6
    );
    owner.client_mut().stop_live().await;
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

async fn scheduled(pic: &ic_testkit::pocket_ic::nonblocking::PocketIc) -> String {
    (pic.get_time().await.as_nanos_since_unix_epoch() / 1_000_000_000 + 15).to_string()
}

async fn advance(pic: &ic_testkit::pocket_ic::nonblocking::PocketIc) {
    pic.stop_progress().await;
    pic.advance_time(Duration::from_secs(20)).await;
    for _ in 0..70 {
        pic.tick().await;
    }
    pic.auto_progress().await;
}
