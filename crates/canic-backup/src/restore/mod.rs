use crate::{
    artifacts::{ArtifactChecksum, ArtifactChecksumError},
    manifest::{
        FleetBackupManifest, FleetMember, IdentityMode, ManifestDesignConformanceReport,
        ManifestValidationError, SourceSnapshot, VerificationCheck, VerificationPlan,
    },
};
use candid::Principal;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
    str::FromStr,
};
use thiserror::Error as ThisError;

///
/// RestoreMapping
///

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RestoreMapping {
    pub members: Vec<RestoreMappingEntry>,
}

impl RestoreMapping {
    /// Resolve the target canister for one source member.
    fn target_for(&self, source_canister: &str) -> Option<&str> {
        self.members
            .iter()
            .find(|entry| entry.source_canister == source_canister)
            .map(|entry| entry.target_canister.as_str())
    }
}

///
/// RestoreMappingEntry
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RestoreMappingEntry {
    pub source_canister: String,
    pub target_canister: String,
}

///
/// RestorePlan
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestorePlan {
    pub backup_id: String,
    pub source_environment: String,
    pub source_root_canister: String,
    pub topology_hash: String,
    pub member_count: usize,
    pub identity_summary: RestoreIdentitySummary,
    pub snapshot_summary: RestoreSnapshotSummary,
    pub verification_summary: RestoreVerificationSummary,
    pub readiness_summary: RestoreReadinessSummary,
    pub operation_summary: RestoreOperationSummary,
    pub ordering_summary: RestoreOrderingSummary,
    #[serde(default)]
    pub design_conformance: Option<ManifestDesignConformanceReport>,
    #[serde(default)]
    pub fleet_verification_checks: Vec<VerificationCheck>,
    pub phases: Vec<RestorePhase>,
}

impl RestorePlan {
    /// Return all planned members in execution order.
    #[must_use]
    pub fn ordered_members(&self) -> Vec<&RestorePlanMember> {
        self.phases
            .iter()
            .flat_map(|phase| phase.members.iter())
            .collect()
    }
}

///
/// RestoreStatus
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreStatus {
    pub status_version: u16,
    pub backup_id: String,
    pub source_environment: String,
    pub source_root_canister: String,
    pub topology_hash: String,
    pub ready: bool,
    pub readiness_reasons: Vec<String>,
    pub verification_required: bool,
    pub member_count: usize,
    pub phase_count: usize,
    #[serde(default)]
    pub planned_snapshot_uploads: usize,
    pub planned_snapshot_loads: usize,
    pub planned_code_reinstalls: usize,
    pub planned_verification_checks: usize,
    #[serde(default)]
    pub planned_operations: usize,
    pub phases: Vec<RestoreStatusPhase>,
}

impl RestoreStatus {
    /// Build the initial no-mutation restore status from a computed plan.
    #[must_use]
    pub fn from_plan(plan: &RestorePlan) -> Self {
        Self {
            status_version: 1,
            backup_id: plan.backup_id.clone(),
            source_environment: plan.source_environment.clone(),
            source_root_canister: plan.source_root_canister.clone(),
            topology_hash: plan.topology_hash.clone(),
            ready: plan.readiness_summary.ready,
            readiness_reasons: plan.readiness_summary.reasons.clone(),
            verification_required: plan.verification_summary.verification_required,
            member_count: plan.member_count,
            phase_count: plan.ordering_summary.phase_count,
            planned_snapshot_uploads: plan
                .operation_summary
                .effective_planned_snapshot_uploads(plan.member_count),
            planned_snapshot_loads: plan.operation_summary.planned_snapshot_loads,
            planned_code_reinstalls: plan.operation_summary.planned_code_reinstalls,
            planned_verification_checks: plan.operation_summary.planned_verification_checks,
            planned_operations: plan
                .operation_summary
                .effective_planned_operations(plan.member_count),
            phases: plan
                .phases
                .iter()
                .map(RestoreStatusPhase::from_plan_phase)
                .collect(),
        }
    }
}

///
/// RestoreStatusPhase
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreStatusPhase {
    pub restore_group: u16,
    pub members: Vec<RestoreStatusMember>,
}

impl RestoreStatusPhase {
    // Build one status phase from one planned restore phase.
    fn from_plan_phase(phase: &RestorePhase) -> Self {
        Self {
            restore_group: phase.restore_group,
            members: phase
                .members
                .iter()
                .map(RestoreStatusMember::from_plan_member)
                .collect(),
        }
    }
}

///
/// RestoreStatusMember
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreStatusMember {
    pub source_canister: String,
    pub target_canister: String,
    pub role: String,
    pub restore_group: u16,
    pub phase_order: usize,
    pub snapshot_id: String,
    pub artifact_path: String,
    pub state: RestoreMemberState,
}

impl RestoreStatusMember {
    // Build one member status row from one planned restore member.
    fn from_plan_member(member: &RestorePlanMember) -> Self {
        Self {
            source_canister: member.source_canister.clone(),
            target_canister: member.target_canister.clone(),
            role: member.role.clone(),
            restore_group: member.restore_group,
            phase_order: member.phase_order,
            snapshot_id: member.source_snapshot.snapshot_id.clone(),
            artifact_path: member.source_snapshot.artifact_path.clone(),
            state: RestoreMemberState::Planned,
        }
    }
}

///
/// RestoreMemberState
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestoreMemberState {
    Planned,
}

///
/// RestoreApplyDryRun
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyDryRun {
    pub dry_run_version: u16,
    pub backup_id: String,
    pub ready: bool,
    pub readiness_reasons: Vec<String>,
    pub member_count: usize,
    pub phase_count: usize,
    pub status_supplied: bool,
    #[serde(default)]
    pub planned_snapshot_uploads: usize,
    pub planned_snapshot_loads: usize,
    pub planned_code_reinstalls: usize,
    pub planned_verification_checks: usize,
    #[serde(default)]
    pub planned_operations: usize,
    pub rendered_operations: usize,
    #[serde(default)]
    pub operation_counts: RestoreApplyOperationKindCounts,
    pub artifact_validation: Option<RestoreApplyArtifactValidation>,
    pub phases: Vec<RestoreApplyDryRunPhase>,
}

impl RestoreApplyDryRun {
    /// Build a no-mutation apply dry-run after validating optional status identity.
    pub fn try_from_plan(
        plan: &RestorePlan,
        status: Option<&RestoreStatus>,
    ) -> Result<Self, RestoreApplyDryRunError> {
        if let Some(status) = status {
            validate_restore_status_matches_plan(plan, status)?;
        }

        Ok(Self::from_validated_plan(plan, status))
    }

    /// Build an apply dry-run and verify all referenced artifacts under a backup root.
    pub fn try_from_plan_with_artifacts(
        plan: &RestorePlan,
        status: Option<&RestoreStatus>,
        backup_root: &Path,
    ) -> Result<Self, RestoreApplyDryRunError> {
        let mut dry_run = Self::try_from_plan(plan, status)?;
        dry_run.artifact_validation = Some(validate_restore_apply_artifacts(plan, backup_root)?);
        Ok(dry_run)
    }

