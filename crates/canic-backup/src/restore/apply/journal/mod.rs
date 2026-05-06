use super::{RestoreApplyDryRun, RestoreApplyDryRunOperation, RestoreApplyDryRunPhase};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error as ThisError;

mod commands;
mod receipts;
mod reports;

pub use commands::{
    RestoreApplyCommandConfig, RestoreApplyCommandPreview, RestoreApplyRunnerCommand,
};
pub use receipts::{
    RestoreApplyCommandOutput, RestoreApplyCommandOutputPair, RestoreApplyOperationReceipt,
    RestoreApplyOperationReceiptOutcome,
};
pub use reports::{
    RestoreApplyJournalReport, RestoreApplyJournalStatus, RestoreApplyNextOperation,
    RestoreApplyPendingSummary, RestoreApplyProgressSummary, RestoreApplyReportOperation,
    RestoreApplyReportOutcome,
};

///
/// RestoreApplyJournal
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyJournal {
    pub journal_version: u16,
    pub backup_id: String,
    pub ready: bool,
    pub blocked_reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_root: Option<String>,
    pub operation_count: usize,
    #[serde(default)]
    pub operation_counts: RestoreApplyOperationKindCounts,
    pub pending_operations: usize,
    pub ready_operations: usize,
    pub blocked_operations: usize,
    pub completed_operations: usize,
    pub failed_operations: usize,
    pub operations: Vec<RestoreApplyJournalOperation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operation_receipts: Vec<RestoreApplyOperationReceipt>,
}

impl RestoreApplyJournal {
    /// Build the initial no-mutation restore apply journal from a dry-run.
    #[must_use]
    pub fn from_dry_run(dry_run: &RestoreApplyDryRun) -> Self {
        let blocked_reasons = restore_apply_blocked_reasons(dry_run);
        let initial_state = if blocked_reasons.is_empty() {
            RestoreApplyOperationState::Ready
        } else {
            RestoreApplyOperationState::Blocked
        };
        let operations = dry_run
            .phases
            .iter()
            .flat_map(|phase| phase.operations.iter())
            .map(|operation| {
                RestoreApplyJournalOperation::from_dry_run_operation(
                    operation,
                    initial_state.clone(),
                    &blocked_reasons,
                )
            })
            .collect::<Vec<_>>();
        let ready_operations = operations
            .iter()
            .filter(|operation| operation.state == RestoreApplyOperationState::Ready)
            .count();
        let blocked_operations = operations
            .iter()
            .filter(|operation| operation.state == RestoreApplyOperationState::Blocked)
            .count();
        let operation_counts = RestoreApplyOperationKindCounts::from_operations(&operations);

        Self {
            journal_version: 1,
            backup_id: dry_run.backup_id.clone(),
            ready: blocked_reasons.is_empty(),
            blocked_reasons,
            backup_root: dry_run
                .artifact_validation
                .as_ref()
                .map(|validation| validation.backup_root.clone()),
            operation_count: operations.len(),
            operation_counts,
            pending_operations: 0,
            ready_operations,
            blocked_operations,
            completed_operations: 0,
            failed_operations: 0,
            operations,
            operation_receipts: Vec::new(),
        }
    }

