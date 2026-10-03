//! Project retained pool evidence without changing its authority or interpreting release safety.

use crate::fleet_ensure::view::release::pool::{
    ReleasePoolCreationFacts, ReleasePoolCreationState, ReleasePoolFacts, ReleasePoolHandoffFact,
    ReleasePoolImportFacts, ReleasePoolImportState,
};
use canic_control_plane::dto::root::RootPoolReleaseResponse;
use canic_core::dto::{
    pool::{CanisterPoolCreationFailure, CanisterPoolCreationProgress},
    pool_import::{PoolImportPhase, PoolImportSourceProgress},
};
use std::collections::BTreeSet;

pub(in crate::fleet_ensure) fn assessment_facts(
    status: &RootPoolReleaseResponse,
) -> ReleasePoolFacts {
    let mut custody_candidates = BTreeSet::new();
    if let Some(bootstrap) = &status.bootstrap {
        custody_candidates.insert(bootstrap.store);
        custody_candidates.extend(bootstrap.sources.iter().copied());
    }
    let import =
        status.capacity_import.as_ref().map(|import| {
            custody_candidates.extend(
                import
                    .reservation
                    .sources
                    .iter()
                    .map(|source| source.canister_id),
            );
            custody_candidates.extend(import.progress.iter().filter_map(
                |progress| match progress {
                    PoolImportSourceProgress::Ready(receipt) => Some(receipt.canister_id),
                    _ => None,
                },
            ));
            ReleasePoolImportFacts {
                sequence: import.reservation.sequence,
                plan_sha256: import.reservation.plan_sha256,
                state: match import.phase {
                    PoolImportPhase::Reserved => ReleasePoolImportState::Reserved,
                    PoolImportPhase::Ready => ReleasePoolImportState::Ready,
                    PoolImportPhase::Released { .. } => ReleasePoolImportState::Released,
                },
                root_settled: import.root_receipt.is_some(),
                call_budget_exhausted: import.paid_calls >= import.reservation.maximum_paid_calls,
                debit_budget_exhausted: import.reserved_debit_cycles
                    >= import.reservation.maximum_root_debit_cycles,
            }
        });
    let creation = status
        .creation
        .as_ref()
        .map(|creation| ReleasePoolCreationFacts {
            operation_id: creation.operation_id,
            state: match creation.progress {
                CanisterPoolCreationProgress::Intent { uncertain_result } => {
                    ReleasePoolCreationState::Intent {
                        uncertain: uncertain_result,
                    }
                }
                CanisterPoolCreationProgress::Created { canister_id, .. } => {
                    custody_candidates.insert(canister_id);
                    ReleasePoolCreationState::Created { canister_id }
                }
                CanisterPoolCreationProgress::WaitingForFunding { .. } => {
                    ReleasePoolCreationState::WaitingForFunding
                }
                CanisterPoolCreationProgress::Blocked { failure } => match failure {
                    CanisterPoolCreationFailure::UnresolvedAfterLedgerWindow => {
                        ReleasePoolCreationState::UnresolvedAfterLedgerWindow
                    }
                    CanisterPoolCreationFailure::LedgerCreationFailed => {
                        ReleasePoolCreationState::LedgerCreationFailed
                    }
                    CanisterPoolCreationFailure::LedgerRejected => {
                        ReleasePoolCreationState::LedgerRejected
                    }
                },
            },
        });
    let handoff = status.handoff.as_ref().map(|handoff| {
        custody_candidates.insert(handoff.canister_id);
        ReleasePoolHandoffFact {
            canister_id: handoff.canister_id,
            recipient: handoff.recipient,
        }
    });
    ReleasePoolFacts {
        root: status.root,
        import,
        creation,
        handoff,
        custody_candidates,
    }
}
