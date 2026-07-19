//! Module: restore::apply::journal::types
//!
//! Responsibility: define restore apply journal operations, states, kinds, and errors.
//! Does not own: command previews, operation receipts, or reporting.
//! Boundary: provides journal row validation and shared journal helper functions.

use crate::{
    artifacts::{ArtifactChecksum, ArtifactChecksumError},
    manifest::VERIFICATION_KIND_STATUS,
    restore::{RestoreApplyDryRun, RestoreApplyDryRunOperation, RestoreApplyDryRunValidationError},
};

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// RestoreApplyJournalOperation
///
/// Durable restore apply operation row.
/// Owned by restore apply journaling and consumed by command preview and runner code.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreApplyJournalOperation {
    pub sequence: usize,
    pub operation: RestoreApplyOperationKind,
    pub state: RestoreApplyOperationState,
    #[serde(deserialize_with = "crate::serialization::required_option")]
    pub state_updated_at: Option<String>,
    pub blocking_reasons: Vec<String>,
    pub member_order: usize,
    pub source_canister: String,
    pub target_canister: String,
    pub role: String,
    #[serde(deserialize_with = "crate::serialization::required_option")]
    pub snapshot_id: Option<String>,
    #[serde(deserialize_with = "crate::serialization::required_option")]
    pub artifact_path: Option<String>,
    #[serde(deserialize_with = "crate::serialization::required_option")]
    pub artifact_checksum: Option<ArtifactChecksum>,
    #[serde(deserialize_with = "crate::serialization::required_option")]
    pub verification_kind: Option<String>,
}

impl RestoreApplyJournalOperation {
    pub(super) fn from_dry_run_operation(
        operation: &RestoreApplyDryRunOperation,
        state: RestoreApplyOperationState,
        blocked_reasons: &[String],
    ) -> Self {
        Self {
            sequence: operation.sequence,
            operation: operation.operation.clone(),
            state: state.clone(),
            state_updated_at: None,
            blocking_reasons: if state == RestoreApplyOperationState::Blocked {
                blocked_reasons.to_vec()
            } else {
                Vec::new()
            },
            member_order: operation.member_order,
            source_canister: operation.source_canister.clone(),
            target_canister: operation.target_canister.clone(),
            role: operation.role.clone(),
            snapshot_id: operation.snapshot_id.clone(),
            artifact_path: operation.artifact_path.clone(),
            artifact_checksum: operation.artifact_checksum.clone(),
            verification_kind: operation.verification_kind.clone(),
        }
    }

    pub(super) fn validate(&self) -> Result<(), RestoreApplyJournalError> {
        validate_apply_journal_nonempty("operations[].source_canister", &self.source_canister)?;
        validate_apply_journal_nonempty("operations[].target_canister", &self.target_canister)?;
        validate_apply_journal_nonempty("operations[].role", &self.role)?;
        if let Some(updated_at) = &self.state_updated_at {
            validate_apply_journal_nonempty("operations[].state_updated_at", updated_at)?;
        }
        self.validate_operation_fields()?;

        match self.state {
            RestoreApplyOperationState::Blocked if self.blocking_reasons.is_empty() => Err(
                RestoreApplyJournalError::BlockedOperationMissingReason(self.sequence),
            ),
            RestoreApplyOperationState::Failed if self.blocking_reasons.is_empty() => Err(
                RestoreApplyJournalError::FailureReasonRequired(self.sequence),
            ),
            RestoreApplyOperationState::Pending
            | RestoreApplyOperationState::Ready
            | RestoreApplyOperationState::Completed
                if !self.blocking_reasons.is_empty() =>
            {
                Err(RestoreApplyJournalError::UnblockedOperationHasReasons(
                    self.sequence,
                ))
            }
            RestoreApplyOperationState::Blocked
            | RestoreApplyOperationState::Failed
            | RestoreApplyOperationState::Pending
            | RestoreApplyOperationState::Ready
            | RestoreApplyOperationState::Completed => Ok(()),
        }
    }

