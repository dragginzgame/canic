use super::*;
use crate::ops::canister_pool::CanisterPoolClaimKey;
use canic_core::ids::ComponentInstanceId;
use std::borrow::Cow;
use std::{
    future::Future,
    task::{Context, Poll, Waker},
};

pub(super) fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 10])
}

pub(super) fn config() -> FleetSubnetCanisterPoolConfig {
    FleetSubnetCanisterPoolConfig {
        minimum_size: 1,
        maximum_size: 8,
        canister_cycles: Cycles::new(500),
        creation_execution_margin: Cycles::new(10),
    }
}

pub(super) fn reservation() -> PoolImportReservation {
    PoolImportReservation {
        sequence: 0,
        plan_sha256: [1; 32],
        root_authority_sha256: [2; 32],
        root: principal(1),
        operator: principal(2),
        subnet: principal(3),
        transitional_controllers: vec![principal(1), principal(2), principal(4)],
        final_controllers: vec![principal(1), principal(4)],
        sources: vec![PoolImportSource {
            canister_id: principal(8),
            controllers: vec![principal(2)],
            module_sha256: Some([7; 32]),
            canister_version: 10,
            stopped: true,
            disposition_sha256: [9; 32],
            observed_cycles: 1_000,
            observed_reserved_cycles: 100,
            minimum_ready_cycles: 500,
            maximum_debit_cycles: 200,
        }],
        observed_root_cycles: 10_000,
        observed_root_reserved_cycles: 1_000,
        minimum_root_cycles: 5_000,
        maximum_root_debit_cycles: 1_000,
        maximum_paid_calls: 10,
    }
}

pub(super) fn identity() -> PoolImportIdentity {
    PoolImportIdentity {
        sequence: 0,
        plan_sha256: [1; 32],
    }
}

fn observed(delta: u64) -> PoolImportObservationView {
    let request = reservation();
    PoolImportObservationView {
        canister_id: request.sources[0].canister_id,
        canister_version: 10 + delta,
        stopped: true,
        snapshots_size_bytes: 0,
        module_sha256: if delta == 3 { None } else { Some([7; 32]) },
        controllers: if delta == 1 {
            request.transitional_controllers
        } else {
            request.final_controllers
        },
        cycles: 1_000 - u128::from(delta) * 10,
        reserved_cycles: 100,
    }
}

fn start() {
    CanisterPoolStore::clear();
    CanisterPoolImportOps::reserve(reservation(), &config(), 1).unwrap();
}

#[test]
fn root_owned_running_source_records_stop_and_recovers_without_duplicate_effects() {
    CanisterPoolStore::clear();
    let mut request = reservation();
    request.sources[0].controllers = vec![request.root];
    request.sources[0].stopped = false;
    CanisterPoolImportOps::reserve(request.clone(), &config(), 1).unwrap();
    let mut current = observed(1);
    current.canister_version = 10;
    current.controllers = vec![request.root];
    current.stopped = false;
    // Running application messages may advance the version before the reviewed stop.
    current.canister_version = 19;
    assert!(CanisterPoolImportOps::issue_controllers(identity(), &current, budget(), 2).is_err());
    CanisterPoolImportOps::issue_stop(identity(), &current, budget(), 2).unwrap();
    let bytes = CanisterPoolStore::state().to_bytes().into_owned();
    CanisterPoolStore::set_state(CanisterPoolStateRecord::from_bytes(Cow::Owned(bytes)));
    let before = CanisterPoolStore::state();
    assert!(CanisterPoolImportOps::issue_stop(identity(), &current, budget(), 2).is_err());
    assert!(CanisterPoolImportOps::observe_stopped(identity(), &current).is_err());
    assert_eq!(CanisterPoolStore::state(), before);
    current.stopped = true;
    current.canister_version = 21;
    CanisterPoolImportOps::observe_stopped(identity(), &current).unwrap();
    assert!(CanisterPoolImportOps::issue_uninstall(identity(), &current, budget()).is_err());
    CanisterPoolImportOps::issue_controllers(identity(), &current, budget(), 2).unwrap();
    current.canister_version = 22;
    current.controllers = request.final_controllers;
    CanisterPoolImportOps::observe_controllers(
        identity(),
        &current,
        &history(&current, PoolImportHistoryKind::Controllers),
    )
    .unwrap();
    CanisterPoolImportOps::issue_uninstall(identity(), &current, budget()).unwrap();
    current.canister_version = 23;
    current.module_sha256 = None;
    let receipt = CanisterPoolImportOps::observe_cleared(
        identity(),
        &current,
        &history(&current, PoolImportHistoryKind::Uninstall),
        3,
    )
    .unwrap();
    let terminal = CanisterPoolStore::state();
    assert_eq!(
        CanisterPoolImportOps::observe_cleared(
            identity(),
            &current,
            &history(&current, PoolImportHistoryKind::Uninstall),
            4
        )
        .unwrap(),
        receipt
    );
    assert_eq!(CanisterPoolStore::state(), terminal);
    assert_eq!(receipt.canister_version, 23);
    assert_eq!(receipt.before_uninstall_canister_version, 22);
    assert_eq!(
        receipt.retained_cycles + receipt.retained_reserved_cycles + receipt.observed_debit_cycles,
        1_100
    );
}

