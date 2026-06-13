use super::should_export_candid_artifacts;

// Keep public Candid export restricted to local/development environments.
#[test]
fn candid_artifact_export_is_dev_only() {
    assert!(should_export_candid_artifacts("local"));
    assert!(!should_export_candid_artifacts("ic"));
}
