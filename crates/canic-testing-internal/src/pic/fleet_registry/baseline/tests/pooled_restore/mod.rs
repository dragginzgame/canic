//! Prove the consumer recipe reuses its captured Fleet through Testkit restoration.

use super::*;
use ic_testkit::pic::BaselinePoolDisposition;

#[test]
pub(super) fn pooled_fleet_restores_installed_state_without_rebuilding() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let pool = CachedPocketIcBaselinePool::new(
        NonZeroUsize::new(1).unwrap(),
        ActiveComponentRegistryBaselineRecipe::new(),
    );
    let (first, outcome) = pool.acquire().expect("build captured Fleet");
    assert_eq!(outcome.disposition, BaselinePoolDisposition::Built);
    let metadata = first.metadata().clone();
    let target = metadata.issuer.canister_id;
    let pic = first.pocket_ic();
    let instance = pic.instance_id();
    let initial = pic.canister_status(target, Some(metadata.root)).unwrap();
    assert!(initial.module_hash.is_some());
    pic.uninstall_canister(target, Some(metadata.root)).unwrap();
    assert_eq!(
        pic.canister_status(target, Some(metadata.root))
            .unwrap()
            .module_hash,
        None
    );
    pic.advance_time(Duration::from_secs(60));
    let current_time = pic.get_time();
    drop(first);

    let (restored, outcome) = pool.acquire().expect("restore captured Fleet");
    assert_eq!(outcome.disposition, BaselinePoolDisposition::Restored);
    let pic = restored.pocket_ic();
    assert_eq!(pic.instance_id(), instance);
    assert_eq!(restored.metadata().issuer.canister_id, target);
    assert!(pic.get_time() >= current_time);
    let observed = pic.canister_status(target, Some(metadata.root)).unwrap();
    assert_eq!(observed.module_hash, initial.module_hash);
    assert_eq!(observed.settings.controllers, initial.settings.controllers);
    // Recipe acquisition already checks each role's readiness and Fleet bindings.
    validate_active_component_registry_baseline(&restored).unwrap();
}
