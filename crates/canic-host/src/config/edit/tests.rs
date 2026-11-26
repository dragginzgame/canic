use super::*;
use crate::config::tests::{config, declaration};
use crate::tests::directory;

#[test]
fn concurrent_first_app_initializers_cannot_replace_a_winner() {
    let root = directory();
    let path = root.join("canic.toml");
    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let path = &path;
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    init_app(path, "demo")
                })
            })
            .collect();
        let winners = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .filter(Result::is_ok)
            .count();
        assert_eq!(winners, 1);
    });
    let config = load_config(&path).unwrap();
    assert_eq!(config.apps.len(), 1);
    assert!(config.apps.contains_key("demo"));
}

#[test]
fn toml_edits_preserve_comments_and_existing_values_in_supported_table_forms() {
    for declarations in [
        "[apps.demo.canisters.app]\nrole = 'entry' # role note\nwasm = '''artifacts/app.wasm'''\n",
        "[apps.demo]\ncanisters = { app = { role = 'entry', wasm = '''artifacts/app.wasm''' } } # role note\n",
        "[apps.demo]\ncanisters.app.role = 'entry' # role note\ncanisters.app.wasm = '''artifacts/app.wasm'''\n",
    ] {
        let dir = directory();
        let path = dir.join("canic.toml");
        let original = format!(
            "# Application notes\n{declarations}\n[apps.demo.fleets.mainnet]\n# Deployment notes\nnetwork = {{ kind = 'Mainnet' }} # trust note\ncanisters = {{}} # bindings note\n\n# End notes\n"
        );
        std::fs::write(&path, &original).unwrap();
        init_app(&path, "another").unwrap();
        init_fleet(&path, "demo", "staging", Network::Mainnet).unwrap();
        add_app_canister(
            &path,
            "demo",
            "worker",
            CanisterDefinition {
                parent: Some("app".into()),
                ..Default::default()
            },
        )
        .unwrap();
        bind_canister(
            &path,
            "demo",
            "mainnet",
            "app",
            CanisterBinding {
                id: "aaaaa-aa".into(),
                profile_manager: false,
            },
        )
        .unwrap();
        let updated = std::fs::read_to_string(&path).unwrap();
        for preserved in [
            "# Application notes",
            "# role note",
            "# Deployment notes",
            "# trust note",
            "# bindings note",
            "# End notes",
            "'entry'",
            "'''artifacts/app.wasm'''",
            "kind = 'Mainnet'",
        ] {
            assert!(updated.contains(preserved), "lost {preserved}: {updated}");
        }
        let selected = load_config(&path)
            .unwrap()
            .select(Some("demo"), Some("mainnet"))
            .unwrap();
        assert_eq!(
            selected.config.canisters["app"].id.as_deref(),
            Some("aaaaa-aa")
        );
        assert_eq!(
            selected.config.canisters["worker"].parent.as_deref(),
            Some("app")
        );
        // Rejected edits retain the complete original source, including comments.
        assert!(init_app(&path, "demo").is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), updated);
        assert!(!dir.join(".canic").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn nested_unknown_fields_refuse_edits_without_changing_config() {
    let root = directory();
    let path = root.join("canic.toml");
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
        let bytes = toml::to_string(&document).map(String::into_bytes).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        for error in [
            load_config(&path).unwrap_err(),
            init_app(&path, "another").unwrap_err(),
            init_fleet(&path, "hello", "staging", Network::Mainnet).unwrap_err(),
            add_app_canister(&path, "hello", "worker", CanisterDefinition::default()).unwrap_err(),
        ] {
            let Error::Supplier(source) = error else {
                panic!("expected TOML decoding error")
            };
            assert!(source.downcast_ref::<toml::de::Error>().is_some());
        }
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(!lock_path(&path).exists());
    }
}

#[test]
fn app_and_fleet_initialization_preserve_evidence_and_refuse_conflicts() {
    let root = directory();
    let path = root.join("canic.toml");
    init_app(&path, "hello").unwrap();
    init_fleet(&path, "hello", "demo", Network::Mainnet).unwrap();
    add_app_canister(&path, "hello", "backend", CanisterDefinition::default()).unwrap();
    let before = load_config(&path).unwrap();
    let journal = before.select(None, None).unwrap().journal_path(&path);
    std::fs::create_dir_all(&journal).unwrap();
    std::fs::write(journal.join("evidence"), b"original").unwrap();
    init_app(&path, "other").unwrap();
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
    for app in ["hello", "../bad"] {
        assert!(init_app(&path, app).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    for (app, fleet) in [("hello", "demo"), ("missing", "new"), ("hello", "../bad")] {
        assert!(init_fleet(&path, app, fleet, Network::Mainnet).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    let lock = crate::sidecar_lock::acquire(&lock_path(&path)).unwrap();
    assert!(init_app(&path, "new").is_err());
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
fn app_definition_bounds_apply_to_resolved_fleets_before_publication() {
    let root = directory();
    let path = root.join("canic.toml");
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
    let path = root.join("many.toml");
    init_config(&path, many).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert!(init_fleet(&path, "hello", "too-many", Network::Mainnet).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let mut app = many.apps["hello"].clone();
    app.fleets.retain(|name, _| name == "demo");
    let all = Config {
        apps: (0..16).map(|i| (format!("app-{i}"), app.clone())).collect(),
    };
    let all_path = root.join("all.toml");
    init_config(&all_path, &all).unwrap();
    assert!(init_app(&all_path, "extra").is_err());
}

#[test]
fn fleet_initialization_does_not_follow_config_symlinks() {
    let root = directory();
    let original = root.join("original.toml");
    init_config(&original, &declaration(&config())).unwrap();
    let before = std::fs::read(&original).unwrap();
    let link = root.join("canic.toml");
    std::os::unix::fs::symlink(&original, &link).unwrap();
    assert!(init_fleet(&link, "hello", "new", Network::Mainnet).is_err());
    assert!(init_app(&link, "other").is_err());
    assert!(add_app_canister(&link, "hello", "child", CanisterDefinition::default()).is_err());
    assert_eq!(std::fs::read(&original).unwrap(), before);
    assert!(!lock_path(&link).exists());
    let missing = root.join("missing.toml");
    let dangling = root.join("dangling.toml");
    std::os::unix::fs::symlink(&missing, &dangling).unwrap();
    assert!(init_app(&dangling, "other").is_err());
    assert!(!missing.exists());
}

#[test]
fn initialization_is_local_bounded_and_preserves_existing_files() {
    let root = directory();
    let path = root.join("canic.toml");
    let config = declaration(&config());
    init_config(&path, &config).unwrap();
    assert_eq!(
        load_config(&path).unwrap().apps["hello"].fleets,
        config.apps["hello"].fleets
    );
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    let before = std::fs::read(&path).unwrap();
    assert!(init_config(&path, &config).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let mut invalid = config.clone();
    invalid
        .apps
        .insert("../escape".into(), config.apps["hello"].clone());
    assert!(init_config(&root.join("bad.toml"), &invalid).is_err());
    let mut invalid = config;
    invalid.apps.clear();
    assert!(invalid.validate().is_err());
    std::fs::write(&path, vec![b' '; 32769]).unwrap();
    assert!(load_config(&path).is_err());
    assert!(!root.join("bad.toml").exists());
}
