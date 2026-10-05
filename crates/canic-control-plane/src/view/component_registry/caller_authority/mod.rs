//! Read-only publication progress passed from the Registry owner to workflow.

use canic_core::{
    control_plane_support::model::caller_authority::{CallerPublicationRecord, CallerReceiptPhase},
    ids::{CallerInstallation, CallerReceiverAuthority, CallerRootAuthority},
};

/// Exact membership operation selected before publication begins.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CallerLifecycleScope {
    ActivateComponent(CallerInstallation),
    ActivateChild(CallerInstallation),
    DenyComponent(CallerInstallation),
    DenySubtree(CallerInstallation),
}

/// Current aggregate publication phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallerJournalPhase {
    Reserving,
    Preparing,
    Prepared,
    Publishing,
    Published,
    Complete,
    Compacting,
    Compacted,
}

/// Receiver lifecycle obligation in the frozen census.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallerRecipientKind {
    Enrollment,
    Update,
    Retirement,
}

/// Original operation authority and current bounded delivery position.
pub struct CallerJournalView {
    pub issuer: CallerRootAuthority,
    pub scope: CallerLifecycleScope,
    pub recipient_count: u32,
    pub phase: CallerJournalPhase,
    pub cursor: u32,
}

/// Receiver evidence needed to deliver the next original step.
pub struct CallerRecipientView {
    pub authority: CallerReceiverAuthority,
    pub before_count: u32,
    pub step_count: u32,
    pub completed_steps: u32,
    pub startup_released: bool,
    pub kind: CallerRecipientKind,
}

/// Original publication and observed receipt phase.
pub struct CallerStepView {
    pub publication: CallerPublicationRecord,
    pub phase: Option<CallerReceiptPhase>,
}

/// Exact currently indexed receiver installation.
pub struct CallerEnrolledReceiverView {
    pub authority: CallerReceiverAuthority,
    pub generation: u64,
    pub retired: bool,
}