    /// Validate the structural consistency of a restore apply journal.
    pub fn validate(&self) -> Result<(), RestoreApplyJournalError> {
        validate_apply_journal_version(self.journal_version)?;
        validate_apply_journal_nonempty("backup_id", &self.backup_id)?;
        if let Some(backup_root) = &self.backup_root {
            validate_apply_journal_nonempty("backup_root", backup_root)?;
        }
        validate_apply_journal_count(
            "operation_count",
            self.operation_count,
            self.operations.len(),
        )?;

        let state_counts = RestoreApplyJournalStateCounts::from_operations(&self.operations);
        let operation_counts = RestoreApplyOperationKindCounts::from_operations(&self.operations);
        self.operation_counts
            .validate_matches_if_supplied(&operation_counts)?;
        validate_apply_journal_count(
            "pending_operations",
            self.pending_operations,
            state_counts.pending,
        )?;
        validate_apply_journal_count(
            "ready_operations",
            self.ready_operations,
            state_counts.ready,
        )?;
        validate_apply_journal_count(
            "blocked_operations",
            self.blocked_operations,
            state_counts.blocked,
        )?;
        validate_apply_journal_count(
            "completed_operations",
            self.completed_operations,
            state_counts.completed,
        )?;
        validate_apply_journal_count(
            "failed_operations",
            self.failed_operations,
            state_counts.failed,
        )?;

        if self.ready && (!self.blocked_reasons.is_empty() || self.blocked_operations > 0) {
            return Err(RestoreApplyJournalError::ReadyJournalHasBlockingState);
        }

        validate_apply_journal_sequences(&self.operations)?;
        for operation in &self.operations {
            operation.validate()?;
        }
        for receipt in &self.operation_receipts {
            receipt.validate_against(self)?;
        }

        Ok(())
    }

    /// Summarize this apply journal for operators and automation.
    #[must_use]
    pub fn status(&self) -> RestoreApplyJournalStatus {
        RestoreApplyJournalStatus::from_journal(self)
    }

    /// Build an operator-oriented report from this apply journal.
    #[must_use]
    pub fn report(&self) -> RestoreApplyJournalReport {
        RestoreApplyJournalReport::from_journal(self)
    }

    /// Return the full next ready operation row, if one is available.
    #[must_use]
    pub fn next_ready_operation(&self) -> Option<&RestoreApplyJournalOperation> {
        self.operations
            .iter()
            .filter(|operation| operation.state == RestoreApplyOperationState::Ready)
            .min_by_key(|operation| operation.sequence)
    }

    /// Return the next ready or pending operation that controls runner progress.
    #[must_use]
    pub fn next_transition_operation(&self) -> Option<&RestoreApplyJournalOperation> {
        self.operations
            .iter()
            .filter(|operation| {
                matches!(
                    operation.state,
                    RestoreApplyOperationState::Ready
                        | RestoreApplyOperationState::Pending
                        | RestoreApplyOperationState::Failed
                )
            })
            .min_by_key(|operation| operation.sequence)
    }

    /// Render the next transitionable operation as a compact runner response.
    #[must_use]
    pub fn next_operation(&self) -> RestoreApplyNextOperation {
        RestoreApplyNextOperation::from_journal(self)
    }

    /// Render the next transitionable operation as a no-execute command preview.
    #[must_use]
    pub fn next_command_preview(&self) -> RestoreApplyCommandPreview {
        RestoreApplyCommandPreview::from_journal(self)
    }

    /// Render the next transitionable operation with a configured command preview.
    #[must_use]
    pub fn next_command_preview_with_config(
        &self,
        config: &RestoreApplyCommandConfig,
    ) -> RestoreApplyCommandPreview {
        RestoreApplyCommandPreview::from_journal_with_config(self, config)
    }

    /// Store one durable operation receipt/output and revalidate the journal.
    pub fn record_operation_receipt(
        &mut self,
        receipt: RestoreApplyOperationReceipt,
    ) -> Result<(), RestoreApplyJournalError> {
        self.operation_receipts.push(receipt);
        if let Err(error) = self.validate() {
            self.operation_receipts.pop();
            return Err(error);
        }

        Ok(())
    }

    /// Mark the next transitionable operation pending and refresh journal counts.
    pub fn mark_next_operation_pending(&mut self) -> Result<(), RestoreApplyJournalError> {
        self.mark_next_operation_pending_at(None)
    }

    /// Mark the next transitionable operation pending with an update marker.
    pub fn mark_next_operation_pending_at(
        &mut self,
        updated_at: Option<String>,
    ) -> Result<(), RestoreApplyJournalError> {
        let sequence = self
            .next_transition_sequence()
            .ok_or(RestoreApplyJournalError::NoTransitionableOperation)?;
        self.mark_operation_pending_at(sequence, updated_at)
    }

    /// Mark one restore apply operation pending and refresh journal counts.
    pub fn mark_operation_pending(
        &mut self,
        sequence: usize,
    ) -> Result<(), RestoreApplyJournalError> {
        self.mark_operation_pending_at(sequence, None)
    }

