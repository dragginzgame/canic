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

// Read one environment's canister keys from the checked-in ICP project topology.
fn icp_environment_canister_keys(environment: &str) -> Vec<String> {
    let path = workspace_root().join("icp.yaml");
    let source = read_text(&path);

    let mut in_environments = false;
    let mut in_target_environment = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed == "environments:" {
            in_environments = true;
            continue;
        }
        if in_environments && !line.starts_with(' ') && !trimmed.is_empty() {
            break;
        }
        if !in_environments {
            continue;
        }
        if let Some(name) = trimmed.strip_prefix("- name: ") {
            in_target_environment = name == environment;
            continue;
        }
        if in_target_environment
            && let Some(names) = trimmed
                .strip_prefix("canisters: [")
                .and_then(|names| names.strip_suffix(']'))
        {
            return names
                .split(',')
                .map(str::trim)
                .map(str::to_string)
                .collect();
        }
    }
    panic!("icp.yaml must define canisters for environment `{environment}`");
}

// Read the deployable flat Component topology from the checked-in test config.
fn test_component_topology_canister_keys() -> Vec<String> {
    let path = workspace_root().join("apps/test/canic.toml");
    let source = read_text(&path);
    let parsed: toml::Value = toml::from_str(&source)
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", path.display()));

    let mut roles = vec!["root".to_string()];
    let component_specs = parsed["component_specs"]
        .as_table()
        .expect("test Component Specs must be a table");
    for component_spec in component_specs.values() {
        roles.push(
            component_spec["component_role"]
                .as_str()
                .expect("Component Spec role")
                .to_string(),
        );
        if let Some(children) = component_spec
            .get("children")
            .and_then(toml::Value::as_table)
        {
            roles.extend(children.keys().cloned());
        }
    }
    roles
}

// Keep the ICP test environment aligned with the test Component topology.
#[test]
fn icp_test_environment_canisters_match_test_component_topology() {
    let icp_keys = icp_environment_canister_keys("test")
        .into_iter()
        .collect::<BTreeSet<_>>();
    let test_component_topology = test_component_topology_canister_keys()
        .into_iter()
        .collect::<BTreeSet<_>>();

    assert_eq!(
        icp_keys, test_component_topology,
        "icp.yaml test-environment canisters must stay aligned with apps/test/canic.toml Component topology"
    );
}

// Keep the staged root release set derivable from the test Component topology.
#[test]
fn test_component_topology_has_derivable_release_set() {
    let release_set = test_component_topology_canister_keys()
        .into_iter()
        .filter(|name| name != "root")
        .collect::<BTreeSet<_>>();

    assert!(!release_set.is_empty());
    assert!(!release_set.contains("root"));
}

// Generated consumers must resolve every maintained CDK export through the facade.
#[test]
#[expect(
    unused_imports,
    reason = "the compiler resolves the generated-consumer export inventory without calling IC APIs"
)]
fn hidden_macro_cdk_exports_compile() {
    use canic::__internal::cdk::api::{
        canister_cycle_balance as _, canister_version as _, is_controller as _, msg_caller as _,
        msg_reply as _, time as _,
    };
    use canic::__internal::cdk::raw::{msg_arg_data_copy as _, msg_arg_data_size as _};
    use canic::__internal::cdk::{
        Principal as _, export_candid as _, futures as _, init as _, inspect_message as _,
        post_upgrade as _, query as _, trap as _, update as _,
    };
}
