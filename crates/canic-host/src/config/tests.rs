use super::edit::lock_path;
use super::*;
use crate::tests::{directory, input};

#[test]
fn fleetless_apps_validate_definitions_and_preserve_existing_workspace() {
    let root = directory();
    let path = root.join("canic.toml");
    std::fs::write(root.join("Cargo.toml"), b"existing workspace").unwrap();
    std::fs::create_dir_all(root.join("apps/toko")).unwrap();
    std::fs::write(root.join("apps/toko/lib.rs"), b"existing application").unwrap();
    let config = init_app(&path, "toko").unwrap();
    assert!(config.select_app(None).unwrap().1.fleets.is_empty());
    assert!(
        config
            .select(None, None)
            .unwrap_err()
            .to_string()
            .contains("no fleets configured")
    );
    let definition = CanisterDefinition {
        wasm: Some("artifacts/toko.wasm".into()),
        build: Some(BuildInput {
            manifest: "Cargo.toml".into(),
            package: "toko-app".into(),
            features: vec!["auth".into()],
            client: None,
        }),
        ..Default::default()
    };
    add_app_canister(&path, "toko", "app", definition.clone()).unwrap();
    let before = std::fs::read(&path).unwrap();
    for invalid in [
        CanisterDefinition {
            role: Some("bad role".into()),
            ..definition.clone()
        },
        CanisterDefinition {
            parent: Some("missing".into()),
            ..definition.clone()
        },
        CanisterDefinition {
            parent: Some("invalid".into()),
            ..definition.clone()
        },
        CanisterDefinition {
            wasm: Some("".into()),
            ..definition.clone()
        },
    ] {
        assert!(add_app_canister(&path, "toko", "invalid", invalid).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
    let mut omitted: toml::Value = toml::from_str(std::str::from_utf8(&before).unwrap()).unwrap();
    omitted["apps"]["toko"]
        .as_table_mut()
        .unwrap()
        .remove("fleets");
    let decoded: Config = omitted.try_into().unwrap();
    decoded.validate().unwrap();
    assert!(decoded.apps["toko"].fleets.is_empty());
    let selected = init_fleet(&path, "toko", "local-demo", Network::Mainnet).unwrap();
    assert!(selected.config.canisters["app"].id.is_none());
    assert_eq!(
        load_config(&path).unwrap().apps["toko"].canisters["app"],
        definition
    );
    assert_eq!(
        std::fs::read(root.join("Cargo.toml")).unwrap(),
        b"existing workspace"
    );
    assert_eq!(
        std::fs::read(root.join("apps/toko/lib.rs")).unwrap(),
        b"existing application"
    );
    assert!(!root.join(".canic").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn catalog_and_selection_share_declared_identity_and_reject_duplicates() {
    let dir = directory();
    let local = dir.join("canic.toml");
    let nested = dir.join("starter/canic.toml");
    std::fs::create_dir(dir.join("starter")).unwrap();
    init_app(&nested, "starter").unwrap();
    init_fleet(&nested, "starter", "mainnet", Network::Mainnet).unwrap();
    assert_eq!(
        resolve_config_path(&dir, None, Some("starter")).unwrap(),
        nested
    );
    assert_eq!(resolve_config_path(&dir, None, None).unwrap(), nested);
    init_app(&local, "profile").unwrap();
    assert_eq!(
        resolve_config_path(&dir, None, Some("starter")).unwrap(),
        nested
    );
    assert_eq!(
        resolve_config_path(&dir, None, Some("profile")).unwrap(),
        local
    );
    assert!(resolve_config_path(&dir, None, Some("missing")).is_err());
    assert!(resolve_config_path(&dir, None, None).is_err());
    assert_eq!(discover_configs(&dir, None).unwrap().len(), 2);
    for explicit in [Path::new("canic.toml"), Path::new("missing.toml")] {
        assert_eq!(
            resolve_config_path(&dir, Some(explicit), Some("starter")).unwrap(),
            dir.join(explicit)
        );
    }
    let selected = load_config(&nested)
        .unwrap()
        .select(Some("starter"), None)
        .unwrap();
    assert_eq!(
        selected.journal_path(&nested),
        dir.join("starter/.canic/apps/starter/fleets/mainnet")
    );
    assert!(!selected.journal_path(&nested).exists());
    // Duplicate identities never silently choose a different deployment journal.
    init_app(&local, "starter").unwrap();
    assert!(resolve_config_path(&dir, None, Some("starter")).is_err());
    std::fs::write(&local, b"malformed").unwrap();
    assert!(resolve_config_path(&dir, None, Some("starter")).is_err());
    assert_eq!(
        resolve_config_path(&dir, Some(&nested), Some("starter")).unwrap(),
        nested
    );
    std::fs::remove_file(&local).unwrap();
    for app in ["../starter", "/starter", "bad\nname"] {
        assert!(resolve_config_path(&dir, None, Some(app)).is_err());
    }
    // Directory names locate files; only the validated contents establish app identity.
    std::fs::write(&nested, b"malformed").unwrap();
    assert!(resolve_config_path(&dir, None, Some("starter")).is_err());
    std::fs::remove_file(&nested).unwrap();
    init_app(&nested, "other").unwrap();
    assert!(resolve_config_path(&dir, None, Some("starter")).is_err());
    assert_eq!(
        resolve_config_path(&dir, None, Some("other")).unwrap(),
        nested
    );
    std::fs::remove_dir_all(dir).unwrap();
}

pub(super) fn config() -> SelectedFleet {
    crate::tests::selected_fleet(
        "hello",
        "demo",
        Network::Mainnet,
        BTreeMap::from([("backend".into(), CanisterConfig::default())]),
    )
}

#[test]
fn discovery_is_bounded_and_does_not_recurse_into_generated_trees() {
    let dir = directory();
    for child in [
        "target",
        "node_modules",
        "vendor",
        ".hidden",
        "project/deeper",
    ] {
        std::fs::create_dir_all(dir.join(child)).unwrap();
        std::fs::write(dir.join(child).join("canic.toml"), b"not a config").unwrap();
    }
    assert!(discover_configs(&dir, None).unwrap().is_empty());
    for index in 0..65 {
        let child = dir.join(format!("app-{index}"));
        std::fs::create_dir(&child).unwrap();
        init_app(&child.join("canic.toml"), &format!("app-{index}")).unwrap();
    }
    assert!(
        discover_configs(&dir, None)
            .unwrap_err()
            .to_string()
            .contains("64 configs")
    );
    assert_eq!(
        discover_configs(&dir, Some(Path::new("app-0/canic.toml")))
            .unwrap()
            .len(),
        1
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn discovery_does_not_follow_child_symlinks() {
    let dir = directory();
    let source = directory();
    init_app(&source.join("canic.toml"), "demo").unwrap();
    std::os::unix::fs::symlink(&source, dir.join("alias")).unwrap();
    std::fs::create_dir(dir.join("child")).unwrap();
    std::os::unix::fs::symlink(source.join("canic.toml"), dir.join("child/canic.toml")).unwrap();
    assert!(discover_configs(&dir, None).unwrap().is_empty());
    assert_eq!(
        discover_configs(&dir, Some(Path::new("alias/canic.toml")))
            .unwrap()
            .len(),
        1
    );
    std::fs::remove_dir_all(dir).unwrap();
    std::fs::remove_dir_all(source).unwrap();
}

fn bootstrap_config() -> SelectedFleet {
    toml::from_str::<Config>("[apps.hello.fleets.demo]\nnetwork = {kind = 'Mainnet'}")
        .unwrap()
        .select(None, None)
        .unwrap()
}

pub(super) fn declaration(selected: &SelectedFleet) -> Config {
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
                    build: None,
                },
            )
        })
        .collect();
    let bindings = selected
        .config
        .canisters
        .iter()
        .filter_map(|(name, slot)| {
            slot.id
                .as_ref()
                .map(|id| (name.clone(), CanisterBinding { id: id.clone() }))
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
                        policy: Default::default(),
                        lifetime: None,

                        standby_pool: None,

                        auth: Vec::new(),
                        network: FleetNetwork::Bound(selected.config.network.clone()),
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
    let path = root.join("canic.toml");
    init_config(&path, &declaration(&config())).unwrap();
    let original = bind_canister(
        &path,
        "hello",
        "demo",
        "backend",
        CanisterBinding {
            id: "aaaaa-aa".into(),
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
            build: None,
        },
    )
    .unwrap();
    let next = init_fleet(&path, "hello", "staging", Network::Mainnet).unwrap();
    assert_eq!(next.config.canisters.len(), 4);
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
    let path = root.join("canic.toml");
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
        let bytes = toml::to_string(&document).map(String::into_bytes).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        assert!(load_config(&path).is_err());
        assert!(init_fleet(&path, "hello", "new", Network::Mainnet).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(!lock_path(&path).exists());
    }
    let duplicate = "[apps.hello.canisters.backend]\n[apps.hello.canisters.backend]\n";
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
    let path = root.join("canic.toml");
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
    let path = root.join("canic.toml");
    init_config(&path, &declaration(&config())).unwrap();
    init_fleet(&path, "hello", "staging", Network::Mainnet).unwrap();
    add_app_canister(&path, "hello", "other", CanisterDefinition::default()).unwrap();
    let binding = || CanisterBinding {
        id: "aaaaa-aa".into(),
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
            root_subnet: None,
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
    let path = root.join("canic.toml");
    init_config(&path, &declaration(&config())).unwrap();
    let loaded = load_config(&path).unwrap();
    let selected = loaded.select(Some("hello"), Some("demo")).unwrap();
    add_app_canister(&path, "hello", "worker", CanisterDefinition::default()).unwrap();
    assert_eq!(
        loaded.select(Some("hello"), Some("demo")).unwrap(),
        selected
    );
    assert_eq!(
        selected.config.canisters["backend"],
        config().config.canisters["backend"]
    );
    assert_eq!(
        load_config(&path)
            .unwrap()
            .select(None, None)
            .unwrap()
            .config
            .canisters
            .len(),
        4
    );
}

#[test]
fn named_plans_retain_identity_and_never_renew_expired_or_conflicting_authority() {
    let root = directory();
    let wasm = root.join("hello.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let journal = root.join("state");
    let (id, plan) =
        create_configured_plan(&journal, &wasm, input(), &bootstrap_config(), "root").unwrap();
    assert_eq!(plan.version, 1);
    assert!(matches!(
        plan.action,
        crate::PlanAction::BootstrapRoot { .. }
    ));
    assert_eq!(plan.deployment.as_ref().unwrap().app, "hello");
    let mut described = bootstrap_config();
    described
        .config
        .canisters
        .insert("manager".into(), CanisterConfig::default());
    let backend = described.config.canisters.get_mut("root").unwrap();
    backend.role = Some("worker".into());
    backend.parent = Some("manager".into());
    assert!(create_configured_plan(&journal, &wasm, input(), &described, "root").is_err());
    // A changed tree cannot turn direct authority into Root-owned authority.
    // The original operation remains intact and available to apply/status.
    assert_eq!(plan::load(&journal.join(&id), &id).unwrap().0, plan);
    assert_eq!(
        create_configured_plan(&journal, &wasm, input(), &bootstrap_config(), "root").unwrap(),
        (id.clone(), plan.clone())
    );
    let mut renamed = plan.clone();
    renamed.deployment.as_mut().unwrap().canister = "other".into();
    assert_ne!(renamed.digest().unwrap(), id);
    let mut conflicting = input();
    conflicting.creation_cycles += 1;
    assert!(
        create_configured_plan(&journal, &wasm, conflicting, &bootstrap_config(), "root").is_err()
    );
    assert!(
        create_configured_plan(&journal, &wasm, input(), &bootstrap_config(), "missing").is_err()
    );
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
        create_configured_plan(&journal, &wasm, input(), &bootstrap_config(), "root").unwrap(),
        (expired_id, expired)
    );
    assert_eq!(std::fs::read_dir(&journal).unwrap().count(), 2); // One operation and the journal lock.
    let _lock = store::Store::open(&journal).unwrap();
    assert!(create_configured_plan(&journal, &wasm, input(), &bootstrap_config(), "root").is_err());
}

#[test]
fn direct_approval_binds_its_named_contract() {
    let root = directory();
    let wasm = root.join("hello.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let (id, plan) = crate::create_plan(&root, &wasm, input()).unwrap();
    let json = serde_json::to_vec(&plan).unwrap();
    assert_eq!(
        serde_json::to_value(&plan).unwrap()["action"]["contract"],
        "canic:deployment-create.v1"
    );
    let mut legacy = b"canic:deployment-plan:v1\0".to_vec();
    legacy.extend(&json);
    assert_ne!(crate::artifact::digest(&legacy), id);
    let decoded: Plan = serde_json::from_slice(&json).unwrap();
    assert_eq!(decoded.digest().unwrap(), id);
}

#[tokio::test]
async fn existing_id_adoption_preserves_legacy_evidence_and_blocks_creation() {
    let root = directory();
    let config_path = root.join("config/canic.toml");
    let journal = root.join("retained");
    let wasm = root.join("app.wasm");
    std::fs::write(&wasm, b"\0asm\x01\0\0\0").unwrap();
    let mut adopted = config();
    adopted.config.network = Network::Local {
        url: "http://127.0.0.1:9".into(),
        root_key: "00".repeat(133),
        root_subnet: None,
        routing_canister: "aaaaa-aa".into(),
    };
    adopted.config.journal = Some("../retained".into());
    let slot = adopted.config.canisters.get_mut("backend").unwrap();
    slot.id = Some("aaaaa-aa".into());

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
    assert!(snapshot.managers.is_empty());
    // An unreachable owner cannot establish occupancy. Duplicate selections
    // must still result in just one observation of each permanent slot.
    assert_eq!(
        snapshot
            .issues
            .iter()
            .filter(|issue| issue.contains("permanent service slot"))
            .count(),
        canic_management_protocol::children::ChildId::HISTORICAL_SERVICES.len()
    );
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
    let config_path = Path::new("project/canic.toml");
    assert_eq!(
        declaration.journal_path(config_path),
        Path::new("project/.canic/apps/hello/fleets/demo")
    );
    declaration.config.journal = Some(".canic".into());
    assert_eq!(
        declaration.journal_path(config_path),
        Path::new("project/.canic")
    );
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
    let (id, _) =
        create_configured_plan(&root, &wasm, input(), &bootstrap_config(), "root").unwrap();
    let mut edited = bootstrap_config();
    edited.config.network = Network::Local {
        url: "http://localhost:8000".into(),
        root_key: "00".repeat(133),
        root_subnet: None,
        routing_canister: "aaaaa-aa".into(),
    };
    let mut changed_input = input();
    changed_input.network = edited.config.network.clone();
    assert!(create_configured_plan(&root, &wasm, changed_input, &edited, "root").is_err());
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
    let config_path = root.join("canic.toml");
    let mut catalog = declaration(&bootstrap_config());
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
    let (demo_id, _) = create_configured_plan(&journal, &wasm, input(), &demo, "root").unwrap();
    let (staging_id, _) =
        create_configured_plan(&journal, &wasm, input(), &staging, "root").unwrap();
    let (other_id, _) = create_configured_plan(&journal, &wasm, input(), &other, "root").unwrap();
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
        root_subnet: None,
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
        crate::now().unwrap(),
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
        .network = FleetNetwork::Bound(Network::Local {
        url: "http://127.0.0.1:9".into(),
        root_key: "00".repeat(133),
        root_subnet: None,
        routing_canister: "aaaaa-aa".into(),
    });
    catalog.validate().unwrap();
    catalog.apps.insert(
        "another-app".into(),
        declaration(&selected).apps.remove("hello").unwrap(),
    );
    assert!(catalog.validate().is_err());
}

#[test]
fn duplicate_toml_names_cannot_silently_replace_app_fleet_or_canister() {
    let root = directory();
    let path = root.join("canic.toml");
    let documents = [
        "[apps.hello]\n[apps.hello]\n",
        "[apps.hello.fleets.demo]\n[apps.hello.fleets.demo]\n",
        "[apps.hello.canisters.backend]\n[apps.hello.canisters.backend]\n",
        "[apps.hello.fleets.demo.canisters.backend]\nid = 'aaaaa-aa'\nid = '2vxsx-fae'\n",
        "[apps]\nhello = {}\nhello = {}\n",
    ];
    for document in documents {
        std::fs::write(&path, document).unwrap();
        assert!(
            load_config(&path)
                .unwrap_err()
                .to_string()
                .contains("duplicate")
        );
    }
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
}

#[test]
fn json_configuration_is_not_a_second_input_format() {
    let dir = directory();
    let path = dir.join("canic.toml");
    let json = serde_json::to_vec(&declaration(&config())).unwrap();
    std::fs::write(&path, &json).unwrap();
    assert!(load_config(&path).is_err());
    assert!(init_app(&path, "another").is_err());
    assert_eq!(std::fs::read(&path).unwrap(), json);
    std::fs::remove_dir_all(dir).unwrap();
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

#[test]
fn pool_only_supplies_its_two_declared_infrastructure_bindings() {
    let source = r#"
[apps.toko.canisters.hub]
[apps.toko.canisters.shard]
role = "account_worker"
parent = "hub"
[apps.toko.fleets.local]
network = { kind = "Mainnet" }
[apps.toko.fleets.local.canisters]
root = { id = "rrkah-fqaaa-aaaaa-aaaaq-cai" }
store = { id = "ryjl3-tyaaa-aaaaa-aaaba-cai" }
hub = { id = "r7inp-6aaaa-aaaaa-aaabq-cai" }
[[apps.toko.fleets.local.shard_pools]]
component = "shard"
root = "root"
store = "store"
release = { name = "shard", version = "v1" }
keys_per_shard = 1
max_shards = 2
creation_budget = 2000000000000
expires_at_ns = 1800000000000000000
"#;
    let config: Config = toml::from_str(source).unwrap();
    assert_eq!(config.select_app(None).unwrap().1.canisters.len(), 4);
    let selected = config.select(None, None).unwrap();
    assert_eq!(
        selected.config.canisters["store"].role.as_deref(),
        Some("wasm-store")
    );
    let mut fleet = selected.config;
    let release = fleet.policy.shard_pools.first().unwrap().release.clone();
    let manifest = crate::publication::release_manifest(release, b"\0asm\x01\0\0\0");
    let nonce = canic_management_protocol::application::Arguments::new(vec![7; 16]).unwrap();
    let old = fleet
        .shard_pool_approval("shard", manifest.clone(), nonce.clone())
        .unwrap();
    assert_eq!(old.arguments, nonce);
    fleet
        .policy
        .shard_pools
        .first_mut()
        .unwrap()
        .install_topology = true;
    let mut missing_role = fleet.clone();
    missing_role.canisters.get_mut("shard").unwrap().role = None;
    let refusal = missing_role
        .shard_pool_approval("shard", manifest.clone(), nonce.clone())
        .unwrap_err();
    assert!(
        refusal
            .to_string()
            .contains("topology install requires a role for shard")
    );
    let approved = fleet
        .shard_pool_approval("shard", manifest, nonce.clone())
        .unwrap();
    let install = canic::topology::Install::decode(approved.arguments.as_slice()).unwrap();
    assert_eq!(install.root.to_text(), approved.root);
    assert_eq!(install.parent.to_text(), approved.hub);
    assert_eq!(install.role, "account_worker");
    assert_eq!(install.application, nonce.as_slice());
    assert!(install.check_root(install.parent).is_err());
    assert!(canic::topology::Install::decode(old.arguments.as_slice()).is_err());
    for invalid in [
        source.replace("store = { id", "typo = { id"),
        source.replace("ryjl3-tyaaa-aaaaa-aaaba-cai", "rrkah-fqaaa-aaaaa-aaaaq-cai"),
        source.replace("max_shards = 2", "max_shards = 9"),
    ] {
        assert!(
            toml::from_str::<Config>(&invalid)
                .unwrap()
                .select(None, None)
                .is_err()
        );
    }
}

#[test]
fn unbound_local_pool_has_capacity_but_no_deployment_target() {
    let source = r#"
[apps.toko.canisters.hub]
[apps.toko.canisters.shard]
parent = "hub"
[apps.toko.fleets.local]
network = { kind = "Local" }
[[apps.toko.fleets.local.shard_pools]]
component = "shard"
root = "root"
store = "store"
release = { name = "shard", version = "v1" }
keys_per_shard = 1
max_shards = 2
creation_budget = 2000000000000
"#;
    let config: Config = toml::from_str(source).unwrap();
    assert!(config.select_app(Some("toko")).is_ok());
    assert!(config.select(Some("toko"), Some("local")).is_err());
    assert_eq!(
        toml::from_str::<Config>(&toml::to_string(&config).unwrap()).unwrap(),
        config
    );
    let expanded = source
        .replace(
            "[apps.toko.fleets.local]",
            "[apps.toko.fleets.local]\nsubnet_child_limit = 8",
        )
        .replace("max_shards = 2", "max_shards = 3\nincrease_from = 2");
    toml::from_str::<Config>(&expanded)
        .unwrap()
        .validate()
        .unwrap();
    for (setting, value, expected) in [
        (
            "keys_per_shard = 1",
            "keys_per_shard = 100001",
            "shard_pool.keys_per_shard = 100001; supported range is 1..=100000",
        ),
        (
            "keys_per_shard = 1",
            "keys_per_shard = 1\nexpires_at_ns = 0",
            "shard_pool.expires_at_ns = 0 must be a positive Unix timestamp",
        ),
    ] {
        let error = toml::from_str::<Config>(&source.replace(setting, value))
            .unwrap()
            .validate()
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(&format!("apps.toko.fleets.local: {expected}")),
            "{error}"
        );
    }
    for keys in [10_000, 100_000] {
        let declared = source.replace("keys_per_shard = 1", &format!("keys_per_shard = {keys}"));
        toml::from_str::<Config>(&declared)
            .unwrap()
            .validate()
            .unwrap();
    }
    toml::from_str::<Config>(
        &expanded.replace("subnet_child_limit = 8", "subnet_child_limit = 20010"),
    )
    .unwrap()
    .validate()
    .unwrap();
    for invalid in [
        expanded.replace("increase_from = 2", "increase_from = 0"),
        expanded.replace("increase_from = 2", "increase_from = 3"),
        expanded.replace("subnet_child_limit = 8", "subnet_child_limit = 2"),
        expanded.replace("subnet_child_limit = 8", "subnet_child_limit = 0"),
        source.replace("keys_per_shard = 1", "keys_per_shard = 0"),
        source.replace("keys_per_shard = 1", "keys_per_shard = 100001"),
        source.replace("max_shards = 2", "max_shards = 9"),
        source.replace("kind = \"Local\"", "kind = \"Local\", url = \"http://localhost:8000\""),
        source.replace("[[apps.toko.fleets.local.shard_pools]]", "[apps.toko.fleets.local.canisters]\nhub = { id = \"aaaaa-aa\" }\n[[apps.toko.fleets.local.shard_pools]]"),
    ] {
        assert!(toml::from_str::<Config>(&invalid).map_or(true, |c| c.validate().is_err()));
    }
}

#[test]
fn registry_can_be_declared_before_fleet_binding() {
    let source = r#"
[apps.toko.canisters.project_hub]
[apps.toko.canisters.project]
parent = "project_hub"
[apps.toko.fleets.local]
network = { kind = "Local" }
[apps.toko.fleets.local.registry]
component = "project"
root = "root"
store = "store"
release = { name = "project", version = "v1" }
max_instances = 2
creation_budget = 2000000000000
"#;
    assert!(toml::from_str::<Config>(&source.replace(".registry]", ".instance_pool]")).is_err());
    let config: Config = toml::from_str(source).unwrap();
    assert!(config.select_app(Some("toko")).is_ok());
    assert!(config.select(Some("toko"), Some("local")).is_err());
    for invalid in [
        source.replace("max_instances = 2", "max_instances = 9"),
        source.replace("parent = \"project_hub\"", ""),
        source.replace("max_instances = 2", "max_instances = 2\nkeys_per_shard = 1"),
    ] {
        assert!(toml::from_str::<Config>(&invalid).map_or(true, |c| c.validate().is_err()));
    }
}

#[test]
fn ephemeral_fleet_requires_fresh_bindings_and_journal() {
    let source = "[apps.a.canisters.hub]\n[apps.a.fleets.test]\nlifetime = 'ephemeral'\nnetwork = { kind = 'Local' }";
    let config: Config = toml::from_str(source).unwrap();
    config.validate().unwrap();
    assert!(config.select(Some("a"), Some("test")).is_err());
    for invalid in [
        format!("{source}\njournal = 'existing'"),
        format!("{source}\n[apps.a.fleets.test.canisters.hub]\nid = 'aaaaa-aa'"),
        source.replace("'Local'", "'Mainnet'"),
        source.replace("'ephemeral'", "'typo'"),
    ] {
        assert!(
            toml::from_str::<Config>(&invalid).map_or(true, |config| config.validate().is_err())
        );
    }
}

#[test]
fn standby_only_fleet_resolves_root_and_frames_the_configured_import() {
    use canic_management_protocol::stored;
    let source = r#"
[apps.reserve.canisters.app]
[apps.reserve.fleets.local]
network = { kind = "Mainnet" }
subnet_child_limit = 4
[apps.reserve.fleets.local.canisters]
root = { id = "rrkah-fqaaa-aaaaa-aaaaq-cai" }
[apps.reserve.fleets.local.standby_pool]
root = "root"
min_available_spares = 2
maximum_creations = 3
expires_at_ns = 1800000000000000000
minimum_import_cycles = 500000000000
[[apps.reserve.fleets.local.standby_pool.imports]]
network = "ic"
principals = ["ryjl3-tyaaa-aaaaa-aaaba-cai", "r7inp-6aaaa-aaaaa-aaabq-cai"]
"#;
    let config: Config = toml::from_str(source).unwrap();
    let selected = config.select(None, None).unwrap();
    let canic_management_protocol::stored::StandbyPolicy { root, .. } =
        selected.config.standby_pool.as_ref().unwrap();
    assert_eq!(root, "root");
    let crate::FleetPlanning::PlacementPools {
        standby: Some(grant),
        capacity: Some(capacity),
        ..
    } = sharding::preview(
        Path::new("canic.toml"),
        &selected,
        "r7inp-6aaaa-aaaaa-aaabq-cai",
    )
    .unwrap()
    else {
        panic!("reserve approval")
    };
    assert_eq!(grant.approval.root, capacity.approval.root);
    for (setting, limit, opted_in) in [
        ("", 40_000, false),
        ("subnet_child_limit = 40000", 40_000, false),
        (
            "subnet_child_limit = 40001\nallow_large_subnet = true",
            40_001,
            true,
        ),
        ("allow_large_subnet = true", 40_000, true),
    ] {
        let config: Config =
            toml::from_str(&source.replace("subnet_child_limit = 4", setting)).unwrap();
        let selected = config.select(None, None).unwrap();
        let crate::FleetPlanning::PlacementPools {
            capacity: Some(capacity),
            ..
        } = sharding::preview(
            Path::new("canic.toml"),
            &selected,
            "r7inp-6aaaa-aaaaa-aaabq-cai",
        )
        .unwrap()
        else {
            panic!("missing capacity approval")
        };
        assert_eq!(capacity.approval.max_children, limit);
        assert_eq!(capacity.approval.allow_large_subnet, opted_in);
    }
    let over: Config =
        toml::from_str(&source.replace("subnet_child_limit = 4", "subnet_child_limit = 40001"))
            .unwrap();
    assert!(
        over.validate()
            .unwrap_err()
            .to_string()
            .contains("allow_large_subnet = true")
    );
    let decoded: stored::StandbyPolicy = ic_wire::decode(
        &grant.request,
        ic_wire::Encoding::Frame {
            method: stored::CONFIGURE_STANDBY,
            direction: ic_wire::Direction::Request,
        },
        stored::LIMITS,
    )
    .unwrap();
    assert_eq!(decoded, grant.approval);
    assert_eq!(decoded.minimum_available, 2);
    assert_eq!(decoded.imports.len(), 2);
    assert_eq!(decoded.imports[1].principal, "r7inp-6aaaa-aaaaa-aaabq-cai");
    assert!(
        toml::from_str::<Config>(&source.replace("network = \"ic\"\n", ""))
            .unwrap_err()
            .to_string()
            .contains("network")
    );
    for (net, imports) in [("Mainnet", "local"), ("Local", "ic")] {
        let mut wrong = source
            .replace("Mainnet", net)
            .replace("network = \"ic\"", &format!("network = {imports:?}"));
        if net == "Local" {
            wrong=wrong.replace("[apps.reserve.fleets.local.canisters]\nroot = { id = \"rrkah-fqaaa-aaaaa-aaaaq-cai\" }\n","");
        }
        assert!(
            toml::from_str::<Config>(&wrong)
                .unwrap()
                .validate()
                .unwrap_err()
                .to_string()
                .contains("standby_pool.imports[0].network")
        );
    }
    assert_eq!(decoded.imports[0].previous_root, None);
    let self_handoff: Config =
        toml::from_str(&format!("{source}previous_root = '{}'\n", decoded.root)).unwrap();
    let selected = self_handoff.select(None, None).unwrap();
    let error = sharding::preview(
        Path::new("canic.toml"),
        &selected,
        "r7inp-6aaaa-aaaaa-aaabq-cai",
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains(&format!(
            "previous_root = {} for {} matches the destination Root",
            decoded.root, decoded.imports[0].principal
        )),
        "{error}"
    );
    let invalid: Config =
        toml::from_str(&source.replace("min_available_spares = 2", "min_available_spares = 5"))
            .unwrap();
    assert!(
        invalid
            .select(None, None)
            .unwrap_err()
            .to_string()
            .contains("standby_pool.min_available_spares = 5 exceeds subnet_child_limit = 4")
    );
}

#[test]
fn expired_pool_deadlines_precede_missing_build_inputs() {
    for (label, table, count) in [
        (
            "registry",
            "[apps.test.fleets.demo.registry]",
            "max_instances",
        ),
        (
            "worker_pool",
            "[[apps.test.fleets.demo.worker_pools]]",
            "max_workers",
        ),
    ] {
        let source = format!(
            r#"
[apps.test.canisters.hub]
wasm = "missing-hub.wasm"
[apps.test.canisters.child]
parent = "hub"
wasm = "missing-child.wasm"
[apps.test.fleets.demo]
network = {{ kind = "Mainnet" }}
subnet_child_limit = 4
[apps.test.fleets.demo.canisters]
root = {{ id = "rrkah-fqaaa-aaaaa-aaaaq-cai" }}
store = {{ id = "ryjl3-tyaaa-aaaaa-aaaba-cai" }}
hub = {{ id = "r7inp-6aaaa-aaaaa-aaabq-cai" }}
{table}
component = "child"
root = "root"
store = "store"
release = {{ name = "child", version = "1" }}
{count} = 1
creation_budget = 0
expires_at_ns = 1
"#
        );
        let config: Config = toml::from_str(&source).unwrap();
        let selected = config.select(None, None).unwrap();
        let error = sharding::preview(Path::new("canic.toml"), &selected, &input().controller)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(&format!("{label}.expires_at_ns = 1 has expired")),
            "{error}"
        );
    }
}

#[test]
fn registry_and_workers_share_infrastructure_but_require_separate_parents() {
    let source = include_str!("../../../../apps/examples/miner-pool/canic.toml");
    let mut config: Config = toml::from_str(source).unwrap();
    let app = config.apps.get_mut("miner-pool").unwrap();
    app.canisters
        .insert("registry".into(), app.canisters["pool"].clone());
    let mut project = app.canisters["miner"].clone();
    project.parent = Some("registry".into());
    app.canisters.insert("project".into(), project);
    let fleet = app.fleets.get_mut("local").unwrap();
    let worker = &fleet.policy.worker_pools[0];
    fleet.policy.registry = Some(RegistryConfig {
        singleton: None,
        component: "project".into(),
        root: worker.root.clone(),
        store: worker.store.clone(),
        release: worker.release.clone(),
        max_instances: 1,
        creation_budget: worker.creation_budget,
        expires_at_ns: worker.expires_at_ns,
        increase_from: None,
        install_args: None,
    });
    config.validate().unwrap();
    for (field, expected) in [
        (
            "root",
            r#"pool "miner".root = "other" must match pool "project".root = "root""#,
        ),
        (
            "store",
            r#"pool "miner".store = "other" must match pool "project".store = "store""#,
        ),
        (
            "component",
            r#"placement component "project" is declared in more than one pool"#,
        ),
        ("parent", r#"both have parent "pool""#),
    ] {
        let mut invalid = config.clone();
        let app = invalid.apps.get_mut("miner-pool").unwrap();
        let worker = &mut app.fleets.get_mut("local").unwrap().policy.worker_pools[0];
        match field {
            "root" => worker.root = "other".into(),
            "store" => worker.store = "other".into(),
            "component" => worker.component = "project".into(),
            _ => app.canisters.get_mut("project").unwrap().parent = Some("pool".into()),
        }
        let error = invalid.validate().unwrap_err().to_string();
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn worker_pools_plan_separately_and_require_the_declared_hub() {
    let source = include_str!("../../../../apps/examples/miner-pool/canic.toml").replace(
        "network = { kind = \"Local\" }",
        "network = { kind = \"Mainnet\" }",
    );
    let dir = directory();
    let mut config: Config = toml::from_str(&source).unwrap();
    let app = config.apps.get_mut("miner-pool").unwrap();
    app.canisters
        .insert("auxiliary".into(), app.canisters["miner"].clone());
    let fleet = app.fleets.get_mut("local").unwrap();
    for (name, id) in [
        ("root", "rrkah-fqaaa-aaaaa-aaaaq-cai"),
        ("store", "ryjl3-tyaaa-aaaaa-aaaba-cai"),
        ("pool", "r7inp-6aaaa-aaaaa-aaabq-cai"),
    ] {
        fleet
            .canisters
            .insert(name.into(), CanisterBinding { id: id.into() });
    }
    fleet.policy.worker_pools[0].expires_at_ns = Some(u64::MAX);
    let mut other = fleet.policy.worker_pools[0].clone();
    other.component = "auxiliary".into();
    fleet.policy.worker_pools.push(other);
    // The same hub may own both kinds of grant and more than one of each.
    for name in ["profiles", "accounts"] {
        let mut child = app.canisters["miner"].clone();
        child.wasm = Some("artifacts/shard.wasm".into());
        app.canisters.insert(name.into(), child);
        fleet.policy.shard_pools.push(ShardPoolConfig {
            component: name.into(),
            root: "root".into(),
            store: "store".into(),
            release: fleet.policy.worker_pools[0].release.clone(),
            max_shards: 1,
            keys_per_shard: 3,
            creation_budget: 0,
            expires_at_ns: Some(u64::MAX),
            install_args: None,
            install_topology: true,
            increase_from: None,
        });
    }
    fleet.policy.shard_pools[1].max_shards = 2;
    fleet.policy.shard_pools[1].increase_from = Some(1);
    let mut selected = config.select(None, None).unwrap();
    let header = b"\0asm\x01\0\0\0";
    let mut hub = header.to_vec();
    let section = b"canic:pool:worker:auxiliary";
    hub.extend([
        0,
        u8::try_from(section.len() + 1).unwrap(),
        u8::try_from(section.len()).unwrap(),
    ]);
    hub.extend(section);
    std::fs::create_dir(dir.join("artifacts")).unwrap();
    std::fs::write(dir.join("artifacts/pool.wasm"), &hub).unwrap();
    std::fs::write(dir.join("artifacts/shard.wasm"), header).unwrap();
    std::fs::write(dir.join("artifacts/miner.wasm"), header).unwrap();
    let plan = |fleet: &SelectedFleet| {
        crate::plan_fleet(
            &dir,
            &dir.join("canic.toml"),
            fleet,
            &input().controller,
            None,
            None,
        )
    };
    let crate::FleetPlanning::PlacementPools {
        workers,
        shards,
        increases,
        ..
    } = plan(&selected).unwrap()
    else {
        panic!("worker approvals")
    };
    let mut previous = shards[1].approval.clone();
    previous.max_shards = 1;
    assert_eq!(increases.len(), 1);
    assert_eq!(
        increases[0].approval,
        canic_management_protocol::stored::IncreasePoolLimit::Shards(previous, 2)
    );
    assert_eq!(shards.len(), 2);
    assert_ne!(shards[0].approval.pool, shards[1].approval.pool);
    assert_eq!(shards[0].approval.hub, workers[0].approval.hub);
    assert_eq!(workers.len(), 2);
    assert_ne!(workers[0].request, workers[1].request);
    assert_eq!(workers[0].approval.hub, workers[1].approval.hub);
    std::fs::write(dir.join("artifacts/miner.wasm"), &hub).unwrap();
    let wrong_hub = plan(&selected).unwrap_err();
    std::fs::write(dir.join("artifacts/miner.wasm"), header).unwrap();
    assert!(
        wrong_hub
            .to_string()
            .contains("hub \"miner\" requires worker pool \"auxiliary\"")
    );
    selected.config.policy.worker_pools.pop();
    assert!(
        plan(&selected)
            .unwrap_err()
            .to_string()
            .contains("hub \"pool\" requires worker pool \"auxiliary\"")
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn registry_plans_project_paid_singletons_without_funding_projects() {
    let source = r#"
[apps.projects.canisters]
root = { role = "root" }
registry = { role = "registry", parent = "root" }
project = { role = "project", parent = "registry", wasm = "project.wasm" }
ledger = { role = "project_ledger", parent = "project", wasm = "ledger.wasm" }
[apps.projects.fleets.local]
network = { kind = "Mainnet" }
subnet_child_limit = 6
top_up_children = [{ child = "ledger", below_cycles = "1T", amount_cycles = "0.5T" }]
[apps.projects.fleets.local.canisters]
root = { id = "rrkah-fqaaa-aaaaa-aaaaq-cai" }
store = { id = "ryjl3-tyaaa-aaaaa-aaaba-cai" }
registry = { id = "r7inp-6aaaa-aaaaa-aaabq-cai" }
[apps.projects.fleets.local.registry]
component = "project"
root = "root"
store = "store"
release = { name = "project", version = "1" }
max_instances = 2
creation_budget = 2000000000000
expires_at_ns = 9000000000000000000
[apps.projects.fleets.local.registry.singleton]
component = "ledger"
release = { name = "ledger", version = "1" }
maximum = 2
creation_budget = 2000000000000
"#;
    assert!(
        toml::from_str::<SourceConfig>(&source.replace(
            "max_instances = 2",
            "max_instances = 2\ncreation_payer = \"root\""
        ))
        .is_err()
    );
    let config: Config = toml::from_str(source).unwrap();
    let fleet = config.select(None, None).unwrap();
    let dir = directory();
    for name in ["ledger.wasm", "project.wasm"] {
        std::fs::write(dir.join(name), b"\0asm\x01\0\0\0").unwrap();
    }
    let crate::FleetPlanning::PlacementPools {
        instances: Some(project),
        singleton: Some(child),
        ..
    } = sharding::preview(&dir.join("canic.toml"), &fleet, &input().controller).unwrap()
    else {
        panic!("expected project and ledger approvals")
    };
    assert_eq!(child.approval.registry, project.approval.hub);
    assert_eq!(child.approval.role, "project_ledger");
    assert!(child.approval.receiver.is_none());
    assert_eq!(fleet.config.policy.top_up_children.len(), 1);
    assert_eq!(fleet.config.policy.top_up_children[0].child(), "ledger");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn fleet_top_ups_select_only_the_named_direct_child() {
    let text = r#"
[apps.toko.canisters.root]
role = "root"
[apps.toko.canisters.project_registry]
parent = "root"
[apps.toko.canisters.project]
parent = "project_registry"
[apps.toko.fleets.local]
network = { kind = "Mainnet" }
top_up_children = ["project_registry"]
"#;
    let path = directory().join("canic.toml");
    std::fs::write(&path, text).unwrap();
    let selected = load_config(&path).unwrap().select(None, None).unwrap();
    assert_eq!(
        selected.config.policy.top_up_children[0].child(),
        "project_registry"
    );
    assert!(
        selected
            .config
            .child_funding("project_registry", 400, u64::MAX)
            .is_err()
    );
    let bound = text.replace(
        r#"["project_registry"]"#,
        r#"[{ child = "project_registry", below_cycles = 100, amount_cycles = 200 }]"#,
    ) + r#"
[apps.toko.fleets.local.canisters]
root = { id = "rrkah-fqaaa-aaaaa-aaaaq-cai" }
project_registry = { id = "ryjl3-tyaaa-aaaaa-aaaba-cai" }
"#;
    std::fs::write(&path, &bound).unwrap();
    let fleet = load_config(&path)
        .unwrap()
        .select(None, None)
        .unwrap()
        .config;
    let rule = fleet
        .child_funding("project_registry", 400, u64::MAX)
        .unwrap();
    assert_eq!(rule.parent.to_text(), "rrkah-fqaaa-aaaaa-aaaaq-cai");
    assert_eq!(rule.child.to_text(), "ryjl3-tyaaa-aaaaa-aaaba-cai");
    assert_eq!(
        (rule.below_cycles, rule.amount_cycles, rule.budget_cycles),
        (100, 200, 400)
    );
    assert!(
        fleet
            .child_funding("project_registry", 199, u64::MAX)
            .is_err()
    );
    assert!(fleet.child_funding("project_registry", 400, 1).is_err());
    assert!(fleet.child_funding("project", 400, u64::MAX).is_err());
    let project_rule = bound.replace("child = \"project_registry\"", "child = \"project\"")
        + "project = { id = \"r7inp-6aaaa-aaaaa-aaabq-cai\" }\n";
    std::fs::write(&path, project_rule).unwrap();
    let project = load_config(&path)
        .unwrap()
        .select(None, None)
        .unwrap()
        .config
        .child_funding("project", 400, u64::MAX)
        .unwrap();
    assert_eq!(project.parent, rule.child);
    for (quantity, cycles) in [
        ("10.3T", 10_300_000_000_000),
        ("0.1B", 100_000_000),
        (".25Q", 250_000_000_000_000),
        ("0.0000000010B", 1),
        ("18446.744073709551615Q", u64::MAX),
    ] {
        let source = bound
            .replace(
                "below_cycles = 100",
                &format!("below_cycles = \"{quantity}\""),
            )
            .replace(
                "amount_cycles = 200",
                &format!("amount_cycles = \"{quantity}\""),
            );
        std::fs::write(&path, source).unwrap();
        let selected = load_config(&path).unwrap().select(None, None).unwrap();
        let approval = selected
            .config
            .child_funding("project_registry", u64::MAX, u64::MAX)
            .unwrap();
        assert_eq!(
            (approval.below_cycles, approval.amount_cycles),
            (cycles, cycles)
        );
    }
    for (entries, error) in [
        (
            r#"{ child = "project", below_cycles = 0, amount_cycles = 200 }"#,
            "below_cycles=0",
        ),
        (
            r#"{ child = "project", below_cycles = 100, amount_cycles = 0 }"#,
            "amount_cycles=0",
        ),
        (
            r#"{ child = "project", below_cycles = "5t", amount_cycles = 1 }"#,
            "uppercase B, T or Q",
        ),
        (
            r#"{ child = "project", below_cycles = 1, amount_cycles = "0.0000000001B" }"#,
            "precision is below one cycle",
        ),
        (
            r#"{ child = "project", below_cycles = "18446.744073709551616Q", amount_cycles = 1 }"#,
            "exceeds u64",
        ),
        (
            r#"{ child = "project", below_cycles = "5e3B", amount_cycles = 1 }"#,
            "unsigned decimal",
        ),
        (r#""missing""#, "unknown component"),
        (r#""root""#, "no declared parent"),
        (r#""project", "project""#, "duplicate component"),
        (
            r#""project", { child = "project", below_cycles = 1, amount_cycles = 1 }"#,
            "duplicate component",
        ),
    ] {
        std::fs::write(
            &path,
            text.replace(r#""project_registry"]"#, &format!("{entries}]")),
        )
        .unwrap();
        assert!(load_config(&path).unwrap_err().to_string().contains(error));
    }
}

#[test]
fn supplied_store_uses_declared_root_and_rejects_conflicting_slots() {
    let source = r#"
[apps.demo]
[apps.demo.canisters.manager]
role = "root"
wasm = "artifacts/custom-root.wasm"
build = { manifest = "../../qualification/Cargo.toml", package = "canic-root" }
"#;
    let config: Config = toml::from_str(source).unwrap();
    let store = &config.apps["demo"].canisters["store"];
    assert_eq!(store.parent.as_deref(), Some("manager"));
    let recipe = store.build.as_ref().unwrap();
    assert_eq!(recipe.manifest, Path::new("../../qualification/Cargo.toml"));
    assert!(recipe.features.is_empty());
    assert!(recipe.client.is_none());
    for (other, reason) in [
        ("[apps.demo.canisters.store]", "conflicts"),
        (
            "[apps.demo.canisters.archive]\nrole = 'wasm-store'",
            "conflicts",
        ),
        ("[apps.demo.canisters.other]\nrole = 'root'", "multiple"),
    ] {
        let error = toml::from_str::<Config>(&format!("{source}\n{other}"))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("apps.demo.canisters") && error.contains(reason),
            "{error}"
        );
    }
    for value in ["true", "false"] {
        assert!(toml::from_str::<Config>(&format!("[apps.demo]\nwasm_store = {value}")).is_err());
    }
}

#[test]
fn managed_root_needs_no_declaration_and_uses_the_app_build_workspace() {
    let source = r#"
[apps.demo.canisters.hub]
parent = "root"
build = { manifest = "../../Cargo.toml", package = "hub" }
wasm = "artifacts/hub.wasm"
"#;
    let config: Config = toml::from_str(source).unwrap();
    let slots = &config.apps["demo"].canisters;
    let root = &slots["root"];
    assert!(root.parent.is_none());
    let recipe = root.build.as_ref().unwrap();
    assert_eq!(recipe.manifest, Path::new("../../Cargo.toml"));
    assert!(recipe.features.is_empty());
    assert_eq!(recipe.client.as_deref(), Some("canic-root-client"));
    assert!(slots.contains_key("store"));
    assert!(toml::from_str::<Config>("[apps.demo.root]").is_err());
}

#[test]
fn adding_first_managed_child_recomputes_both_supplied_recipes() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("canic.toml");
    std::fs::write(&path, "[apps.demo]\n# Managed infrastructure\n").unwrap();
    let config = add_app_canister(
        &path,
        "demo",
        "hub",
        CanisterDefinition {
            parent: Some("root".into()),
            wasm: Some("artifacts/hub.wasm".into()),
            build: Some(BuildInput {
                manifest: "../Cargo.toml".into(),
                package: "hub".into(),
                features: vec![],
                client: None,
            }),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(config, load_config(&path).unwrap());
    for slot in ["root", "store"] {
        assert_eq!(
            config.apps["demo"].canisters[slot]
                .build
                .as_ref()
                .unwrap()
                .manifest,
            Path::new("../Cargo.toml")
        );
    }
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains("# Managed infrastructure"));
    let source: SourceConfig = toml::from_str(&text).unwrap();
    assert_eq!(
        source.apps["demo"].canisters.keys().collect::<Vec<_>>(),
        ["hub"]
    );
}

#[test]
fn standard_root_build_is_shared_across_fleets() {
    let miner = include_str!("../../../../apps/examples/miner-pool/canic.toml");
    let directory = include_str!("../../../../apps/examples/user-directory/canic.toml");
    let mut source: SourceConfig = toml::from_str(miner).unwrap();
    let mut other: SourceConfig = toml::from_str(directory).unwrap();
    let other = other
        .apps
        .remove("user-directory")
        .unwrap()
        .fleets
        .remove("local")
        .unwrap();
    let app = source.apps.get_mut("miner-pool").unwrap();
    app.fleets.insert("second".into(), other);
    let config = Config::try_from(source).unwrap();
    assert!(
        config.apps["miner-pool"].canisters["root"]
            .build
            .as_ref()
            .unwrap()
            .features
            .is_empty()
    );
}

#[test]
fn retired_backup_table_is_refused() {
    let source = "[apps.counter.fleets.demo]
network = {kind = 'Mainnet'}
[apps.counter.fleets.demo.backup]
storage = 'local'";
    assert!(toml::from_str::<Config>(source).is_err());
}

#[test]
fn snapshot_reader_policy_validates_authenticated_unique_principals() {
    let valid = (0..10)
        .map(|i| ic_platform_types::Principal::from_slice(&[i, 2]).to_text())
        .collect::<Vec<_>>();
    for readers in [vec![], valid.clone()] {
        let policy = SnapshotConfig {
            download_principals: readers,
            roles: None,
        };
        policy.validate().unwrap();
    }
    for readers in [
        vec![valid[0].clone(); 2],
        vec!["aaaaa-aa".into()],
        vec!["2vxsx-fae".into()],
        vec!["bad".into()],
        {
            let mut ids = valid;
            ids.push(ic_platform_types::Principal::from_slice(&[22, 2]).to_text());
            ids
        },
    ] {
        let policy = SnapshotConfig {
            download_principals: readers,
            roles: None,
        };
        assert!(
            policy
                .validate()
                .unwrap_err()
                .to_string()
                .contains("snapshots.download_principals")
        );
    }
}

#[test]
fn snapshot_reader_preview_scopes_access_to_the_selected_role_and_child() {
    let source = r#"
[apps.example.canisters.projects]
role = "project"
parent = "root"
[apps.example.canisters.accounts]
role = "account"
parent = "root"
[apps.example.fleets.main]
network = {kind = "Mainnet"}
[apps.example.fleets.main.canisters]
root = {id = "rrkah-fqaaa-aaaaa-aaaaq-cai"}
projects = {id = "ryjl3-tyaaa-aaaaa-aaaba-cai"}
accounts = {id = "r7inp-6aaaa-aaaaa-aaabq-cai"}
[apps.example.fleets.main.snapshots]
roles = ["project"]
download_principals = ["renrk-eyaaa-aaaaa-aaada-cai"]
"#;
    let config: Config = toml::from_str(source).unwrap();
    config.validate().unwrap();
    let selected = config.select(None, None).unwrap();
    let journal = directory();
    let approval = selected.snapshot_readers(&journal, "projects").unwrap();
    assert_eq!(approval.root.to_text(), "rrkah-fqaaa-aaaaa-aaaaq-cai");
    assert_eq!(approval.child.to_text(), "ryjl3-tyaaa-aaaaa-aaaba-cai");
    assert_eq!(approval.readers[0].to_text(), "renrk-eyaaa-aaaaa-aaada-cai");
    assert!(
        selected
            .snapshot_readers(&journal, "accounts")
            .unwrap()
            .readers
            .is_empty()
    );
    assert!(selected.snapshot_readers(&journal, "store").is_err());
    assert!(selected.snapshot_readers(&journal, "root").is_err());
    for name in ["backup", "backup-history"] {
        let source = source
            .replace("projects", name)
            .replace("project", "backup");
        let config: Config = toml::from_str(&source).unwrap();
        config.validate().unwrap();
        let fleet = config.select(None, None).unwrap();
        assert_eq!(fleet.snapshot_readers(&journal, name).unwrap(), approval);
    }
    let invalid: Config =
        toml::from_str(&source.replace("roles = [\"project\"]", "roles = [\"root\"]")).unwrap();
    assert!(
        invalid
            .validate()
            .unwrap_err()
            .to_string()
            .contains("snapshots.roles")
    );
}
