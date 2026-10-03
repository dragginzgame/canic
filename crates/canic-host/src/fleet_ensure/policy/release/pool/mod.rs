//! Preserve pool recovery ownership without promoting history or exhaustion to reset authority.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::view::release::pool::{
    ReleasePoolCreationDisposition, ReleasePoolCreationState, ReleasePoolFacts,
    ReleasePoolImportDisposition, ReleasePoolImportState, ReleaseRootPoolAssessment,
};

/// Classify retained work; all custody, spending and producer checks remain with their owners.
pub(in crate::fleet_ensure) fn assess_pool(facts: ReleasePoolFacts) -> ReleaseRootPoolAssessment {
    let import = facts.import.as_ref().map(|import| match import.state {
        ReleasePoolImportState::Reserved => ReleasePoolImportDisposition::ImportRecovery,
        ReleasePoolImportState::Ready if !import.root_settled => {
            ReleasePoolImportDisposition::RootSettlement
        }
        ReleasePoolImportState::Ready => ReleasePoolImportDisposition::PublicationRecovery,
        ReleasePoolImportState::Released => ReleasePoolImportDisposition::RecordedCompletion,
    });
    let creation = facts
        .creation
        .as_ref()
        .map(|creation| match creation.state {
            ReleasePoolCreationState::Intent { uncertain: false }
            | ReleasePoolCreationState::WaitingForFunding
            | ReleasePoolCreationState::LedgerCreationFailed
            | ReleasePoolCreationState::LedgerRejected => {
                ReleasePoolCreationDisposition::OwnerCancellation
            }
            ReleasePoolCreationState::Intent { uncertain: true } => {
                ReleasePoolCreationDisposition::LedgerReconciliation
            }
            ReleasePoolCreationState::Created { canister_id } => {
                ReleasePoolCreationDisposition::InventoryRecovery { canister_id }
            }
            ReleasePoolCreationState::UnresolvedAfterLedgerWindow => {
                ReleasePoolCreationDisposition::UnresolvedLedgerCreation
            }
        });
    ReleaseRootPoolAssessment {
        facts,
        import,
        creation,
    }
}
