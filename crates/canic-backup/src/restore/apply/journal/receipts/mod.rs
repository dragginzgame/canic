use super::{
    RestoreApplyJournal, RestoreApplyJournalError, RestoreApplyJournalOperation,
    RestoreApplyOperationKind, RestoreApplyRunnerCommand, validate_apply_journal_nonempty,
};
use serde::{Deserialize, Serialize};

///
/// RestoreApplyOperationReceipt
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyOperationReceipt {
    pub sequence: usize,
    pub operation: RestoreApplyOperationKind,
    #[serde(default)]
    pub outcome: RestoreApplyOperationReceiptOutcome,
    pub source_canister: String,
    pub target_canister: String,
    #[serde(default)]
    pub attempt: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<RestoreApplyRunnerCommand>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<RestoreApplyCommandOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<RestoreApplyCommandOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_snapshot_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uploaded_snapshot_id: Option<String>,
}

impl RestoreApplyOperationReceipt {
    /// Build a durable completed-command receipt for the apply journal.
    #[must_use]
    pub(in crate::restore) fn command_completed(
        operation: &RestoreApplyJournalOperation,
        command: RestoreApplyRunnerCommand,
        status: String,
        updated_at: Option<String>,
        output: RestoreApplyCommandOutputPair,
        attempt: usize,
        uploaded_snapshot_id: Option<String>,
    ) -> Self {
        Self::from_operation(
            operation,
            operation.operation.clone(),
            RestoreApplyOperationReceiptOutcome::CommandCompleted,
            RestoreApplyOperationReceiptDetails {
                attempt,
                updated_at,
                command: Some(command),
                status: Some(status),
                stdout: Some(output.stdout),
                stderr: Some(output.stderr),
                uploaded_snapshot_id,
                failure_reason: None,
            },
        )
    }

    /// Build a durable failed-command receipt for the apply journal.
    #[must_use]
    pub(in crate::restore) fn command_failed(
        operation: &RestoreApplyJournalOperation,
        command: RestoreApplyRunnerCommand,
        status: String,
        updated_at: Option<String>,
        output: RestoreApplyCommandOutputPair,
        attempt: usize,
        failure_reason: String,
    ) -> Self {
        Self::from_operation(
            operation,
            operation.operation.clone(),
            RestoreApplyOperationReceiptOutcome::CommandFailed,
            RestoreApplyOperationReceiptDetails {
                attempt,
                updated_at,
                command: Some(command),
                status: Some(status),
                stdout: Some(output.stdout),
                stderr: Some(output.stderr),
                failure_reason: Some(failure_reason),
                uploaded_snapshot_id: None,
            },
        )
    }

    // Build one durable receipt row from shared operation metadata.
    fn from_operation(
        operation: &RestoreApplyJournalOperation,
        operation_kind: RestoreApplyOperationKind,
        outcome: RestoreApplyOperationReceiptOutcome,
        details: RestoreApplyOperationReceiptDetails,
    ) -> Self {
        Self {
            sequence: operation.sequence,
            operation: operation_kind,
            outcome,
            source_canister: operation.source_canister.clone(),
            target_canister: operation.target_canister.clone(),
            attempt: details.attempt,
            updated_at: details.updated_at,
            command: details.command,
            status: details.status,
            stdout: details.stdout,
            stderr: details.stderr,
            failure_reason: details.failure_reason,
            source_snapshot_id: operation.snapshot_id.clone(),
            artifact_path: operation.artifact_path.clone(),
            uploaded_snapshot_id: details.uploaded_snapshot_id,
        }
    }

    // Return whether this upload receipt satisfies one later load operation.
    pub(super) fn matches_load_operation(&self, load: &RestoreApplyJournalOperation) -> bool {
        self.operation == RestoreApplyOperationKind::UploadSnapshot
            && self.outcome == RestoreApplyOperationReceiptOutcome::CommandCompleted
            && load.operation == RestoreApplyOperationKind::LoadSnapshot
            && self.source_canister == load.source_canister
            && self.target_canister == load.target_canister
            && self.source_snapshot_id == load.snapshot_id
            && self.artifact_path == load.artifact_path
            && self
                .uploaded_snapshot_id
                .as_ref()
                .is_some_and(|id| !id.trim().is_empty())
    }