    /// Mark one restore apply operation pending with an update marker.
    pub fn mark_operation_pending_at(
        &mut self,
        sequence: usize,
        updated_at: Option<String>,
    ) -> Result<(), RestoreApplyJournalError> {
        self.transition_operation(
            sequence,
            RestoreApplyOperationState::Pending,
            Vec::new(),
            updated_at,
        )
    }

    /// Mark the current pending operation ready again and refresh counts.
    pub fn mark_next_operation_ready(&mut self) -> Result<(), RestoreApplyJournalError> {
        self.mark_next_operation_ready_at(None)
    }

    /// Mark the current pending operation ready again with an update marker.
    pub fn mark_next_operation_ready_at(
        &mut self,
        updated_at: Option<String>,
    ) -> Result<(), RestoreApplyJournalError> {
        let operation = self
            .next_transition_operation()
            .ok_or(RestoreApplyJournalError::NoTransitionableOperation)?;
        if operation.state != RestoreApplyOperationState::Pending {
            return Err(RestoreApplyJournalError::NoPendingOperation);
        }

        self.mark_operation_ready_at(operation.sequence, updated_at)
    }

    /// Mark one restore apply operation ready again and refresh journal counts.
    pub fn mark_operation_ready(
        &mut self,
        sequence: usize,
    ) -> Result<(), RestoreApplyJournalError> {
        self.mark_operation_ready_at(sequence, None)
    }

    /// Mark one restore apply operation ready again with an update marker.
    pub fn mark_operation_ready_at(
        &mut self,
        sequence: usize,
        updated_at: Option<String>,
    ) -> Result<(), RestoreApplyJournalError> {
        self.transition_operation(
            sequence,
            RestoreApplyOperationState::Ready,
            Vec::new(),
            updated_at,
        )
    }

    /// Retry one failed restore apply operation by moving it back to ready.
    pub fn retry_failed_operation_at(
        &mut self,
        sequence: usize,
        updated_at: Option<String>,
    ) -> Result<(), RestoreApplyJournalError> {
        self.transition_operation(
            sequence,
            RestoreApplyOperationState::Ready,
            Vec::new(),
            updated_at,
        )
    }

    /// Mark one restore apply operation completed and refresh journal counts.
    pub fn mark_operation_completed(
        &mut self,
        sequence: usize,
    ) -> Result<(), RestoreApplyJournalError> {
        self.mark_operation_completed_at(sequence, None)
    }

    /// Mark one restore apply operation completed with an update marker.
    pub fn mark_operation_completed_at(
        &mut self,
        sequence: usize,
        updated_at: Option<String>,
    ) -> Result<(), RestoreApplyJournalError> {
        self.transition_operation(
            sequence,
            RestoreApplyOperationState::Completed,
            Vec::new(),
            updated_at,
        )
    }

    /// Mark one restore apply operation failed and refresh journal counts.
    pub fn mark_operation_failed(
        &mut self,
        sequence: usize,
        reason: String,
    ) -> Result<(), RestoreApplyJournalError> {
        self.mark_operation_failed_at(sequence, reason, None)
    }

    /// Mark one restore apply operation failed with an update marker.
    pub fn mark_operation_failed_at(
        &mut self,
        sequence: usize,
        reason: String,
        updated_at: Option<String>,
    ) -> Result<(), RestoreApplyJournalError> {
        if reason.trim().is_empty() {
            return Err(RestoreApplyJournalError::FailureReasonRequired(sequence));
        }

        self.transition_operation(
            sequence,
            RestoreApplyOperationState::Failed,
            vec![reason],
            updated_at,
        )
    }

