//! Convert retained replay DTOs into passive facts without interpreting release safety.

use crate::fleet_ensure::view::release::receipts::{
    ReleaseAccountingFacts, ReleaseIntentFact, ReleaseIntentState, ReleaseReplayFacts,
    ReleaseReplayState,
};
use canic_contracts::dto::release_receipts::{
    ReplayReleaseEntry, ReplayReleaseIntent, ReplayReleaseIntentState, ReplayReleasePhase,
    ReplayReleaseRecoveryReason,
};

pub(in crate::fleet_ensure) fn assessment_facts(entry: &ReplayReleaseEntry) -> ReleaseReplayFacts {
    let status = match entry.phase {
        ReplayReleasePhase::Reserved => ReleaseReplayState::Reserved,
        ReplayReleasePhase::ExternalEffectInFlight => ReleaseReplayState::ExternalEffectInFlight,
        ReplayReleasePhase::Committed => ReleaseReplayState::Committed,
        ReplayReleasePhase::RecoveryRequired(reason) => match reason {
            ReplayReleaseRecoveryReason::ExternalEffectStatusUnknown => {
                ReleaseReplayState::ExternalEffectStatusUnknown
            }
            ReplayReleaseRecoveryReason::ComponentChildLifecycleInterrupted => {
                ReleaseReplayState::ComponentChildLifecycleInterrupted
            }
            ReplayReleaseRecoveryReason::ResponseCommitFailed => {
                ReleaseReplayState::ResponseCommitFailed
            }
            ReplayReleaseRecoveryReason::CostSettlementFailed => {
                ReleaseReplayState::CostSettlementFailed
            }
        },
    };
    ReleaseReplayFacts {
        slot: entry.slot,
        operation_id: entry.operation_id,
        status,
        accounting: entry
            .cost_guard_settlement
            .as_ref()
            .map(|settlement| ReleaseAccountingFacts {
                quota: intent_fact(settlement.quota_intent_id, settlement.quota.as_ref()),
                reservation: intent_fact(
                    settlement.reservation_intent_id,
                    settlement.reservation.as_ref(),
                ),
            }),
    }
}

fn intent_fact(intent_id: u64, intent: Option<&ReplayReleaseIntent>) -> ReleaseIntentFact {
    ReleaseIntentFact {
        intent_id,
        state: intent.map_or(ReleaseIntentState::Missing, |intent| match intent.state {
            ReplayReleaseIntentState::Pending => ReleaseIntentState::Pending,
            ReplayReleaseIntentState::Committed => ReleaseIntentState::Committed,
            ReplayReleaseIntentState::Aborted => ReleaseIntentState::Aborted,
        }),
    }
}