    // Build a no-mutation apply dry-run after any supplied status is validated.
    fn from_validated_plan(plan: &RestorePlan, status: Option<&RestoreStatus>) -> Self {
        let mut next_sequence = 0;
        let phases = plan
            .phases
            .iter()
            .map(|phase| RestoreApplyDryRunPhase::from_plan_phase(phase, &mut next_sequence))
            .collect::<Vec<_>>();
        let mut phases = phases;
        append_fleet_verification_operations(plan, &mut phases, &mut next_sequence);
        let rendered_operations = phases
            .iter()
            .map(|phase| phase.operations.len())
            .sum::<usize>();
        let operation_counts = RestoreApplyOperationKindCounts::from_dry_run_phases(&phases);

        Self {
            dry_run_version: 1,
            backup_id: plan.backup_id.clone(),
            ready: status.map_or(plan.readiness_summary.ready, |status| status.ready),
            readiness_reasons: status.map_or_else(
                || plan.readiness_summary.reasons.clone(),
                |status| status.readiness_reasons.clone(),
            ),
            member_count: plan.member_count,
            phase_count: plan.ordering_summary.phase_count,
            status_supplied: status.is_some(),
            planned_snapshot_uploads: plan
                .operation_summary
                .effective_planned_snapshot_uploads(plan.member_count),
            planned_snapshot_loads: plan.operation_summary.planned_snapshot_loads,
            planned_code_reinstalls: plan.operation_summary.planned_code_reinstalls,
            planned_verification_checks: plan.operation_summary.planned_verification_checks,
            planned_operations: plan
                .operation_summary
                .effective_planned_operations(plan.member_count),
            rendered_operations,
            operation_counts,
            artifact_validation: None,
            phases,
        }
    }
}

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
    const fn operation_counts_supplied(&self) -> bool {
        !self.operation_counts.is_empty() || self.operations.is_empty()
    }

    // Find the uploaded target snapshot ID required by one load operation.
    fn uploaded_snapshot_id_for_load(&self, load: &RestoreApplyJournalOperation) -> Option<&str> {
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
    /// Build a completed upload receipt from the uploaded target-side snapshot ID.
    #[must_use]
    pub fn completed_upload(
        operation: &RestoreApplyJournalOperation,
        uploaded_snapshot_id: String,
    ) -> Self {
        Self {
            sequence: operation.sequence,
            operation: RestoreApplyOperationKind::UploadSnapshot,
            outcome: RestoreApplyOperationReceiptOutcome::CommandCompleted,
            source_canister: operation.source_canister.clone(),
            target_canister: operation.target_canister.clone(),
            attempt: 1,
            updated_at: None,
            command: None,
            status: None,
            stdout: None,
            stderr: None,
            failure_reason: None,
            source_snapshot_id: operation.snapshot_id.clone(),
            artifact_path: operation.artifact_path.clone(),
            uploaded_snapshot_id: Some(uploaded_snapshot_id),
        }
    }

    /// Build a durable completed-command receipt for the apply journal.
    #[must_use]
    pub fn command_completed(
        operation: &RestoreApplyJournalOperation,
        command: RestoreApplyRunnerCommand,
        status: String,
        updated_at: Option<String>,
        output: RestoreApplyCommandOutputPair,
        attempt: usize,
        uploaded_snapshot_id: Option<String>,
    ) -> Self {
        Self {
            sequence: operation.sequence,
            operation: operation.operation.clone(),
            outcome: RestoreApplyOperationReceiptOutcome::CommandCompleted,
            source_canister: operation.source_canister.clone(),
            target_canister: operation.target_canister.clone(),
            attempt,
            updated_at,
            command: Some(command),
            status: Some(status),
            stdout: Some(output.stdout),
            stderr: Some(output.stderr),
            failure_reason: None,
            source_snapshot_id: operation.snapshot_id.clone(),
            artifact_path: operation.artifact_path.clone(),
            uploaded_snapshot_id,
        }
    }

    /// Build a durable failed-command receipt for the apply journal.
    #[must_use]
    pub fn command_failed(
        operation: &RestoreApplyJournalOperation,
        command: RestoreApplyRunnerCommand,
        status: String,
        updated_at: Option<String>,
        output: RestoreApplyCommandOutputPair,
        attempt: usize,
        failure_reason: String,
    ) -> Self {
        Self {
            sequence: operation.sequence,
            operation: operation.operation.clone(),
            outcome: RestoreApplyOperationReceiptOutcome::CommandFailed,
            source_canister: operation.source_canister.clone(),
            target_canister: operation.target_canister.clone(),
            attempt,
            updated_at,
            command: Some(command),
            status: Some(status),
            stdout: Some(output.stdout),
            stderr: Some(output.stderr),
            failure_reason: Some(failure_reason),
            source_snapshot_id: operation.snapshot_id.clone(),
            artifact_path: operation.artifact_path.clone(),
            uploaded_snapshot_id: None,
        }
    }

    // Return whether this upload receipt satisfies one later load operation.
    fn matches_load_operation(&self, load: &RestoreApplyJournalOperation) -> bool {
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
    fn validate_against(
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
    pub fn from_bytes(bytes: &[u8], limit: usize) -> Self {
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
pub struct RestoreApplyCommandOutputPair {
    pub stdout: RestoreApplyCommandOutput,
    pub stderr: RestoreApplyCommandOutput,
}

impl RestoreApplyCommandOutputPair {
    /// Build bounded stdout/stderr command output payloads.
    #[must_use]
    pub fn from_bytes(stdout: &[u8], stderr: &[u8], limit: usize) -> Self {
        Self {
            stdout: RestoreApplyCommandOutput::from_bytes(stdout, limit),
            stderr: RestoreApplyCommandOutput::from_bytes(stderr, limit),
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
/// RestoreApplyJournalStatus
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyJournalStatus {
    pub status_version: u16,
    pub backup_id: String,
    pub ready: bool,
    pub complete: bool,
    pub blocked_reasons: Vec<String>,
    pub operation_count: usize,
    #[serde(default)]
    pub operation_counts: RestoreApplyOperationKindCounts,
    pub operation_counts_supplied: bool,
    pub progress: RestoreApplyProgressSummary,
    pub pending_summary: RestoreApplyPendingSummary,
    pub pending_operations: usize,
    pub ready_operations: usize,
    pub blocked_operations: usize,
    pub completed_operations: usize,
    pub failed_operations: usize,
    pub next_ready_sequence: Option<usize>,
    pub next_ready_operation: Option<RestoreApplyOperationKind>,
    pub next_transition_sequence: Option<usize>,
    pub next_transition_state: Option<RestoreApplyOperationState>,
    pub next_transition_operation: Option<RestoreApplyOperationKind>,
    pub next_transition_updated_at: Option<String>,
}

impl RestoreApplyJournalStatus {
    /// Build a compact status projection from a restore apply journal.
    #[must_use]
    pub fn from_journal(journal: &RestoreApplyJournal) -> Self {
        let next_ready = journal.next_ready_operation();
        let next_transition = journal.next_transition_operation();

        Self {
            status_version: 1,
            backup_id: journal.backup_id.clone(),
            ready: journal.ready,
            complete: journal.operation_count > 0
                && journal.completed_operations == journal.operation_count,
            blocked_reasons: journal.blocked_reasons.clone(),
            operation_count: journal.operation_count,
            operation_counts: RestoreApplyOperationKindCounts::from_operations(&journal.operations),
            operation_counts_supplied: journal.operation_counts_supplied(),
            progress: RestoreApplyProgressSummary::from_journal(journal),
            pending_summary: RestoreApplyPendingSummary::from_journal(journal),
            pending_operations: journal.pending_operations,
            ready_operations: journal.ready_operations,
            blocked_operations: journal.blocked_operations,
            completed_operations: journal.completed_operations,
            failed_operations: journal.failed_operations,
            next_ready_sequence: next_ready.map(|operation| operation.sequence),
            next_ready_operation: next_ready.map(|operation| operation.operation.clone()),
            next_transition_sequence: next_transition.map(|operation| operation.sequence),
            next_transition_state: next_transition.map(|operation| operation.state.clone()),
            next_transition_operation: next_transition.map(|operation| operation.operation.clone()),
            next_transition_updated_at: next_transition
                .and_then(|operation| operation.state_updated_at.clone()),
        }
    }
}

///
/// RestoreApplyJournalReport
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "apply reports intentionally expose stable JSON flags for operators and CI"
)]
pub struct RestoreApplyJournalReport {
    pub report_version: u16,
    pub backup_id: String,
    pub outcome: RestoreApplyReportOutcome,
    pub attention_required: bool,
    pub ready: bool,
    pub complete: bool,
    pub blocked_reasons: Vec<String>,
    pub operation_count: usize,
    #[serde(default)]
    pub operation_counts: RestoreApplyOperationKindCounts,
    pub operation_counts_supplied: bool,
    pub progress: RestoreApplyProgressSummary,
    pub pending_summary: RestoreApplyPendingSummary,
    pub pending_operations: usize,
    pub ready_operations: usize,
    pub blocked_operations: usize,
    pub completed_operations: usize,
    pub failed_operations: usize,
    pub next_transition: Option<RestoreApplyReportOperation>,
    pub pending: Vec<RestoreApplyReportOperation>,
    pub failed: Vec<RestoreApplyReportOperation>,
    pub blocked: Vec<RestoreApplyReportOperation>,
}

impl RestoreApplyJournalReport {
    /// Build a compact operator report from a restore apply journal.
    #[must_use]
    pub fn from_journal(journal: &RestoreApplyJournal) -> Self {
        let complete =
            journal.operation_count > 0 && journal.completed_operations == journal.operation_count;
        let outcome = RestoreApplyReportOutcome::from_journal(journal, complete);
        let pending = report_operations_with_state(journal, RestoreApplyOperationState::Pending);
        let failed = report_operations_with_state(journal, RestoreApplyOperationState::Failed);
        let blocked = report_operations_with_state(journal, RestoreApplyOperationState::Blocked);

        Self {
            report_version: 1,
            backup_id: journal.backup_id.clone(),
            outcome: outcome.clone(),
            attention_required: outcome.attention_required(),
            ready: journal.ready,
            complete,
            blocked_reasons: journal.blocked_reasons.clone(),
            operation_count: journal.operation_count,
            operation_counts: RestoreApplyOperationKindCounts::from_operations(&journal.operations),
            operation_counts_supplied: journal.operation_counts_supplied(),
            progress: RestoreApplyProgressSummary::from_journal(journal),
            pending_summary: RestoreApplyPendingSummary::from_journal(journal),
            pending_operations: journal.pending_operations,
            ready_operations: journal.ready_operations,
            blocked_operations: journal.blocked_operations,
            completed_operations: journal.completed_operations,
            failed_operations: journal.failed_operations,
            next_transition: journal
                .next_transition_operation()
                .map(RestoreApplyReportOperation::from_journal_operation),
            pending,
            failed,
            blocked,
        }
    }
}

///
/// RestoreApplyPendingSummary
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyPendingSummary {
    pub pending_operations: usize,
    pub pending_operation_available: bool,
    pub pending_sequence: Option<usize>,
    pub pending_operation: Option<RestoreApplyOperationKind>,
    pub pending_updated_at: Option<String>,
    pub pending_updated_at_known: bool,
}

impl RestoreApplyPendingSummary {
    /// Build a compact pending-operation summary from a restore apply journal.
    #[must_use]
    pub fn from_journal(journal: &RestoreApplyJournal) -> Self {
        let pending = journal
            .operations
            .iter()
            .filter(|operation| operation.state == RestoreApplyOperationState::Pending)
            .min_by_key(|operation| operation.sequence);
        let pending_updated_at = pending.and_then(|operation| operation.state_updated_at.clone());
        let pending_updated_at_known = pending_updated_at
            .as_deref()
            .is_some_and(known_state_update_marker);

        Self {
            pending_operations: journal.pending_operations,
            pending_operation_available: pending.is_some(),
            pending_sequence: pending.map(|operation| operation.sequence),
            pending_operation: pending.map(|operation| operation.operation.clone()),
            pending_updated_at,
            pending_updated_at_known,
        }
    }
}

// Return whether a journal update marker can be compared by automation.
fn known_state_update_marker(value: &str) -> bool {
    !value.trim().is_empty() && value != "unknown"
}

///
/// RestoreApplyProgressSummary
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyProgressSummary {
    pub operation_count: usize,
    pub completed_operations: usize,
    pub remaining_operations: usize,
    pub transitionable_operations: usize,
    pub attention_operations: usize,
    pub completion_basis_points: usize,
}

impl RestoreApplyProgressSummary {
    /// Build a compact progress summary from restore apply journal counters.
    #[must_use]
    pub const fn from_journal(journal: &RestoreApplyJournal) -> Self {
        let remaining_operations = journal
            .operation_count
            .saturating_sub(journal.completed_operations);
        let transitionable_operations = journal.ready_operations + journal.pending_operations;
        let attention_operations =
            journal.pending_operations + journal.blocked_operations + journal.failed_operations;
        let completion_basis_points =
            completion_basis_points(journal.completed_operations, journal.operation_count);

        Self {
            operation_count: journal.operation_count,
            completed_operations: journal.completed_operations,
            remaining_operations,
            transitionable_operations,
            attention_operations,
            completion_basis_points,
        }
    }
}

// Return completion as basis points so JSON stays deterministic and integer-only.
const fn completion_basis_points(completed_operations: usize, operation_count: usize) -> usize {
    if operation_count == 0 {
        return 0;
    }

    completed_operations.saturating_mul(10_000) / operation_count
}

///
/// RestoreApplyReportOutcome
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestoreApplyReportOutcome {
    Empty,
    Complete,
    Failed,
    Blocked,
    Pending,
    InProgress,
}

impl RestoreApplyReportOutcome {
    // Classify the journal into one high-level operator outcome.
    const fn from_journal(journal: &RestoreApplyJournal, complete: bool) -> Self {
        if journal.operation_count == 0 {
            return Self::Empty;
        }
        if complete {
            return Self::Complete;
        }
        if journal.failed_operations > 0 {
            return Self::Failed;
        }
        if !journal.ready || journal.blocked_operations > 0 {
            return Self::Blocked;
        }
        if journal.pending_operations > 0 {
            return Self::Pending;
        }
        Self::InProgress
    }

    // Return whether this outcome needs operator or automation attention.
    const fn attention_required(&self) -> bool {
        matches!(self, Self::Failed | Self::Blocked | Self::Pending)
    }
}

///
/// RestoreApplyReportOperation
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyReportOperation {
    pub sequence: usize,
    pub operation: RestoreApplyOperationKind,
    pub state: RestoreApplyOperationState,
    pub restore_group: u16,
    pub phase_order: usize,
    pub role: String,
    pub source_canister: String,
    pub target_canister: String,
    pub state_updated_at: Option<String>,
    pub reasons: Vec<String>,
}

impl RestoreApplyReportOperation {
    // Build one compact report row from one journal operation.
    fn from_journal_operation(operation: &RestoreApplyJournalOperation) -> Self {
        Self {
            sequence: operation.sequence,
            operation: operation.operation.clone(),
            state: operation.state.clone(),
            restore_group: operation.restore_group,
            phase_order: operation.phase_order,
            role: operation.role.clone(),
            source_canister: operation.source_canister.clone(),
            target_canister: operation.target_canister.clone(),
            state_updated_at: operation.state_updated_at.clone(),
            reasons: operation.blocking_reasons.clone(),
        }
    }
}

// Return compact report rows for operations in one state.
fn report_operations_with_state(
    journal: &RestoreApplyJournal,
    state: RestoreApplyOperationState,
) -> Vec<RestoreApplyReportOperation> {
    journal
        .operations
        .iter()
        .filter(|operation| operation.state == state)
        .map(RestoreApplyReportOperation::from_journal_operation)
        .collect()
}

///
/// RestoreApplyNextOperation
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyNextOperation {
    pub response_version: u16,
    pub backup_id: String,
    pub ready: bool,
    pub complete: bool,
    pub operation_available: bool,
    pub blocked_reasons: Vec<String>,
    pub operation: Option<RestoreApplyJournalOperation>,
}

impl RestoreApplyNextOperation {
    /// Build a compact next-operation response from a restore apply journal.
    #[must_use]
    pub fn from_journal(journal: &RestoreApplyJournal) -> Self {
        let complete =
            journal.operation_count > 0 && journal.completed_operations == journal.operation_count;
        let operation = journal.next_transition_operation().cloned();

        Self {
            response_version: 1,
            backup_id: journal.backup_id.clone(),
            ready: journal.ready,
            complete,
            operation_available: operation.is_some(),
            blocked_reasons: journal.blocked_reasons.clone(),
            operation,
        }
    }
}

///
/// RestoreApplyCommandPreview
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "runner preview exposes machine-readable availability and safety flags"
)]
pub struct RestoreApplyCommandPreview {
    pub response_version: u16,
    pub backup_id: String,
    pub ready: bool,
    pub complete: bool,
    pub operation_available: bool,
    pub command_available: bool,
    pub blocked_reasons: Vec<String>,
    pub operation: Option<RestoreApplyJournalOperation>,
    pub command: Option<RestoreApplyRunnerCommand>,
}

impl RestoreApplyCommandPreview {
    /// Build a no-execute runner command preview from a restore apply journal.
    #[must_use]
    pub fn from_journal(journal: &RestoreApplyJournal) -> Self {
        Self::from_journal_with_config(journal, &RestoreApplyCommandConfig::default())
    }

    /// Build a configured no-execute runner command preview from a journal.
    #[must_use]
    pub fn from_journal_with_config(
        journal: &RestoreApplyJournal,
        config: &RestoreApplyCommandConfig,
    ) -> Self {
        let complete =
            journal.operation_count > 0 && journal.completed_operations == journal.operation_count;
        let operation = journal.next_transition_operation().cloned();
        let command = operation.as_ref().and_then(|operation| {
            RestoreApplyRunnerCommand::from_operation(operation, journal, config)
        });

        Self {
            response_version: 1,
            backup_id: journal.backup_id.clone(),
            ready: journal.ready,
            complete,
            operation_available: operation.is_some(),
            command_available: command.is_some(),
            blocked_reasons: journal.blocked_reasons.clone(),
            operation,
            command,
        }
    }
}

///
/// RestoreApplyCommandConfig
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyCommandConfig {
    pub program: String,
    pub network: Option<String>,
}

impl Default for RestoreApplyCommandConfig {
    /// Build the default restore apply command preview configuration.
    fn default() -> Self {
        Self {
            program: "dfx".to_string(),
            network: None,
        }
    }
}

///
/// RestoreApplyRunnerCommand
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyRunnerCommand {
    pub program: String,
    pub args: Vec<String>,
    pub mutates: bool,
    pub requires_stopped_canister: bool,
    pub note: String,
}

impl RestoreApplyRunnerCommand {
    // Build a no-execute dfx command preview for one ready operation.
    fn from_operation(
        operation: &RestoreApplyJournalOperation,
        journal: &RestoreApplyJournal,
        config: &RestoreApplyCommandConfig,
    ) -> Option<Self> {
        match operation.operation {
            RestoreApplyOperationKind::UploadSnapshot => {
                let artifact_path = upload_artifact_command_path(operation, journal)?;
                Some(Self {
                    program: config.program.clone(),
                    args: dfx_canister_args(
                        config,
                        vec![
                            "snapshot".to_string(),
                            "upload".to_string(),
                            "--dir".to_string(),
                            artifact_path,
                            operation.target_canister.clone(),
                        ],
                    ),
                    mutates: true,
                    requires_stopped_canister: false,
                    note: "uploads the downloaded snapshot artifact to the target canister"
                        .to_string(),
                })
            }
            RestoreApplyOperationKind::LoadSnapshot => {
                let snapshot_id = journal.uploaded_snapshot_id_for_load(operation)?;
                Some(Self {
                    program: config.program.clone(),
                    args: dfx_canister_args(
                        config,
                        vec![
                            "snapshot".to_string(),
                            "load".to_string(),
                            operation.target_canister.clone(),
                            snapshot_id.to_string(),
                        ],
                    ),
                    mutates: true,
                    requires_stopped_canister: true,
                    note: "loads the uploaded snapshot into the target canister".to_string(),
                })
            }
            RestoreApplyOperationKind::ReinstallCode => Some(Self {
                program: config.program.clone(),
                args: dfx_canister_args(
                    config,
                    vec![
                        "install".to_string(),
                        "--mode".to_string(),
                        "reinstall".to_string(),
                        "--yes".to_string(),
                        operation.target_canister.clone(),
                    ],
                ),
                mutates: true,
                requires_stopped_canister: false,
                note: "reinstalls target canister code using the local dfx project configuration"
                    .to_string(),
            }),
            RestoreApplyOperationKind::VerifyMember | RestoreApplyOperationKind::VerifyFleet => {
                match operation.verification_kind.as_deref() {
                    Some("status") => Some(Self {
                        program: config.program.clone(),
                        args: dfx_canister_args(
                            config,
                            vec!["status".to_string(), operation.target_canister.clone()],
                        ),
                        mutates: false,
                        requires_stopped_canister: false,
                        note: verification_command_note(
                            &operation.operation,
                            "checks target canister status",
                            "checks target fleet root canister status",
                        )
                        .to_string(),
                    }),
                    Some(_) => {
                        let method = operation.verification_method.as_ref()?;
                        Some(Self {
                            program: config.program.clone(),
                            args: dfx_canister_args(
                                config,
                                vec![
                                    "call".to_string(),
                                    "--query".to_string(),
                                    operation.target_canister.clone(),
                                    method.clone(),
                                ],
                            ),
                            mutates: false,
                            requires_stopped_canister: false,
                            note: verification_command_note(
                                &operation.operation,
                                "runs the declared verification method as a query call",
                                "runs the declared fleet verification method as a query call",
                            )
                            .to_string(),
                        })
                    }
                    None => None,
                }
            }
        }
    }
}

// Return an operator note for member-level or fleet-level verification commands.
const fn verification_command_note(
    operation: &RestoreApplyOperationKind,
    member_note: &'static str,
    fleet_note: &'static str,
) -> &'static str {
    match operation {
        RestoreApplyOperationKind::VerifyFleet => fleet_note,
        RestoreApplyOperationKind::UploadSnapshot
        | RestoreApplyOperationKind::LoadSnapshot
        | RestoreApplyOperationKind::ReinstallCode
        | RestoreApplyOperationKind::VerifyMember => member_note,
    }
}

// Build `dfx canister` arguments with the optional network selector.
fn dfx_canister_args(config: &RestoreApplyCommandConfig, mut tail: Vec<String>) -> Vec<String> {
    let mut args = vec!["canister".to_string()];
    if let Some(network) = &config.network {
        args.push("--network".to_string());
        args.push(network.clone());
    }
    args.append(&mut tail);
    args
}

// Resolve upload artifact paths the same way validation resolved them.
fn upload_artifact_command_path(
    operation: &RestoreApplyJournalOperation,
    journal: &RestoreApplyJournal,
) -> Option<String> {
    let artifact_path = operation.artifact_path.as_ref()?;
    let path = Path::new(artifact_path);
    if path.is_absolute() {
        return Some(artifact_path.clone());
    }

    let backup_root = journal.backup_root.as_ref()?;
    let is_safe = path
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir));
    if !is_safe {
        return None;
    }

    Some(
        Path::new(backup_root)
            .join(path)
            .to_string_lossy()
            .to_string(),
    )
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

// Verify every planned restore artifact against one backup directory root.
fn validate_restore_apply_artifacts(
    plan: &RestorePlan,
    backup_root: &Path,
) -> Result<RestoreApplyArtifactValidation, RestoreApplyDryRunError> {
    let mut checks = Vec::new();

    for member in plan.ordered_members() {
        checks.push(validate_restore_apply_artifact(member, backup_root)?);
    }

    let members_with_expected_checksums = checks
        .iter()
        .filter(|check| check.checksum_expected.is_some())
        .count();
    let artifacts_present = checks.iter().all(|check| check.exists);
    let checksums_verified = members_with_expected_checksums == plan.member_count
        && checks.iter().all(|check| check.checksum_verified);

    Ok(RestoreApplyArtifactValidation {
        backup_root: backup_root.to_string_lossy().to_string(),
        checked_members: checks.len(),
        artifacts_present,
        checksums_verified,
        members_with_expected_checksums,
        checks,
    })
}

// Verify one planned restore artifact path and checksum.
fn validate_restore_apply_artifact(
    member: &RestorePlanMember,
    backup_root: &Path,
) -> Result<RestoreApplyArtifactCheck, RestoreApplyDryRunError> {
    let artifact_path = safe_restore_artifact_path(
        &member.source_canister,
        &member.source_snapshot.artifact_path,
    )?;
    let resolved_path = backup_root.join(&artifact_path);

    if !resolved_path.exists() {
        return Err(RestoreApplyDryRunError::ArtifactMissing {
            source_canister: member.source_canister.clone(),
            artifact_path: member.source_snapshot.artifact_path.clone(),
            resolved_path: resolved_path.to_string_lossy().to_string(),
        });
    }

    let (checksum_actual, checksum_verified) =
        if let Some(expected) = &member.source_snapshot.checksum {
            let checksum = ArtifactChecksum::from_path(&resolved_path).map_err(|source| {
                RestoreApplyDryRunError::ArtifactChecksum {
                    source_canister: member.source_canister.clone(),
                    artifact_path: member.source_snapshot.artifact_path.clone(),
                    source,
                }
            })?;
            checksum.verify(expected).map_err(|source| {
                RestoreApplyDryRunError::ArtifactChecksum {
                    source_canister: member.source_canister.clone(),
                    artifact_path: member.source_snapshot.artifact_path.clone(),
                    source,
                }
            })?;
            (Some(checksum.hash), true)
        } else {
            (None, false)
        };

    Ok(RestoreApplyArtifactCheck {
        source_canister: member.source_canister.clone(),
        target_canister: member.target_canister.clone(),
        snapshot_id: member.source_snapshot.snapshot_id.clone(),
        artifact_path: member.source_snapshot.artifact_path.clone(),
        resolved_path: resolved_path.to_string_lossy().to_string(),
        exists: true,
        checksum_algorithm: member.source_snapshot.checksum_algorithm.clone(),
        checksum_expected: member.source_snapshot.checksum.clone(),
        checksum_actual,
        checksum_verified,
    })
}

// Reject absolute paths and parent traversal before joining with the backup root.
fn safe_restore_artifact_path(
    source_canister: &str,
    artifact_path: &str,
) -> Result<PathBuf, RestoreApplyDryRunError> {
    let path = Path::new(artifact_path);
    let is_safe = path
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir));

    if is_safe {
        return Ok(path.to_path_buf());
    }

    Err(RestoreApplyDryRunError::ArtifactPathEscapesBackup {
        source_canister: source_canister.to_string(),
        artifact_path: artifact_path.to_string(),
    })
}