    // Apply one legal operation state transition and revalidate the journal.
    fn transition_operation(
        &mut self,
        sequence: usize,
        next_state: RestoreApplyOperationState,
        blocking_reasons: Vec<String>,
        updated_at: Option<String>,
    ) -> Result<(), RestoreApplyJournalError> {
        let index = self
            .operations
            .iter()
            .position(|operation| operation.sequence == sequence)
            .ok_or(RestoreApplyJournalError::OperationNotFound(sequence))?;
        let operation = &self.operations[index];

        if !operation.can_transition_to(&next_state) {
            return Err(RestoreApplyJournalError::InvalidOperationTransition {
                sequence,
                from: operation.state.clone(),
                to: next_state,
            });
        }

        self.validate_operation_transition_order(operation, &next_state)?;

        let operation = &mut self.operations[index];
        operation.state = next_state;
        operation.blocking_reasons = blocking_reasons;
        operation.state_updated_at = updated_at;
        self.refresh_operation_counts();
        self.validate()
    }

    // Ensure fresh operation transitions advance in journal order.
    fn validate_operation_transition_order(
        &self,
        operation: &RestoreApplyJournalOperation,
        next_state: &RestoreApplyOperationState,
    ) -> Result<(), RestoreApplyJournalError> {
        if operation.state == *next_state {
            return Ok(());
        }

        let next_sequence = self
            .next_transition_sequence()
            .ok_or(RestoreApplyJournalError::NoTransitionableOperation)?;

        if operation.sequence == next_sequence {
            return Ok(());
        }

        Err(RestoreApplyJournalError::OutOfOrderOperationTransition {
            requested: operation.sequence,
            next: next_sequence,
        })
    }

    // Return the next operation sequence that can be advanced by a runner.
    fn next_transition_sequence(&self) -> Option<usize> {
        self.next_transition_operation()
            .map(|operation| operation.sequence)
    }

    // Recompute operation counts after a journal operation state change.
    fn refresh_operation_counts(&mut self) {
        let state_counts = RestoreApplyJournalStateCounts::from_operations(&self.operations);
        self.operation_count = self.operations.len();
        self.operation_counts = RestoreApplyOperationKindCounts::from_operations(&self.operations);
        self.pending_operations = state_counts.pending;
        self.ready_operations = state_counts.ready;
        self.blocked_operations = state_counts.blocked;
        self.completed_operations = state_counts.completed;
        self.failed_operations = state_counts.failed;
    }

    // Return whether this journal carried a persisted operation-kind receipt.
    pub(super) const fn operation_counts_supplied(&self) -> bool {
        !self.operation_counts.is_empty() || self.operations.is_empty()
    }

    // Return whether every planned operation has completed.
    pub(super) const fn is_complete(&self) -> bool {
        self.operation_count > 0 && self.completed_operations == self.operation_count
    }

    // Recompute operation-kind counts from concrete operation rows.
    pub(super) fn operation_kind_counts(&self) -> RestoreApplyOperationKindCounts {
        RestoreApplyOperationKindCounts::from_operations(&self.operations)
    }

    // Find the uploaded target snapshot ID required by one load operation.
    pub(super) fn uploaded_snapshot_id_for_load(
        &self,
        load: &RestoreApplyJournalOperation,
    ) -> Option<&str> {
        self.operation_receipts
            .iter()
            .find(|receipt| {
                receipt.matches_load_operation(load)
                    && self.operations.iter().any(|operation| {
                        operation.sequence == receipt.sequence
                            && operation.operation == RestoreApplyOperationKind::UploadSnapshot
                            && operation.state == RestoreApplyOperationState::Completed
                    })
            })
            .and_then(|receipt| receipt.uploaded_snapshot_id.as_deref())
    }
}

// Validate the supported restore apply journal format version.
const fn validate_apply_journal_version(version: u16) -> Result<(), RestoreApplyJournalError> {
    if version == 1 {
        return Ok(());
    }

    Err(RestoreApplyJournalError::UnsupportedVersion(version))
}

// Validate required nonempty restore apply journal fields.
fn validate_apply_journal_nonempty(
    field: &'static str,
    value: &str,
) -> Result<(), RestoreApplyJournalError> {
    if !value.trim().is_empty() {
        return Ok(());
    }

    Err(RestoreApplyJournalError::MissingField(field))
}

