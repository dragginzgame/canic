//! Qualify snapshot reuse through the Root topology's production test recipe.

use canic_tests::root::{RootSetupProfile, harness::setup_root};
use std::time::Duration;

#[test]
fn root_topology_restores_the_same_instance_and_current_time() {
    let first = setup_root(RootSetupProfile::Capability);
    let root = first.root_id;
    let coordinator = first.coordinator_id;
    let coordinator_initial = first.pic.canister_status(coordinator, None).unwrap();
    assert!(coordinator_initial.module_hash.is_some());
    first.pic.uninstall_canister(coordinator, None).unwrap();
    let instance = first.pic.instance_id();
    let initial = first.pic.canister_status(root, None).unwrap();
    assert!(initial.module_hash.is_some());
    first.pic.uninstall_canister(root, None).unwrap();
    assert_eq!(
        first.pic.canister_status(root, None).unwrap().module_hash,
        None
    );
    first.pic.advance_time(Duration::from_secs(60));
    let current_time = first.pic.get_time();
    drop(first);

    let restored = setup_root(RootSetupProfile::Capability);
    assert_eq!(restored.pic.instance_id(), instance);
    assert_eq!(restored.root_id, root);
    assert!(restored.pic.get_time() >= current_time);
    assert_eq!(restored.coordinator_id, coordinator);
    let coordinator_restored = restored.pic.canister_status(coordinator, None).unwrap();
    assert_eq!(
        coordinator_restored.module_hash,
        coordinator_initial.module_hash
    );
    assert_eq!(
        coordinator_restored.settings.controllers,
        coordinator_initial.settings.controllers
    );
    let observed = restored.pic.canister_status(root, None).unwrap();
    drop(restored);
    assert_eq!(observed.module_hash, initial.module_hash);
    assert_eq!(observed.settings.controllers, initial.settings.controllers);
    // Acquisition drives bootstrap, child readiness and captured topology checks.
}
