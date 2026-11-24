use super::*;

fn package(name: &str, source: Option<&str>, version: &str) -> Package {
    Package {
        features: Default::default(),
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
    let consumer = Path::new("/consumer/Cargo.toml");
    let mut root = package("canic-root", None, "0.253.0");
    root.manifest_path = dir.path().join("Cargo.toml");
    let mut graph = Metadata {
        workspace_members: vec![root.id.clone()],
        packages: vec![root],
    };
    assert_eq!(root_manifest(consumer, &graph).unwrap(), consumer);
    graph.workspace_members.clear();
    std::fs::write(dir.path().join("Cargo.lock"), "").unwrap();
    assert!(root_manifest(consumer, &graph).is_err());
    graph.packages[0].source = Some(REGISTRY.into());
    assert_eq!(
        root_manifest(consumer, &graph).unwrap(),
        dir.path().join("Cargo.toml")
    );
    std::fs::remove_file(dir.path().join("Cargo.lock")).unwrap();
    assert!(root_manifest(consumer, &graph).is_err());
    graph
        .packages
        .push(package("canic-root", Some(REGISTRY), "0.252.0"));
    assert!(
        root_manifest(consumer, &graph)
            .unwrap_err()
            .to_string()
            .contains("multiple")
    );
}
