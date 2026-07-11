use super::*;

// Ensure empty-root command errors explain root registry setup.
#[test]
fn root_registry_hint_explains_empty_root_canister() {
    let hint = root_registry_hint("the canister contains no Wasm module")
        .expect("empty wasm hint should be available");

    assert!(hint.contains("canic install"));
    assert!(hint.contains("no Canic root code is installed"));
}

// Ensure missing-root hints use deployment-target wording, not fleet-state wording.
#[test]
fn root_registry_hint_explains_missing_deployment_root() {
    let hint = root_registry_hint("Cannot find canister id")
        .expect("missing canister hint should be available");

    assert!(hint.contains("deployment target"));
    assert!(hint.contains("canic fleet config <fleet-template>"));
    assert!(!hint.contains("this fleet"));
}

// Ensure a panicked live-query worker cannot silently remove its canister.
#[test]
fn live_query_worker_panic_is_reported_for_its_canister() {
    let registry = vec![RegistryEntry {
        pid: "aaaaa-aa".to_string(),
        role: Some("root".to_string()),
        kind: Some("root".to_string()),
        parent_pid: None,
        module_hash: None,
    }];

    let values =
        collect_visible_entry_values(&registry, None, OBSERVATION_ERROR.to_string(), |_| {
            panic!("simulated query worker panic")
        })
        .expect("visible registry entries should resolve");

    assert_eq!(
        values.get("aaaaa-aa").map(String::as_str),
        Some(OBSERVATION_ERROR)
    );
}
