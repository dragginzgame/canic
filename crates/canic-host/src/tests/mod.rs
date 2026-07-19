use super::should_export_candid_artifacts;
use canic_core::ids::BuildNetwork;

// Keep public Candid export restricted to local/development environments.
#[test]
fn candid_artifact_export_is_dev_only() {
    assert!(should_export_candid_artifacts(BuildNetwork::Local));
    assert!(!should_export_candid_artifacts(BuildNetwork::Ic));
}