    fn validate_operation_fields(&self) -> Result<(), RestoreApplyJournalError> {
        match self.operation {
            RestoreApplyOperationKind::UploadSnapshot => {
                self.validate_required_field(
                    "operations[].artifact_path",
                    self.artifact_path.as_ref(),
                )?;
                self.validate_artifact_checksum()
            }
            RestoreApplyOperationKind::LoadSnapshot => {
                self.validate_required_field(
                    "operations[].snapshot_id",
                    self.snapshot_id.as_ref(),
                )?;
                self.validate_required_field(
                    "operations[].artifact_path",
                    self.artifact_path.as_ref(),
                )?;
                self.validate_artifact_checksum()
            }
            RestoreApplyOperationKind::StopCanister | RestoreApplyOperationKind::StartCanister => {
                Ok(())
            }
            RestoreApplyOperationKind::VerifyMember
            | RestoreApplyOperationKind::VerifyDeployment => {
                let kind = self.validate_required_field(
                    "operations[].verification_kind",
                    self.verification_kind.as_ref(),
                )?;
                if kind != VERIFICATION_KIND_STATUS {
                    return Err(RestoreApplyJournalError::UnsupportedVerificationKind {
                        sequence: self.sequence,
                        kind: kind.to_string(),
                    });
                }
                Ok(())
            }
        }
    }

    fn validate_artifact_checksum(&self) -> Result<(), RestoreApplyJournalError> {
        if self.state == RestoreApplyOperationState::Blocked && self.artifact_checksum.is_none() {
            return Ok(());
        }
        let checksum = self.artifact_checksum.as_ref().ok_or_else(|| {
            RestoreApplyJournalError::OperationMissingField {
                sequence: self.sequence,
                operation: self.operation.clone(),
                field: "operations[].artifact_checksum",
            }
        })?;
        checksum
            .validate()
            .map_err(|source| RestoreApplyJournalError::ArtifactChecksum {
                sequence: self.sequence,
                source,
            })
    }

    fn validate_required_field<'a>(
        &self,
        field: &'static str,
        value: Option<&'a String>,
    ) -> Result<&'a str, RestoreApplyJournalError> {
        let value = value.map(String::as_str).ok_or_else(|| {
            RestoreApplyJournalError::OperationMissingField {
                sequence: self.sequence,
                operation: self.operation.clone(),
                field,
            }
        })?;
        if value.trim().is_empty() {
            return Err(RestoreApplyJournalError::OperationMissingField {
                sequence: self.sequence,
                operation: self.operation.clone(),
                field,
            });
        }

        Ok(value)
    }

    pub(super) const fn can_transition_to(&self, next_state: &RestoreApplyOperationState) -> bool {
        match (&self.state, next_state) {
            (
                RestoreApplyOperationState::Ready | RestoreApplyOperationState::Pending,
                RestoreApplyOperationState::Pending,
            )
            | (
                RestoreApplyOperationState::Pending | RestoreApplyOperationState::Failed,
                RestoreApplyOperationState::Ready,
            )
            | (
                RestoreApplyOperationState::Ready
                | RestoreApplyOperationState::Pending
                | RestoreApplyOperationState::Completed,
                RestoreApplyOperationState::Completed,
            )
            | (
                RestoreApplyOperationState::Ready
                | RestoreApplyOperationState::Pending
                | RestoreApplyOperationState::Failed,
                RestoreApplyOperationState::Failed,
            ) => true,
            (
                RestoreApplyOperationState::Blocked
                | RestoreApplyOperationState::Completed
                | RestoreApplyOperationState::Failed
                | RestoreApplyOperationState::Pending
                | RestoreApplyOperationState::Ready,
                _,
            ) => false,
        }
    }
}

///
/// RestoreApplyOperationState
///
/// Durable lifecycle state for one restore apply operation.
/// Owned by restore apply journaling and used by runners to advance work.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestoreApplyOperationState {
    Pending,
    Ready,
    Blocked,
    Completed,
    Failed,
}

///
/// RestoreApplyJournalError
///
/// Typed restore apply journal validation or transition failure.
/// Owned by restore apply journaling and returned before unsafe mutation proceeds.
///

#[derive(Debug, ThisError)]
pub enum RestoreApplyJournalError {
    #[error("restore apply journal operation {sequence} has invalid artifact checksum")]
    ArtifactChecksum {
        sequence: usize,
        #[source]
        source: ArtifactChecksumError,
    },

