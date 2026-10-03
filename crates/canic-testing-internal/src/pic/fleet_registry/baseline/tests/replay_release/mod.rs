//! Controller-only replay discovery against production Root and Coordinator Wasm.

use candid::{CandidType, Principal};
use canic::{
    dto::release_receipts::{
        ReplayReleaseAuthentication, ReplayReleaseEffect, ReplayReleaseEntry, ReplayReleasePhase,
    },
    protocol,
};
use canic_host::fleet_ensure::ops::release::receipts::{ReleaseReceiptsError, decode_response};
use ic_testkit::pic::PocketIc;

#[derive(CandidType)]
enum Request {
    ReplayRelease(Option<[u8; 32]>),
}

/// Read the complete small fixture, proving replay, controller denial and unchanged balance.
pub(super) fn collect(pic: &PocketIc, owner: Principal, method: &str) -> Vec<ReplayReleaseEntry> {
    let read = |caller, cursor| {
        let argument = if method == protocol::CANIC_OBSERVABILITY {
            candid::encode_one(canic_control_plane::dto::fleet_coordinator::CoordinatorObservabilityRequest::ReplayRelease(cursor))
        } else {
            candid::encode_one(Request::ReplayRelease(cursor))
        }.unwrap();
        let bytes = pic
            .query_call(owner, caller, method, argument)
            .expect("release receipt query transport");
        let mut remaining = 8 * 1024 * 1024;
        decode_response(owner, &bytes, &mut remaining)
    };
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
        Err(ReleaseReceiptsError::Rejected { rejection, .. }) if rejection.code() == canic_core::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()));
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
    assert!(entry.cost_guard_settlement.is_some());
    assert_eq!(
        entry.effect,
        Some(ReplayReleaseEffect::ManagementCall {
            canister: child,
            method: "deposit_cycles".into(),
        })
    );
}
