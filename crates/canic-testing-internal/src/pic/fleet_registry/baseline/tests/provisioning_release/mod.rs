//! Verify retained provisioning discovery against the installed production Root.

use candid::Principal;
use canic::protocol;
use canic_contracts::dto::{
    root::{RootProvisioningReleaseKey, RootProvisioningReleasePhase},
    wire::projection::release_provisioning::Request,
};
use ic_testkit::pic::PocketIc;

/// Discover the original operation in both interrupted and completed states without resuming it.
pub(super) fn assert_census(
    pic: &PocketIc,
    root: Principal,
    operation_id: [u8; 32],
    phase: RootProvisioningReleasePhase,
) {
    let expected = super::observed_root_provisioning(pic, root, operation_id).unwrap();
    let balance = pic.cycle_balance(root);
    let read = |caller, cursor| {
        let bytes = pic
            .query_call(
                root,
                caller,
                protocol::CANIC_ROOT_STATUS,
                candid::encode_one(Request::ProvisioningRelease(cursor)).unwrap(),
            )
            .expect("provisioning census transport");
        let mut remaining = 8 * 1024 * 1024;
        canic_host::fleet_ensure::ops::release::provisioning::decode_response(
            root,
            &bytes,
            &mut remaining,
        )
    };
    let mut cursor = None;
    let mut found = false;
    let mut complete = false;
    // This small fixture has a bounded journal; a broken cursor must fail without hanging.
    for _ in 0..16 {
        let page = read(Principal::anonymous(), cursor).unwrap();
        assert_eq!(page.root, root);
        assert_eq!(read(Principal::anonymous(), cursor).unwrap(), page);
        if let Some(entry) = &page.entry
            && entry.key == RootProvisioningReleaseKey::Provisioning(operation_id)
        {
            assert!(!found);
            assert_eq!(entry.plan_hash, expected.plan_hash);
            assert_eq!(entry.phase, phase);
            assert_eq!(entry.last_failure, expected.last_failure);
            assert_eq!(entry.delivery_in_flight, None);
            found = true;
        }
        if let Some(next) = page.next_after {
            assert!(cursor.is_none_or(|previous| next > previous));
            assert_eq!(page.entry.as_ref().map(|entry| entry.key), Some(next));
            cursor = Some(next);
        } else {
            complete = true;
            break;
        }
    }
    assert!(found);
    assert!(complete, "provisioning census must reach its terminal page");
    assert!(matches!(read(Principal::from_slice(&[0x92; 29]), None),
        Err(canic_host::fleet_ensure::ops::release::provisioning::ReleaseProvisioningError::Rejected { rejection, .. })
            if rejection.code() == canic_contracts::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()));
    assert_eq!(pic.cycle_balance(root), balance);
    assert_eq!(
        super::observed_root_provisioning(pic, root, operation_id).unwrap(),
        expected
    );
}
