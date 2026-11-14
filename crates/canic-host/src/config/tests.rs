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
