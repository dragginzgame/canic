//! Controller-only replay discovery against production Root and Coordinator Wasm.

use candid::Principal;
use canic::dto::release_intents::IntentReleaseEntry;
use canic::dto::release_receipts::ReplayReleaseAuthentication;
use canic::dto::release_receipts::ReplayReleaseEffect;
use canic::dto::release_receipts::ReplayReleaseEntry;
use canic::dto::release_receipts::ReplayReleaseIntentState;
use canic::dto::release_receipts::ReplayReleasePhase;
use canic::protocol;
use canic_contracts::dto::wire::projection::release_receipts::Request;
use canic_host::fleet_ensure::ops::release::{
    intents,
    intents::{ReleaseIntentsError, entry_key},
    receipts::{ReleaseReceiptsError, decode_response},
};
use ic_testkit::pic::PocketIc;

/// Read the complete small fixture, proving replay, controller denial and unchanged balance.
pub(super) fn collect(pic: &PocketIc, owner: Principal, method: &str) -> Vec<ReplayReleaseEntry> {
    let read = |caller, cursor| {
        let argument = if method == protocol::CANIC_OBSERVABILITY {
            candid::encode_one(canic_contracts::dto::fleet_coordinator::CoordinatorObservabilityRequest::ReplayRelease(cursor))
        } else {
            candid::encode_one(Request::ReplayRelease(cursor))
        }.unwrap();
        let bytes = pic
            .query_call(owner, caller, method, argument)
            .expect("release receipt query transport");
        let mut remaining = 8 * 1024 * 1024;
        decode_response(owner, &bytes, &mut remaining)
    };
    let _ = collect_intents(pic, owner, method);
    let balance = pic.cycle_balance(owner);
    let mut cursor = None;
    let mut complete = false;
    let mut entries = Vec::new();
    for _ in 0..128 {
        let page = read(Principal::anonymous(), cursor).unwrap();
        assert_eq!(page.owner, owner);
        assert_eq!(read(Principal::anonymous(), cursor).unwrap(), page);
        if let Some(entry) = &page.entry {
            assert!(cursor.is_none_or(|previous| entry.slot > previous));
            entries.push(entry.clone());
        }
        if let Some(next) = page.next_after {
            assert_eq!(page.entry.as_ref().map(|entry| entry.slot), Some(next));
            cursor = Some(next);
        } else {
            complete = true;
            break;
        }
    }
    assert!(complete, "small fixture receipt census must terminate");
    assert!(matches!(read(Principal::from_slice(&[99; 29]), None),
        Err(ReleaseReceiptsError::Rejected { rejection, .. }) if rejection.code() == canic_contracts::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()));
    assert_eq!(pic.cycle_balance(owner), balance);
    entries
}

/// Verify a real paid deposit retains its original caller, target, accounting IDs and result phase.
pub(super) fn assert_funding_receipt(
    pic: &PocketIc,
    root: Principal,
    child: Principal,
    operation: [u8; 32],
) {
    let receipts = collect(pic, root, protocol::CANIC_ROOT_STATUS);
    let matches = receipts
        .iter()
        .filter(|entry| {
            entry.operation_id == operation && entry.command_kind == "root.request_cycles.v1"
        })
        .collect::<Vec<_>>();
    let [entry] = matches.as_slice() else {
        panic!("one original paid child-funding receipt");
    };
    assert_eq!(entry.actor, child);
    assert_eq!(
        entry.authentication,
        ReplayReleaseAuthentication::DirectCaller
    );
    assert_eq!(entry.phase, ReplayReleasePhase::Committed);
    let settlement = entry
        .cost_guard_settlement
        .as_ref()
        .expect("paid accounting identity");
    assert_ne!(settlement.quota_intent_id, settlement.reservation_intent_id);
    let canonical = collect_intents(pic, root, protocol::CANIC_ROOT_STATUS);
    for (id, expected) in [
        (settlement.quota_intent_id, &settlement.quota),
        (settlement.reservation_intent_id, &settlement.reservation),
    ] {
        assert!(canonical.iter().any(|entry| matches!(entry,
            canic::dto::release_intents::IntentReleaseEntry::Local { intent_id, record }
            if *intent_id == id && Some(record) == expected.as_ref()
        )));
    }

    let quota = settlement.quota.as_ref().expect("retained quota");
    let reservation = settlement
        .reservation
        .as_ref()
        .expect("retained cycle reservation");
    assert_eq!(quota.state, ReplayReleaseIntentState::Committed);
    assert_eq!(reservation.state, ReplayReleaseIntentState::Committed);
    assert_eq!(quota.quantity, 1);
    assert!(reservation.quantity > 0);
    assert_eq!(
        entry.effect,
        Some(ReplayReleaseEffect::ManagementCall {
            canister: child,
            method: "deposit_cycles".into(),
        })
    );
}

/// Query both canonical stores on production Wasm and prove controller-only, effect-free replay.
pub(super) fn collect_intents(
    pic: &PocketIc,
    owner: Principal,
    method: &str,
) -> Vec<canic::dto::release_intents::IntentReleaseEntry> {
    use canic_contracts::dto::wire::projection::release_intents::Request as IntentRequest;
    let read = |caller, cursor| {
        let argument = candid::encode_one(IntentRequest::IntentRelease(cursor)).unwrap();
        let bytes = pic
            .query_call(owner, caller, method, argument)
            .expect("canonical accounting query transport");
        intents::decode_response(owner, &bytes, &mut (8 * 1024 * 1024))
    };
    let balance = pic.cycle_balance(owner);
    let mut entries: Vec<IntentReleaseEntry> = Vec::new();
    let mut cursor = None;
    let mut complete = false;
    for _ in 0..256 {
        let page = read(Principal::anonymous(), cursor).unwrap();
        assert_eq!(page.owner, owner);
        assert_eq!(read(Principal::anonymous(), cursor).unwrap(), page);
        if let Some(entry) = page.entry {
            let key = entry_key(&entry);
            assert!(cursor.is_none_or(|previous| previous < key));
            assert!(page.next_after.is_none_or(|next| next == key));
            entries.push(entry);
        }
        cursor = page.next_after;
        if cursor.is_none() {
            complete = true;
            break;
        }
    }
    assert!(complete, "small canonical accounting census must terminate");
    assert!(
        matches!(read(Principal::from_slice(&[99; 29]), None), Err(ReleaseIntentsError::Rejected { rejection, .. }) if rejection.code() == canic_contracts::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code())
    );
    assert_eq!(pic.cycle_balance(owner), balance);
    entries
}