    // Validate one durable operation receipt against the journal operation rows.
    pub(super) fn validate_against(
        &self,
        journal: &RestoreApplyJournal,
    ) -> Result<(), RestoreApplyJournalError> {
        let operation = journal
            .operations
            .iter()
            .find(|operation| operation.sequence == self.sequence)
            .ok_or(RestoreApplyJournalError::OperationReceiptOperationNotFound(
                self.sequence,
            ))?;
        if operation.operation != self.operation
            || operation.source_canister != self.source_canister
            || operation.target_canister != self.target_canister
        {
            return Err(RestoreApplyJournalError::OperationReceiptMismatch {
                sequence: self.sequence,
            });
        }
        validate_apply_journal_nonempty(
            "operation_receipts[].updated_at",
            self.updated_at.as_deref().unwrap_or_default(),
        )?;
        let command =
            Self::validate_present("operation_receipts[].command", self.command.as_ref())?;
        validate_apply_journal_nonempty("operation_receipts[].command.program", &command.program)?;
        validate_apply_journal_nonempty("operation_receipts[].command.note", &command.note)?;
        if command.args.is_empty() {
            return Err(RestoreApplyJournalError::MissingField(
                "operation_receipts[].command.args",
            ));
        }
        validate_apply_journal_nonempty(
            "operation_receipts[].status",
            self.status.as_deref().unwrap_or_default(),
        )?;
        Self::validate_present("operation_receipts[].stdout", self.stdout.as_ref())?;
        Self::validate_present("operation_receipts[].stderr", self.stderr.as_ref())?;
        if self.operation == RestoreApplyOperationKind::UploadSnapshot {
            validate_apply_journal_nonempty(
                "operation_receipts[].source_snapshot_id",
                self.source_snapshot_id.as_deref().unwrap_or_default(),
            )?;
            validate_apply_journal_nonempty(
                "operation_receipts[].artifact_path",
                self.artifact_path.as_deref().unwrap_or_default(),
            )?;
            if self.outcome == RestoreApplyOperationReceiptOutcome::CommandCompleted {
                validate_apply_journal_nonempty(
                    "operation_receipts[].uploaded_snapshot_id",
                    self.uploaded_snapshot_id.as_deref().unwrap_or_default(),
                )?;
            }
        }
        if self.outcome == RestoreApplyOperationReceiptOutcome::CommandFailed {
            validate_apply_journal_nonempty(
                "operation_receipts[].failure_reason",
                self.failure_reason.as_deref().unwrap_or_default(),
            )?;
        }

        Ok(())
    }

    fn validate_present<'a, T>(
        field: &'static str,
        value: Option<&'a T>,
    ) -> Result<&'a T, RestoreApplyJournalError> {
        value.ok_or(RestoreApplyJournalError::MissingField(field))
    }
}

///
/// RestoreApplyOperationReceiptOutcome
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestoreApplyOperationReceiptOutcome {
    #[default]
    CommandCompleted,
    CommandFailed,
}

///
/// RestoreApplyOperationReceiptDetails
///

#[derive(Default)]
struct RestoreApplyOperationReceiptDetails {
    attempt: usize,
    updated_at: Option<String>,
    command: Option<RestoreApplyRunnerCommand>,
    status: Option<String>,
    stdout: Option<RestoreApplyCommandOutput>,
    stderr: Option<RestoreApplyCommandOutput>,
    failure_reason: Option<String>,
    uploaded_snapshot_id: Option<String>,
}

///
/// RestoreApplyCommandOutput
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyCommandOutput {
    pub text: String,
    pub truncated: bool,
    pub original_bytes: usize,
}

impl RestoreApplyCommandOutput {
    /// Build a bounded UTF-8-ish command output payload for durable receipts.
    #[must_use]
    pub(in crate::restore) fn from_bytes(bytes: &[u8], limit: usize) -> Self {
        let original_bytes = bytes.len();
        let start = original_bytes.saturating_sub(limit);
        Self {
            text: String::from_utf8_lossy(&bytes[start..]).to_string(),
            truncated: start > 0,
            original_bytes,
        }
    }
}

///
/// RestoreApplyCommandOutputPair
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(in crate::restore) struct RestoreApplyCommandOutputPair {
    pub stdout: RestoreApplyCommandOutput,
    pub stderr: RestoreApplyCommandOutput,
}

impl RestoreApplyCommandOutputPair {
    /// Build bounded stdout/stderr command output payloads.
    #[must_use]
    pub(in crate::restore) fn from_bytes(stdout: &[u8], stderr: &[u8], limit: usize) -> Self {
        Self {
            stdout: RestoreApplyCommandOutput::from_bytes(stdout, limit),
            stderr: RestoreApplyCommandOutput::from_bytes(stderr, limit),
        }
    }
}
