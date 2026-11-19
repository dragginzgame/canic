use super::*;

fn source() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

#[test]
fn starter_config_resolves_supplied_root_store_and_generated_app_without_effects() {
    let parent = crate::tests::directory();
    let destination = parent.join("starter");
    let config = create_starter(&destination, "hello", &source()).unwrap();
    for path in ["src/lib.rs", "src/endpoints/mod.rs"] {
        assert_eq!(
            std::fs::read(destination.join("starter-app").join(path)).unwrap(),
            std::fs::read(source().join("apps/template").join(path)).unwrap(),
            "starter template {path}"
        );
    }
    let initial = crate::load_config(&config).unwrap();
    assert!(initial.apps["hello"].fleets.is_empty());
    assert!(!destination.join(".canic").exists());
    crate::init_fleet(&config, "hello", "mainnet", crate::Network::Mainnet).unwrap();
    let loaded = crate::load_config(&config).unwrap();
    let selected = loaded.select(None, None).unwrap();
    assert_eq!(selected.app, "hello");
    let store = &selected.config.canisters["store"];
    assert_eq!(store.role.as_deref(), Some("wasm-store"));
    assert_eq!(store.parent.as_deref(), Some("root"));
    let recipe = loaded.apps["hello"].canisters["store"]
        .build
        .as_ref()
        .unwrap();
    assert_eq!(recipe.package, "ic-wasm-store");
    assert_eq!(recipe.manifest, source().join("Cargo.toml"));
    assert_eq!(
        selected.wasm_path(&config, "store", None).unwrap(),
        destination.join("artifacts/store.wasm")
    );
    assert_eq!(
        selected.config.canisters["app"].parent.as_deref(),
        Some("root")
    );
    assert_eq!(
        selected.config.canisters["root"].role.as_deref(),
        Some("root")
    );
    assert!(
        selected
            .config
            .canisters
            .values()
            .all(|slot| slot.id.is_none())
    );
    assert_eq!(
        selected.wasm_path(&config, "app", None).unwrap(),
        destination.join("artifacts/app.wasm")
    );
    assert!(!selected.journal_path(&config).exists());
    assert!(!destination.join("Cargo.lock").exists());
    let workspace = manifest(&destination.join("Cargo.toml")).unwrap();
    let original = manifest(&source().join("Cargo.toml")).unwrap();
    assert_eq!(
        workspace["workspace"]["lints"],
        original["workspace"]["lints"]
    );
    assert_eq!(
        workspace["workspace"]["package"]["publish"].as_bool(),
        Some(false)
    );
    let before = std::fs::read(&config).unwrap();
    assert!(matches!(
        create_starter(&destination, "other", &source()),
        Err(Error::Invalid(reason)) if reason == "starter destination already exists; nothing changed"
    ));
    assert_eq!(std::fs::read(config).unwrap(), before);
    assert_eq!(std::fs::read_dir(&parent).unwrap().count(), 1);
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn invalid_names_or_source_are_refused_before_any_staging() {
    let parent = crate::tests::directory();
    let destination = parent.join("new");
    assert!(matches!(
        create_starter(&destination, "../escape", &source()),
        Err(Error::Invalid(reason)) if reason.contains("names must be")
    ));
    assert_eq!(std::fs::read_dir(&parent).unwrap().count(), 0);
    assert!(matches!(
        create_starter(&destination, "hello", &parent),
        Err(Error::Supplier(error)) if error.downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == std::io::ErrorKind::NotFound)
    ));
    assert_eq!(std::fs::read_dir(&parent).unwrap().count(), 0);
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn app_build_admits_all_recipes_before_creating_a_lock_or_output() {
    let parent = crate::tests::directory();
    let destination = parent.join("starter");
    let path = create_starter(&destination, "hello", &source()).unwrap();
    let original = crate::load_config(&path).unwrap();
    for case in 0..10 {
        let mut config = original.clone();
        let canisters = &mut config.apps.get_mut("hello").unwrap().canisters;
        let root = canisters.get_mut("root").unwrap();
        match case {
            0 => root.build = None,
            1 => root.build.as_mut().unwrap().manifest = "starter-app/Cargo.toml".into(),
            2 => root.wasm = Some("artifacts/app.wasm".into()),
            3 => root.wasm = Some(destination.join("artifacts/app.wasm")),
            4 => root.wasm = Some("./artifacts/./app.wasm".into()),
            5 => root.wasm = Some("artifacts/missing/../app.wasm".into()),
            6 => root.wasm = Some("Cargo.toml/.".into()),
            7 => root.wasm = Some("Cargo.toml/".into()),
            8 => root.wasm = Some("Cargo.toml/../output.wasm".into()),
            _ => root.wasm = Some("Cargo.toml/child.wasm".into()),
        }
        std::fs::write(&path, crate::config::encode_config(&config).unwrap()).unwrap();
        assert!(crate::build_app(&path, None, None).is_err());
        assert!(crate::build_app(&path, None, Some("app")).is_err());
        assert!(!destination.join("Cargo.lock").exists());
        assert!(!destination.join("artifacts").exists());
        assert!(!destination.join(".canic").exists());
    }
    std::fs::write(&path, crate::config::encode_config(&original).unwrap()).unwrap();
    assert!(matches!(
        crate::build_app(&path, None, Some("missing")),
        Err(Error::Invalid(_))
    ));
    assert!(!destination.join("Cargo.lock").exists());
    assert!(!destination.join("artifacts").exists());
    std::fs::remove_dir_all(parent).unwrap();
}

#[cfg(unix)]
#[test]
fn app_build_refuses_output_aliases_before_creating_a_lock_or_output() {
    let parent = crate::tests::directory();
    let destination = parent.join("starter");
    let path = create_starter(&destination, "hello", &source()).unwrap();
    std::os::unix::fs::symlink(".", destination.join("alias")).unwrap();
    std::os::unix::fs::symlink("artifacts/app.wasm", destination.join("app-link.wasm")).unwrap();
    for alias in ["alias/artifacts/app.wasm", "app-link.wasm"] {
        let mut config = crate::load_config(&path).unwrap();
        config
            .apps
            .get_mut("hello")
            .unwrap()
            .canisters
            .get_mut("root")
            .unwrap()
            .wasm = Some(alias.into());
        std::fs::write(&path, crate::config::encode_config(&config).unwrap()).unwrap();
        for selected in [None, Some("app")] {
            assert!(matches!(
                crate::build_app(&path, None, selected),
                Err(Error::Invalid(_))
            ));
            assert!(!destination.join("Cargo.lock").exists());
            assert!(!destination.join("artifacts").exists());
        }
    }
    assert_eq!(
        std::fs::read_link(destination.join("alias")).unwrap(),
        PathBuf::from(".")
    );
    assert_eq!(
        std::fs::read_link(destination.join("app-link.wasm")).unwrap(),
        PathBuf::from("artifacts/app.wasm")
    );
    std::fs::remove_dir_all(parent).unwrap();
}

#[cfg(unix)]
#[test]
fn app_build_preserves_config_and_cargo_inputs_including_unselected_recipes() {
    let parent = crate::tests::directory();
    let destination = parent.join("starter");
    let path = create_starter(&destination, "hello", &source()).unwrap();
    let original = crate::load_config(&path).unwrap();
    let manifest = destination.join("Cargo.toml");
    let manifest_bytes = std::fs::read(&manifest).unwrap();
    std::os::unix::fs::symlink(".", destination.join("alias")).unwrap();
    std::os::unix::fs::symlink("Cargo.lock", destination.join("lock-link")).unwrap();
    for output in [
        PathBuf::from("canic.toml"),
        PathBuf::from("Cargo.toml"),
        PathBuf::from("Cargo.lock"),
        PathBuf::from("alias/canic.toml"),
        PathBuf::from("lock-link"),
        source().join("Cargo.toml"),
        source().join("Cargo.lock"),
    ] {
        let mut config = original.clone();
        config
            .apps
            .get_mut("hello")
            .unwrap()
            .canisters
            .get_mut("app")
            .unwrap()
            .wasm = Some(output);
        let bytes = crate::config::encode_config(&config).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        for selected in [None, Some("app"), Some("root")] {
            let error = crate::build_app(&path, None, selected).unwrap_err();
            assert!(error.to_string().contains("would replace"), "{error}");
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
            assert_eq!(std::fs::read(&manifest).unwrap(), manifest_bytes);
            assert!(!destination.join("Cargo.lock").exists());
            assert!(!destination.join("artifacts").exists());
            assert!(!destination.join(".canic").exists());
        }
    }
    std::fs::remove_dir_all(parent).unwrap();
}

#[cfg(unix)]
#[test]
fn source_alias_resolves_physically_and_destination_symlinks_are_not_replaced() {
    let parent = crate::tests::directory();
    let alias = parent.join("source with 'quote");
    std::os::unix::fs::symlink(source(), &alias).unwrap();
    let destination = parent.join("starter");
    create_starter(&destination, "hello", &alias).unwrap();
    let workspace = manifest(&destination.join("Cargo.toml")).unwrap();
    assert_eq!(
        workspace["workspace"]["dependencies"]["canic"]["path"].as_str(),
        source().join("crates/canic").to_str()
    );
    for target in [parent.join("absent"), destination] {
        let link = parent.join("occupied");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(create_starter(&link, "other", &source()).is_err());
        assert_eq!(std::fs::read_link(&link).unwrap(), target);
        std::fs::remove_file(link).unwrap();
    }
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn concurrent_initializers_publish_one_complete_project() {
    let parent = crate::tests::directory();
    let destination = parent.join("starter");
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let attempts: Vec<_> = ["first", "second"]
        .into_iter()
        .map(|app| {
            let destination = destination.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                create_starter(&destination, app, &source())
            })
        })
        .collect();
    let results: Vec<_> = attempts
        .into_iter()
        .map(|attempt| attempt.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    let config = crate::load_config(&destination.join("canic.toml")).unwrap();
    assert_eq!(config.select_app(None).unwrap().1.canisters.len(), 3);
    assert!(destination.join("starter-app/src/lib.rs").is_file());
    assert!(
        destination
            .join("starter-app/src/endpoints/mod.rs")
            .is_file()
    );
    assert!(destination.join("README.md").is_file());
    std::fs::remove_dir_all(parent).unwrap();
}
