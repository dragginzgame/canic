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
    let canisters = selected
        .config
        .canisters
        .iter()
        .map(|(name, slot)| {
            (
                name.clone(),
                CanisterDefinition {
                    role: slot.role.clone(),
                    parent: slot.parent.clone(),
                    wasm: slot.wasm.clone(),
                },
            )
        })
        .collect();
    let bindings = selected
        .config
        .canisters
        .iter()
        .filter_map(|(name, slot)| {
            slot.id.as_ref().map(|id| {
                (
                    name.clone(),
                    CanisterBinding {
                        id: id.clone(),
                        profile_manager: slot.profile_manager,
                    },
                )
            })
        })
        .collect();
    Config {
        apps: BTreeMap::from([(
            selected.app.clone(),
            AppConfig {
                canisters,
                fleets: BTreeMap::from([(
                    selected.fleet.clone(),
                    FleetDeclaration {
                        network: selected.config.network.clone(),
                        journal: selected.config.journal.clone(),
                        canisters: bindings,
                    },
                )]),
            },
        )]),
    }
}

#[test]
fn reusable_app_definitions_resolve_across_fleets_without_copying_bindings() {
    let root = directory();
    let path = root.join("canic.json");
    init_config(&path, &declaration(&config())).unwrap();
    let original = bind_canister(
        &path,
        "hello",
        "demo",
        "backend",
        CanisterBinding {
            id: "aaaaa-aa".into(),
            ..Default::default()
        },
    )
    .unwrap();
    add_app_canister(
        &path,
        "hello",
        "worker",
        CanisterDefinition {
            role: Some("worker".into()),
            parent: Some("backend".into()),
            wasm: Some("build/worker.wasm".into()),
        },
    )
    .unwrap();
    let next = init_fleet(&path, "hello", "staging", Network::Mainnet).unwrap();
    assert_eq!(next.config.canisters.len(), 2);
    assert!(next.config.canisters.values().all(|c| c.id.is_none()));
    let loaded = load_config(&path).unwrap();
    assert!(loaded.apps["hello"].fleets["staging"].canisters.is_empty());
    let first = loaded.select(Some("hello"), Some("demo")).unwrap();
    assert_eq!(
        first.config.canisters["backend"].id,
        original.config.canisters["backend"].id
    );
    assert_eq!(
        first.config.canisters["worker"],
        next.config.canisters["worker"]
    );
    assert_ne!(first.journal_path(&path), next.journal_path(&path));
    assert_eq!(
        next.wasm_path(&path, "worker", None).unwrap(),
        root.join("build/worker.wasm")
    );
    assert!(
        next.wasm_path(&path, "worker", Some(Path::new("override.wasm")))
            .is_err()
    );
    assert!(next.wasm_path(&path, "backend", None).is_err());
    assert!(!root.join(".canic").exists());
}