pub(super) fn finish() -> PoolImportSourceReceipt {
    CanisterPoolImportOps::issue_controllers(identity(), &observed(1), budget(), 2).unwrap();
    CanisterPoolImportOps::observe_controllers(
        identity(),
        &observed(2),
        &history(&observed(2), PoolImportHistoryKind::Controllers),
    )
    .unwrap();
    CanisterPoolImportOps::issue_uninstall(identity(), &observed(2), budget()).unwrap();
    CanisterPoolImportOps::observe_cleared(
        identity(),
        &observed(3),
        &history(&observed(3), PoolImportHistoryKind::Uninstall),
        2,
    )
    .unwrap()
}

#[test]
fn capacity_import_root_reservation_fences_all_pool_mutations() {
    start();
    let id = observed(1).canister_id;
    let before = CanisterPoolStore::state();
    assert_eq!(CanisterPoolImportOps::next_sequence(), 1);
    assert!(CanisterPoolOps::has_pending_lifecycle_work());
    assert!(RootComponentProvisioningOps::require_ordinary_allocation_open().is_err());
    assert!(CanisterPoolOps::initialize_imports(&config(), &[principal(9)], 2).is_err());
    assert!(CanisterPoolOps::prepare_import_reinspection(id, &Cycles::new(500), 2).is_err());
    assert!(CanisterPoolOps::mark_ready(id, Cycles::new(990), 2).is_err());
    assert!(CanisterPoolOps::mark_underfunded(id, Cycles::new(100), "deficit".into(), 2).is_err());
    assert!(CanisterPoolOps::retry_reset(id, &Cycles::new(500), 2).is_err());
    assert!(CanisterPoolImportOps::release(identity(), [5; 32]).is_err());
    assert_eq!(CanisterPoolStore::state(), before);
    assert_eq!(CanisterPoolOps::non_store_asset_count(), 0);
    let mut reserved_capacity = config();
    reserved_capacity.maximum_size = 1;
    assert!(CanisterPoolOps::asset_capacity_is_exhausted(
        &reserved_capacity
    ));
}

#[test]
fn capacity_import_root_lost_reply_reconciles_once_and_keeps_original_receipt() {
    start();
    CanisterPoolImportOps::issue_controllers(identity(), &observed(1), budget(), 2).unwrap();
    let bytes = CanisterPoolStore::state().to_bytes().into_owned();
    CanisterPoolStore::set_state(CanisterPoolStateRecord::from_bytes(Cow::Owned(bytes)));
    assert!(
        CanisterPoolImportOps::issue_controllers(identity(), &observed(1), budget(), 2).is_err()
    );
    assert!(
        CanisterPoolImportOps::observe_controllers(
            identity(),
            &observed(1),
            &history(&observed(1), PoolImportHistoryKind::Controllers)
        )
        .is_err()
    );
    CanisterPoolImportOps::observe_controllers(
        identity(),
        &observed(2),
        &history(&observed(2), PoolImportHistoryKind::Controllers),
    )
    .unwrap();
    CanisterPoolImportOps::issue_uninstall(identity(), &observed(2), budget()).unwrap();
    assert!(CanisterPoolImportOps::issue_uninstall(identity(), &observed(2), budget()).is_err());
    assert!(
        CanisterPoolImportOps::observe_cleared(
            identity(),
            &observed(2),
            &history(&observed(2), PoolImportHistoryKind::Uninstall),
            2
        )
        .is_err()
    );
    let receipt = CanisterPoolImportOps::observe_cleared(
        identity(),
        &observed(3),
        &history(&observed(3), PoolImportHistoryKind::Uninstall),
        2,
    )
    .unwrap();
    assert_eq!(receipt.observed_debit_cycles, 30);
    assert_eq!(
        CanisterPoolImportOps::status(identity()).unwrap().phase,
        PoolImportPhase::Ready
    );
    let claim = CanisterPoolClaimKey {
        component: ComponentInstanceId::from_generated_bytes([1; 32]),
        operation_id: [5; 32],
    };
    assert!(
        CanisterPoolOps::claim_smallest_sufficient_ready(&claim, &Cycles::new(500), 3).is_err()
    );
    CanisterPoolImportOps::settle(identity(), 9_980, 990).unwrap();
    let terminal = CanisterPoolImportOps::release(identity(), [5; 32]).unwrap();
    assert_eq!(
        CanisterPoolOps::claim_smallest_sufficient_ready(&claim, &Cycles::new(500), 4).unwrap(),
        Some(receipt.canister_id)
    );
    CanisterPoolOps::finalize_claim(&claim, receipt.canister_id, 5).unwrap();
    let workload = CanisterPoolStore::get(&receipt.canister_id).unwrap();
    assert_eq!(
        CanisterPoolImportOps::release(identity(), [5; 32]).unwrap(),
        terminal
    );
    assert_eq!(
        CanisterPoolImportOps::reserve(reservation(), &config(), 6).unwrap(),
        terminal
    );
    assert_eq!(
        CanisterPoolImportOps::observe_cleared(
            identity(),
            &observed(3),
            &history(&observed(3), PoolImportHistoryKind::Uninstall),
            6
        )
        .unwrap(),
        receipt
    );
    assert_eq!(
        CanisterPoolStore::get(&receipt.canister_id).unwrap(),
        workload
    );
    assert!(CanisterPoolImportOps::release(identity(), [6; 32]).is_err());
}

