//! Module: fleet_ensure::view::terminal_source
//!
//! Responsibility: project completed source evidence for a retirement assessment.
//! Does not own: executable plans, replay, persistence or remote effects.
//! Boundary: completed actions describe receipts; they never enter the effect driver.

pub(in crate::fleet_ensure) mod inventory;

/// Locally verified historical receipts, never live authority or permission to reinstall.
#[derive(Debug)]
pub struct CompletedReceiptAuditView {
    pub documents: crate::fleet_ensure::model::FleetTerminalSourceRecord,
    pub effect_count: usize,
    pub phase_count: usize,
    pub source_operator: String,
    pub cycles_ledger: String,
    pub initial_controlled_cycles: u128,
    pub initial_operator_cycles: u128,
    pub initial_estate_funding_cycles_by_root: std::collections::BTreeMap<String, u128>,
    pub recorded_funding_cycles: u128,
    pub recorded_operator_debit_cycles: u128,
    pub original_maximum_execution_burn_cycles: u128,
}

use crate::fleet_ensure::model::{
    CycleConservation, EffectRecord, EnsureAction, FleetEnsureSuccessorPhaseRecord,
    FleetTerminalSourceRecord, ReviewedDesiredFleetRecord,
};

/// One bounded snapshot of completed document claims, independent of execution schemas.
/// Byte bindings and common identities do not establish receipt validity or live authority.
#[derive(Debug)]
pub(in crate::fleet_ensure) struct CompletedDocumentsView {
    pub bindings: FleetTerminalSourceRecord,
    pub plan: serde_json::Value,
    pub journal: serde_json::Value,
    pub state: serde_json::Value,
}

/// Bounded, read-only evidence from a complete operation and its immutable phases.
#[derive(Clone, Debug)]
pub struct TerminalSourceView {
    pub documents: FleetTerminalSourceRecord,
    pub planned_at_time: u64,
    pub reviewed_desired: ReviewedDesiredFleetRecord,
    pub conservation: CycleConservation,
    pub journal: TerminalJournalView,
    pub actions: Vec<EnsureAction>,
}

///
/// TerminalJournalView
///
/// Completed payment and phase evidence, never an executable or writable journal.
///

#[derive(Clone, Debug)]
pub struct TerminalJournalView {
    pub effects: Vec<EffectRecord>,
    pub successor_phases: Vec<FleetEnsureSuccessorPhaseRecord>,
    pub initial_controlled_cycles: u128,
    pub initial_operator_cycles: u128,
    pub initial_estate_funding_cycles_by_root: std::collections::BTreeMap<String, u128>,
}
