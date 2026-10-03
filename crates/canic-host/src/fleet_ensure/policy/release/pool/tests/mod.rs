//! Recovery ownership survives budget exhaustion and unrelated retained custody evidence.

use super::*;
use crate::fleet_ensure::view::release::pool::{ReleasePoolCreationFacts, ReleasePoolImportFacts};
use candid::Principal;
use std::collections::BTreeSet;

fn facts() -> ReleasePoolFacts {
    ReleasePoolFacts {
        root: Principal::from_slice(&[1]),
        import: None,
        creation: None,
        handoff: None,
        custody_candidates: BTreeSet::new(),
    }
}

#[test]
fn import_recovery_does_not_depend_on_exhausted_budgets_or_terminal_bookkeeping() {
    for (state, settled, expected) in [
        (
            ReleasePoolImportState::Reserved,
            false,
            ReleasePoolImportDisposition::ImportRecovery,
        ),
        (
            ReleasePoolImportState::Reserved,
            true,
            ReleasePoolImportDisposition::ImportRecovery,
        ),
        (
            ReleasePoolImportState::Ready,
            false,
            ReleasePoolImportDisposition::RootSettlement,
        ),
        (
            ReleasePoolImportState::Ready,
            true,
            ReleasePoolImportDisposition::PublicationRecovery,
        ),
        (
            ReleasePoolImportState::Released,
            false,
            ReleasePoolImportDisposition::RecordedCompletion,
        ),
        (
            ReleasePoolImportState::Released,
            true,
            ReleasePoolImportDisposition::RecordedCompletion,
        ),
    ] {
        for call_budget_exhausted in [false, true] {
            for debit_budget_exhausted in [false, true] {
                let mut original = facts();
                original.import = Some(ReleasePoolImportFacts {
                    sequence: 7,
                    plan_sha256: [8; 32],
                    state,
                    root_settled: settled,
                    call_budget_exhausted,
                    debit_budget_exhausted,
                });
                let assessed = assess_pool(original.clone());
                assert_eq!(assessed.import, Some(expected));
                assert_eq!(assessed.facts, original);
            }
        }
    }
}

#[test]
fn creation_outcome_preserves_uncertainty_and_exact_created_identity() {
    let id = Principal::from_slice(&[2]);
    for (state, expected) in [
        (
            ReleasePoolCreationState::Intent { uncertain: false },
            ReleasePoolCreationDisposition::OwnerCancellation,
        ),
        (
            ReleasePoolCreationState::Intent { uncertain: true },
            ReleasePoolCreationDisposition::LedgerReconciliation,
        ),
        (
            ReleasePoolCreationState::Created { canister_id: id },
            ReleasePoolCreationDisposition::InventoryRecovery { canister_id: id },
        ),
        (
            ReleasePoolCreationState::WaitingForFunding,
            ReleasePoolCreationDisposition::OwnerCancellation,
        ),
        (
            ReleasePoolCreationState::LedgerCreationFailed,
            ReleasePoolCreationDisposition::OwnerCancellation,
        ),
        (
            ReleasePoolCreationState::LedgerRejected,
            ReleasePoolCreationDisposition::OwnerCancellation,
        ),
        (
            ReleasePoolCreationState::UnresolvedAfterLedgerWindow,
            ReleasePoolCreationDisposition::UnresolvedLedgerCreation,
        ),
    ] {
        let mut original = facts();
        original.creation = Some(ReleasePoolCreationFacts {
            operation_id: [9; 32],
            state,
        });
        let assessed = assess_pool(original.clone());
        assert_eq!(assessed.creation, Some(expected));
        assert_eq!(assessed.facts, original);
    }
}

#[test]
fn empty_owner_reports_no_observed_work_without_inventing_readiness() {
    let original = facts();
    let assessed = assess_pool(original.clone());
    assert_eq!(assessed.facts, original);
    assert_eq!(assessed.import, None);
    assert_eq!(assessed.creation, None);
}
