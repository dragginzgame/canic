//! Complete inventory enumeration rejects missing pages, drift and duplicate ownership.

use super::*;
use crate::fleet_ensure::policy::capacity_import::tests::principal;
use canic_core::{
    cdk::types::Cycles,
    dto::pool::{CanisterPoolAsset, CanisterPoolAssetOrigin},
    ids::FleetSubnetCanisterPoolConfig,
};

fn page() -> CanisterPoolResponse {
    CanisterPoolResponse {
        config: FleetSubnetCanisterPoolConfig {
            minimum_size: 0,
            maximum_size: 10,
            canister_cycles: Cycles::new(1_000),
            creation_execution_margin: Cycles::new(100),
        },
        tracked: 2,
        store: 1,
        store_deletion_pending: 0,
        pooled: 1,
        workload: 0,
        surplus: 1,
        ready: 1,
        pending_reset: 0,
        claimed: 0,
        recycling: 0,
        handing_off: 0,
        failed: 0,
        completed_handoffs: 0,
        pending_creation: None,
        pending_handoff: None,
        entries: vec![
            CanisterPoolAsset {
                canister_id: principal(4),
                creation_receipt: None,
                cycles: Cycles::new(1_000),
                origin: CanisterPoolAssetOrigin::InfrastructureStore,
                status: CanisterPoolAssetStatus::Store,
                added_at_ns: 0,
                updated_at_ns: 0,
            },
            CanisterPoolAsset {
                canister_id: principal(5),
                creation_receipt: None,
                cycles: Cycles::new(1_000),
                origin: CanisterPoolAssetOrigin::Imported,
                status: CanisterPoolAssetStatus::Ready,
                added_at_ns: 0,
                updated_at_ns: 0,
            },
        ],
        next_start_after: None,
    }
}

#[test]
fn complete_pages_bind_one_store_and_every_asset() {
    let mut first = page();
    let mut second = first.clone();
    first.entries.pop();
    first.next_start_after = Some(principal(4));
    second.entries.remove(0);
    let mut pages = Pages::new(principal(4), PoolScope::Destination);
    assert!(!pages.push(first).unwrap());
    assert!(pages.push(second).unwrap());
    assert_eq!(pages.finish().unwrap().tracked, 2);
}

#[test]
fn missing_or_repeated_membership_and_summary_drift_reject() {
    let mut missing = page();
    missing.entries.pop();
    let mut pages = Pages::new(principal(4), PoolScope::Destination);
    pages.push(missing).unwrap();
    assert!(matches!(
        pages.finish(),
        Err(CapacityImportJournalError::InventoryInvalid)
    ));
    let mut first = page();
    first.entries.pop();
    first.next_start_after = Some(principal(4));
    let mut pages = Pages::new(principal(4), PoolScope::Destination);
    pages.push(first.clone()).unwrap();
    assert!(matches!(
        pages.push(first.clone()),
        Err(CapacityImportJournalError::InventoryInvalid)
    ));
    let mut pages = Pages::new(principal(4), PoolScope::Destination);
    pages.push(first).unwrap();
    let mut changed = page();
    changed.entries.remove(0);
    changed.completed_handoffs += 1;
    assert!(matches!(
        pages.push(changed),
        Err(CapacityImportJournalError::InventoryInvalid)
    ));
}

#[test]
fn pending_physical_assets_block_host_handoffs() {
    let mut pending = page();
    pending.entries[1].status = CanisterPoolAssetStatus::PendingReset;
    pending.pending_reset = 1;
    pending.ready = 0;
    let mut pages = Pages::new(principal(4), PoolScope::Destination);
    assert!(matches!(
        pages.push(pending),
        Err(CapacityImportJournalError::InventoryInvalid)
    ));
}

#[test]
fn another_roots_pending_reset_still_proves_every_owned_identity() {
    let mut pending = page();
    pending.entries[1].status = CanisterPoolAssetStatus::PendingReset;
    pending.pending_reset = 1;
    pending.ready = 0;
    let mut pages = Pages::new(principal(4), PoolScope::OtherRoot);
    assert!(pages.push(pending).unwrap());
    assert_eq!(pages.finish().unwrap().tracked, 2);
    assert!(pages.seen.contains(&principal(5)));
}

#[test]
fn failed_assets_remain_owned_without_blocking_a_quiet_destination() {
    for scope in [PoolScope::Destination, PoolScope::OtherRoot] {
        let mut failed = page();
        failed.entries[1].status = CanisterPoolAssetStatus::Failed {
            reason: "retained reset failure".into(),
        };
        failed.failed = 1;
        failed.ready = 0;
        let mut pages = Pages::new(principal(4), scope);
        assert!(pages.push(failed).unwrap());
        assert_eq!(pages.finish().unwrap().tracked, 2);
        assert!(pages.seen.contains(&principal(5)));
    }
}
