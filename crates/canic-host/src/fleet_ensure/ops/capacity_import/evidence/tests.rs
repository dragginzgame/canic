//! Host receipt admission rejects substituted identities, incomplete phases and false accounting.

use super::*;
use crate::fleet_ensure::policy::capacity_import::tests::{plan, principal};
use canic_core::dto::pool_import::{PoolImportRootReceipt, PoolImportSourceReceipt};

pub(in crate::fleet_ensure) fn settled(plan: &CapacityImportPlanRecord) -> PoolImportStatus {
    PoolImportStatus {
        reserved_at_ns: 1,
        reservation: root_reservation(plan).unwrap(),
        progress: plan
            .sources
            .iter()
            .map(|source| {
                PoolImportSourceProgress::Ready(PoolImportSourceReceipt {
                    root_sender_canister_version: 17,
                    canister_id: source.binding.canister_id,
                    canister_version: source.binding.canister_version + 3,
                    before_uninstall_canister_version: source.binding.canister_version + 2,
                    retained_cycles: source.observed_cycles - 20,
                    retained_reserved_cycles: source.observed_reserved_cycles - 10,
                    observed_debit_cycles: 30,
                })
            })
            .collect(),
        phase: PoolImportPhase::Ready,
        paid_calls: 8,
        reserved_debit_cycles: 50,
        last_root_cycles: plan.root_budget.observed_cycles - 20,
        root_receipt: Some(PoolImportRootReceipt {
            retained_cycles: plan.root_budget.observed_cycles - 20,
            retained_reserved_cycles: plan.root_budget.observed_reserved_cycles - 10,
            observed_debit_cycles: 30,
        }),
    }
}

#[test]
fn running_root_receipt_binds_observed_stopped_version_and_exact_uninstall() {
    let mut selected = plan();
    selected.sources[0].binding.controllers = vec![selected.authority.root];
    selected.sources[0].binding.stopped = false;
    let selected = crate::fleet_ensure::ops::capacity_import::prepare_review(
        selected.authority,
        selected.sources,
        selected.root_budget,
    )
    .unwrap();
    let mut status = settled(&selected);
    let PoolImportSourceProgress::Ready(receipt) = &mut status.progress[0] else {
        unreachable!()
    };
    receipt.before_uninstall_canister_version += 20;
    receipt.canister_version = receipt.before_uninstall_canister_version + 1;
    validate_root_status(&selected, &status).unwrap();
    let PoolImportSourceProgress::Ready(receipt) = &mut status.progress[0] else {
        unreachable!()
    };
    receipt.canister_version += 1;
    assert!(matches!(
        validate_root_status(&selected, &status),
        Err(CapacityImportReviewError::RootEvidenceMismatch)
    ));
}

#[test]
fn capacity_import_root_evidence_requires_complete_correlated_terminal_receipts() {
    let plan = plan();
    let valid = settled(&plan);
    validate_root_status(&plan, &valid).unwrap();
    let mutations: [fn(&mut PoolImportStatus); 6] = [
        |s| s.progress[0] = PoolImportSourceProgress::UninstallIssued,
        |s| s.phase = PoolImportPhase::Reserved,
        |s| s.paid_calls = u32::MAX,
        |s| s.reserved_debit_cycles = u128::MAX,
        |s| s.last_root_cycles -= 1,
        |s| {
            s.phase = PoolImportPhase::Released {
                publication_sha256: [0; 32],
            }
        },
    ];
    for change in mutations {
        let mut wrong = valid.clone();
        change(&mut wrong);
        assert!(matches!(
            validate_root_status(&plan, &wrong),
            Err(CapacityImportReviewError::RootEvidenceMismatch)
        ));
    }
    let mut released = valid;
    released.phase = PoolImportPhase::Released {
        publication_sha256: [1; 32],
    };
    validate_root_status(&plan, &released).unwrap();
    released.root_receipt = None;
    assert!(matches!(
        validate_root_status(&plan, &released),
        Err(CapacityImportReviewError::RootEvidenceMismatch)
    ));
}

#[test]
fn capacity_import_root_evidence_accounts_native_reserved_and_exact_original_debit() {
    let plan = plan();
    let valid = settled(&plan);
    let mutations: [fn(&mut PoolImportSourceReceipt); 7] = [
        |r| r.canister_id = principal(99),
        |r| r.canister_version += 1,
        |r| r.retained_reserved_cycles += 1,
        |r| r.retained_cycles += 1,
        |r| r.observed_debit_cycles += 1,
        |r| {
            r.retained_cycles = 700;
            r.retained_reserved_cycles = 370;
        },
        |r| r.retained_cycles = u128::MAX,
    ];
    for change in mutations {
        let mut wrong = valid.clone();
        let PoolImportSourceProgress::Ready(receipt) = &mut wrong.progress[0] else {
            unreachable!()
        };
        change(receipt);
        assert!(matches!(
            validate_root_status(&plan, &wrong),
            Err(CapacityImportReviewError::RootEvidenceMismatch)
        ));
    }
    let mut wrong_root = valid;
    wrong_root
        .root_receipt
        .as_mut()
        .unwrap()
        .retained_reserved_cycles += 1;
    assert!(matches!(
        validate_root_status(&plan, &wrong_root),
        Err(CapacityImportReviewError::RootEvidenceMismatch)
    ));
}

#[test]
fn capacity_import_root_evidence_accepts_partial_progress_without_claiming_completion() {
    let plan = plan();
    let mut partial = settled(&plan);
    partial.phase = PoolImportPhase::Reserved;
    partial.progress[0] = PoolImportSourceProgress::ControllersIssued;
    partial.root_receipt = None;
    validate_root_status(&plan, &partial).unwrap();
    partial.reservation.root = principal(100);
    assert!(matches!(
        validate_root_status(&plan, &partial),
        Err(CapacityImportReviewError::ReservationMismatch)
    ));
}
