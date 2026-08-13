use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate directory should have a parent")
        .parent()
        .expect("workspace root should exist")
        .to_path_buf()
}

fn rust_sources_under(path: &Path) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    for entry in fs::read_dir(path).expect("macro source directory should be readable") {
        let path = entry.expect("macro source entry should be readable").path();
        if path.is_dir() {
            sources.extend(rust_sources_under(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
    sources
}

fn custom_identifiers(source: &str) -> BTreeSet<String> {
    let mut identifiers = BTreeSet::new();
    let mut remaining = source;
    while let Some(offset) = remaining.find("canic_") {
        let candidate = &remaining[offset..];
        let length = candidate
            .bytes()
            .take_while(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
            .count();
        identifiers.insert(candidate[..length].to_string());
        remaining = &candidate[length..];
    }
    identifiers
}

fn consumed_custom_cfgs(root: &Path) -> BTreeSet<String> {
    rust_sources_under(&root.join("crates/canic/src/macros"))
        .into_iter()
        .flat_map(|path| {
            let source = fs::read_to_string(path).expect("macro source should be readable");
            source
                .split("#[cfg")
                .skip(1)
                .filter_map(|suffix| suffix.split_once(']').map(|(expression, _)| expression))
                .flat_map(custom_identifiers)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn emitted_custom_cfgs(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter(|line| line.contains("cargo:rustc-cfg=canic_"))
        .flat_map(custom_identifiers)
        .collect()
}

#[test]
fn custom_cfg_catalog_is_exact_and_singly_owned() {
    let root = workspace_root();
    let expected = canic::__build::CANIC_CUSTOM_CFG_NAMES
        .iter()
        .map(|name| (*name).to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(expected.len(), canic::__build::CANIC_CUSTOM_CFG_NAMES.len());
    assert_eq!(consumed_custom_cfgs(&root), expected);

    let build_macro = fs::read_to_string(root.join("crates/canic/src/macros/build.rs"))
        .expect("build macro source should be readable");
    assert_eq!(emitted_custom_cfgs(&build_macro), expected);
    assert!(!build_macro.contains("cargo:rustc-check-cfg=cfg(canic_"));
    assert!(build_macro.contains("$crate::__build::CANIC_CUSTOM_CFG_NAMES"));

    let facade_build = fs::read_to_string(root.join("crates/canic/build.rs"))
        .expect("facade build script should be readable");
    assert!(facade_build.contains("include!(\"src/build_support/cfg_catalog.rs\")"));
    assert!(facade_build.contains("for custom_cfg in CANIC_CUSTOM_CFG_NAMES"));
    assert!(!facade_build.contains("cargo:rustc-check-cfg=cfg(canic_"));

    let core_build = fs::read_to_string(root.join("crates/canic-core/build.rs"))
        .expect("core build script should be readable");
    assert!(!core_build.contains("canic_is_root"));

    for removed in [
        "CANIC_APP_ROLE",
        "CANIC_APP=",
        "CANIC_CANISTER_ROLE_DECLARED",
        "CANIC_CANISTER_ROLE_ATTACHED",
    ] {
        assert!(
            !build_macro.contains(removed),
            "removed compile-time output returned: {removed}"
        );
    }
}
