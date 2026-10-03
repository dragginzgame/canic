//! Interpret replay phase independently of expired or released cost bookkeeping.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::view::release::receipts::{
    ReleaseIntentState, ReleaseReplayAssessment, ReleaseReplayDisposition, ReleaseReplayFacts,
    ReleaseReplayState,
};

/// Preserve the original recovery owner; accounting alone never establishes payment outcome.
pub(in crate::fleet_ensure) fn assess_receipt(
    facts: &ReleaseReplayFacts,
) -> ReleaseReplayAssessment {
    let disposition = match &facts.status {
        ReleaseReplayState::Committed => ReleaseReplayDisposition::RecordedCompletion,
        ReleaseReplayState::Reserved => ReleaseReplayDisposition::ReservedOperation,
        ReleaseReplayState::ExternalEffectInFlight
        | ReleaseReplayState::ExternalEffectStatusUnknown => {
            ReleaseReplayDisposition::EffectReconciliation
        }
        ReleaseReplayState::ComponentChildLifecycleInterrupted => {
            ReleaseReplayDisposition::ChildLifecycleRecovery
        }
        ReleaseReplayState::ResponseCommitFailed => ReleaseReplayDisposition::ResponseRecovery,
        ReleaseReplayState::CostSettlementFailed => ReleaseReplayDisposition::AccountingRecovery,
    };
    let mut pending_intents = Vec::new();
    let mut missing_intents = Vec::new();
    if let Some(accounting) = facts.accounting {
        for intent in [accounting.quota, accounting.reservation] {
            match intent.state {
                ReleaseIntentState::Pending => pending_intents.push(intent.intent_id),
                ReleaseIntentState::Missing => missing_intents.push(intent.intent_id),
                ReleaseIntentState::Committed | ReleaseIntentState::Aborted => {}
            }
        }
    }
    ReleaseReplayAssessment {
        slot: facts.slot,
        operation_id: facts.operation_id,
        disposition,
        pending_intents,
        missing_intents,
    }
}