// Validate that a supplied restore status belongs to the restore plan.
fn validate_restore_status_matches_plan(
    plan: &RestorePlan,
    status: &RestoreStatus,
) -> Result<(), RestoreApplyDryRunError> {
    validate_status_string_field("backup_id", &plan.backup_id, &status.backup_id)?;
    validate_status_string_field(
        "source_environment",
        &plan.source_environment,
        &status.source_environment,
    )?;
    validate_status_string_field(
        "source_root_canister",
        &plan.source_root_canister,
        &status.source_root_canister,
    )?;
    validate_status_string_field("topology_hash", &plan.topology_hash, &status.topology_hash)?;
    validate_status_usize_field("member_count", plan.member_count, status.member_count)?;
    validate_status_usize_field(
        "phase_count",
        plan.ordering_summary.phase_count,
        status.phase_count,
    )?;
    Ok(())
}

// Validate one string field shared by restore plan and status.
fn validate_status_string_field(
    field: &'static str,
    plan: &str,
    status: &str,
) -> Result<(), RestoreApplyDryRunError> {
    if plan == status {
        return Ok(());
    }

    Err(RestoreApplyDryRunError::StatusPlanMismatch {
        field,
        plan: plan.to_string(),
        status: status.to_string(),
    })
}

// Validate one numeric field shared by restore plan and status.
const fn validate_status_usize_field(
    field: &'static str,
    plan: usize,
    status: usize,
) -> Result<(), RestoreApplyDryRunError> {
    if plan == status {
        return Ok(());
    }

    Err(RestoreApplyDryRunError::StatusPlanCountMismatch {
        field,
        plan,
        status,
    })
}

