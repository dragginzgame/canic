//! Pool-owner facts and remaining work, independent of wire contracts and release authority.

use crate::fleet_ensure::view::release::FleetReleasePoolView;
use candid::Principal;
use std::collections::BTreeSet;

/// Recorded import phase; terminal history does not establish current custody.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleasePoolImportState {
    Reserved,
    Ready,
    Released,
}

/// Immutable import identity and observations of its original bounded authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleasePoolImportFacts {
    pub sequence: u64,
    pub plan_sha256: [u8; 32],
    pub state: ReleasePoolImportState,
    /// The original owner retained its Root settlement receipt.
    pub root_settled: bool,
    /// Used calls have reached the original ceiling; no replacement budget is implied.
    pub call_budget_exhausted: bool,
    /// Reserved worst-case debit reached its ceiling; this is not actual cycles spent.
    pub debit_budget_exhausted: bool,
}

/// Creation state supplied by the original owner, without inferring an external outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleasePoolCreationState {
    Intent { uncertain: bool },
    Created { canister_id: Principal },
    WaitingForFunding,
    LedgerCreationFailed,
    LedgerRejected,
    UnresolvedAfterLedgerWindow,
}

/// Exact creation identity; the full request remains in the collected evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleasePoolCreationFacts {
    pub operation_id: [u8; 32],
    pub state: ReleasePoolCreationState,
}

/// Original handoff recipient must remain visible until custody is reconciled.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleasePoolHandoffFact {
    pub canister_id: Principal,
    pub recipient: Principal,
}

/// Passive facts for one authenticated Root; candidates require fresh custody observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleasePoolFacts {
    pub root: Principal,
    pub import: Option<ReleasePoolImportFacts>,
    pub creation: Option<ReleasePoolCreationFacts>,
    pub handoff: Option<ReleasePoolHandoffFact>,
    pub custody_candidates: BTreeSet<Principal>,
}

/// Work retained by the import owner; no case grants more calls or debit authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleasePoolImportDisposition {
    ImportRecovery,
    RootSettlement,
    PublicationRecovery,
    RecordedCompletion,
}

/// Creation-owner work; cancellation remains subject to its original accounting checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleasePoolCreationDisposition {
    OwnerCancellation,
    LedgerReconciliation,
    InventoryRecovery { canister_id: Principal },
    UnresolvedLedgerCreation,
}

/// Assessment keeps original identities and budget exhaustion separate from recovery decisions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseRootPoolAssessment {
    pub facts: ReleasePoolFacts,
    pub import: Option<ReleasePoolImportDisposition>,
    pub creation: Option<ReleasePoolCreationDisposition>,
}

/// Original complete evidence and its assessments; neither is a producer fence or reset permit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetReleasePoolAssessment {
    pub evidence: FleetReleasePoolView,
    pub roots: Vec<ReleaseRootPoolAssessment>,
}
