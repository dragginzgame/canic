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
    assert!(hint.contains("canic config <fleet-template>"));
    assert!(!hint.contains("this fleet"));
}