    #[error(transparent)]
    InvalidDryRun(#[from] RestoreApplyDryRunValidationError),

    #[error("unsupported restore apply journal version {0}")]
    UnsupportedVersion(u16),

    #[error("restore apply journal field {0} is required")]
    MissingField(&'static str),

    #[error("restore apply journal count {field} mismatch: reported={reported}, actual={actual}")]
    CountMismatch {
        field: &'static str,
        reported: usize,
        actual: usize,
    },

    #[error("restore apply journal has duplicate operation sequence {0}")]
    DuplicateSequence(usize),

    #[error("restore apply journal is missing operation sequence {0}")]
    MissingSequence(usize),

    #[error("ready restore apply journal cannot include blocked reasons or blocked operations")]
    ReadyJournalHasBlockingState,

    #[error("blocked restore apply journal operation {0} is missing a blocking reason")]
    BlockedOperationMissingReason(usize),

    #[error("unblocked restore apply journal operation {0} cannot have blocking reasons")]
    UnblockedOperationHasReasons(usize),

    #[error("restore apply journal operation {sequence} {operation:?} is missing field {field}")]
    OperationMissingField {
        sequence: usize,
        operation: RestoreApplyOperationKind,
        field: &'static str,
    },

    #[error("restore apply journal operation {sequence} uses unsupported verification kind {kind}")]
    UnsupportedVerificationKind { sequence: usize, kind: String },

    #[error("restore apply journal operation {0} was not found")]
    OperationNotFound(usize),

    #[error("restore apply journal operation {sequence} cannot transition from {from:?} to {to:?}")]
    InvalidOperationTransition {
        sequence: usize,
        from: RestoreApplyOperationState,
        to: RestoreApplyOperationState,
    },

    #[error("restore apply journal receipt for operation {sequence} has invalid attempt {attempt}")]
    InvalidOperationReceiptAttempt { sequence: usize, attempt: usize },

    #[error("failed restore apply journal operation {0} requires a reason")]
    FailureReasonRequired(usize),

    #[error("restore apply journal has no operation that can be advanced")]
    NoTransitionableOperation,

    #[error("restore apply journal has no pending operation to release")]
    NoPendingOperation,

    #[error("restore apply journal has no failed operation to recover")]
    NoFailedOperation,

    #[error("restore apply journal operation {requested} cannot advance before operation {next}")]
    OutOfOrderOperationTransition { requested: usize, next: usize },

    #[error("restore apply journal receipt references missing operation {0}")]
    OperationReceiptOperationNotFound(usize),

    #[error(
        "restore apply journal has duplicate receipt for operation {sequence} attempt {attempt}"
    )]
    DuplicateOperationReceiptAttempt { sequence: usize, attempt: usize },

    #[error("restore apply journal receipt does not match operation {sequence}")]
    OperationReceiptMismatch { sequence: usize },
}

///
/// RestoreApplyOperationKind
///
/// Restore apply operation kind rendered into runner commands.
/// Owned by restore apply journaling and shared by dry-runs, journals, and receipts.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestoreApplyOperationKind {
    StopCanister,
    StartCanister,
    UploadSnapshot,
    LoadSnapshot,
    VerifyMember,
    VerifyDeployment,
}

pub(super) const fn validate_apply_journal_version(
    version: u16,
) -> Result<(), RestoreApplyJournalError> {
    if version == 1 {
        return Ok(());
    }

    Err(RestoreApplyJournalError::UnsupportedVersion(version))
}

pub(super) fn validate_apply_journal_nonempty(
    field: &'static str,
    value: &str,
) -> Result<(), RestoreApplyJournalError> {
    if !value.trim().is_empty() {
        return Ok(());
    }

    Err(RestoreApplyJournalError::MissingField(field))
}

pub(super) const fn validate_apply_journal_count(
    field: &'static str,
    reported: usize,
    actual: usize,
) -> Result<(), RestoreApplyJournalError> {
    if reported == actual {
        return Ok(());
    }

    Err(RestoreApplyJournalError::CountMismatch {
        field,
        reported,
        actual,
    })
}

pub(super) fn validate_apply_journal_sequences(
    operations: &[RestoreApplyJournalOperation],
) -> Result<(), RestoreApplyJournalError> {
    let mut sequences = BTreeSet::new();
    for operation in operations {
        if !sequences.insert(operation.sequence) {
            return Err(RestoreApplyJournalError::DuplicateSequence(
                operation.sequence,
            ));
        }
    }

    for expected in 0..operations.len() {
        if !sequences.contains(&expected) {
            return Err(RestoreApplyJournalError::MissingSequence(expected));
        }
    }

    Ok(())
}

pub(super) fn restore_apply_blocked_reasons(dry_run: &RestoreApplyDryRun) -> Vec<String> {
    let mut reasons = dry_run.readiness_reasons.clone();

    match &dry_run.artifact_validation {
        Some(validation) => {
            if !validation.artifacts_present {
                reasons.push("missing-artifacts".to_string());
            }
            if !validation.checksums_verified {
                reasons.push("artifact-checksum-validation-incomplete".to_string());
            }
        }
        None => reasons.push("missing-artifact-validation".to_string()),
    }

    reasons.sort();
    reasons.dedup();
    reasons
}