///
/// RestoreApplyArtifactValidation
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyArtifactValidation {
    pub backup_root: String,
    pub checked_members: usize,
    pub artifacts_present: bool,
    pub checksums_verified: bool,
    pub members_with_expected_checksums: usize,
    pub checks: Vec<RestoreApplyArtifactCheck>,
}

///
/// RestoreApplyArtifactCheck
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyArtifactCheck {
    pub source_canister: String,
    pub target_canister: String,
    pub snapshot_id: String,
    pub artifact_path: String,
    pub resolved_path: String,
    pub exists: bool,
    pub checksum_algorithm: String,
    pub checksum_expected: Option<String>,
    pub checksum_actual: Option<String>,
    pub checksum_verified: bool,
}

///
/// RestoreApplyDryRunPhase
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyDryRunPhase {
    pub restore_group: u16,
    pub operations: Vec<RestoreApplyDryRunOperation>,
}

impl RestoreApplyDryRunPhase {
    // Build one dry-run phase from one restore plan phase.
    fn from_plan_phase(phase: &RestorePhase, next_sequence: &mut usize) -> Self {
        let mut operations = Vec::new();

        for member in &phase.members {
            push_member_operation(
                &mut operations,
                next_sequence,
                RestoreApplyOperationKind::UploadSnapshot,
                member,
                None,
            );
            push_member_operation(
                &mut operations,
                next_sequence,
                RestoreApplyOperationKind::LoadSnapshot,
                member,
                None,
            );

            for check in &member.verification_checks {
                push_member_operation(
                    &mut operations,
                    next_sequence,
                    RestoreApplyOperationKind::VerifyMember,
                    member,
                    Some(check),
                );
            }
        }

        Self {
            restore_group: phase.restore_group,
            operations,
        }
    }
}

