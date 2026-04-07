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

// Parse one shell array assignment from the reference canister roster script.
fn parse_shell_array(source: &str, name: &str) -> Vec<String> {
    let start = source
        .find(&format!("{name}=("))
        .unwrap_or_else(|| panic!("missing shell array `{name}`"));
    let rest = &source[start..];
    let body_start = rest
        .find('(')
        .unwrap_or_else(|| panic!("missing array body for `{name}`"))
        + 1;
    let body_end = rest[body_start..]
        .find(')')
        .unwrap_or_else(|| panic!("unterminated array body for `{name}`"))
        + body_start;

    rest[body_start..body_end]
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.trim_matches('"').to_string())
        .collect()
}

// Read the static reference canister roster from the repo-local shell helper.
fn reference_canisters() -> Vec<String> {
    let path = workspace_root().join("scripts/app/reference_canisters.sh");
    let source = read_text(&path);
    parse_shell_array(&source, "REFERENCE_CANISTERS")
}

// Read the static root release-set roster from the repo-local shell helper.
fn root_release_set_canisters() -> Vec<String> {
    let path = workspace_root().join("scripts/app/reference_canisters.sh");
    let source = read_text(&path);
    parse_shell_array(&source, "ROOT_RELEASE_SET_CANISTERS")
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

// Keep the visible dfx canister list aligned with the explicit reference roster helper.
#[test]
fn dfx_visible_canisters_match_reference_roster() {
    let dfx_keys = dfx_canister_keys().into_iter().collect::<BTreeSet<_>>();
    let reference = reference_canisters().into_iter().collect::<BTreeSet<_>>();

    assert_eq!(
        dfx_keys, reference,
        "dfx.json canister keys must stay aligned with scripts/app/reference_canisters.sh"
    );
}

// Keep the staged root release set aligned with the reference roster minus `root`.
#[test]
fn root_release_set_matches_reference_roster_without_root() {
    let release_set = root_release_set_canisters()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let expected = reference_canisters()
        .into_iter()
        .filter(|name| name != "root")
        .collect::<BTreeSet<_>>();

    assert_eq!(
        release_set, expected,
        "ROOT_RELEASE_SET_CANISTERS must stay aligned with the reference roster minus root"
    );
}
