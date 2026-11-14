use super::*;
use crate::tests::{directory, input};

fn config() -> SelectedFleet {
    SelectedFleet {
        app: "hello".into(),
        fleet: "demo".into(),
        config: FleetConfig {
            network: Network::Mainnet,
            journal: None,
            canisters: BTreeMap::from([("backend".into(), CanisterConfig::default())]),
        },
    }
}

fn declaration(selected: &SelectedFleet) -> Config {
    Config {
        version: 2,
        apps: BTreeMap::from([(
            selected.app.clone(),
            AppConfig {
                fleets: BTreeMap::from([(selected.fleet.clone(), selected.config.clone())]),
            },
        )]),
    }
}

#[test]
fn app_initialization_preserves_other_apps_and_keeps_journals_independent() {
    let root = directory();
    let path = root.join("canic.json");
    let first = init_app(&path, "first", "mainnet", Network::Mainnet).unwrap();
    assert_eq!(first, load_config(&path).unwrap());
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    add_canister(
        &path,
        "first",
        "mainnet",
        "backend",
        Some("rrkah-fqaaa-aaaaa-aaaaq-cai"),
    )
    .unwrap();
    let before = load_config(&path).unwrap();
    let original = before.select(Some("first"), Some("mainnet")).unwrap();
    let journal = original.journal_path(&path);
    std::fs::create_dir_all(&journal).unwrap();
    std::fs::write(journal.join("retained-evidence"), b"original").unwrap();
    let updated = init_app(&path, "second", "mainnet", Network::Mainnet).unwrap();
    assert_eq!(updated, load_config(&path).unwrap());
    assert_eq!(updated.apps["first"], before.apps["first"]);
    let added = updated.select(Some("second"), Some("mainnet")).unwrap();
    assert!(added.config.canisters.is_empty());
    assert_eq!(
        added.journal_path(&path),
        root.join(".canic/apps/second/fleets/mainnet")
    );
    assert!(!added.journal_path(&path).exists());
    assert_eq!(
        std::fs::read(journal.join("retained-evidence")).unwrap(),
        b"original"
    );
    assert!(updated.select(None, Some("mainnet")).is_err());
    let bytes = std::fs::read(&path).unwrap();
    for (app, fleet) in [
        ("first", "another"),
        ("second", "mainnet"),
        ("../bad", "mainnet"),
        ("third", "../bad"),
    ] {
        assert!(init_app(&path, app, fleet, Network::Mainnet).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    let lock = ic_host_fs::durable::open_private_regular_lock_file(&lock_path(&path)).unwrap();
    ic_host_fs::durable::try_lock_exclusive(&lock).unwrap();
    assert!(init_app(&path, "third", "mainnet", Network::Mainnet).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    drop(lock);
    init_app(&path, "third", "mainnet", Network::Mainnet).unwrap();
}

#[test]
fn concurrent_first_app_initializers_do_not_lose_successful_declarations() {
    let root = directory();
    let path = root.join("canic.json");
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = ["first", "second"]
            .into_iter()
            .map(|app| {
                let path = &path;
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    (app, init_app(path, app, "mainnet", Network::Mainnet))
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    let config = load_config(&path).unwrap();
    let succeeded = results.iter().filter(|(_, result)| result.is_ok()).count();
    assert!(succeeded > 0);
    assert_eq!(config.apps.len(), succeeded);
    for (app, result) in results {
        assert_eq!(config.apps.contains_key(app), result.is_ok());
    }
    assert!(!root.join(".canic").exists());
}

#[test]
fn app_initialization_refuses_invalid_input_and_app_capacity_without_replacing_config() {
    let root = directory();
    let path = root.join("canic.json");
    assert!(init_app(&path, "../bad", "mainnet", Network::Mainnet).is_err());
    assert!(!path.exists());
    for bytes in [b"invalid JSON".to_vec(), vec![b' '; MAX_CONFIG_BYTES + 1]] {
        std::fs::write(&path, &bytes).unwrap();
        assert!(init_app(&path, "new", "mainnet", Network::Mainnet).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(!lock_path(&path).exists());
    }
    let full = Config {
        version: 2,
        apps: (0..16)
            .map(|i| {
                (
                    format!("app-{i}"),
                    declaration(&config()).apps.remove("hello").unwrap(),
                )
            })
            .collect(),
    };
    let bytes = serde_json::to_vec(&full).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    assert!(init_app(&path, "extra", "mainnet", Network::Mainnet).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert!(!root.join(".canic").exists());
}

#[test]
fn existing_id_declarations_are_network_scoped_and_do_not_authorize_creation() {
    let root = directory();
    let path = root.join("canic.json");
    init_config(&path, &declaration(&config())).unwrap();
    init_fleet(&path, "hello", "staging", Network::Mainnet).unwrap();
    let id = "rrkah-fqaaa-aaaaa-aaaaq-cai";
    let selected = add_canister(&path, "hello", "demo", "existing", Some(id)).unwrap();
    assert_eq!(
        selected.config.canisters["existing"].id.as_deref(),
        Some(id)
    );
    assert!(!selected.config.canisters["existing"].profile_manager);
    let before = std::fs::read(&path).unwrap();
    for (fleet, slot, principal) in [
        ("demo", "duplicate", id),
        ("staging", "duplicate", id),
        ("demo", "existing", "aaaaa-aa"),
        ("demo", "backend", id),
        ("demo", "invalid", "not-a-principal"),
    ] {
        assert!(add_canister(&path, "hello", fleet, slot, Some(principal)).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
    let journal = selected.journal_path(&path);
    assert!(matches!(
        create_configured_plan(
            &journal,
            &root.join("unused.wasm"),
            input(),
            &selected,
            "existing"
        ),
        Err(Error::Invalid(_))
    ));
    assert!(!journal.exists());
    // Equal principals on separate exact network bindings are independent identities.
    init_fleet(
        &path,
        "hello",
        "local",
        Network::Local {
            url: "http://127.0.0.1:9".into(),
            root_key: "00".repeat(133),
            routing_canister: "aaaaa-aa".into(),
        },
    )
    .unwrap();
    add_canister(&path, "hello", "local", "existing", Some(id)).unwrap();
    assert!(!root.join(".canic").exists());
}

#[test]
fn loaded_config_and_selection_retain_one_validated_snapshot() {
    let root = directory();
    let path = root.join("canic.json");
    init_config(&path, &declaration(&config())).unwrap();
    let loaded = load_config(&path).unwrap();
    let selected = loaded.select(Some("hello"), Some("demo")).unwrap();
    add_canister(&path, "hello", "demo", "worker", None).unwrap();
    assert_eq!(
        loaded.select(Some("hello"), Some("demo")).unwrap(),
        selected
    );
    assert_eq!(selected, config());
    let fresh = load_config(&path)
        .unwrap()
        .select(Some("hello"), Some("demo"))
        .unwrap();
    assert!(fresh.config.canisters.contains_key("worker"));
    assert_eq!(fresh.config.canisters.len(), 2);
}

#[test]
fn nested_unknown_fields_refuse_edits_without_changing_config() {
    let root = directory();
    let path = root.join("canic.json");
    for pointer in [
        "/apps/hello",
        "/apps/hello/fleets/demo",
        "/apps/hello/fleets/demo/canisters/backend",
    ] {
        let mut document = serde_json::to_value(declaration(&config())).unwrap();
        document.pointer_mut(pointer).unwrap()["unexpected"] = true.into();
        let bytes = serde_json::to_vec(&document).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        for error in [
            load_config(&path).unwrap_err(),
            init_app(&path, "another", "mainnet", Network::Mainnet).unwrap_err(),
            init_fleet(&path, "hello", "staging", Network::Mainnet).unwrap_err(),
            add_canister(&path, "hello", "demo", "worker", None).unwrap_err(),
        ] {
            let Error::Supplier(source) = error else {
                panic!("expected retained JSON decoding error");
            };
            assert!(
                source
                    .downcast_ref::<serde_json::Error>()
                    .unwrap()
                    .is_data()
            );
        }
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(!lock_path(&path).exists());
        assert!(!root.join(".canic").exists());
    }
}

#[test]
fn canister_add_preserves_bound_slots_and_other_fleets_without_opening_journals() {
    let root = directory();
    let path = root.join("canic.json");
    let mut config = declaration(&config());
    let owner = config.apps.get_mut("hello").unwrap();
    owner
        .fleets
        .insert("other".into(), owner.fleets["demo"].clone());
    let fleet = owner.fleets.get_mut("demo").unwrap();
    fleet.journal = Some("original-journal".into());
    fleet.canisters.get_mut("backend").unwrap().id = Some("aaaaa-aa".into());
    init_config(&path, &config).unwrap();
    let selected = add_canister(&path, "hello", "demo", "worker", None).unwrap();
    assert_eq!(
        selected.config.canisters["worker"],
        CanisterConfig::default()
    );
    assert_eq!(
        selected.config.journal.as_deref(),
        Some(Path::new("original-journal"))
    );
    assert_eq!(
        selected.config.canisters["backend"].id.as_deref(),
        Some("aaaaa-aa")
    );
    let updated = load_config(&path).unwrap();
    assert_eq!(
        updated.apps["hello"].fleets["other"],
        config.apps["hello"].fleets["other"]
    );
    assert!(!root.join("original-journal").exists());
    assert!(!root.join(".canic").exists());
    let before = std::fs::read(&path).unwrap();
    for (app, fleet, slot) in [
        ("hello", "demo", "backend"),
        ("hello", "demo", "worker"),
        ("missing", "demo", "new"),
        ("hello", "missing", "new"),
        ("hello", "demo", "../escape"),
    ] {
        assert!(add_canister(&path, app, fleet, slot, None).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
    let lock = ic_host_fs::durable::open_private_regular_lock_file(&lock_path(&path)).unwrap();
    ic_host_fs::durable::try_lock_exclusive(&lock).unwrap();
    assert!(
        add_canister(&path, "hello", "demo", "busy", None)
            .unwrap_err()
            .to_string()
            .contains("another Canic command")
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    drop(lock);
    add_canister(&path, "hello", "demo", "after-lock", None).unwrap();
}

#[test]
fn canister_add_cannot_exceed_fleet_or_config_capacity() {
    let root = directory();
    let path = root.join("canic.json");
    let mut config = declaration(&config());
    let app = config.apps.get_mut("hello").unwrap();
    app.fleets.get_mut("demo").unwrap().canisters = (0..32)
        .map(|i| (format!("slot-{i}"), CanisterConfig::default()))
        .collect();
    for i in 0..3 {
        app.fleets
            .insert(format!("other-{i}"), app.fleets["demo"].clone());
    }
    app.fleets.insert(
        "empty".into(),
        FleetConfig {
            network: Network::Mainnet,
            journal: None,
            canisters: BTreeMap::new(),
        },
    );
    init_config(&path, &config).unwrap();
    let before = std::fs::read(&path).unwrap();
    for fleet in ["demo", "empty"] {
        assert!(add_canister(&path, "hello", fleet, "extra", None).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}

#[test]
fn fleet_initialization_preserves_existing_declarations_and_rejects_conflicts() {
    let root = directory();
    let path = root.join("canic.json");
    let mut existing = declaration(&config());
    existing
        .apps
        .get_mut("hello")
        .unwrap()
        .fleets
        .get_mut("demo")
        .unwrap()
        .journal = Some("original-journal".into());
    existing
        .apps
        .insert("other".into(), existing.apps["hello"].clone());
    init_config(&path, &existing).unwrap();
    let selected = init_fleet(&path, "hello", "staging", Network::Mainnet).unwrap();
    let updated = load_config(&path).unwrap();
    assert_eq!(updated.apps["other"], existing.apps["other"]);
    assert_eq!(
        updated.apps["hello"].fleets["demo"],
        existing.apps["hello"].fleets["demo"]
    );
    assert_eq!(selected.app, "hello");
    assert_eq!(selected.fleet, "staging");
    assert!(selected.config.canisters.is_empty());
    assert!(selected.config.journal.is_none());
    assert_eq!(
        selected.journal_path(&path),
        root.join(".canic/apps/hello/fleets/staging")
    );
    assert!(!root.join(".canic").exists());
    assert!(!root.join("original-journal").exists());
    let before = std::fs::read(&path).unwrap();
    for (app, fleet) in [
        ("hello", "staging"),
        ("missing", "new"),
        ("hello", "../escape"),
    ] {
        assert!(init_fleet(&path, app, fleet, Network::Mainnet).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
    let lock = ic_host_fs::durable::open_private_regular_lock_file(&lock_path(&path)).unwrap();
    ic_host_fs::durable::try_lock_exclusive(&lock).unwrap();
    assert!(
        init_fleet(&path, "hello", "busy", Network::Mainnet)
            .unwrap_err()
            .to_string()
            .contains("another Canic command")
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    drop(lock);
    init_fleet(&path, "hello", "after-lock", Network::Mainnet).unwrap();
    assert!(
        load_config(&path).unwrap().apps["hello"]
            .fleets
            .contains_key("after-lock")
    );
}

#[test]
fn fleet_initialization_refuses_limits_and_unreadable_inputs_without_replacing_config() {
    let root = directory();
    let path = root.join("canic.json");
    assert!(init_fleet(&path, "hello", "demo", Network::Mainnet).is_err());
    assert!(!lock_path(&path).exists());
    let empty = FleetConfig {
        network: Network::Mainnet,
        journal: None,
        canisters: BTreeMap::new(),
    };
    let full = Config {
        version: 2,
        apps: BTreeMap::from([(
            "hello".into(),
            AppConfig {
                fleets: (0..16)
                    .map(|i| (format!("fleet-{i}"), empty.clone()))
                    .collect(),
            },
        )]),
    };
    init_config(&path, &full).unwrap();
    let before = std::fs::read(&path).unwrap();
    assert!(init_fleet(&path, "hello", "extra", Network::Mainnet).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let oversized = Config {
        version: 2,
        apps: (0..16)
            .map(|i| {
                (
                    format!("app-{i}"),
                    AppConfig {
                        fleets: (0..16)
                            .map(|j| (format!("{j:064}"), empty.clone()))
                            .collect(),
                    },
                )
            })
            .collect(),
    };
    oversized.validate().unwrap();
    assert!(serde_json::to_vec_pretty(&oversized).unwrap().len() > MAX_CONFIG_BYTES);
    assert!(init_config(&root.join("oversized.json"), &oversized).is_err());
    assert!(!root.join("oversized.json").exists());
    std::fs::write(&path, b"invalid JSON").unwrap();
    assert!(init_fleet(&path, "hello", "extra", Network::Mainnet).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"invalid JSON");
}

#[cfg(unix)]
#[test]
fn fleet_initialization_does_not_follow_config_symlinks() {
    let root = directory();
    let original = root.join("original.json");
    init_config(&original, &declaration(&config())).unwrap();
    let before = std::fs::read(&original).unwrap();
    let link = root.join("canic.json");
    std::os::unix::fs::symlink(&original, &link).unwrap();
    assert!(init_fleet(&link, "hello", "staging", Network::Mainnet).is_err());
    assert!(init_app(&link, "another", "mainnet", Network::Mainnet).is_err());
    assert_eq!(std::fs::read(&original).unwrap(), before);
    assert!(!lock_path(&link).exists());
    let missing = root.join("missing.json");
    let dangling = root.join("dangling.json");
    std::os::unix::fs::symlink(&missing, &dangling).unwrap();
    assert!(init_app(&dangling, "another", "mainnet", Network::Mainnet).is_err());
    assert!(!missing.exists());
    assert!(!lock_path(&dangling).exists());
}

#[test]
fn initialization_is_local_bounded_and_preserves_existing_files() {
    let root = directory();
    let path = root.join("canic.json");
    let config = declaration(&config());
    init_config(&path, &config).unwrap();
    assert_eq!(load_config(&path).unwrap(), config);
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    let before = std::fs::read(&path).unwrap();
    assert!(init_config(&path, &config).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let mut invalid = config.clone();
    invalid
        .apps
        .insert("../escape".into(), config.apps["hello"].clone());
    assert!(init_config(&root.join("bad.json"), &invalid).is_err());
    let mut invalid = config;
    invalid.version = 99;
    assert!(invalid.validate().is_err());
    std::fs::write(&path, vec![b' '; 32769]).unwrap();
    assert!(load_config(&path).is_err());
    assert!(!root.join("bad.json").exists());
}

#[test]
fn named_plans_retain_identity_and_never_renew_expired_or_conflicting_authority() {
    let root = directory();
    let wasm = root.join("hello.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let journal = root.join("state");
    let (id, plan) =
        create_configured_plan(&journal, &wasm, input(), &config(), "backend").unwrap();
    assert_eq!(plan.version, 2);
    assert_eq!(plan.deployment.as_ref().unwrap().app, "hello");
    assert_eq!(
        create_configured_plan(&journal, &wasm, input(), &config(), "backend").unwrap(),
        (id.clone(), plan.clone())
    );
    let mut renamed = plan.clone();
    renamed.deployment.as_mut().unwrap().canister = "other".into();
    assert_ne!(renamed.digest().unwrap(), id);
    let mut conflicting = input();
    conflicting.creation_cycles += 1;
    assert!(create_configured_plan(&journal, &wasm, conflicting, &config(), "backend").is_err());
    assert!(create_configured_plan(&journal, &wasm, input(), &config(), "missing").is_err());
    let mut expired = plan;
    expired.created_at_ns = 1;
    expired.expires_at_ns = 2;
    let expired_id = expired.digest().unwrap();
    std::fs::rename(journal.join(id), journal.join(&expired_id)).unwrap();
    std::fs::write(
        journal.join(&expired_id).join("plan.json"),
        serde_json::to_vec(&expired).unwrap(),
    )
    .unwrap();
    assert_eq!(
        create_configured_plan(&journal, &wasm, input(), &config(), "backend").unwrap(),
        (expired_id, expired)
    );
    assert_eq!(std::fs::read_dir(&journal).unwrap().count(), 2); // One operation and the journal lock.
    let _lock = store::Store::open(&journal).unwrap();
    assert!(create_configured_plan(&journal, &wasm, input(), &config(), "backend").is_err());
}

#[test]
fn legacy_plan_bytes_and_digests_remain_unchanged() {
    let root = directory();
    let wasm = root.join("hello.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let (id, plan) = crate::create_plan(&root, &wasm, input()).unwrap();
    let json = serde_json::to_vec(&plan).unwrap();
    assert!(!String::from_utf8_lossy(&json).contains("deployment"));
    let mut legacy = b"canic:deployment-plan:v1\0".to_vec();
    legacy.extend(&json);
    assert_eq!(crate::artifact::digest(&legacy), id);
    let decoded: Plan = serde_json::from_slice(&json).unwrap();
    assert_eq!(decoded.digest().unwrap(), id);
}

#[tokio::test]
async fn existing_id_adoption_preserves_legacy_evidence_and_blocks_creation() {
    let root = directory();
    let config_path = root.join("config/canic.json");
    let journal = root.join("retained");
    let wasm = root.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let mut adopted = config();
    adopted.config.network = Network::Local {
        url: "http://127.0.0.1:9".into(),
        root_key: "00".repeat(133),
        routing_canister: "aaaaa-aa".into(),
    };
    adopted.config.journal = Some("../retained".into());
    let slot = adopted.config.canisters.get_mut("backend").unwrap();
    slot.id = Some("aaaaa-aa".into());
    slot.profile_manager = true;
    let legacy_input = PlanInput {
        network: adopted.config.network.clone(),
        target: Some("aaaaa-aa".into()),
        previous_module: Some("00".repeat(32)),
        creation_cycles: 0,
        ..input()
    };
    let (id, plan) = crate::create_plan(&journal, &wasm, legacy_input).unwrap();
    let plan_path = journal.join(&id).join("plan.json");
    let before = std::fs::read(&plan_path).unwrap();
    let attempt = journal.join(&id).join("install");
    std::fs::create_dir(&attempt).unwrap();
    std::fs::write(attempt.join("signed.json"), "PRIVATE_ATTEMPT").unwrap();
    init_config(&config_path, &declaration(&adopted)).unwrap();
    let loaded = load_config(&config_path)
        .unwrap()
        .select(None, None)
        .unwrap();
    assert_eq!(
        std::fs::canonicalize(loaded.journal_path(&config_path)).unwrap(),
        journal
    );
    let snapshot = crate::fleet_with_config(&journal, &["aaaaa-aa".into()], Some(&loaded))
        .await
        .unwrap();
    assert_eq!(snapshot.operations.len(), 1);
    assert_eq!(snapshot.operations[0].operation, id);
    assert!(snapshot.operations[0].deployment.is_none());
    assert_eq!(
        snapshot.operations[0].effects["install"],
        "signed_attempt_retained_outcome_unknown"
    );
    assert_eq!(snapshot.canisters.len(), 1);
    assert_eq!(snapshot.canisters[0].canister, "aaaaa-aa");
    assert!(snapshot.canisters[0].observation_error.is_some());
    assert_eq!(snapshot.managers.len(), 1); // Explicit and configured selections deduplicate.
    assert!(snapshot.managers[0].observation_error.is_some());
    assert!(
        !serde_json::to_string(&snapshot)
            .unwrap()
            .contains("PRIVATE_ATTEMPT")
    );
    assert!(
        create_configured_plan(
            &journal,
            &wasm,
            PlanInput {
                network: adopted.config.network,
                ..input()
            },
            &loaded,
            "backend"
        )
        .is_err()
    );
    assert_eq!(std::fs::read(plan_path).unwrap(), before);
    assert_eq!(plan.digest().unwrap(), id);
    assert_eq!(
        std::fs::read_to_string(attempt.join("signed.json")).unwrap(),
        "PRIVATE_ATTEMPT"
    );
    assert_eq!(std::fs::read_dir(&journal).unwrap().count(), 2);
}

#[test]
fn existing_declarations_validate_identity_and_journal_selection() {
    let mut declaration = config();
    let config_path = Path::new("project/canic.json");
    assert_eq!(
        declaration.journal_path(config_path),
        Path::new("project/.canic/apps/hello/fleets/demo")
    );
    declaration.config.journal = Some(".canic".into());
    assert_eq!(
        declaration.journal_path(config_path),
        Path::new("project/.canic")
    );
    let slot = declaration.config.canisters.get_mut("backend").unwrap();
    slot.profile_manager = true;
    assert!(declaration.validate().is_err());
    declaration.config.canisters.get_mut("backend").unwrap().id = Some("invalid".into());
    assert!(declaration.validate().is_err());
    let slot = declaration.config.canisters.get_mut("backend").unwrap();
    slot.id = Some("aaaaa-aa".into());
    let duplicate = slot.clone();
    assert!(declaration.validate().is_ok());
    declaration
        .config
        .canisters
        .insert("duplicate".into(), duplicate);
    assert!(declaration.validate().is_err());
}

#[tokio::test]
async fn initialized_fleet_observes_undeployed_slots_without_creating_state() {
    let root = directory().join("missing-state");
    let fleet = crate::fleet_with_config(&root, &[], Some(&config()))
        .await
        .unwrap();
    assert_eq!(fleet.configuration, Some(config()));
    assert!(fleet.canisters.is_empty());
    assert!(!root.exists());
}

#[tokio::test]
async fn config_edits_do_not_rebind_existing_network_authority() {
    let root = directory();
    let wasm = root.join("hello.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let (id, _) = create_configured_plan(&root, &wasm, input(), &config(), "backend").unwrap();
    let mut edited = config();
    edited.config.network = Network::Local {
        url: "http://localhost:8000".into(),
        root_key: "00".repeat(133),
        routing_canister: "aaaaa-aa".into(),
    };
    let mut changed_input = input();
    changed_input.network = edited.config.network.clone();
    assert!(create_configured_plan(&root, &wasm, changed_input, &edited, "backend").is_err());
    let snapshot = crate::fleet_with_config(&root, &[], Some(&edited))
        .await
        .unwrap();
    assert!(
        snapshot
            .issues
            .iter()
            .any(|issue| issue.contains("network differs"))
    );
    assert_eq!(snapshot.operations[0].operation, id);
    assert!(snapshot.operations[0].canister.is_none());
}

#[tokio::test]
async fn one_app_has_multiple_isolated_fleets_and_requires_explicit_selection() {
    let root = directory();
    let config_path = root.join("canic.json");
    let mut catalog = declaration(&config());
    let definition = catalog.apps.get_mut("hello").unwrap();
    definition
        .fleets
        .insert("staging".into(), definition.fleets["demo"].clone());
    assert!(catalog.select(None, None).is_err());
    let demo = catalog.select(Some("hello"), Some("demo")).unwrap();
    let staging = catalog.select(Some("hello"), Some("staging")).unwrap();
    assert_ne!(
        demo.journal_path(&config_path),
        staging.journal_path(&config_path)
    );
    let other = catalog.apps["hello"].clone();
    catalog.apps.insert("other-app".into(), other);
    assert!(catalog.select(None, Some("demo")).is_err());
    let other = catalog.select(Some("other-app"), Some("demo")).unwrap();
    assert_ne!(
        demo.journal_path(&config_path),
        other.journal_path(&config_path)
    );
    let wasm = root.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    // Even a shared explicit journal cannot merge pending named operations.
    let journal = root.join("shared");
    let (demo_id, _) = create_configured_plan(&journal, &wasm, input(), &demo, "backend").unwrap();
    let (staging_id, _) =
        create_configured_plan(&journal, &wasm, input(), &staging, "backend").unwrap();
    let (other_id, _) =
        create_configured_plan(&journal, &wasm, input(), &other, "backend").unwrap();
    for (selected, id) in [
        (&demo, &demo_id),
        (&staging, &staging_id),
        (&other, &other_id),
    ] {
        selected.check_operation(&journal, id).unwrap();
        let snapshot = crate::fleet_with_config(&journal, &[], Some(selected))
            .await
            .unwrap();
        assert_eq!(snapshot.operations.len(), 1);
        assert_eq!(&snapshot.operations[0].operation, id);
        assert!(snapshot.issues.is_empty());
    }
    assert!(demo.check_operation(&journal, &staging_id).is_err());
    assert!(demo.check_operation(&journal, &other_id).is_err());
    assert_eq!(crate::fleet(&journal).await.unwrap().operations.len(), 3);
}

#[tokio::test]
async fn legacy_upgrades_follow_retained_named_identity_without_rewriting_evidence() {
    let root = directory();
    let wasm = root.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let mut selected = config();
    selected.config.network = Network::Local {
        url: "http://127.0.0.1:9".into(),
        root_key: "00".repeat(133),
        routing_canister: "aaaaa-aa".into(),
    };
    let upgrade = || PlanInput {
        network: selected.config.network.clone(),
        target: Some("aaaaa-aa".into()),
        previous_module: Some("00".repeat(32)),
        creation_cycles: 0,
        ..input()
    };
    // A retained named target establishes the association; mutable config has no ID.
    let (named, _) = plan::retain(
        &root,
        &wasm,
        upgrade(),
        Some(DeploymentName {
            app: selected.app.clone(),
            fleet: selected.fleet.clone(),
            canister: "backend".into(),
        }),
    )
    .unwrap();
    let (legacy, _) = crate::create_plan(&root, &wasm, upgrade()).unwrap();
    let (unassigned, _) = crate::create_plan(&root, &wasm, input()).unwrap();
    let snapshot = crate::fleet_with_config(&root, &[], Some(&selected))
        .await
        .unwrap();
    assert_eq!(snapshot.operations.len(), 2);
    assert!(snapshot.operations.iter().any(|op| op.operation == named));
    assert!(snapshot.operations.iter().any(|op| op.operation == legacy));
    assert!(
        snapshot
            .issues
            .iter()
            .any(|issue| issue.contains(&unassigned))
    );
    selected.check_operation(&root, &legacy).unwrap();
    selected.check_target(&root, "aaaaa-aa").unwrap();
    let mut other = selected.clone();
    other.fleet = "other".into();
    assert!(other.check_operation(&root, &legacy).is_err());
    assert!(other.check_target(&root, "aaaaa-aa").is_err());
}

#[test]
fn one_physical_canister_cannot_belong_to_two_fleets_on_the_same_network() {
    let mut selected = config();
    selected.config.canisters.get_mut("backend").unwrap().id = Some("aaaaa-aa".into());
    let mut catalog = declaration(&selected);
    catalog
        .apps
        .get_mut("hello")
        .unwrap()
        .fleets
        .insert("other".into(), selected.config.clone());
    assert!(
        catalog
            .validate()
            .unwrap_err()
            .to_string()
            .contains("two fleets")
    );
    // A different trust binding may legitimately have an equal principal.
    catalog
        .apps
        .get_mut("hello")
        .unwrap()
        .fleets
        .get_mut("other")
        .unwrap()
        .network = Network::Local {
        url: "http://127.0.0.1:9".into(),
        root_key: "00".repeat(133),
        routing_canister: "aaaaa-aa".into(),
    };
    catalog.validate().unwrap();
    catalog.apps.insert(
        "another-app".into(),
        declaration(&selected).apps.remove("hello").unwrap(),
    );
    assert!(catalog.validate().is_err());
}

#[test]
fn duplicate_json_names_cannot_silently_replace_app_fleet_or_canister() {
    let root = directory();
    let path = root.join("canic.json");
    let app = serde_json::to_string(&declaration(&config()).apps["hello"]).unwrap();
    let fleet = serde_json::to_string(&config().config).unwrap();
    let documents = [
        r#"{"version":2,"apps":{"hello":APP,"hello":APP}}"#.replace("APP", &app),
        r#"{"version":2,"apps":{"hello":{"fleets":{"demo":FLEET,"demo":FLEET}}}}"#.replace("FLEET", &fleet),
        r#"{"version":2,"apps":{"hello":{"fleets":{"demo":{"network":{"kind":"Mainnet"},"canisters":{"backend":{},"backend":{}}}}}}}"#.into(),
    ];
    for document in documents {
        std::fs::write(&path, document).unwrap();
        assert!(
            load_config(&path)
                .unwrap_err()
                .to_string()
                .contains("duplicate configuration name")
        );
    }
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
}

#[test]
fn declared_upgrade_selection_does_not_create_a_missing_journal() {
    let root = directory().join("missing");
    let mut selected = config();
    selected.config.canisters.get_mut("backend").unwrap().id = Some("aaaaa-aa".into());
    selected.check_target(&root, "aaaaa-aa").unwrap();
    assert!(selected.check_target(&root, "2vxsx-fae").is_err());
    assert!(!root.exists());
}