// Append one member-level dry-run operation using the current phase order.
fn push_member_operation(
    operations: &mut Vec<RestoreApplyDryRunOperation>,
    next_sequence: &mut usize,
    operation: RestoreApplyOperationKind,
    member: &RestorePlanMember,
    check: Option<&VerificationCheck>,
) {
    let sequence = *next_sequence;
    *next_sequence += 1;

    operations.push(RestoreApplyDryRunOperation {
        sequence,
        operation,
        restore_group: member.restore_group,
        phase_order: member.phase_order,
        source_canister: member.source_canister.clone(),
        target_canister: member.target_canister.clone(),
        role: member.role.clone(),
        snapshot_id: Some(member.source_snapshot.snapshot_id.clone()),
        artifact_path: Some(member.source_snapshot.artifact_path.clone()),
        verification_kind: check.map(|check| check.kind.clone()),
        verification_method: check.and_then(|check| check.method.clone()),
    });
}

// Append fleet-level verification checks after all member operations.
fn append_fleet_verification_operations(
    plan: &RestorePlan,
    phases: &mut [RestoreApplyDryRunPhase],
    next_sequence: &mut usize,
) {
    if plan.fleet_verification_checks.is_empty() {
        return;
    }

    let Some(phase) = phases.last_mut() else {
        return;
    };
    let root = plan
        .phases
        .iter()
        .flat_map(|phase| phase.members.iter())
        .find(|member| member.source_canister == plan.source_root_canister);
    let source_canister = root.map_or_else(
        || plan.source_root_canister.clone(),
        |member| member.source_canister.clone(),
    );
    let target_canister = root.map_or_else(
        || plan.source_root_canister.clone(),
        |member| member.target_canister.clone(),
    );
    let restore_group = phase.restore_group;

    for check in &plan.fleet_verification_checks {
        push_fleet_operation(
            &mut phase.operations,
            next_sequence,
            restore_group,
            &source_canister,
            &target_canister,
            check,
        );
    }
}