#[test]
fn capacity_import_root_rejects_changed_evidence_without_advancing_intent() {
    start();
    let before = CanisterPoolStore::state();
    let mut changed = observed(1);
    changed.module_sha256 = None;
    assert!(CanisterPoolImportOps::issue_controllers(identity(), &changed, budget(), 2).is_err());
    changed = observed(1);
    changed.canister_version += 1;
    assert!(CanisterPoolImportOps::issue_controllers(identity(), &changed, budget(), 2).is_err());
    changed = observed(1);
    changed.controllers.push(principal(9));
    assert!(CanisterPoolImportOps::issue_controllers(identity(), &changed, budget(), 2).is_err());
    changed = observed(1);
    changed.cycles = 799;
    assert!(CanisterPoolImportOps::issue_controllers(identity(), &changed, budget(), 2).is_err());
    assert_eq!(CanisterPoolStore::state(), before);
}

#[test]
fn capacity_import_root_paid_attempts_survive_replay_without_budget_rebase() {
    start();
    CanisterPoolImportOps::reserve_paid_call(identity(), 600, 9_950).unwrap();
    let before = CanisterPoolStore::state();
    CanisterPoolImportOps::reserve(reservation(), &config(), 2).unwrap();
    assert_eq!(CanisterPoolStore::state(), before);
    assert!(CanisterPoolImportOps::reserve_paid_call(identity(), 301, 9_900).is_err());
    assert_eq!(CanisterPoolStore::state(), before);
    CanisterPoolImportOps::reserve_paid_call(identity(), 300, 9_900).unwrap();
    assert!(CanisterPoolImportOps::reserve_paid_call(identity(), 1, 9_899).is_err());
}

#[test]
fn capacity_import_root_monotonic_identity_rejects_reused_old_plan() {
    start();
    finish();
    CanisterPoolImportOps::settle(identity(), 9_980, 990).unwrap();
    CanisterPoolImportOps::release(identity(), [5; 32]).unwrap();
    let mut next = reservation();
    next.sequence = 1;
    next.plan_sha256 = [2; 32];
    next.sources[0].canister_id = principal(9);
    CanisterPoolImportOps::reserve(next, &config(), 3).unwrap();
    let before = CanisterPoolStore::state();
    assert!(CanisterPoolImportOps::reserve(reservation(), &config(), 4).is_err());
    assert!(
        CanisterPoolImportOps::issue_controllers(identity(), &observed(1), budget(), 2).is_err()
    );
    assert_eq!(CanisterPoolStore::state(), before);
}

#[test]
fn capacity_import_root_admission_rejects_whole_conflicting_or_overfull_batch() {
    CanisterPoolStore::clear();
    let mut request = reservation();
    let mut source = request.sources[0].clone();
    source.canister_id = principal(9);
    request.sources.push(source);
    CanisterPoolOps::initialize_store(principal(9), 1).unwrap();
    assert!(CanisterPoolImportOps::reserve(request.clone(), &config(), 2).is_err());
    assert!(CanisterPoolStore::get(&principal(8)).is_none());
    assert_eq!(CanisterPoolImportOps::next_sequence(), 0);
    CanisterPoolStore::clear();
    let mut small = config();
    small.maximum_size = 1;
    assert!(CanisterPoolImportOps::reserve(request, &small, 2).is_err());
    assert_eq!(CanisterPoolOps::non_store_asset_count(), 0);
    assert_eq!(CanisterPoolImportOps::next_sequence(), 0);
}

