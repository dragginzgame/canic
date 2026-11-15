use super::*;

fn package(name: &str, source: Option<&str>, version: &str) -> Package {
    Package {
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
            assert!(selected(&Metadata { packages }).is_err());
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
        assert!(selected(&Metadata { packages }).is_err());
    }
}
