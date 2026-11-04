//! Module: pic::fleet_registry::baseline::tests::timer_ingress
//!
//! Responsibility: qualify private timer ingress refusal across generated roles.
//! Boundary: installed role hooks reject before update execution charges cycles.

use super::*;

#[test]
pub(super) fn private_timer_dispatch_is_refused_by_every_generated_role() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let fixture = setup_fresh_active_component_registry();
    let pic = fixture.pic();
    let targets = [
        ("coordinator", fixture.coordinator, Principal::anonymous()),
        ("root", fixture.root, Principal::anonymous()),
        ("store", fixture.wasm_store, fixture.root),
        ("issuer", fixture.issuer.canister_id, fixture.root),
        ("verifier", fixture.verifier.canister_id, fixture.root),
    ];
    for (role, canister, controller) in targets {
        for caller in [controller, Principal::from_slice(&[0x99; 29])] {
            for payload in [Vec::new(), 0_u64.to_le_bytes().to_vec(), vec![0; 32 * 1024]] {
                let before = pic.cycle_balance(canister).unwrap();
                let error = pic
                    .update_call(canister, caller, "<ic-timers internal> dispatch", payload)
                    .expect_err("private dispatch must be refused during inspection");
                let ic_testkit::pocket_ic::ClientError::Canister(error) = error else {
                    panic!("expected a canister rejection for {role}");
                };
                assert_eq!(
                    error.error_code,
                    ic_testkit::pocket_ic::ErrorCode::CanisterRejectedMessage,
                    "{role}"
                );
                assert_eq!(pic.cycle_balance(canister).unwrap(), before, "{role}");
            }
        }
    }
    wait_for_role_overviews_ready(pic, targets, 60, "after private timer ingress refusal").unwrap();
    drop(fixture);
}
