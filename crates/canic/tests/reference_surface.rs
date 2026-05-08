use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

// Return the repository root so release-surface fixtures can be read from disk.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate directory should have a parent")
        .parent()
        .expect("workspace root should exist")
        .to_path_buf()
}

// Read one checked-in text fixture from disk.
fn read_text(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

// Read the visible canister keys from the checked-in reference dfx topology.
fn dfx_canister_keys() -> Vec<String> {
    let path = workspace_root().join("dfx.json");
    let source = read_text(&path);
    let parsed: Value = serde_json::from_str(&source)
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", path.display()));

    parsed["canisters"]
        .as_object()
        .expect("dfx.json canisters must be an object")
        .keys()
        .cloned()
        .collect()
}

// Read the root-subnet canister keys from the checked-in demo Canic config.
fn demo_root_subnet_canister_keys() -> Vec<String> {
    let path = workspace_root().join("fleets/demo/canic.toml");
    let source = read_text(&path);
    let parsed: toml::Value = toml::from_str(&source)
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", path.display()));

    parsed["subnets"]["prime"]["canisters"]
        .as_table()
        .expect("demo root subnet canisters must be a table")
        .keys()
        .cloned()
        .collect()
}

// Keep the visible dfx canister list aligned with the demo root subnet.
#[test]
fn dfx_visible_canisters_match_demo_root_subnet() {
    let dfx_keys = dfx_canister_keys().into_iter().collect::<BTreeSet<_>>();
    let demo_root_subnet = demo_root_subnet_canister_keys()
        .into_iter()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        dfx_keys, demo_root_subnet,
        "dfx.json canister keys must stay aligned with fleets/demo/canic.toml root subnet"
    );
}

// Keep the staged root release set derivable from the demo root subnet.
#[test]
fn demo_root_subnet_has_derivable_release_set() {
    let release_set = demo_root_subnet_canister_keys()
        .into_iter()
        .filter(|name| name != "root")
        .collect::<BTreeSet<_>>();

    assert!(!release_set.is_empty());
    assert!(!release_set.contains("root"));
}
