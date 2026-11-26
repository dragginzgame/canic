use super::*;

fn package(name: &str, source: Option<&str>, version: &str) -> Package {
    Package {
        id: format!("{name}@{version}"),
        name: name.into(),
        version: version.into(),
        source: source.map(str::to_owned),
        manifest_path: format!("/registry/{name}-{version}/Cargo.toml").into(),
    }
}

#[test]
fn only_one_matching_locked_registry_pair_can_select_runtime_bytes() {
    let runtime = || package("ic-wasm-store", Some(REGISTRY), "0.2.0");
    let protocol = || package("ic-wasm-store-protocol", Some(REGISTRY), "0.2.0");
    assert_eq!(
        selected(&Metadata {
            workspace_members: vec![],
            packages: vec![runtime(), protocol()]
        })
        .unwrap()
        .manifest_path,
        PathBuf::from("/registry/ic-wasm-store-0.2.0/Cargo.toml")
    );
    for name in ["ic-wasm-store", "ic-wasm-store-protocol"] {
        for source in [
            None,
            Some("registry+https://example.org/index"),
            Some(
                "git+https://github.com/dragginzgame/ic-wasm-store?rev=0123456789012345678901234567890123456789#0123456789012345678901234567890123456789",
            ),
        ] {
            let mut packages = vec![runtime(), protocol()];
            packages.iter_mut().find(|p| p.name == name).unwrap().source =
                source.map(str::to_owned);
            assert!(
                selected(&Metadata {
                    workspace_members: vec![],
                    packages
                })
                .is_err()
            );
        }
    }
    for packages in [
        vec![],
        vec![runtime()],
        vec![protocol()],
        vec![runtime(), runtime(), protocol()],
        vec![runtime(), protocol(), protocol()],
        vec![
            runtime(),
            package("ic-wasm-store-protocol", Some(REGISTRY), "0.2.1"),
        ],
    ] {
        assert!(
            selected(&Metadata {
                workspace_members: vec![],
                packages
            })
            .is_err()
        );
    }
}

#[test]
fn root_selection_requires_one_locked_archive_or_the_workspace_member() {
    let dir = tempfile::tempdir().unwrap();
    let consumer = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let context = || BuildSource::workspace(&consumer).unwrap();
    let mut root = package("canic-root", None, "0.253.0");
    root.manifest_path = dir.path().join("Cargo.toml");
    let mut graph = Metadata {
        workspace_members: vec![root.id.clone()],
        packages: vec![root],
    };
    assert_eq!(
        canic_manifest(context(), &graph, "canic-root")
            .unwrap()
            .manifest,
        consumer
    );
    graph.workspace_members.clear();
    std::fs::write(dir.path().join("Cargo.lock"), "").unwrap();
    assert!(canic_manifest(context(), &graph, "canic-root").is_err());
    graph.packages[0].source = Some(REGISTRY.into());
    assert_eq!(
        canic_manifest(context(), &graph, "canic-root")
            .unwrap()
            .manifest,
        dir.path().join("Cargo.toml")
    );
    std::fs::remove_file(dir.path().join("Cargo.lock")).unwrap();
    assert!(canic_manifest(context(), &graph, "canic-root").is_err());
    graph
        .packages
        .push(package("canic-root", Some(REGISTRY), "0.252.0"));
    assert!(
        canic_manifest(context(), &graph, "canic-root")
            .unwrap_err()
            .to_string()
            .contains("multiple")
    );
}

#[test]
fn nested_archive_uses_its_lock_and_the_consumers_toolchain_without_local_patches() {
    let dir = tempfile::tempdir().unwrap();
    let consumer = dir.path().join("Cargo.toml");
    let archive = dir.path().join("cache/package/Cargo.toml");
    let lock = "version = 4\n[[package]]\nname = 'fixture'\nversion = '0.1.0'\n";
    for manifest in [&consumer, &archive] {
        std::fs::create_dir_all(manifest.parent().unwrap().join("src")).unwrap();
        std::fs::write(
            manifest,
            "[workspace]\n[package]\nname='fixture'\nversion='0.1.0'\nedition='2024'",
        )
        .unwrap();
        std::fs::write(manifest.with_file_name("src").join("lib.rs"), "").unwrap();
        std::fs::write(manifest.with_file_name("Cargo.lock"), lock).unwrap();
    }
    std::fs::create_dir(dir.path().join(".cargo")).unwrap();
    std::fs::write(
        dir.path().join(".cargo/config.toml"),
        "[patch.crates-io]\nfixture-patch = { path = 'missing-local-patch' }",
    )
    .unwrap();
    let consumer = BuildSource::workspace(&consumer).unwrap();
    assert!(consumer.metadata().is_err());
    let archive = consumer.archive(&archive).unwrap();
    let graph = archive.metadata().unwrap();
    assert_eq!(graph.packages[0].manifest_path, archive.manifest);
    let compiler = |command: &mut Command| {
        let output = command.args(["-Vv"]).output().unwrap();
        assert!(output.status.success());
        output.stdout
    };
    assert_eq!(
        compiler(&mut consumer.cargo()),
        compiler(&mut archive.cargo())
    );
    assert_eq!(
        std::fs::read_to_string(archive.manifest.with_file_name("Cargo.lock")).unwrap(),
        lock
    );
}
