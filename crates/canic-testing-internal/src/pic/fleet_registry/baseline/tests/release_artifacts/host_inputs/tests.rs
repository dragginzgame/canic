//! Cache admission and invalidation qualification for standalone native test sources.

use super::*;
use ic_testkit::artifacts::{
    ArtifactCacheOutcome, ArtifactCachePreparation, prepare_artifact_cache,
};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "canic-host-source-cache-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join("host/src/tests")).unwrap();
    fs::write(root.join("host/Cargo.toml"), "host manifest").unwrap();
    fs::write(
        root.join("host/src/lib.rs"),
        "mod production;\n#[cfg(test)] mod tests;\n",
    )
    .unwrap();
    fs::write(
        root.join("host/src/production.rs"),
        "pub fn value() -> u8 { 1 }\n",
    )
    .unwrap();
    fs::write(root.join("host/src/tests/mod.rs"), "mod capacity;\n").unwrap();
    fs::write(
        root.join("host/src/tests/capacity.rs"),
        "#[test] fn capacity() {}\n",
    )
    .unwrap();
    root
}

fn acquire(root: &Path) -> ArtifactCacheOutcome {
    let host = root.join("host");
    let excluded = standalone_test_sources(&host).unwrap();
    let spec = bind_path(
        ArtifactCacheSpec::new(&root.join("cache"), "host-inputs", "canic/host-inputs/v1")
            .with_output("artifact", &root.join("artifact")),
        Path::new("native-host"),
        &host,
        &[],
        &excluded,
    )
    .unwrap();
    match prepare_artifact_cache(&spec).unwrap() {
        ArtifactCachePreparation::Reused(record) => ArtifactCacheOutcome::Reused(record),
        ArtifactCachePreparation::Build(transaction) => {
            fs::write(transaction.output_path("artifact").unwrap(), b"artifact").unwrap();
            transaction.commit().unwrap()
        }
    }
}

#[test]
fn standalone_host_test_edits_reuse_but_producer_changes_invalidate() {
    let root = fixture();
    let first = acquire(&root);
    assert!(!first.is_reused());
    let tests = root.join("host/src/tests/capacity.rs");
    fs::write(&tests, "#[test] fn changed_capacity() {}\n").unwrap();
    let replay = acquire(&root);
    assert!(replay.is_reused());
    assert_eq!(first.record().key(), replay.record().key());
    for (path, contents) in [
        ("src/production.rs", "pub fn value() -> u8 { 2 }\n"),
        ("src/new_producer.rs", "pub fn new_input() {}\n"),
        ("Cargo.toml", "changed manifest"),
        ("src/production-data.bin", "changed producer data"),
    ] {
        fs::write(root.join("host").join(path), contents).unwrap();
        assert!(!acquire(&root).is_reused(), "changed producer input {path}");
    }
    fs::remove_file(root.join("host/src/new_producer.rs")).unwrap();
    assert!(!acquire(&root).is_reused());
    fs::create_dir(root.join("host/empty-input")).unwrap();
    assert!(!acquire(&root).is_reused());
    let long_directory = root
        .join("host/src")
        .join("long_producer_module_".repeat(4))
        .join("nested_producer_module_".repeat(4));
    fs::create_dir_all(&long_directory).unwrap();
    let source = long_directory.join("producer.rs");
    fs::write(&source, "pub fn before() {}\n").unwrap();
    assert!(!acquire(&root).is_reused());
    fs::write(&source, "pub fn after() {}\n").unwrap();
    assert!(!acquire(&root).is_reused());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn embedded_test_source_and_uncertain_module_paths_remain_inputs() {
    let root = fixture();
    for production in [
        "pub const DATA: &str = include_str!(\"tests/capacity.rs\");",
        "#[path = \"tests/capacity.rs\"] mod actual_producer;",
        "pub const DATA: &str = include_str!(env!(\"PRODUCER_SOURCE\"));",
        "include!(\"tests/capacity.rs\");",
        "macro_rules! input { () => { include_str!(\"tests/capacity.rs\") }; }",
        "#[cfg(not(test))] mod tests;",
    ] {
        fs::write(
            root.join("host/src/lib.rs"),
            format!("#[cfg(test)] mod tests;\n{production}\n"),
        )
        .unwrap();
        fs::write(
            root.join("host/src/tests/capacity.rs"),
            "#[test] fn before() {}\n",
        )
        .unwrap();
        let before = acquire(&root);
        fs::write(
            root.join("host/src/tests/capacity.rs"),
            "#[test] fn after() {}\n",
        )
        .unwrap();
        let after = acquire(&root);
        assert_ne!(before.record().key(), after.record().key());
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cfg_classification_only_excludes_impossible_non_test_branches() {
    for (condition, excluded) in [
        ("test", true),
        ("all(test, unix)", true),
        ("any(test, all(test, feature = \"example\"))", true),
        ("any(test, unix)", false),
        ("not(test)", false),
        ("all()", false),
        ("any()", false),
    ] {
        let syntax = syn::parse_file(&format!("#[cfg({condition})] mod tests;")).unwrap();
        let Item::Mod(module) = &syntax.items[0] else {
            unreachable!()
        };
        assert_eq!(requires_test(&module.attrs), excluded, "{condition}");
    }
}

#[test]
fn real_host_policy_tests_are_outside_the_fixture_producer() {
    let workspace = ic_testkit::artifacts::workspace_root_for(env!("CARGO_MANIFEST_DIR"));
    let host = workspace.join("crates/canic-host");
    let excluded = standalone_test_sources(&host).unwrap();
    assert!(excluded.contains(&host.join("src/fleet_ensure/policy/tests.rs")));
    assert!(excluded.contains(&host.join("src/icp/query/tests.rs")));
    assert!(!excluded.contains(&host.join("src/icp/query/mod.rs")));
    for module in ["artifact", "cache"] {
        let directory = host.join("src/canister_build").join(module);
        assert!(excluded.contains(&directory.join("tests.rs")));
        assert!(!excluded.contains(&directory.join("mod.rs")));
    }
}

#[test]
fn excluded_test_edits_still_abort_an_in_flight_cache_transaction() {
    use ic_testkit::artifacts::{WasmBuildSpec, resolve_cargo_build_inputs};
    let root = fixture();
    let host = root.join("host");
    fs::write(host.join("Cargo.toml"), "[package]\nname = \"native-producer\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[workspace]\n").unwrap();
    fs::write(
        host.join("Cargo.lock"),
        "version = 4\n[[package]]\nname = \"native-producer\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let build = WasmBuildSpec::new(&host, &root.join("target"), &["native-producer"], "release");
    let inputs = resolve_cargo_build_inputs(&build).unwrap();
    let cache = FixtureArtifactCache::bind(
        ArtifactCacheSpec::new(&root.join("cache"), "host-guard", "canic/host-guard/v1")
            .with_output("artifact", &root.join("artifact")),
        &host,
        inputs,
    );
    let ArtifactCachePreparation::Build(transaction) = prepare_artifact_cache(&cache.spec).unwrap()
    else {
        panic!("empty cache");
    };
    fs::write(transaction.output_path("artifact").unwrap(), b"candidate").unwrap();
    cache.require_unchanged();
    fs::write(
        host.join("src/tests/capacity.rs"),
        "#[test] fn changed_during_build() {}\n",
    )
    .unwrap();
    assert!(std::panic::catch_unwind(|| cache.require_unchanged()).is_err());
    drop(transaction);
    assert!(matches!(
        prepare_artifact_cache(&cache.spec).unwrap(),
        ArtifactCachePreparation::Build(_)
    ));
    fs::remove_dir_all(root).unwrap();
}
