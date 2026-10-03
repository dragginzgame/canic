//! Passive replay assessment facts and results; original evidence remains separately retained.

use crate::fleet_ensure::view::release::FleetReleaseReceiptsView;
use candid::Principal;
use std::collections::BTreeMap;

/// Host-owned projection of the source's recorded phase, without DTO or storage coupling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseReplayState {
    Reserved,
    ExternalEffectInFlight,
    Committed,
    ExternalEffectStatusUnknown,
    ComponentChildLifecycleInterrupted,
    ResponseCommitFailed,
    CostSettlementFailed,
}

/// Local accounting observation, independent of any external effect's outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseIntentState {
    Missing,
    Pending,
    Committed,
    Aborted,
}

/// Exact linked identity and stored state; expiry does not imply settlement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReleaseIntentFact {
    pub intent_id: u64,
    pub state: ReleaseIntentState,
}

/// Original quota and cycle reservation kept distinct, without summing their quantities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReleaseAccountingFacts {
    pub quota: ReleaseIntentFact,
    pub reservation: ReleaseIntentFact,
}

/// Source-owner replay phase and accounting facts supplied to pure policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseReplayFacts {
    pub slot: [u8; 32],
    pub operation_id: [u8; 32],
    pub status: ReleaseReplayState,
    pub accounting: Option<ReleaseAccountingFacts>,
}

/// Remaining replay-owner work; no variant authorizes dispatch or establishes release readiness.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseReplayDisposition {
    /// A recorded terminal response, which may describe success or refusal.
    RecordedCompletion,
    /// The receipt is reserved; its operation owner must establish the current outcome.
    ReservedOperation,
    /// An issued effect still needs its original reconciliation authority.
    EffectReconciliation,
    /// The child lifecycle journal owns interruption recovery.
    ChildLifecycleRecovery,
    /// The original owner must recover its response, without redispatching the effect.
    ResponseRecovery,
    /// The original owner must reconcile cost accounting before committing its response.
    AccountingRecovery,
}

/// Receipt disposition and separate accounting observations, keyed back to original evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseReplayAssessment {
    pub slot: [u8; 32],
    pub operation_id: [u8; 32],
    pub disposition: ReleaseReplayDisposition,
    pub pending_intents: Vec<u64>,
    pub missing_intents: Vec<u64>,
}

/// Complete collected pages alongside their assessments; neither is a producer fence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetReleaseReplayAssessment {
    pub evidence: FleetReleaseReceiptsView,
    pub owners: BTreeMap<Principal, Vec<ReleaseReplayAssessment>>,
}