// Append one fleet-level dry-run verification operation.
fn push_fleet_operation(
    operations: &mut Vec<RestoreApplyDryRunOperation>,
    next_sequence: &mut usize,
    restore_group: u16,
    source_canister: &str,
    target_canister: &str,
    check: &VerificationCheck,
) {
    let sequence = *next_sequence;
    *next_sequence += 1;
    let phase_order = operations.len();

    operations.push(RestoreApplyDryRunOperation {
        sequence,
        operation: RestoreApplyOperationKind::VerifyFleet,
        restore_group,
        phase_order,
        source_canister: source_canister.to_string(),
        target_canister: target_canister.to_string(),
        role: "fleet".to_string(),
        snapshot_id: None,
        artifact_path: None,
        verification_kind: Some(check.kind.clone()),
        verification_method: check.method.clone(),
    });
}

///
/// RestoreApplyDryRunOperation
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyDryRunOperation {
    pub sequence: usize,
    pub operation: RestoreApplyOperationKind,
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

///
/// RestoreApplyDryRunError
///

#[derive(Debug, ThisError)]
pub enum RestoreApplyDryRunError {
    #[error("restore status field {field} does not match plan: plan={plan}, status={status}")]
    StatusPlanMismatch {
        field: &'static str,
        plan: String,
        status: String,
    },

    #[error("restore status field {field} does not match plan: plan={plan}, status={status}")]
    StatusPlanCountMismatch {
        field: &'static str,
        plan: usize,
        status: usize,
    },

    #[error("restore artifact path for {source_canister} escapes backup root: {artifact_path}")]
    ArtifactPathEscapesBackup {
        source_canister: String,
        artifact_path: String,
    },

    #[error(
        "restore artifact for {source_canister} is missing: {artifact_path} at {resolved_path}"
    )]
    ArtifactMissing {
        source_canister: String,
        artifact_path: String,
        resolved_path: String,
    },

    #[error("restore artifact checksum failed for {source_canister} at {artifact_path}: {source}")]
    ArtifactChecksum {
        source_canister: String,
        artifact_path: String,
        #[source]
        source: ArtifactChecksumError,
    },
}

///
/// RestoreIdentitySummary
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreIdentitySummary {
    pub mapping_supplied: bool,
    pub all_sources_mapped: bool,
    pub fixed_members: usize,
    pub relocatable_members: usize,
    pub in_place_members: usize,
    pub mapped_members: usize,
    pub remapped_members: usize,
}

///
/// RestoreSnapshotSummary
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "restore summaries intentionally expose machine-readable readiness flags"
)]
pub struct RestoreSnapshotSummary {
    pub all_members_have_module_hash: bool,
    pub all_members_have_wasm_hash: bool,
    pub all_members_have_code_version: bool,
    pub all_members_have_checksum: bool,
    pub members_with_module_hash: usize,
    pub members_with_wasm_hash: usize,
    pub members_with_code_version: usize,
    pub members_with_checksum: usize,
}

///
/// RestoreVerificationSummary
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreVerificationSummary {
    pub verification_required: bool,
    pub all_members_have_checks: bool,
    pub fleet_checks: usize,
    pub member_check_groups: usize,
    pub member_checks: usize,
    pub members_with_checks: usize,
    pub total_checks: usize,
}

///
/// RestoreReadinessSummary
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreReadinessSummary {
    pub ready: bool,
    pub reasons: Vec<String>,
}

///
/// RestoreOperationSummary
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreOperationSummary {
    #[serde(default)]
    pub planned_snapshot_uploads: usize,
    pub planned_snapshot_loads: usize,
    pub planned_code_reinstalls: usize,
    pub planned_verification_checks: usize,
    #[serde(default)]
    pub planned_operations: usize,
    pub planned_phases: usize,
}

impl RestoreOperationSummary {
    /// Return planned snapshot uploads, deriving the value for older plan JSON.
    #[must_use]
    pub const fn effective_planned_snapshot_uploads(&self, member_count: usize) -> usize {
        if self.planned_snapshot_uploads == 0 && member_count > 0 {
            return member_count;
        }

        self.planned_snapshot_uploads
    }

    /// Return total planned operations, deriving the value for older plan JSON.
    #[must_use]
    pub const fn effective_planned_operations(&self, member_count: usize) -> usize {
        if self.planned_operations == 0 {
            return self.effective_planned_snapshot_uploads(member_count)
                + self.planned_snapshot_loads
                + self.planned_code_reinstalls
                + self.planned_verification_checks;
        }

        self.planned_operations
    }
}

///
/// RestoreOrderingSummary
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreOrderingSummary {
    pub phase_count: usize,
    pub dependency_free_members: usize,
    pub in_group_parent_edges: usize,
    pub cross_group_parent_edges: usize,
}

///
/// RestorePhase
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestorePhase {
    pub restore_group: u16,
    pub members: Vec<RestorePlanMember>,
}

///
/// RestorePlanMember
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestorePlanMember {
    pub source_canister: String,
    pub target_canister: String,
    pub role: String,
    pub parent_source_canister: Option<String>,
    pub parent_target_canister: Option<String>,
    pub ordering_dependency: Option<RestoreOrderingDependency>,
    pub phase_order: usize,
    pub restore_group: u16,
    pub identity_mode: IdentityMode,
    pub verification_class: String,
    pub verification_checks: Vec<VerificationCheck>,
    pub source_snapshot: SourceSnapshot,
}

///
/// RestoreOrderingDependency
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreOrderingDependency {
    pub source_canister: String,
    pub target_canister: String,
    pub relationship: RestoreOrderingRelationship,
}

///
/// RestoreOrderingRelationship
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestoreOrderingRelationship {
    ParentInSameGroup,
    ParentInEarlierGroup,
}

///
/// RestorePlanner
///

pub struct RestorePlanner;

impl RestorePlanner {
    /// Build a no-mutation restore plan from the manifest and optional target mapping.
    pub fn plan(
        manifest: &FleetBackupManifest,
        mapping: Option<&RestoreMapping>,
    ) -> Result<RestorePlan, RestorePlanError> {
        manifest.validate()?;
        if let Some(mapping) = mapping {
            validate_mapping(mapping)?;
            validate_mapping_sources(manifest, mapping)?;
        }

        let members = resolve_members(manifest, mapping)?;
        let identity_summary = restore_identity_summary(&members, mapping.is_some());
        let snapshot_summary = restore_snapshot_summary(&members);
        let verification_summary = restore_verification_summary(manifest, &members);
        let readiness_summary = restore_readiness_summary(&snapshot_summary, &verification_summary);
        validate_restore_group_dependencies(&members)?;
        let phases = group_and_order_members(members)?;
        let ordering_summary = restore_ordering_summary(&phases);
        let operation_summary =
            restore_operation_summary(manifest.fleet.members.len(), &verification_summary, &phases);

        Ok(RestorePlan {
            backup_id: manifest.backup_id.clone(),
            source_environment: manifest.source.environment.clone(),
            source_root_canister: manifest.source.root_canister.clone(),
            topology_hash: manifest.fleet.topology_hash.clone(),
            member_count: manifest.fleet.members.len(),
            identity_summary,
            snapshot_summary,
            verification_summary,
            readiness_summary,
            operation_summary,
            ordering_summary,
            design_conformance: Some(manifest.design_conformance_report()),
            fleet_verification_checks: manifest.verification.fleet_checks.clone(),
            phases,
        })
    }
}

