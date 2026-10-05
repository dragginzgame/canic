//! Convert durable publication records into read-only workflow evidence.

use crate::{
    ops::component_registry::caller_authority::{CallerReceiverPlan, RootCallerOps},
    storage::stable::component_registry::caller_authority as stored,
    view::component_registry::caller_authority as view,
};
use candid::Principal;
use canic_core::{
    control_plane_support::error::InternalError,
    ids::{CallerInstallation, CallerRootAuthority, CanisterRole},
};

impl RootCallerOps {
    /// Project original operation authority without exposing mutable storage records.
    pub fn operation_view(operation: [u8; 32]) -> Option<view::CallerJournalView> {
        Self::operation(operation).map(journal)
    }

    /// Freeze the workflow-selected scope through the existing journal owner.
    pub fn begin_publication(
        operation: [u8; 32],
        issuer: CallerRootAuthority,
        scope: view::CallerLifecycleScope,
        plans: Vec<CallerReceiverPlan>,
    ) -> Result<(), InternalError> {
        let scope = match scope {
            view::CallerLifecycleScope::ActivateComponent(source) => {
                stored::CallerLifecycleScope::ActivateComponent(source)
            }
            view::CallerLifecycleScope::ActivateChild(source) => {
                stored::CallerLifecycleScope::ActivateChild(source)
            }
            view::CallerLifecycleScope::DenyComponent(source) => {
                stored::CallerLifecycleScope::DenyComponent(source)
            }
            view::CallerLifecycleScope::DenySubtree(source) => {
                stored::CallerLifecycleScope::DenySubtree(source)
            }
        };
        Self::begin(operation, issuer, scope, plans).map(|_| ())
    }

    /// Advance durable delivery progress and return only its workflow projection.
    pub fn next_phase_view(operation: [u8; 32]) -> Result<view::CallerJournalView, InternalError> {
        Self::next_phase(operation).map(journal)
    }

    /// Read one original recipient's delivery progress.
    pub fn recipient_view(
        operation: [u8; 32],
        ordinal: u32,
    ) -> Result<view::CallerRecipientView, InternalError> {
        Self::recipient(operation, ordinal).map(|row| view::CallerRecipientView {
            authority: row.authority,
            before_count: row.before_count,
            step_count: row.step_count,
            completed_steps: row.completed_steps,
            startup_released: row.startup_released,
            kind: match row.kind {
                stored::CallerRecipientKind::Enrollment => view::CallerRecipientKind::Enrollment,
                stored::CallerRecipientKind::Update => view::CallerRecipientKind::Update,
                stored::CallerRecipientKind::Retirement => view::CallerRecipientKind::Retirement,
            },
        })
    }

    /// Read the immutable publication and observed receipt for one delivery step.
    pub fn step_view(
        operation: [u8; 32],
        recipient: u32,
        ordinal: u32,
    ) -> Result<view::CallerStepView, InternalError> {
        Self::step(operation, recipient, ordinal).map(|row| view::CallerStepView {
            publication: row.publication,
            phase: row.phase,
        })
    }

    /// Project the confirmed receiver installation.
    pub fn receiver_view(canister: Principal) -> Option<view::CallerEnrolledReceiverView> {
        Self::receiver(canister).map(receiver)
    }

    /// Resolve live receiver projections by exact role.
    pub fn receiver_views(
        role: &CanisterRole,
    ) -> Result<Vec<view::CallerEnrolledReceiverView>, InternalError> {
        Self::receivers(role).map(|rows| rows.into_iter().map(receiver).collect())
    }

    /// Resolve the original source's indexed live recipients.
    pub fn source_receiver_views(
        source: &CallerInstallation,
    ) -> Result<Vec<view::CallerEnrolledReceiverView>, InternalError> {
        Self::source_receivers(source).map(|rows| rows.into_iter().map(receiver).collect())
    }
}

fn receiver(row: stored::CallerEnrolledReceiverRecord) -> view::CallerEnrolledReceiverView {
    view::CallerEnrolledReceiverView {
        authority: row.authority,
        generation: row.generation,
        retired: row.retired,
    }
}

fn journal(row: stored::CallerJournalRecord) -> view::CallerJournalView {
    view::CallerJournalView {
        issuer: row.issuer,
        recipient_count: row.recipient_count,
        cursor: row.cursor,
        phase: match row.phase {
            stored::CallerJournalPhase::Reserving => view::CallerJournalPhase::Reserving,
            stored::CallerJournalPhase::Preparing => view::CallerJournalPhase::Preparing,
            stored::CallerJournalPhase::Prepared => view::CallerJournalPhase::Prepared,
            stored::CallerJournalPhase::Publishing => view::CallerJournalPhase::Publishing,
            stored::CallerJournalPhase::Published => view::CallerJournalPhase::Published,
            stored::CallerJournalPhase::Complete => view::CallerJournalPhase::Complete,
            stored::CallerJournalPhase::Compacting => view::CallerJournalPhase::Compacting,
            stored::CallerJournalPhase::Compacted => view::CallerJournalPhase::Compacted,
        },
        scope: match row.scope {
            stored::CallerLifecycleScope::ActivateComponent(source) => {
                view::CallerLifecycleScope::ActivateComponent(source)
            }
            stored::CallerLifecycleScope::ActivateChild(source) => {
                view::CallerLifecycleScope::ActivateChild(source)
            }
            stored::CallerLifecycleScope::DenyComponent(source) => {
                view::CallerLifecycleScope::DenyComponent(source)
            }
            stored::CallerLifecycleScope::DenySubtree(source) => {
                view::CallerLifecycleScope::DenySubtree(source)
            }
        },
    }
}