#[test]
fn capacity_import_root_partial_sources_hold_the_fence_until_every_receipt_exists() {
    CanisterPoolStore::clear();
    let mut request = reservation();
    let mut source = request.sources[0].clone();
    source.canister_id = principal(9);
    request.sources.push(source);
    CanisterPoolImportOps::reserve(request, &config(), 1).unwrap();
    finish();
    assert!(CanisterPoolImportOps::release(identity(), [5; 32]).is_err());
    let mut first = observed(1);
    first.canister_id = principal(9);
    let mut second = observed(2);
    second.canister_id = principal(9);
    let mut third = observed(3);
    third.canister_id = principal(9);
    CanisterPoolImportOps::issue_controllers(identity(), &first, budget(), 2).unwrap();
    CanisterPoolImportOps::observe_controllers(
        identity(),
        &second,
        &history(&second, PoolImportHistoryKind::Controllers),
    )
    .unwrap();
    CanisterPoolImportOps::issue_uninstall(identity(), &second, budget()).unwrap();
    CanisterPoolImportOps::observe_cleared(
        identity(),
        &third,
        &history(&third, PoolImportHistoryKind::Uninstall),
        2,
    )
    .unwrap();
    CanisterPoolImportOps::settle(identity(), 9_980, 990).unwrap();
    CanisterPoolImportOps::release(identity(), [5; 32]).unwrap();
    assert!(!CanisterPoolImportOps::is_active());
}

fn budget() -> PoolImportCallBudgetView {
    PoolImportCallBudgetView {
        sender_canister_version: 100,
        maximum_debit_cycles: 10,
        observed_root_cycles: 10_000,
    }
}

#[test]
fn capacity_import_root_refuses_running_code_and_preserves_exhausted_effect_intent() {
    CanisterPoolStore::clear();
    let mut request = reservation();
    request.sources[0].stopped = false;
    assert!(CanisterPoolImportOps::reserve(request, &config(), 1).is_err());
    assert_eq!(CanisterPoolImportOps::next_sequence(), 0);
    let mut request = reservation();
    request.maximum_paid_calls = 1;
    CanisterPoolImportOps::reserve(request, &config(), 1).unwrap();
    CanisterPoolImportOps::issue_controllers(identity(), &observed(1), budget(), 2).unwrap();
    CanisterPoolImportOps::observe_controllers(
        identity(),
        &observed(2),
        &history(&observed(2), PoolImportHistoryKind::Controllers),
    )
    .unwrap();
    let before = CanisterPoolStore::state();
    assert!(CanisterPoolImportOps::issue_uninstall(identity(), &observed(2), budget()).is_err());
    assert_eq!(CanisterPoolStore::state(), before);
}

#[test]
fn capacity_import_root_largest_reservation_fits_terminal_evidence_and_rejects_omission() {
    CanisterPoolStore::clear();
    let mut request = reservation();
    let mut large_config = config();
    large_config.maximum_size = u32::try_from(MAX_FLEET_CAPACITY_IMPORT_SOURCES).unwrap();
    let template = request.sources[0].clone();
    request.sources = (0..MAX_FLEET_CAPACITY_IMPORT_SOURCES)
        .map(|index| {
            let mut source = template.clone();
            let mut bytes = [255; 29];
            bytes[..2].copy_from_slice(&u16::try_from(index).unwrap().to_be_bytes());
            source.canister_id = Principal::from_slice(&bytes);
            source
        })
        .collect();
    request.sources.sort_by_key(|source| source.canister_id);
    CanisterPoolImportOps::reserve(request, &large_config, 1).unwrap();
    let state = CanisterPoolStore::state();
    require_completion_fits(&state).unwrap();
    assert!(state.to_bytes().len() < CANISTER_POOL_STATE_MAX_BYTES as usize);
    let mut encoded = serde_json::to_value(&state).unwrap();
    encoded.as_object_mut().unwrap().remove("capacity_import");
    assert!(serde_json::from_value::<CanisterPoolStateRecord>(encoded).is_err());
}

