//! Current destructive reset review and its immutable local-publication identity.

use crate::fleet_ensure::model::{
    FleetEnsureJournalRecord, FleetEnsurePlan, FleetEnsureStateRecord,
    completed_handoff::CompletedEstatePublicationReviewRecord,
};

/// Read-only operator review; the publication digest approves this exact current plan.
#[derive(Clone, Debug, serde::Serialize)]
pub struct CompletedResetReviewView {
    pub publication: CompletedEstatePublicationReviewRecord,
    pub plan: FleetEnsurePlan,
    pub committed: bool,
}

/// Fresh current records compiled before replacing any original evidence.
pub(in crate::fleet_ensure) struct CompletedResetTargetView {
    pub plan: FleetEnsurePlan,
    pub journal: FleetEnsureJournalRecord,
    pub state: FleetEnsureStateRecord,
}

/// Complete current native/reserved and default-Ledger observations after reset effects.
#[derive(Clone, Debug)]
pub struct CompletedResetBalancesView {
    pub canisters: std::collections::BTreeMap<String, crate::fleet_ensure::model::completed_handoff::preparation::CompletedPreparationBalanceRecord>,
    pub ledger: std::collections::BTreeMap<String, u128>,
    pub operator_cycles: u128,
}