///
/// RestorePlanError
///

#[derive(Debug, ThisError)]
pub enum RestorePlanError {
    #[error(transparent)]
    InvalidManifest(#[from] ManifestValidationError),

    #[error("field {field} must be a valid principal: {value}")]
    InvalidPrincipal { field: &'static str, value: String },

    #[error("mapping contains duplicate source canister {0}")]
    DuplicateMappingSource(String),

    #[error("mapping contains duplicate target canister {0}")]
    DuplicateMappingTarget(String),

    #[error("mapping references unknown source canister {0}")]
    UnknownMappingSource(String),

    #[error("mapping is missing source canister {0}")]
    MissingMappingSource(String),

    #[error("fixed-identity member {source_canister} cannot be mapped to {target_canister}")]
    FixedIdentityRemap {
        source_canister: String,
        target_canister: String,
    },

    #[error("restore plan contains duplicate target canister {0}")]
    DuplicatePlanTarget(String),

    #[error("restore group {0} contains a parent cycle or unresolved dependency")]
    RestoreOrderCycle(u16),

    #[error(
        "restore plan places parent {parent_source_canister} in group {parent_restore_group} after child {child_source_canister} in group {child_restore_group}"
    )]
    ParentRestoreGroupAfterChild {
        child_source_canister: String,
        parent_source_canister: String,
        child_restore_group: u16,
        parent_restore_group: u16,
    },
}

// Validate a user-supplied restore mapping before applying it to the manifest.
fn validate_mapping(mapping: &RestoreMapping) -> Result<(), RestorePlanError> {
    let mut sources = BTreeSet::new();
    let mut targets = BTreeSet::new();

    for entry in &mapping.members {
        validate_principal("mapping.members[].source_canister", &entry.source_canister)?;
        validate_principal("mapping.members[].target_canister", &entry.target_canister)?;

        if !sources.insert(entry.source_canister.clone()) {
            return Err(RestorePlanError::DuplicateMappingSource(
                entry.source_canister.clone(),
            ));
        }

        if !targets.insert(entry.target_canister.clone()) {
            return Err(RestorePlanError::DuplicateMappingTarget(
                entry.target_canister.clone(),
            ));
        }
    }

    Ok(())
}

// Ensure mappings only reference members declared in the manifest.
fn validate_mapping_sources(
    manifest: &FleetBackupManifest,
    mapping: &RestoreMapping,
) -> Result<(), RestorePlanError> {
    let sources = manifest
        .fleet
        .members
        .iter()
        .map(|member| member.canister_id.as_str())
        .collect::<BTreeSet<_>>();

    for entry in &mapping.members {
        if !sources.contains(entry.source_canister.as_str()) {
            return Err(RestorePlanError::UnknownMappingSource(
                entry.source_canister.clone(),
            ));
        }
    }

    Ok(())
}

// Resolve source manifest members into target restore members.
fn resolve_members(
    manifest: &FleetBackupManifest,
    mapping: Option<&RestoreMapping>,
) -> Result<Vec<RestorePlanMember>, RestorePlanError> {
    let mut plan_members = Vec::with_capacity(manifest.fleet.members.len());
    let mut targets = BTreeSet::new();
    let mut source_to_target = BTreeMap::new();

    for member in &manifest.fleet.members {
        let target = resolve_target(member, mapping)?;
        if !targets.insert(target.clone()) {
            return Err(RestorePlanError::DuplicatePlanTarget(target));
        }

        source_to_target.insert(member.canister_id.clone(), target.clone());
        plan_members.push(RestorePlanMember {
            source_canister: member.canister_id.clone(),
            target_canister: target,
            role: member.role.clone(),
            parent_source_canister: member.parent_canister_id.clone(),
            parent_target_canister: None,
            ordering_dependency: None,
            phase_order: 0,
            restore_group: member.restore_group,
            identity_mode: member.identity_mode.clone(),
            verification_class: member.verification_class.clone(),
            verification_checks: concrete_member_verification_checks(
                member,
                &manifest.verification,
            ),
            source_snapshot: member.source_snapshot.clone(),
        });
    }

    for member in &mut plan_members {
        member.parent_target_canister = member
            .parent_source_canister
            .as_ref()
            .and_then(|parent| source_to_target.get(parent))
            .cloned();
    }

    Ok(plan_members)
}

// Resolve all concrete verification checks that apply to one restore member role.
fn concrete_member_verification_checks(
    member: &FleetMember,
    verification: &VerificationPlan,
) -> Vec<VerificationCheck> {
    let mut checks = member
        .verification_checks
        .iter()
        .filter(|check| verification_check_applies_to_role(check, &member.role))
        .cloned()
        .collect::<Vec<_>>();

    for group in &verification.member_checks {
        if group.role != member.role {
            continue;
        }

        checks.extend(
            group
                .checks
                .iter()
                .filter(|check| verification_check_applies_to_role(check, &member.role))
                .cloned(),
        );
    }

    checks
}

// Return whether a verification check's role filter includes one member role.
fn verification_check_applies_to_role(check: &VerificationCheck, role: &str) -> bool {
    check.roles.is_empty() || check.roles.iter().any(|check_role| check_role == role)
}

// Resolve one member's target canister, enforcing identity continuity.
fn resolve_target(
    member: &FleetMember,
    mapping: Option<&RestoreMapping>,
) -> Result<String, RestorePlanError> {
    let target = match mapping {
        Some(mapping) => mapping
            .target_for(&member.canister_id)
            .ok_or_else(|| RestorePlanError::MissingMappingSource(member.canister_id.clone()))?
            .to_string(),
        None => member.canister_id.clone(),
    };

    if matches!(member.identity_mode, IdentityMode::Fixed) && target != member.canister_id {
        return Err(RestorePlanError::FixedIdentityRemap {
            source_canister: member.canister_id.clone(),
            target_canister: target,
        });
    }

    Ok(target)
}

// Summarize identity and mapping decisions before grouping restore phases.
fn restore_identity_summary(
    members: &[RestorePlanMember],
    mapping_supplied: bool,
) -> RestoreIdentitySummary {
    let mut summary = RestoreIdentitySummary {
        mapping_supplied,
        all_sources_mapped: false,
        fixed_members: 0,
        relocatable_members: 0,
        in_place_members: 0,
        mapped_members: 0,
        remapped_members: 0,
    };

    for member in members {
        match member.identity_mode {
            IdentityMode::Fixed => summary.fixed_members += 1,
            IdentityMode::Relocatable => summary.relocatable_members += 1,
        }

        if member.source_canister == member.target_canister {
            summary.in_place_members += 1;
        } else {
            summary.remapped_members += 1;
        }
        if mapping_supplied {
            summary.mapped_members += 1;
        }
    }

    summary.all_sources_mapped = mapping_supplied && summary.mapped_members == members.len();

    summary
}