fn history(
    observed: &PoolImportObservationView,
    kind: PoolImportHistoryKind,
) -> PoolImportHistoryView {
    PoolImportHistoryView {
        canister_id: observed.canister_id,
        canister_version: observed.canister_version,
        module_sha256: observed.module_sha256,
        controllers: observed.controllers.clone(),
        originator: reservation().root,
        sender_canister_version: 100,
        kind,
    }
}

#[test]
fn capacity_import_root_cannot_release_a_matching_wipe_from_another_controller_or_attempt() {
    start();
    CanisterPoolImportOps::issue_controllers(identity(), &observed(1), budget(), 2).unwrap();
    CanisterPoolImportOps::observe_controllers(
        identity(),
        &observed(2),
        &history(&observed(2), PoolImportHistoryKind::Controllers),
    )
    .unwrap();
    CanisterPoolImportOps::issue_uninstall(identity(), &observed(2), budget()).unwrap();
    let before = CanisterPoolStore::state();
    let mut wrong = history(&observed(3), PoolImportHistoryKind::Uninstall);
    wrong.originator = principal(4);
    assert!(CanisterPoolImportOps::observe_cleared(identity(), &observed(3), &wrong, 2).is_err());
    wrong.originator = reservation().root;
    wrong.sender_canister_version += 1;
    assert!(CanisterPoolImportOps::observe_cleared(identity(), &observed(3), &wrong, 2).is_err());
    wrong.sender_canister_version -= 1;
    wrong.kind = PoolImportHistoryKind::Controllers;
    assert!(CanisterPoolImportOps::observe_cleared(identity(), &observed(3), &wrong, 2).is_err());
    assert_eq!(CanisterPoolStore::state(), before);
    assert!(CanisterPoolImportOps::is_active());
    assert!(CanisterPoolImportOps::release(identity(), [5; 32]).is_err());
}

#[test]
fn capacity_import_root_rechecks_snapshots_before_custody_and_ready() {
    start();
    let before = CanisterPoolStore::state();
    let mut with_snapshot = observed(1);
    with_snapshot.snapshots_size_bytes = 1;
    assert!(
        CanisterPoolImportOps::issue_controllers(identity(), &with_snapshot, budget(), 2).is_err()
    );
    assert_eq!(CanisterPoolStore::state(), before);
    assert!(CanisterPoolStore::get(&with_snapshot.canister_id).is_none());
    CanisterPoolImportOps::issue_controllers(identity(), &observed(1), budget(), 2).unwrap();
    CanisterPoolImportOps::observe_controllers(
        identity(),
        &observed(2),
        &history(&observed(2), PoolImportHistoryKind::Controllers),
    )
    .unwrap();
    CanisterPoolImportOps::issue_uninstall(identity(), &observed(2), budget()).unwrap();
    let before = CanisterPoolStore::state();
    with_snapshot = observed(3);
    with_snapshot.snapshots_size_bytes = 1;
    assert!(
        CanisterPoolImportOps::observe_cleared(
            identity(),
            &with_snapshot,
            &history(&with_snapshot, PoolImportHistoryKind::Uninstall),
            3
        )
        .is_err()
    );
    assert_eq!(CanisterPoolStore::state(), before);
    assert!(CanisterPoolImportOps::is_active());
}

#[test]
fn capacity_import_root_retains_reserve_and_accounts_its_debit() {
    start();
    let first = observed(1);
    CanisterPoolImportOps::issue_controllers(identity(), &first, budget(), 2).unwrap();
    let mut second = observed(2);
    second.reserved_cycles += 10;
    CanisterPoolImportOps::observe_controllers(
        identity(),
        &second,
        &history(&second, PoolImportHistoryKind::Controllers),
    )
    .unwrap();
    CanisterPoolImportOps::issue_uninstall(identity(), &second, budget()).unwrap();
    let mut third = observed(3);
    third.reserved_cycles = 110;
    let receipt = CanisterPoolImportOps::observe_cleared(
        identity(),
        &third,
        &history(&third, PoolImportHistoryKind::Uninstall),
        3,
    )
    .unwrap();
    assert_eq!(receipt.retained_cycles, 970);
    assert_eq!(receipt.retained_reserved_cycles, 110);
    assert_eq!(receipt.observed_debit_cycles, 20);
    assert_eq!(
        receipt.retained_cycles + receipt.retained_reserved_cycles + receipt.observed_debit_cycles,
        1_100
    );
}

