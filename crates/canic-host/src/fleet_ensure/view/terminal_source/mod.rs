//! Module: fleet_ensure::view::terminal_source
//!
//! Responsibility: project completed source evidence for a retirement assessment.
//! Does not own: executable plans, replay, persistence or remote effects.
//! Boundary: completed actions describe receipts; they never enter the effect driver.

use crate::fleet_ensure::model::{
    CycleConservation, EffectRecord, EnsureAction, FleetEnsureSuccessorPhaseRecord,
    FleetTerminalSourceRecord, ReviewedDesiredFleetRecord,
};

/// Bounded, read-only evidence from a complete operation and its immutable phases.
#[derive(Clone, Debug)]
pub struct TerminalSourceView {
    pub documents: FleetTerminalSourceRecord,
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