// Summarize snapshot provenance completeness before grouping restore phases.
fn restore_snapshot_summary(members: &[RestorePlanMember]) -> RestoreSnapshotSummary {
    let members_with_module_hash = members
        .iter()
        .filter(|member| member.source_snapshot.module_hash.is_some())
        .count();
    let members_with_wasm_hash = members
        .iter()
        .filter(|member| member.source_snapshot.wasm_hash.is_some())
        .count();
    let members_with_code_version = members
        .iter()
        .filter(|member| member.source_snapshot.code_version.is_some())
        .count();
    let members_with_checksum = members
        .iter()
        .filter(|member| member.source_snapshot.checksum.is_some())
        .count();

    RestoreSnapshotSummary {
        all_members_have_module_hash: members_with_module_hash == members.len(),
        all_members_have_wasm_hash: members_with_wasm_hash == members.len(),
        all_members_have_code_version: members_with_code_version == members.len(),
        all_members_have_checksum: members_with_checksum == members.len(),
        members_with_module_hash,
        members_with_wasm_hash,
        members_with_code_version,
        members_with_checksum,
    }
}

// Summarize whether restore planning has the metadata required for automation.
fn restore_readiness_summary(
    snapshot: &RestoreSnapshotSummary,
    verification: &RestoreVerificationSummary,
) -> RestoreReadinessSummary {
    let mut reasons = Vec::new();

    if !snapshot.all_members_have_module_hash {
        reasons.push("missing-module-hash".to_string());
    }
    if !snapshot.all_members_have_wasm_hash {
        reasons.push("missing-wasm-hash".to_string());
    }
    if !snapshot.all_members_have_code_version {
        reasons.push("missing-code-version".to_string());
    }
    if !snapshot.all_members_have_checksum {
        reasons.push("missing-snapshot-checksum".to_string());
    }
    if !verification.all_members_have_checks {
        reasons.push("missing-verification-checks".to_string());
    }

    RestoreReadinessSummary {
        ready: reasons.is_empty(),
        reasons,
    }
}

// Summarize restore verification work declared by the manifest and members.
fn restore_verification_summary(
    manifest: &FleetBackupManifest,
    members: &[RestorePlanMember],
) -> RestoreVerificationSummary {
    let fleet_checks = manifest.verification.fleet_checks.len();
    let member_check_groups = manifest.verification.member_checks.len();
    let member_checks = members
        .iter()
        .map(|member| member.verification_checks.len())
        .sum::<usize>();
    let members_with_checks = members
        .iter()
        .filter(|member| !member.verification_checks.is_empty())
        .count();

    RestoreVerificationSummary {
        verification_required: true,
        all_members_have_checks: members_with_checks == members.len(),
        fleet_checks,
        member_check_groups,
        member_checks,
        members_with_checks,
        total_checks: fleet_checks + member_checks,
    }
}

// Summarize the concrete restore operations implied by a no-mutation plan.
const fn restore_operation_summary(
    member_count: usize,
    verification_summary: &RestoreVerificationSummary,
    phases: &[RestorePhase],
) -> RestoreOperationSummary {
    RestoreOperationSummary {
        planned_snapshot_uploads: member_count,
        planned_snapshot_loads: member_count,
        planned_code_reinstalls: 0,
        planned_verification_checks: verification_summary.total_checks,
        planned_operations: member_count + member_count + verification_summary.total_checks,
        planned_phases: phases.len(),
    }
}

// Reject group assignments that would restore a child before its parent.
fn validate_restore_group_dependencies(
    members: &[RestorePlanMember],
) -> Result<(), RestorePlanError> {
    let groups_by_source = members
        .iter()
        .map(|member| (member.source_canister.as_str(), member.restore_group))
        .collect::<BTreeMap<_, _>>();

    for member in members {
        let Some(parent) = &member.parent_source_canister else {
            continue;
        };
        let Some(parent_group) = groups_by_source.get(parent.as_str()) else {
            continue;
        };

        if *parent_group > member.restore_group {
            return Err(RestorePlanError::ParentRestoreGroupAfterChild {
                child_source_canister: member.source_canister.clone(),
                parent_source_canister: parent.clone(),
                child_restore_group: member.restore_group,
                parent_restore_group: *parent_group,
            });
        }
    }

    Ok(())
}

// Group members and apply parent-before-child ordering inside each group.
fn group_and_order_members(
    members: Vec<RestorePlanMember>,
) -> Result<Vec<RestorePhase>, RestorePlanError> {
    let mut groups = BTreeMap::<u16, Vec<RestorePlanMember>>::new();
    for member in members {
        groups.entry(member.restore_group).or_default().push(member);
    }

    groups
        .into_iter()
        .map(|(restore_group, members)| {
            let members = order_group(restore_group, members)?;
            Ok(RestorePhase {
                restore_group,
                members,
            })
        })
        .collect()
}

// Topologically order one group using manifest parent relationships.
fn order_group(
    restore_group: u16,
    members: Vec<RestorePlanMember>,
) -> Result<Vec<RestorePlanMember>, RestorePlanError> {
    let mut remaining = members;
    let group_sources = remaining
        .iter()
        .map(|member| member.source_canister.clone())
        .collect::<BTreeSet<_>>();
    let mut emitted = BTreeSet::new();
    let mut ordered = Vec::with_capacity(remaining.len());

    while !remaining.is_empty() {
        let Some(index) = remaining
            .iter()
            .position(|member| parent_satisfied(member, &group_sources, &emitted))
        else {
            return Err(RestorePlanError::RestoreOrderCycle(restore_group));
        };

        let mut member = remaining.remove(index);
        member.phase_order = ordered.len();
        member.ordering_dependency = ordering_dependency(&member, &group_sources);
        emitted.insert(member.source_canister.clone());
        ordered.push(member);
    }

    Ok(ordered)
}

// Describe the topology dependency that controlled a member's restore ordering.
fn ordering_dependency(
    member: &RestorePlanMember,
    group_sources: &BTreeSet<String>,
) -> Option<RestoreOrderingDependency> {
    let parent_source = member.parent_source_canister.as_ref()?;
    let parent_target = member.parent_target_canister.as_ref()?;
    let relationship = if group_sources.contains(parent_source) {
        RestoreOrderingRelationship::ParentInSameGroup
    } else {
        RestoreOrderingRelationship::ParentInEarlierGroup
    };

    Some(RestoreOrderingDependency {
        source_canister: parent_source.clone(),
        target_canister: parent_target.clone(),
        relationship,
    })
}

// Summarize the dependency ordering metadata exposed in the restore plan.
fn restore_ordering_summary(phases: &[RestorePhase]) -> RestoreOrderingSummary {
    let mut summary = RestoreOrderingSummary {
        phase_count: phases.len(),
        dependency_free_members: 0,
        in_group_parent_edges: 0,
        cross_group_parent_edges: 0,
    };

    for member in phases.iter().flat_map(|phase| phase.members.iter()) {
        match &member.ordering_dependency {
            Some(dependency)
                if dependency.relationship == RestoreOrderingRelationship::ParentInSameGroup =>
            {
                summary.in_group_parent_edges += 1;
            }
            Some(dependency)
                if dependency.relationship == RestoreOrderingRelationship::ParentInEarlierGroup =>
            {
                summary.cross_group_parent_edges += 1;
            }
            Some(_) => {}
            None => summary.dependency_free_members += 1,
        }
    }

    summary
}

// Determine whether a member's in-group parent has already been emitted.
fn parent_satisfied(
    member: &RestorePlanMember,
    group_sources: &BTreeSet<String>,
    emitted: &BTreeSet<String>,
) -> bool {
    match &member.parent_source_canister {
        Some(parent) if group_sources.contains(parent) => emitted.contains(parent),
        _ => true,
    }
}

// Validate textual principal fields used in mappings.
fn validate_principal(field: &'static str, value: &str) -> Result<(), RestorePlanError> {
    Principal::from_str(value)
        .map(|_| ())
        .map_err(|_| RestorePlanError::InvalidPrincipal {
            field,
            value: value.to_string(),
        })
}

#[cfg(test)]
mod tests;