#[test]
fn capacity_import_root_requires_its_terminal_cycle_receipt_before_releasing_capacity() {
    start();
    assert!(CanisterPoolImportOps::settle(identity(), 9_980, 990).is_err());
    finish();
    assert!(CanisterPoolImportOps::release(identity(), [5; 32]).is_err());
    let before = CanisterPoolStore::state();
    assert!(CanisterPoolImportOps::settle(identity(), 9_000, 900).is_err());
    assert_eq!(CanisterPoolStore::state(), before);
    let completed = CanisterPoolImportOps::settle(identity(), 9_980, 990).unwrap();
    let receipt = completed.root_receipt.as_ref().unwrap();
    assert_eq!(receipt.observed_debit_cycles, 30);
    assert_eq!(
        receipt.retained_cycles + receipt.retained_reserved_cycles + receipt.observed_debit_cycles,
        11_000
    );
    assert_eq!(
        CanisterPoolImportOps::settle(identity(), 0, 0).unwrap(),
        completed
    );
    assert!(CanisterPoolImportOps::reserve_paid_call(identity(), 1, 9_979).is_err());
    CanisterPoolImportOps::release(identity(), [5; 32]).unwrap();
}

#[test]
fn capacity_import_root_execution_refunds_do_not_rebase_the_reserved_debit() {
    start();
    CanisterPoolImportOps::reserve_paid_call(identity(), 100, 9_900).unwrap();
    CanisterPoolImportOps::reserve_paid_call(identity(), 100, 9_950).unwrap();
    let record = CanisterPoolImportOps::status(identity()).unwrap();
    assert_eq!(record.reserved_debit_cycles, 300);
    assert_eq!(record.paid_calls, 2);
    assert!(CanisterPoolImportOps::reserve_paid_call(identity(), 701, 9_990).is_err());
    assert_eq!(CanisterPoolImportOps::status(identity()).unwrap(), record);
}

#[test]
fn capacity_import_paid_execution_excludes_competing_calls_until_completion() {
    start();
    let running = CanisterPoolImportOps::claim_execution(identity()).unwrap();
    let before = CanisterPoolStore::state();
    for _ in 0..2 {
        assert_eq!(
            CanisterPoolImportOps::claim_execution(identity())
                .unwrap_err()
                .public_error(),
            InternalError::unavailable().public_error()
        );
    }
    assert_eq!(CanisterPoolStore::state(), before);
    drop(running);
    let _next = CanisterPoolImportOps::claim_execution(identity()).unwrap();
    assert_eq!(CanisterPoolStore::state(), before);
}

#[test]
fn capacity_import_cancelled_execution_keeps_issued_intent_and_spent_allowance() {
    start();
    let mut pending = Box::pin(async {
        let _execution = CanisterPoolImportOps::claim_execution(identity()).unwrap();
        CanisterPoolImportOps::issue_controllers(identity(), &observed(1), budget(), 2).unwrap();
        std::future::pending::<()>().await;
    });
    assert_eq!(
        pending
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop())),
        Poll::Pending
    );
    let issued = CanisterPoolStore::state();
    assert_eq!(
        CanisterPoolImportOps::claim_execution(identity())
            .unwrap_err()
            .public_error(),
        InternalError::unavailable().public_error()
    );
    drop(pending);
    let _recovery = CanisterPoolImportOps::claim_execution(identity()).unwrap();
    assert_eq!(CanisterPoolStore::state(), issued);
    assert!(
        CanisterPoolImportOps::issue_controllers(identity(), &observed(1), budget(), 3).is_err()
    );
    assert_eq!(CanisterPoolStore::state(), issued);
    CanisterPoolImportOps::observe_controllers(
        identity(),
        &observed(2),
        &history(&observed(2), PoolImportHistoryKind::Controllers),
    )
    .unwrap();
}

#[test]
fn successful_paid_callbacks_release_unused_reserve_without_resetting_call_count() {
    start();
    let first = CanisterPoolImportOps::reserve_paid_call(identity(), 600, 9_950).unwrap();
    CanisterPoolImportOps::complete_paid_call(identity(), first, 9_940).unwrap();
    let status = CanisterPoolImportOps::status(identity()).unwrap();
    assert_eq!(status.paid_calls, 1);
    assert_eq!(status.reserved_debit_cycles, 60);
    assert_eq!(status.last_root_cycles, 9_940);
    let second = CanisterPoolImportOps::reserve_paid_call(identity(), 900, 9_930).unwrap();
    CanisterPoolImportOps::complete_paid_call(identity(), second, 9_920).unwrap();
    let status = CanisterPoolImportOps::status(identity()).unwrap();
    assert_eq!(status.paid_calls, 2);
    assert_eq!(status.reserved_debit_cycles, 80);
}

