//! Qualify exact account selection, bounded replies and signed Ledger queries.

mod pocketic;

use super::*;
use crate::fleet_ensure::policy::release::tests::fixture;

#[test]
fn balances_preserve_zero_and_full_precision_and_refuse_invalid_replies() {
    for value in [0, 1, u128::MAX] {
        assert_eq!(
            decode_balance(&candid::encode_one(Nat::from(value)).unwrap()),
            Ok(value)
        );
    }
    let overflow = Nat::from(u128::MAX) + Nat::from(1_u8);
    assert_eq!(
        decode_balance(&candid::encode_one(overflow).unwrap()),
        Err(ReleaseAccountStage::Overflow)
    );
    for bytes in [
        vec![0; RESPONSE_BYTES + 1],
        b"DIDL".to_vec(),
        candid::encode_one("not a balance").unwrap(),
    ] {
        assert_eq!(decode_balance(&bytes), Err(ReleaseAccountStage::Decode));
    }
    let extra = candid::encode_args((Nat::from(1_u8), vec![1_u8; RESPONSE_BYTES * 2])).unwrap();
    assert_eq!(decode_balance(&extra), Err(ReleaseAccountStage::Decode));
    // Valid Candid with seventeen unused empty-record types must still respect
    // the header work bound even though its actual result is the small Nat 1.
    let mut header = b"DIDL".to_vec();
    header.push(17);
    for _ in 0..17 {
        header.extend([0x6c, 0]);
    }
    header.extend([1, 0x7d, 1]);
    assert_eq!(candid::decode_one::<Nat>(&header).unwrap(), Nat::from(1_u8));
    assert_eq!(decode_balance(&header), Err(ReleaseAccountStage::Decode));
}

#[test]
fn account_selection_rejects_missing_known_accounts_and_uncontrolled_owners() {
    let (review, _) = fixture();
    validate_account_inventory(&review).unwrap();
    let mut wrong = review.clone();
    wrong.accounts.clear();
    assert_eq!(
        validate_account_inventory(&wrong),
        Err(FleetReleaseError::Accounts)
    );
    wrong = review.clone();
    wrong.accounts[0].owner = Principal::anonymous();
    assert_eq!(
        validate_account_inventory(&wrong),
        Err(FleetReleaseError::Accounts)
    );
    wrong = review;
    wrong.accounts[0].recovery_artifact_sha256 = [0; 32];
    assert_eq!(
        validate_account_inventory(&wrong),
        Err(FleetReleaseError::Accounts)
    );
}
