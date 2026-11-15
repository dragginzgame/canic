use super::*;

fn package(source: Option<&str>) -> Package {
    Package {
        name: "ic-wasm-store-protocol".into(),
        source: source.map(str::to_owned),
        manifest_path: "/supplier/crates/ic-wasm-store-protocol/Cargo.toml".into(),
    }
}

#[test]
fn only_one_exact_locked_supplier_source_can_select_runtime_bytes() {
    let revision = "0123456789012345678901234567890123456789";
    let pinned =
        format!("git+https://github.com/dragginzgame/ic-wasm-store?rev={revision}#{revision}");
    assert_eq!(
        selected(Metadata {
            packages: vec![package(Some(&pinned))]
        })
        .unwrap(),
        PathBuf::from("/supplier/crates/ic-wasm-store-protocol/Cargo.toml")
    );
    for source in [
        None,
        Some("registry+https://github.com/rust-lang/crates.io-index"),
        Some(
            "git+https://github.com/dragginzgame/ic-wasm-store?branch=main#0123456789012345678901234567890123456789",
        ),
        Some(
            "git+https://github.com/dragginzgame/ic-wasm-store?rev=0123456789012345678901234567890123456789#1123456789012345678901234567890123456789",
        ),
    ] {
        assert!(
            selected(Metadata {
                packages: vec![package(source)]
            })
            .is_err()
        );
    }
    assert!(selected(Metadata { packages: vec![] }).is_err());
    assert!(
        selected(Metadata {
            packages: vec![package(Some(&pinned)), package(Some(&pinned))]
        })
        .is_err()
    );
}