#[test]
fn lost_and_stale_callbacks_keep_their_conservative_reservation() {
    start();
    let lost = CanisterPoolImportOps::reserve_paid_call(identity(), 600, 9_950).unwrap();
    let bytes = CanisterPoolStore::state().to_bytes().into_owned();
    CanisterPoolStore::set_state(CanisterPoolStateRecord::from_bytes(Cow::Owned(bytes)));
    let completed = CanisterPoolImportOps::reserve_paid_call(identity(), 300, 9_940).unwrap();
    CanisterPoolImportOps::complete_paid_call(identity(), completed, 9_930).unwrap();
    let before = CanisterPoolStore::state();
    assert_eq!(
        CanisterPoolImportOps::status(identity())
            .unwrap()
            .reserved_debit_cycles,
        670
    );
    assert!(CanisterPoolImportOps::complete_paid_call(identity(), lost, 9_920).is_err());
    assert!(CanisterPoolImportOps::complete_paid_call(identity(), completed, 9_920).is_err());
    assert_eq!(CanisterPoolStore::state(), before);
}

#[test]
fn complete_mainnet_sized_import_keeps_reserve_separate_from_observed_debit() {
    use canic_core::control_plane_support::policy::pool_import;
    CanisterPoolStore::clear();
    let mut request = reservation();
    request.sources = (30..54)
        .map(|id| PoolImportSource {
            canister_id: principal(id),
            ..request.sources[0].clone()
        })
        .collect();
    let calls = pool_import::recommended_calls(request.sources.len()).unwrap();
    let quote = 50_000_000_000;
    request.maximum_paid_calls = calls;
    request.maximum_root_debit_cycles = pool_import::required_debit(quote, calls).unwrap();
    request.observed_root_cycles = 400_000_000_000_000;
    let mut pool = config();
    pool.maximum_size = 24;
    CanisterPoolImportOps::reserve(request.clone(), &pool, 1).unwrap();
    let mut cycles = request.observed_root_cycles;
    for _ in 0..pool_import::minimum_calls(request.sources.len()).unwrap() {
        let receipt = CanisterPoolImportOps::reserve_paid_call(identity(), quote, cycles).unwrap();
        cycles -= 500_000_000;
        CanisterPoolImportOps::complete_paid_call(identity(), receipt, cycles).unwrap();
    }
    let status = CanisterPoolImportOps::status(identity()).unwrap();
    assert_eq!(
        status.reserved_debit_cycles,
        request.observed_root_cycles - cycles
    );
    assert_eq!(
        status.paid_calls,
        pool_import::minimum_calls(request.sources.len()).unwrap()
    );
    assert!(status.paid_calls < calls);
}

#[test]
fn capacity_import_root_credits_preserve_debit_and_uncertain_call_authority() {
    start();
    let first = CanisterPoolImportOps::reserve_paid_call(identity(), 600, 9_950).unwrap();
    CanisterPoolImportOps::complete_paid_call(identity(), first, 9_940).unwrap();
    let lost = CanisterPoolImportOps::reserve_paid_call(identity(), 300, 11_000).unwrap();
    let encoded = CanisterPoolStore::state().to_bytes().into_owned();
    CanisterPoolStore::set_state(CanisterPoolStateRecord::from_bytes(Cow::Owned(encoded)));
    let successful = CanisterPoolImportOps::reserve_paid_call(identity(), 100, 12_000).unwrap();
    CanisterPoolImportOps::complete_paid_call(identity(), successful, 13_000).unwrap();
    let retained = CanisterPoolImportOps::status(identity()).unwrap();
    assert_eq!(retained.reservation, reservation());
    assert_eq!(retained.paid_calls, 3);
    assert_eq!(retained.reserved_debit_cycles, 360);
    assert_eq!(retained.last_root_cycles, 13_000);
    assert!(CanisterPoolImportOps::complete_paid_call(identity(), lost, 14_000).is_err());
    assert!(CanisterPoolImportOps::reserve_paid_call(identity(), 641, 14_000).is_err());
    assert_eq!(CanisterPoolImportOps::status(identity()).unwrap(), retained);
    let next = CanisterPoolImportOps::reserve_paid_call(identity(), 100, 12_990).unwrap();
    CanisterPoolImportOps::complete_paid_call(identity(), next, 12_985).unwrap();
    assert_eq!(
        CanisterPoolImportOps::status(identity())
            .unwrap()
            .reserved_debit_cycles,
        375
    );
}