// Validate one reported restore apply journal count.
const fn validate_apply_journal_count(
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

// Validate operation sequence values are unique and contiguous from zero.
fn validate_apply_journal_sequences(
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

///
/// RestoreApplyJournalStateCounts
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct RestoreApplyJournalStateCounts {
    pending: usize,
    ready: usize,
    blocked: usize,
    completed: usize,
    failed: usize,
}

impl RestoreApplyJournalStateCounts {
    // Count operation states from concrete journal operation rows.
    fn from_operations(operations: &[RestoreApplyJournalOperation]) -> Self {
        let mut counts = Self::default();
        for operation in operations {
            match operation.state {
                RestoreApplyOperationState::Pending => counts.pending += 1,
                RestoreApplyOperationState::Ready => counts.ready += 1,
                RestoreApplyOperationState::Blocked => counts.blocked += 1,
                RestoreApplyOperationState::Completed => counts.completed += 1,
                RestoreApplyOperationState::Failed => counts.failed += 1,
            }
        }
        counts
    }
}

///
/// RestoreApplyOperationKindCounts
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyOperationKindCounts {
    pub snapshot_uploads: usize,
    pub snapshot_loads: usize,
    pub code_reinstalls: usize,
    pub member_verifications: usize,
    pub fleet_verifications: usize,
    pub verification_operations: usize,
}

impl RestoreApplyOperationKindCounts {
    /// Count restore apply journal operations by runner operation kind.
    #[must_use]
    pub fn from_operations(operations: &[RestoreApplyJournalOperation]) -> Self {
        let mut counts = Self::default();
        for operation in operations {
            counts.record(&operation.operation);
        }
        counts
    }

    /// Validate this count object against concrete operations when it was supplied.
    pub fn validate_matches_if_supplied(
        &self,
        expected: &Self,
    ) -> Result<(), RestoreApplyJournalError> {
        if self.is_empty() && !expected.is_empty() {
            return Ok(());
        }

        validate_apply_journal_count(
            "operation_counts.snapshot_uploads",
            self.snapshot_uploads,
            expected.snapshot_uploads,
        )?;
        validate_apply_journal_count(
            "operation_counts.snapshot_loads",
            self.snapshot_loads,
            expected.snapshot_loads,
        )?;
        validate_apply_journal_count(
            "operation_counts.code_reinstalls",
            self.code_reinstalls,
            expected.code_reinstalls,
        )?;
        validate_apply_journal_count(
            "operation_counts.member_verifications",
            self.member_verifications,
            expected.member_verifications,
        )?;
        validate_apply_journal_count(
            "operation_counts.fleet_verifications",
            self.fleet_verifications,
            expected.fleet_verifications,
        )?;
        validate_apply_journal_count(
            "operation_counts.verification_operations",
            self.verification_operations,
            expected.verification_operations,
        )
    }

    // Return whether no operation-kind counts are present.
    const fn is_empty(&self) -> bool {
        self.snapshot_uploads == 0
            && self.snapshot_loads == 0
            && self.code_reinstalls == 0
            && self.member_verifications == 0
            && self.fleet_verifications == 0
            && self.verification_operations == 0
    }

    /// Count restore apply dry-run operations by runner operation kind.
    #[must_use]
    pub fn from_dry_run_phases(phases: &[RestoreApplyDryRunPhase]) -> Self {
        let mut counts = Self::default();
        for operation in phases.iter().flat_map(|phase| {
            phase
                .operations
                .iter()
                .map(|operation| &operation.operation)
        }) {
            counts.record(operation);
        }
        counts
    }

    // Record one operation kind in the aggregate count object.
    const fn record(&mut self, operation: &RestoreApplyOperationKind) {
        match operation {
            RestoreApplyOperationKind::UploadSnapshot => self.snapshot_uploads += 1,
            RestoreApplyOperationKind::LoadSnapshot => self.snapshot_loads += 1,
            RestoreApplyOperationKind::ReinstallCode => self.code_reinstalls += 1,
            RestoreApplyOperationKind::VerifyMember => {
                self.member_verifications += 1;
                self.verification_operations += 1;
            }
            RestoreApplyOperationKind::VerifyFleet => {
                self.fleet_verifications += 1;
                self.verification_operations += 1;
            }
        }
    }
}

// Explain why an apply journal is blocked before mutation is allowed.
fn restore_apply_blocked_reasons(dry_run: &RestoreApplyDryRun) -> Vec<String> {
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

///
/// RestoreApplyJournalOperation
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyJournalOperation {
    pub sequence: usize,
    pub operation: RestoreApplyOperationKind,
    pub state: RestoreApplyOperationState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_updated_at: Option<String>,
    pub blocking_reasons: Vec<String>,
    pub restore_group: u16,
    pub phase_order: usize,
    pub source_canister: String,
    pub target_canister: String,
    pub role: String,
    pub snapshot_id: Option<String>,
    pub artifact_path: Option<String>,
    pub verification_kind: Option<String>,
    pub verification_method: Option<String>,
}

impl RestoreApplyJournalOperation {
    // Build one initial journal operation from the dry-run operation row.
    fn from_dry_run_operation(
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
            restore_group: operation.restore_group,
            phase_order: operation.phase_order,
            source_canister: operation.source_canister.clone(),
            target_canister: operation.target_canister.clone(),
            role: operation.role.clone(),
            snapshot_id: operation.snapshot_id.clone(),
            artifact_path: operation.artifact_path.clone(),
            verification_kind: operation.verification_kind.clone(),
            verification_method: operation.verification_method.clone(),
        }
    }

    // Validate one restore apply journal operation row.
    fn validate(&self) -> Result<(), RestoreApplyJournalError> {
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

    // Validate fields required by the operation kind before runner command rendering.
    fn validate_operation_fields(&self) -> Result<(), RestoreApplyJournalError> {
        match self.operation {
            RestoreApplyOperationKind::UploadSnapshot => self
                .validate_required_field("operations[].artifact_path", self.artifact_path.as_ref())
                .map(|_| ()),
            RestoreApplyOperationKind::LoadSnapshot => self
                .validate_required_field("operations[].snapshot_id", self.snapshot_id.as_ref())
                .map(|_| ()),
            RestoreApplyOperationKind::ReinstallCode => Ok(()),
            RestoreApplyOperationKind::VerifyMember | RestoreApplyOperationKind::VerifyFleet => {
                let kind = self.validate_required_field(
                    "operations[].verification_kind",
                    self.verification_kind.as_ref(),
                )?;
                if kind == "status" {
                    return Ok(());
                }
                self.validate_required_field(
                    "operations[].verification_method",
                    self.verification_method.as_ref(),
                )
                .map(|_| ())
            }
        }
    }

    // Return one required optional field after checking it is present and nonempty.
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

    // Decide whether an operation can move to the requested next state.
    const fn can_transition_to(&self, next_state: &RestoreApplyOperationState) -> bool {
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

#[derive(Debug, ThisError)]
pub enum RestoreApplyJournalError {
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

    #[error("restore apply journal operation {0} was not found")]
    OperationNotFound(usize),

    #[error("restore apply journal operation {sequence} cannot transition from {from:?} to {to:?}")]
    InvalidOperationTransition {
        sequence: usize,
        from: RestoreApplyOperationState,
        to: RestoreApplyOperationState,
    },

    #[error("failed restore apply journal operation {0} requires a reason")]
    FailureReasonRequired(usize),

    #[error("restore apply journal has no operation that can be advanced")]
    NoTransitionableOperation,

    #[error("restore apply journal has no pending operation to release")]
    NoPendingOperation,

    #[error("restore apply journal operation {requested} cannot advance before operation {next}")]
    OutOfOrderOperationTransition { requested: usize, next: usize },

    #[error("restore apply journal receipt references missing operation {0}")]
    OperationReceiptOperationNotFound(usize),

    #[error("restore apply journal receipt does not match operation {sequence}")]
    OperationReceiptMismatch { sequence: usize },
}
///
/// RestoreApplyOperationKind
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestoreApplyOperationKind {
    UploadSnapshot,
    LoadSnapshot,
    ReinstallCode,
    VerifyMember,
    VerifyFleet,
}
