//! Module: fleet_ensure::view::terminal_source
//!
//! Responsibility: project completed source evidence for a retirement assessment.
//! Does not own: executable plans, replay, persistence or remote effects.
//! Boundary: completed actions describe receipts; they never enter the effect driver.

use crate::fleet_ensure::model::{
    CycleConservation, EnsureAction, FleetEnsureJournalRecord, FleetTerminalSourceRecord,
    ReviewedDesiredFleetRecord,
};

/// Bounded, read-only evidence from a complete operation and its immutable phases.
#[derive(Clone, Debug)]
pub struct TerminalSourceView {
    pub documents: FleetTerminalSourceRecord,
    pub reviewed_desired: ReviewedDesiredFleetRecord,
    pub conservation: CycleConservation,
    pub journal: FleetEnsureJournalRecord,
    pub actions: Vec<EnsureAction>,
}
