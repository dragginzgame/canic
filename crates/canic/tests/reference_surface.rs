use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

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

// Read the visible canister keys from the checked-in ICP project topology.
fn icp_canister_keys() -> Vec<String> {
    let path = workspace_root().join("icp.yaml");
    let source = read_text(&path);

    let mut names = Vec::new();
    let mut in_canisters = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed == "canisters:" {
            in_canisters = true;
            continue;
        }
        if in_canisters && !line.starts_with(' ') && !trimmed.is_empty() {
            break;
        }
        if in_canisters && let Some(name) = trimmed.strip_prefix("- name: ") {
            names.push(name.to_string());
        }
    }
    assert!(!names.is_empty(), "icp.yaml must define canisters");
    names
}

// Read the root-subnet canister keys from the checked-in test Canic config.
fn test_root_subnet_canister_keys() -> Vec<String> {
    let path = workspace_root().join("fleets/test/canic.toml");
    let source = read_text(&path);
    let parsed: toml::Value = toml::from_str(&source)
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", path.display()));

    parsed["subnets"]["prime"]["canisters"]
        .as_table()
        .expect("test root subnet canisters must be a table")
        .keys()
        .cloned()
        .collect()
}

// Keep the visible ICP canister list aligned with the test root subnet.
#[test]
fn icp_visible_canisters_match_test_root_subnet() {
    let icp_keys = icp_canister_keys().into_iter().collect::<BTreeSet<_>>();
    let test_root_subnet = test_root_subnet_canister_keys()
        .into_iter()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        icp_keys, test_root_subnet,
        "icp.yaml canister keys must stay aligned with fleets/test/canic.toml root subnet"
    );
}

// Keep the staged root release set derivable from the test root subnet.
#[test]
fn test_root_subnet_has_derivable_release_set() {
    let release_set = test_root_subnet_canister_keys()
        .into_iter()
        .filter(|name| name != "root")
        .collect::<BTreeSet<_>>();

    assert!(!release_set.is_empty());
    assert!(!release_set.contains("root"));
}