#[test]
fn capacity_import_source_credits_keep_custody_receipts_and_effect_free_replay() {
    start();
    let original = reservation();
    let mut current = observed(1);
    current.cycles = 2_000;
    CanisterPoolImportOps::issue_controllers(identity(), &current, budget(), 2).unwrap();
    current = observed(2);
    current.cycles = 3_000;
    CanisterPoolImportOps::observe_controllers(
        identity(),
        &current,
        &history(&current, PoolImportHistoryKind::Controllers),
    )
    .unwrap();
    current.cycles = 4_000;
    CanisterPoolImportOps::issue_uninstall(identity(), &current, budget()).unwrap();
    let bytes = CanisterPoolStore::state().to_bytes().into_owned();
    CanisterPoolStore::set_state(CanisterPoolStateRecord::from_bytes(Cow::Owned(bytes)));
    current = observed(3);
    current.cycles = 5_000;
    let receipt = CanisterPoolImportOps::observe_cleared(
        identity(),
        &current,
        &history(&current, PoolImportHistoryKind::Uninstall),
        3,
    )
    .unwrap();
    assert_eq!(receipt.observed_debit_cycles, 0);
    assert_eq!(receipt.retained_cycles, 5_000);
    assert_eq!(
        CanisterPoolImportOps::status(identity())
            .unwrap()
            .reservation,
        original
    );
    let settled = CanisterPoolImportOps::settle(identity(), 12_000, 1_000).unwrap();
    assert_eq!(
        settled.root_receipt.as_ref().unwrap().observed_debit_cycles,
        0
    );
    CanisterPoolImportOps::release(identity(), [5; 32]).unwrap();
    let terminal = CanisterPoolStore::state();
    current.cycles += 1_000;
    assert_eq!(
        CanisterPoolImportOps::observe_cleared(
            identity(),
            &current,
            &history(&current, PoolImportHistoryKind::Uninstall),
            4
        )
        .unwrap(),
        receipt
    );
    assert_eq!(
        CanisterPoolImportOps::settle(identity(), 15_000, 1_000)
            .unwrap()
            .root_receipt,
        settled.root_receipt
    );
    CanisterPoolImportOps::release(identity(), [5; 32]).unwrap();
    assert_eq!(CanisterPoolStore::state(), terminal);
}

#[test]
fn exhausted_next_mutation_keeps_confirmation_and_source_drift_refuses_recovery() {
    start();
    CanisterPoolImportOps::issue_controllers(identity(), &observed(1), budget(), 2).unwrap();
    let sample = observed(2);
    CanisterPoolImportOps::observe_controllers(
        identity(),
        &sample,
        &history(&sample, PoolImportHistoryKind::Controllers),
    )
    .unwrap();
    for _ in 1..reservation().maximum_paid_calls {
        CanisterPoolImportOps::reserve_paid_call(identity(), 1, 10_000).unwrap();
    }
    let before = CanisterPoolStore::state();
    assert_eq!(
        CanisterPoolImportOps::issue_uninstall(identity(), &sample, budget())
            .unwrap_err()
            .public_error(),
        InternalError::resource_exhausted().public_error()
    );
    assert_eq!(CanisterPoolStore::state(), before);
    assert!(matches!(
        CanisterPoolImportOps::status(identity()).unwrap().progress[0],
        PoolImportSourceProgress::ControllersConfirmed
    ));
    let mut changed = sample;
    changed.canister_version += 1;
    assert_eq!(
        CanisterPoolImportOps::issue_uninstall(identity(), &changed, budget())
            .unwrap_err()
            .public_error(),
        InternalError::conflict().public_error()
    );
    assert_eq!(CanisterPoolStore::state(), before);
}

#[test]
fn release_census_retains_exhausted_import_without_refunding_allowances() {
    start();
    let request = reservation();
    for _ in 0..request.maximum_paid_calls {
        CanisterPoolImportOps::reserve_paid_call(identity(), 100, request.observed_root_cycles)
            .unwrap();
    }
    assert!(
        CanisterPoolImportOps::reserve_paid_call(identity(), 100, request.observed_root_cycles)
            .is_err()
    );
    let before = CanisterPoolStore::state();
    let retained = CanisterPoolImportOps::status(identity()).unwrap();
    let observed = CanisterPoolOps::release_status(request.root).unwrap();
    assert_eq!(observed.capacity_import, Some(retained));
    assert_eq!(
        observed.capacity_import.as_ref().unwrap().paid_calls,
        request.maximum_paid_calls
    );
    assert_eq!(
        observed
            .capacity_import
            .as_ref()
            .unwrap()
            .reserved_debit_cycles,
        request.maximum_root_debit_cycles
    );
    assert_eq!(CanisterPoolStore::state(), before);
    assert!(CanisterPoolOps::release_status(principal(99)).is_err());
    assert_eq!(CanisterPoolStore::state(), before);
}