#[test]
fn source_ownership_refuses_fleet_overrides_unknown_bindings_and_old_layout() {
    let root = directory();
    let path = root.join("canic.json");
    let base = serde_json::to_value(declaration(&config())).unwrap();
    let mut invalid = Vec::new();
    for field in ["role", "parent", "wasm"] {
        let mut document = base.clone();
        document["apps"]["hello"]["fleets"]["demo"]["canisters"]["backend"] =
            serde_json::json!({"id":"aaaaa-aa", field:"override"});
        invalid.push(document);
    }
    let mut unknown = base.clone();
    unknown["apps"]["hello"]["fleets"]["demo"]["canisters"]["unknown"] =
        serde_json::json!({"id":"aaaaa-aa"});
    invalid.push(unknown);
    let mut old = base;
    old["version"] = 2.into();
    invalid.push(old);
    for document in invalid {
        let bytes = serde_json::to_vec(&document).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        assert!(load_config(&path).is_err());
        assert!(init_fleet(&path, "hello", "new", Network::Mainnet).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(!lock_path(&path).exists());
    }
    let duplicate = r#"{"apps":{"hello":{"canisters":{"backend":{},"backend":{}},"fleets":{"demo":{"network":{"kind":"Mainnet"},"canisters":{}}}}}}"#;
    std::fs::write(&path, duplicate).unwrap();
    assert!(load_config(&path).is_err());
}

#[test]
fn declared_topology_rejects_missing_parents_cycles_and_invalid_roles() {
    let mut selected = config();
    selected
        .config
        .canisters
        .insert("manager".into(), CanisterConfig::default());
    selected.config.canisters.get_mut("backend").unwrap().parent = Some("manager".into());
    declaration(&selected).validate().unwrap();
    for parent in ["missing", "other-fleet/manager", "backend"] {
        let mut invalid = selected.clone();
        invalid.config.canisters.get_mut("backend").unwrap().parent = Some(parent.into());
        assert!(declaration(&invalid).validate().is_err());
    }
    let mut cycle = selected.clone();
    cycle.config.canisters.get_mut("manager").unwrap().parent = Some("backend".into());
    assert!(declaration(&cycle).validate().is_err());
    for role in ["", "bad\nrole", "bad/role"] {
        let mut invalid = selected.clone();
        invalid.config.canisters.get_mut("backend").unwrap().role = Some(role.into());
        assert!(declaration(&invalid).validate().is_err());
    }
    let root = directory();
    let path = root.join("canic.json");
    init_config(&path, &declaration(&selected)).unwrap();
    let before = std::fs::read(&path).unwrap();
    for definition in [
        CanisterDefinition {
            parent: Some("missing".into()),
            ..Default::default()
        },
        CanisterDefinition {
            wasm: Some("".into()),
            ..Default::default()
        },
    ] {
        assert!(add_app_canister(&path, "hello", "child", definition).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}

#[test]
fn existing_id_declarations_are_network_scoped_and_do_not_authorize_creation() {
    let root = directory();
    let path = root.join("canic.json");
    init_config(&path, &declaration(&config())).unwrap();
    init_fleet(&path, "hello", "staging", Network::Mainnet).unwrap();
    add_app_canister(&path, "hello", "other", CanisterDefinition::default()).unwrap();
    let binding = || CanisterBinding {
        id: "aaaaa-aa".into(),
        ..Default::default()
    };
    let selected = bind_canister(&path, "hello", "demo", "backend", binding()).unwrap();
    let before = std::fs::read(&path).unwrap();
    for (fleet, slot) in [
        ("demo", "backend"),
        ("demo", "other"),
        ("staging", "backend"),
        ("demo", "unknown"),
    ] {
        assert!(bind_canister(&path, "hello", fleet, slot, binding()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
    let journal = selected.journal_path(&path);
    assert!(
        create_configured_plan(
            &journal,
            &root.join("unused.wasm"),
            input(),
            &selected,
            "backend"
        )
        .is_err()
    );
    assert!(!journal.exists());
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
    bind_canister(&path, "hello", "local", "backend", binding()).unwrap();
    assert!(!root.join(".canic").exists());
}

#[test]
fn loaded_config_and_selection_retain_one_validated_snapshot() {
    let root = directory();
    let path = root.join("canic.json");
    init_config(&path, &declaration(&config())).unwrap();
    let loaded = load_config(&path).unwrap();
    let selected = loaded.select(Some("hello"), Some("demo")).unwrap();
    add_app_canister(&path, "hello", "worker", CanisterDefinition::default()).unwrap();
    assert_eq!(
        loaded.select(Some("hello"), Some("demo")).unwrap(),
        selected
    );
    assert_eq!(selected, config());
    assert_eq!(
        load_config(&path)
            .unwrap()
            .select(None, None)
            .unwrap()
            .config
            .canisters
            .len(),
        2
    );
}

#[test]
fn nested_unknown_fields_refuse_edits_without_changing_config() {
    let root = directory();
    let path = root.join("canic.json");
    let mut selected = config();
    selected.config.canisters.get_mut("backend").unwrap().id = Some("aaaaa-aa".into());
    for pointer in [
        "/apps/hello",
        "/apps/hello/canisters/backend",
        "/apps/hello/fleets/demo",
        "/apps/hello/fleets/demo/canisters/backend",
    ] {
        let mut document = serde_json::to_value(declaration(&selected)).unwrap();
        document.pointer_mut(pointer).unwrap()["unexpected"] = true.into();
        let bytes = serde_json::to_vec(&document).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        for error in [
            load_config(&path).unwrap_err(),
            init_app(&path, "another", "mainnet", Network::Mainnet).unwrap_err(),
            init_fleet(&path, "hello", "staging", Network::Mainnet).unwrap_err(),
            add_app_canister(&path, "hello", "worker", CanisterDefinition::default()).unwrap_err(),
        ] {
            let Error::Supplier(source) = error else {
                panic!("expected JSON decoding error")
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
    }
}

#[test]
fn app_and_fleet_initialization_preserve_evidence_and_refuse_conflicts() {
    let root = directory();
    let path = root.join("canic.json");
    init_app(&path, "hello", "demo", Network::Mainnet).unwrap();
    add_app_canister(&path, "hello", "backend", CanisterDefinition::default()).unwrap();
    let before = load_config(&path).unwrap();
    let journal = before.select(None, None).unwrap().journal_path(&path);
    std::fs::create_dir_all(&journal).unwrap();
    std::fs::write(journal.join("evidence"), b"original").unwrap();
    init_app(&path, "other", "demo", Network::Mainnet).unwrap();
    init_fleet(&path, "hello", "staging", Network::Mainnet).unwrap();
    let after = load_config(&path).unwrap();
    assert_eq!(
        after.apps["hello"].fleets["demo"],
        before.apps["hello"].fleets["demo"]
    );
    assert!(after.select(None, Some("demo")).is_err());
    assert!(after.select(Some("hello"), None).is_err());
    assert_eq!(
        std::fs::read(journal.join("evidence")).unwrap(),
        b"original"
    );
    let bytes = std::fs::read(&path).unwrap();
    for (app, fleet) in [("hello", "demo"), ("../bad", "new"), ("new", "../bad")] {
        assert!(init_app(&path, app, fleet, Network::Mainnet).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    for (app, fleet) in [("hello", "demo"), ("missing", "new"), ("hello", "../bad")] {
        assert!(init_fleet(&path, app, fleet, Network::Mainnet).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    let lock = ic_host_fs::durable::open_private_regular_lock_file(&lock_path(&path)).unwrap();
    ic_host_fs::durable::try_lock_exclusive(&lock).unwrap();
    assert!(init_app(&path, "new", "demo", Network::Mainnet).is_err());
    assert!(init_fleet(&path, "hello", "busy", Network::Mainnet).is_err());
    assert!(add_app_canister(&path, "hello", "busy", CanisterDefinition::default()).is_err());
    assert!(
        bind_canister(
            &path,
            "hello",
            "demo",
            "backend",
            CanisterBinding {
                id: "aaaaa-aa".into(),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    drop(lock);
    init_fleet(&path, "hello", "after-lock", Network::Mainnet).unwrap();
}

#[test]
fn concurrent_first_app_initializers_cannot_replace_a_winner() {
    let root = directory();
    let path = root.join("canic.json");
    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| {
        let handles: Vec<_> = ["one", "two"]
            .into_iter()
            .map(|app| {
                let path = &path;
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    (app, init_app(path, app, "demo", Network::Mainnet))
                })
            })
            .collect();
        for handle in handles {
            let (app, result) = handle.join().unwrap();
            if result.is_ok() {
                assert!(load_config(&path).unwrap().apps.contains_key(app));
            }
        }
    });
    assert!(!load_config(&path).unwrap().apps.is_empty());
}

#[test]
fn app_definition_bounds_apply_to_resolved_fleets_before_publication() {
    let root = directory();
    let path = root.join("canic.json");
    let mut c = declaration(&config());
    let app = c.apps.get_mut("hello").unwrap();
    app.canisters = (0..32)
        .map(|i| (format!("slot-{i}"), CanisterDefinition::default()))
        .collect();
    for i in 0..3 {
        app.fleets
            .insert(format!("fleet-{i}"), app.fleets["demo"].clone());
    }
    init_config(&path, &c).unwrap();
    let before = std::fs::read(&path).unwrap();
    assert!(add_app_canister(&path, "hello", "extra", CanisterDefinition::default()).is_err());
    assert!(init_fleet(&path, "hello", "extra", Network::Mainnet).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let mut many = declaration(&config());
    let app = many.apps.get_mut("hello").unwrap();
    app.canisters.clear();
    for i in 0..15 {
        app.fleets
            .insert(format!("fleet-{i}"), app.fleets["demo"].clone());
    }
    many.validate().unwrap();
    app_limit(&root, &many);
    assert!(!root.join(".canic").exists());
}

fn app_limit(root: &Path, many: &Config) {
    let path = root.join("many.json");
    init_config(&path, many).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert!(init_fleet(&path, "hello", "too-many", Network::Mainnet).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let mut app = many.apps["hello"].clone();
    app.fleets.retain(|name, _| name == "demo");
    let all = Config {
        apps: (0..16).map(|i| (format!("app-{i}"), app.clone())).collect(),
    };
    let all_path = root.join("all.json");
    init_config(&all_path, &all).unwrap();
    assert!(init_app(&all_path, "extra", "demo", Network::Mainnet).is_err());
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
    assert!(init_fleet(&link, "hello", "new", Network::Mainnet).is_err());
    assert!(init_app(&link, "other", "demo", Network::Mainnet).is_err());
    assert!(add_app_canister(&link, "hello", "child", CanisterDefinition::default()).is_err());
    assert_eq!(std::fs::read(&original).unwrap(), before);
    assert!(!lock_path(&link).exists());
    let missing = root.join("missing.json");
    let dangling = root.join("dangling.json");
    std::os::unix::fs::symlink(&missing, &dangling).unwrap();
    assert!(init_app(&dangling, "other", "demo", Network::Mainnet).is_err());
    assert!(!missing.exists());
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
    invalid.apps.clear();
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
    let mut described = config();
    described
        .config
        .canisters
        .insert("manager".into(), CanisterConfig::default());
    let backend = described.config.canisters.get_mut("backend").unwrap();
    backend.role = Some("worker".into());
    backend.parent = Some("manager".into());
    assert!(create_configured_plan(&journal, &wasm, input(), &described, "backend").is_err());
    // A changed tree cannot turn direct authority into Root-owned authority.
    // The original operation remains intact and available to apply/status.
    assert_eq!(plan::load(&journal.join(&id), &id).unwrap().0, plan);
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
    assert_eq!(
        snapshot.operations[0].network_id,
        input().network.binding_id().unwrap()
    );
    assert_ne!(
        snapshot.operations[0].network_id,
        edited.config.network.binding_id().unwrap()
    );
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
    assert_eq!(catalog.select_app(None).unwrap().0, "hello");
    assert!(catalog.select_app(Some("missing")).is_err());
    assert!(catalog.select(None, None).is_err());
    let demo = catalog.select(Some("hello"), Some("demo")).unwrap();
    let staging = catalog.select(Some("hello"), Some("staging")).unwrap();
    assert_ne!(
        demo.journal_path(&config_path),
        staging.journal_path(&config_path)
    );
    let other = catalog.apps["hello"].clone();
    catalog.apps.insert("other-app".into(), other);
    assert!(catalog.select_app(None).is_err());
    assert_eq!(catalog.select_app(Some("hello")).unwrap().0, "hello");
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
        None,
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
    catalog.apps.get_mut("hello").unwrap().fleets.insert(
        "other".into(),
        declaration(&selected).apps["hello"].fleets["demo"].clone(),
    );
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
    let fleet =
        serde_json::to_string(&declaration(&config()).apps["hello"].fleets["demo"]).unwrap();
    let documents = [
        r#"{"apps":{"hello":APP,"hello":APP}}"#.replace("APP", &app),
        r#"{"apps":{"hello":{"canisters":{},"fleets":{"demo":FLEET,"demo":FLEET}}}}"#.replace("FLEET", &fleet),
        r#"{"apps":{"hello":{"canisters":{},"fleets":{"demo":{"network":{"kind":"Mainnet"},"canisters":{"backend":{"id":"aaaaa-aa"},"backend":{"id":"aaaaa-aa"}}}}}}}"#.into(),
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
