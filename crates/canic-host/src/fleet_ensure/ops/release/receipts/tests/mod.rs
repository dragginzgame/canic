//! Retained uncertainty, exact pagination and bounded wire decoding.

use super::*;
use canic_core::dto::release_receipts::{
    ReplayReleaseAuthentication, ReplayReleaseEffect, ReplayReleaseEntry, ReplayReleasePhase,
    ReplayReleaseRecoveryReason, ReplayReleaseSettlement,
};

pub(in crate::fleet_ensure::ops::release) fn fixture(owner: Principal) -> ReplayReleaseResponse {
    ReplayReleaseResponse {
        owner,
        entry: Some(ReplayReleaseEntry {
            slot: [1; 32],
            command_kind: "root.request_cycles.v1".into(),
            operation_id: [2; 32],
            actor: Principal::from_slice(&[3]),
            authentication: ReplayReleaseAuthentication::DirectCaller,
            payload_hash_schema_version: 1,
            payload_hash: [4; 32],
            phase: ReplayReleasePhase::RecoveryRequired(
                ReplayReleaseRecoveryReason::CostSettlementFailed,
            ),
            created_at_ns: 1,
            updated_at_ns: 2,
            expires_at_ns: Some(3),
            cost_guard_settlement: Some(ReplayReleaseSettlement {
                quota_intent_id: 5,
                reservation_intent_id: 6,
            }),
            effect: Some(ReplayReleaseEffect::ManagementCall {
                canister: Principal::from_slice(&[7]),
                method: "deposit_cycles".into(),
            }),
        }),
        next_after: None,
    }
}

pub(in crate::fleet_ensure::ops::release) fn wire(page: ReplayReleaseResponse) -> Vec<u8> {
    candid::encode_one(Ok::<_, CanicError>(Response::ReplayRelease(page))).unwrap()
}

#[test]
fn preserves_expired_uncertainty_original_actor_and_accounting_including_terminal_history() {
    let owner = Principal::from_slice(&[1]);
    let mut first = fixture(owner);
    first.next_after = Some(first.entry.as_ref().unwrap().slot);
    let mut terminal = fixture(owner);
    terminal.entry.as_mut().unwrap().slot = [9; 32];
    terminal.entry.as_mut().unwrap().phase = ReplayReleasePhase::Committed;
    let mut pages = Pages::new(owner);
    let mut remaining = MAXIMUM_CENSUS_BYTES;
    for page in [&first, &terminal] {
        let bytes = wire(page.clone());
        let decoded = decode_response(owner, &bytes, &mut remaining).unwrap();
        assert_eq!(&decoded, page);
        assert_eq!(pages.push(decoded).unwrap(), page.next_after.is_none());
    }
    assert_eq!(pages.pages, vec![first, terminal]);
}

#[test]
fn refuses_foreign_owners_and_nonadvancing_missing_or_mismatched_rows_without_partial_admission() {
    let owner = Principal::from_slice(&[1]);
    let mut first = fixture(owner);
    first.next_after = Some(first.entry.as_ref().unwrap().slot);
    let mut cases = Vec::new();
    let mut invalid = fixture(Principal::anonymous());
    cases.push((invalid, ReleaseReceiptsStage::Binding));
    cases.push((fixture(owner), ReleaseReceiptsStage::Pagination));
    invalid = fixture(owner);
    invalid.entry = None;
    cases.push((invalid, ReleaseReceiptsStage::Pagination));
    invalid = fixture(owner);
    invalid.entry.as_mut().unwrap().slot = [9; 32];
    invalid.next_after = Some([8; 32]);
    cases.push((invalid, ReleaseReceiptsStage::Pagination));
    for (invalid, expected) in cases {
        let mut pages = Pages::new(owner);
        assert!(!pages.push(first.clone()).unwrap());
        assert_eq!(pages.push(invalid), Err(expected));
        assert_eq!(pages.pages, vec![first.clone()]);
    }
}

#[test]
fn exact_record_allowance_requires_a_terminal_page() {
    let owner = Principal::from_slice(&[1]);
    let mut pages = Pages::new(owner);
    for index in 0..MAXIMUM_RECEIPTS {
        let mut slot = [0; 32];
        slot[..8].copy_from_slice(&u64::try_from(index).unwrap().to_be_bytes());
        let mut page = fixture(owner);
        page.entry.as_mut().unwrap().slot = slot;
        page.next_after = Some(slot);
        if index == MAXIMUM_RECEIPTS - 1 {
            assert_eq!(pages.push(page.clone()), Err(ReleaseReceiptsStage::Budget));
            page.next_after = None;
            assert!(pages.push(page).unwrap());
        } else {
            assert!(!pages.push(page).unwrap());
        }
    }
    assert_eq!(pages.pages.len(), MAXIMUM_RECEIPTS);
}

#[test]
fn rejects_raw_aggregate_and_skipping_budget_exhaustion() {
    #[derive(CandidType)]
    enum Extended {
        ReplayRelease { owner: Principal, extra: Vec<()> },
    }
    let owner = Principal::from_slice(&[1]);
    let bytes = wire(fixture(owner));
    assert!(matches!(
        decode_response(owner, &bytes, &mut 0),
        Err(ReleaseReceiptsError::Observation {
            stage: ReleaseReceiptsStage::Budget,
            ..
        })
    ));
    let skipped = candid::encode_one(Ok::<_, CanicError>(Extended::ReplayRelease {
        owner,
        extra: vec![(); RESPONSE_BYTES * 8 + 1],
    }))
    .unwrap();
    assert!(skipped.len() < RESPONSE_BYTES);
    assert!(
        candid::decode_one::<Result<Response, CanicError>>(&skipped)
            .unwrap()
            .is_ok()
    );
    for bytes in [b"DIDL".to_vec(), vec![0; RESPONSE_BYTES + 1], skipped] {
        let mut remaining = MAXIMUM_CENSUS_BYTES;
        assert!(matches!(
            decode_response(owner, &bytes, &mut remaining),
            Err(ReleaseReceiptsError::Observation {
                stage: ReleaseReceiptsStage::Decode,
                ..
            })
        ));
    }
}

#[test]
fn distinguishes_empty_inventory_from_typed_refusal() {
    let owner = Principal::from_slice(&[1]);
    let empty = ReplayReleaseResponse {
        owner,
        entry: None,
        next_after: None,
    };
    let mut remaining = MAXIMUM_CENSUS_BYTES;
    assert!(
        Pages::new(owner)
            .push(decode_response(owner, &wire(empty), &mut remaining).unwrap())
            .unwrap()
    );
    let rejected =
        CanicError::from_registered(canic_core::diagnostics::codes::AUTHORITY_UNAVAILABLE);
    let bytes = candid::encode_one(Err::<Response, _>(rejected)).unwrap();
    assert!(
        matches!(decode_response(owner, &bytes, &mut remaining), Err(ReleaseReceiptsError::Rejected { rejection, .. }) if rejection.code() == rejected.code())
    );
}
