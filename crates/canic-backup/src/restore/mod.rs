use crate::{
    artifacts::{ArtifactChecksum, ArtifactChecksumError},
    manifest::{
        FleetBackupManifest, FleetMember, IdentityMode, ManifestValidationError, SourceSnapshot,
        VerificationCheck,
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
    pub planned_snapshot_loads: usize,
    pub planned_code_reinstalls: usize,
    pub planned_verification_checks: usize,
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
            planned_snapshot_loads: plan.operation_summary.planned_snapshot_loads,
            planned_code_reinstalls: plan.operation_summary.planned_code_reinstalls,
            planned_verification_checks: plan.operation_summary.planned_verification_checks,
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
    pub planned_snapshot_loads: usize,
    pub planned_code_reinstalls: usize,
    pub planned_verification_checks: usize,
    pub rendered_operations: usize,
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
        let rendered_operations = phases
            .iter()
            .map(|phase| phase.operations.len())
            .sum::<usize>();

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
            planned_snapshot_loads: plan.operation_summary.planned_snapshot_loads,
            planned_code_reinstalls: plan.operation_summary.planned_code_reinstalls,
            planned_verification_checks: plan.operation_summary.planned_verification_checks,
            rendered_operations,
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
    pub operation_count: usize,
    pub pending_operations: usize,
    pub ready_operations: usize,
    pub blocked_operations: usize,
    pub completed_operations: usize,
    pub failed_operations: usize,
    pub operations: Vec<RestoreApplyJournalOperation>,
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

        Self {
            journal_version: 1,
            backup_id: dry_run.backup_id.clone(),
            ready: blocked_reasons.is_empty(),
            blocked_reasons,
            operation_count: operations.len(),
            pending_operations: 0,
            ready_operations,
            blocked_operations,
            completed_operations: 0,
            failed_operations: 0,
            operations,
        }
    }

    /// Validate the structural consistency of a restore apply journal.
    pub fn validate(&self) -> Result<(), RestoreApplyJournalError> {
        validate_apply_journal_version(self.journal_version)?;
        validate_apply_journal_nonempty("backup_id", &self.backup_id)?;
        validate_apply_journal_count(
            "operation_count",
            self.operation_count,
            self.operations.len(),
        )?;

        let state_counts = RestoreApplyJournalStateCounts::from_operations(&self.operations);
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

        Ok(())
    }

    /// Summarize this apply journal for operators and automation.
    #[must_use]
    pub fn status(&self) -> RestoreApplyJournalStatus {
        RestoreApplyJournalStatus::from_journal(self)
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
                    RestoreApplyOperationState::Ready | RestoreApplyOperationState::Pending
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
        self.pending_operations = state_counts.pending;
        self.ready_operations = state_counts.ready;
        self.blocked_operations = state_counts.blocked;
        self.completed_operations = state_counts.completed;
        self.failed_operations = state_counts.failed;
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
        let command = operation
            .as_ref()
            .and_then(|operation| RestoreApplyRunnerCommand::from_operation(operation, config));

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
        config: &RestoreApplyCommandConfig,
    ) -> Option<Self> {
        match operation.operation {
            RestoreApplyOperationKind::UploadSnapshot => {
                let artifact_path = operation.artifact_path.as_ref()?;
                Some(Self {
                    program: config.program.clone(),
                    args: dfx_canister_args(
                        config,
                        vec![
                            "snapshot".to_string(),
                            "upload".to_string(),
                            "--dir".to_string(),
                            artifact_path.clone(),
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
                let snapshot_id = operation.snapshot_id.as_ref()?;
                Some(Self {
                    program: config.program.clone(),
                    args: dfx_canister_args(
                        config,
                        vec![
                            "snapshot".to_string(),
                            "load".to_string(),
                            operation.target_canister.clone(),
                            snapshot_id.clone(),
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
            RestoreApplyOperationKind::VerifyMember => {
                match operation.verification_kind.as_deref() {
                    Some("status") => Some(Self {
                        program: config.program.clone(),
                        args: dfx_canister_args(
                            config,
                            vec!["status".to_string(), operation.target_canister.clone()],
                        ),
                        mutates: false,
                        requires_stopped_canister: false,
                        note: "checks target canister status".to_string(),
                    }),
                    Some(_) => {
                        let method = operation.verification_method.as_ref()?;
                        Some(Self {
                            program: config.program.clone(),
                            args: dfx_canister_args(
                                config,
                                vec![
                                    "call".to_string(),
                                    operation.target_canister.clone(),
                                    method.clone(),
                                ],
                            ),
                            mutates: false,
                            requires_stopped_canister: false,
                            note: "calls the declared verification method".to_string(),
                        })
                    }
                    None => None,
                }
            }
        }
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

    // Decide whether an operation can move to the requested next state.
    const fn can_transition_to(&self, next_state: &RestoreApplyOperationState) -> bool {
        match (&self.state, next_state) {
            (
                RestoreApplyOperationState::Ready | RestoreApplyOperationState::Pending,
                RestoreApplyOperationState::Pending,
            )
            | (RestoreApplyOperationState::Pending, RestoreApplyOperationState::Ready)
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
            push_member_operation(
                &mut operations,
                next_sequence,
                RestoreApplyOperationKind::ReinstallCode,
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
    pub planned_snapshot_loads: usize,
    pub planned_code_reinstalls: usize,
    pub planned_verification_checks: usize,
    pub planned_phases: usize,
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
            verification_checks: member.verification_checks.clone(),
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
    let role_check_counts = manifest
        .verification
        .member_checks
        .iter()
        .map(|group| (group.role.as_str(), group.checks.len()))
        .collect::<BTreeMap<_, _>>();
    let inline_member_checks = members
        .iter()
        .map(|member| member.verification_checks.len())
        .sum::<usize>();
    let role_member_checks = members
        .iter()
        .map(|member| {
            role_check_counts
                .get(member.role.as_str())
                .copied()
                .unwrap_or(0)
        })
        .sum::<usize>();
    let member_checks = inline_member_checks + role_member_checks;
    let members_with_checks = members
        .iter()
        .filter(|member| {
            !member.verification_checks.is_empty()
                || role_check_counts.contains_key(member.role.as_str())
        })
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
        planned_snapshot_loads: member_count,
        planned_code_reinstalls: member_count,
        planned_verification_checks: verification_summary.total_checks,
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
mod tests {
    use super::*;
    use crate::manifest::{
        BackupUnit, BackupUnitKind, ConsistencyMode, ConsistencySection, FleetSection,
        MemberVerificationChecks, SourceMetadata, SourceSnapshot, ToolMetadata, VerificationCheck,
        VerificationPlan,
    };
    use std::{
        env, fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    const ROOT: &str = "aaaaa-aa";
    const CHILD: &str = "renrk-eyaaa-aaaaa-aaada-cai";
    const CHILD_TWO: &str = "r7inp-6aaaa-aaaaa-aaabq-cai";
    const TARGET: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    // Build a one-operation ready journal for command preview tests.
    fn command_preview_journal(
        operation: RestoreApplyOperationKind,
        verification_kind: Option<&str>,
        verification_method: Option<&str>,
    ) -> RestoreApplyJournal {
        let journal = RestoreApplyJournal {
            journal_version: 1,
            backup_id: "fbk_test_001".to_string(),
            ready: true,
            blocked_reasons: Vec::new(),
            operation_count: 1,
            pending_operations: 0,
            ready_operations: 1,
            blocked_operations: 0,
            completed_operations: 0,
            failed_operations: 0,
            operations: vec![RestoreApplyJournalOperation {
                sequence: 0,
                operation,
                state: RestoreApplyOperationState::Ready,
                state_updated_at: None,
                blocking_reasons: Vec::new(),
                restore_group: 1,
                phase_order: 0,
                source_canister: ROOT.to_string(),
                target_canister: ROOT.to_string(),
                role: "root".to_string(),
                snapshot_id: Some("snap-root".to_string()),
                artifact_path: Some("artifacts/root".to_string()),
                verification_kind: verification_kind.map(str::to_string),
                verification_method: verification_method.map(str::to_string),
            }],
        };

        journal.validate().expect("journal should validate");
        journal
    }

    // Build one valid manifest with a parent and child in the same restore group.
    fn valid_manifest(identity_mode: IdentityMode) -> FleetBackupManifest {
        FleetBackupManifest {
            manifest_version: 1,
            backup_id: "fbk_test_001".to_string(),
            created_at: "2026-04-10T12:00:00Z".to_string(),
            tool: ToolMetadata {
                name: "canic".to_string(),
                version: "v1".to_string(),
            },
            source: SourceMetadata {
                environment: "local".to_string(),
                root_canister: ROOT.to_string(),
            },
            consistency: ConsistencySection {
                mode: ConsistencyMode::CrashConsistent,
                backup_units: vec![BackupUnit {
                    unit_id: "whole-fleet".to_string(),
                    kind: BackupUnitKind::WholeFleet,
                    roles: vec!["root".to_string(), "app".to_string()],
                    consistency_reason: None,
                    dependency_closure: Vec::new(),
                    topology_validation: "subtree-closed".to_string(),
                    quiescence_strategy: None,
                }],
            },
            fleet: FleetSection {
                topology_hash_algorithm: "sha256".to_string(),
                topology_hash_input: "sorted(pid,parent_pid,role,module_hash)".to_string(),
                discovery_topology_hash: HASH.to_string(),
                pre_snapshot_topology_hash: HASH.to_string(),
                topology_hash: HASH.to_string(),
                members: vec![
                    fleet_member("app", CHILD, Some(ROOT), identity_mode, 1),
                    fleet_member("root", ROOT, None, IdentityMode::Fixed, 1),
                ],
            },
            verification: VerificationPlan {
                fleet_checks: Vec::new(),
                member_checks: Vec::new(),
            },
        }
    }

    // Build one manifest member for restore planning tests.
    fn fleet_member(
        role: &str,
        canister_id: &str,
        parent_canister_id: Option<&str>,
        identity_mode: IdentityMode,
        restore_group: u16,
    ) -> FleetMember {
        FleetMember {
            role: role.to_string(),
            canister_id: canister_id.to_string(),
            parent_canister_id: parent_canister_id.map(str::to_string),
            subnet_canister_id: None,
            controller_hint: Some(ROOT.to_string()),
            identity_mode,
            restore_group,
            verification_class: "basic".to_string(),
            verification_checks: vec![VerificationCheck {
                kind: "call".to_string(),
                method: Some("canic_ready".to_string()),
                roles: Vec::new(),
            }],
            source_snapshot: SourceSnapshot {
                snapshot_id: format!("snap-{role}"),
                module_hash: Some(HASH.to_string()),
                wasm_hash: Some(HASH.to_string()),
                code_version: Some("v0.30.0".to_string()),
                artifact_path: format!("artifacts/{role}"),
                checksum_algorithm: "sha256".to_string(),
                checksum: Some(HASH.to_string()),
            },
        }
    }

    // Ensure in-place restore planning sorts parent before child.
    #[test]
    fn in_place_plan_orders_parent_before_child() {
        let manifest = valid_manifest(IdentityMode::Relocatable);

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let ordered = plan.ordered_members();

        assert_eq!(plan.backup_id, "fbk_test_001");
        assert_eq!(plan.source_environment, "local");
        assert_eq!(plan.source_root_canister, ROOT);
        assert_eq!(plan.topology_hash, HASH);
        assert_eq!(plan.member_count, 2);
        assert_eq!(plan.identity_summary.fixed_members, 1);
        assert_eq!(plan.identity_summary.relocatable_members, 1);
        assert_eq!(plan.identity_summary.in_place_members, 2);
        assert_eq!(plan.identity_summary.mapped_members, 0);
        assert_eq!(plan.identity_summary.remapped_members, 0);
        assert!(plan.verification_summary.verification_required);
        assert!(plan.verification_summary.all_members_have_checks);
        assert!(plan.readiness_summary.ready);
        assert!(plan.readiness_summary.reasons.is_empty());
        assert_eq!(plan.verification_summary.fleet_checks, 0);
        assert_eq!(plan.verification_summary.member_check_groups, 0);
        assert_eq!(plan.verification_summary.member_checks, 2);
        assert_eq!(plan.verification_summary.members_with_checks, 2);
        assert_eq!(plan.verification_summary.total_checks, 2);
        assert_eq!(plan.ordering_summary.phase_count, 1);
        assert_eq!(plan.ordering_summary.dependency_free_members, 1);
        assert_eq!(plan.ordering_summary.in_group_parent_edges, 1);
        assert_eq!(plan.ordering_summary.cross_group_parent_edges, 0);
        assert_eq!(ordered[0].phase_order, 0);
        assert_eq!(ordered[1].phase_order, 1);
        assert_eq!(ordered[0].source_canister, ROOT);
        assert_eq!(ordered[1].source_canister, CHILD);
        assert_eq!(
            ordered[1].ordering_dependency,
            Some(RestoreOrderingDependency {
                source_canister: ROOT.to_string(),
                target_canister: ROOT.to_string(),
                relationship: RestoreOrderingRelationship::ParentInSameGroup,
            })
        );
    }

    // Ensure cross-group parent dependencies are exposed when the parent phase is earlier.
    #[test]
    fn plan_reports_parent_dependency_from_earlier_group() {
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        manifest.fleet.members[0].restore_group = 2;
        manifest.fleet.members[1].restore_group = 1;

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let ordered = plan.ordered_members();

        assert_eq!(plan.phases.len(), 2);
        assert_eq!(plan.ordering_summary.phase_count, 2);
        assert_eq!(plan.ordering_summary.dependency_free_members, 1);
        assert_eq!(plan.ordering_summary.in_group_parent_edges, 0);
        assert_eq!(plan.ordering_summary.cross_group_parent_edges, 1);
        assert_eq!(ordered[0].source_canister, ROOT);
        assert_eq!(ordered[1].source_canister, CHILD);
        assert_eq!(
            ordered[1].ordering_dependency,
            Some(RestoreOrderingDependency {
                source_canister: ROOT.to_string(),
                target_canister: ROOT.to_string(),
                relationship: RestoreOrderingRelationship::ParentInEarlierGroup,
            })
        );
    }

    // Ensure restore planning fails when groups would restore a child before its parent.
    #[test]
    fn plan_rejects_parent_in_later_restore_group() {
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        manifest.fleet.members[0].restore_group = 1;
        manifest.fleet.members[1].restore_group = 2;

        let err = RestorePlanner::plan(&manifest, None)
            .expect_err("parent-after-child group ordering should fail");

        assert!(matches!(
            err,
            RestorePlanError::ParentRestoreGroupAfterChild { .. }
        ));
    }

    // Ensure fixed identities cannot be remapped.
    #[test]
    fn fixed_identity_member_cannot_be_remapped() {
        let manifest = valid_manifest(IdentityMode::Fixed);
        let mapping = RestoreMapping {
            members: vec![
                RestoreMappingEntry {
                    source_canister: ROOT.to_string(),
                    target_canister: ROOT.to_string(),
                },
                RestoreMappingEntry {
                    source_canister: CHILD.to_string(),
                    target_canister: TARGET.to_string(),
                },
            ],
        };

        let err = RestorePlanner::plan(&manifest, Some(&mapping))
            .expect_err("fixed member remap should fail");

        assert!(matches!(err, RestorePlanError::FixedIdentityRemap { .. }));
    }

    // Ensure relocatable identities may be mapped when all members are covered.
    #[test]
    fn relocatable_member_can_be_mapped() {
        let manifest = valid_manifest(IdentityMode::Relocatable);
        let mapping = RestoreMapping {
            members: vec![
                RestoreMappingEntry {
                    source_canister: ROOT.to_string(),
                    target_canister: ROOT.to_string(),
                },
                RestoreMappingEntry {
                    source_canister: CHILD.to_string(),
                    target_canister: TARGET.to_string(),
                },
            ],
        };

        let plan = RestorePlanner::plan(&manifest, Some(&mapping)).expect("plan should build");
        let child = plan
            .ordered_members()
            .into_iter()
            .find(|member| member.source_canister == CHILD)
            .expect("child member should be planned");

        assert_eq!(plan.identity_summary.fixed_members, 1);
        assert_eq!(plan.identity_summary.relocatable_members, 1);
        assert_eq!(plan.identity_summary.in_place_members, 1);
        assert_eq!(plan.identity_summary.mapped_members, 2);
        assert_eq!(plan.identity_summary.remapped_members, 1);
        assert_eq!(child.target_canister, TARGET);
        assert_eq!(child.parent_target_canister, Some(ROOT.to_string()));
    }

    // Ensure restore plans carry enough metadata for operator preflight.
    #[test]
    fn plan_members_include_snapshot_and_verification_metadata() {
        let manifest = valid_manifest(IdentityMode::Relocatable);

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let root = plan
            .ordered_members()
            .into_iter()
            .find(|member| member.source_canister == ROOT)
            .expect("root member should be planned");

        assert_eq!(root.identity_mode, IdentityMode::Fixed);
        assert_eq!(root.verification_class, "basic");
        assert_eq!(root.verification_checks[0].kind, "call");
        assert_eq!(root.source_snapshot.snapshot_id, "snap-root");
        assert_eq!(root.source_snapshot.artifact_path, "artifacts/root");
    }

    // Ensure restore plans make mapping mode explicit.
    #[test]
    fn plan_includes_mapping_summary() {
        let manifest = valid_manifest(IdentityMode::Relocatable);
        let in_place = RestorePlanner::plan(&manifest, None).expect("plan should build");

        assert!(!in_place.identity_summary.mapping_supplied);
        assert!(!in_place.identity_summary.all_sources_mapped);
        assert_eq!(in_place.identity_summary.mapped_members, 0);

        let mapping = RestoreMapping {
            members: vec![
                RestoreMappingEntry {
                    source_canister: ROOT.to_string(),
                    target_canister: ROOT.to_string(),
                },
                RestoreMappingEntry {
                    source_canister: CHILD.to_string(),
                    target_canister: TARGET.to_string(),
                },
            ],
        };
        let mapped = RestorePlanner::plan(&manifest, Some(&mapping)).expect("plan should build");

        assert!(mapped.identity_summary.mapping_supplied);
        assert!(mapped.identity_summary.all_sources_mapped);
        assert_eq!(mapped.identity_summary.mapped_members, 2);
        assert_eq!(mapped.identity_summary.remapped_members, 1);
    }

    // Ensure restore plans summarize snapshot provenance completeness.
    #[test]
    fn plan_includes_snapshot_summary() {
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        manifest.fleet.members[1].source_snapshot.module_hash = None;
        manifest.fleet.members[1].source_snapshot.wasm_hash = None;
        manifest.fleet.members[1].source_snapshot.checksum = None;

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");

        assert!(!plan.snapshot_summary.all_members_have_module_hash);
        assert!(!plan.snapshot_summary.all_members_have_wasm_hash);
        assert!(plan.snapshot_summary.all_members_have_code_version);
        assert!(!plan.snapshot_summary.all_members_have_checksum);
        assert_eq!(plan.snapshot_summary.members_with_module_hash, 1);
        assert_eq!(plan.snapshot_summary.members_with_wasm_hash, 1);
        assert_eq!(plan.snapshot_summary.members_with_code_version, 2);
        assert_eq!(plan.snapshot_summary.members_with_checksum, 1);
        assert!(!plan.readiness_summary.ready);
        assert_eq!(
            plan.readiness_summary.reasons,
            [
                "missing-module-hash",
                "missing-wasm-hash",
                "missing-snapshot-checksum"
            ]
        );
    }

    // Ensure restore plans summarize manifest-level verification work.
    #[test]
    fn plan_includes_verification_summary() {
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        manifest.verification.fleet_checks.push(VerificationCheck {
            kind: "fleet-ready".to_string(),
            method: None,
            roles: Vec::new(),
        });
        manifest
            .verification
            .member_checks
            .push(MemberVerificationChecks {
                role: "app".to_string(),
                checks: vec![VerificationCheck {
                    kind: "app-ready".to_string(),
                    method: Some("ready".to_string()),
                    roles: Vec::new(),
                }],
            });

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");

        assert!(plan.verification_summary.verification_required);
        assert!(plan.verification_summary.all_members_have_checks);
        assert_eq!(plan.verification_summary.fleet_checks, 1);
        assert_eq!(plan.verification_summary.member_check_groups, 1);
        assert_eq!(plan.verification_summary.member_checks, 3);
        assert_eq!(plan.verification_summary.members_with_checks, 2);
        assert_eq!(plan.verification_summary.total_checks, 4);
    }

    // Ensure restore plans summarize the concrete operation counts automation will schedule.
    #[test]
    fn plan_includes_operation_summary() {
        let manifest = valid_manifest(IdentityMode::Relocatable);

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");

        assert_eq!(plan.operation_summary.planned_snapshot_loads, 2);
        assert_eq!(plan.operation_summary.planned_code_reinstalls, 2);
        assert_eq!(plan.operation_summary.planned_verification_checks, 2);
        assert_eq!(plan.operation_summary.planned_phases, 1);
    }

    // Ensure initial restore status mirrors the no-mutation restore plan.
    #[test]
    fn restore_status_starts_all_members_as_planned() {
        let manifest = valid_manifest(IdentityMode::Relocatable);

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let status = RestoreStatus::from_plan(&plan);

        assert_eq!(status.status_version, 1);
        assert_eq!(status.backup_id.as_str(), plan.backup_id.as_str());
        assert_eq!(
            status.source_environment.as_str(),
            plan.source_environment.as_str()
        );
        assert_eq!(
            status.source_root_canister.as_str(),
            plan.source_root_canister.as_str()
        );
        assert_eq!(status.topology_hash.as_str(), plan.topology_hash.as_str());
        assert!(status.ready);
        assert!(status.readiness_reasons.is_empty());
        assert!(status.verification_required);
        assert_eq!(status.member_count, 2);
        assert_eq!(status.phase_count, 1);
        assert_eq!(status.planned_snapshot_loads, 2);
        assert_eq!(status.planned_code_reinstalls, 2);
        assert_eq!(status.planned_verification_checks, 2);
        assert_eq!(status.phases.len(), 1);
        assert_eq!(status.phases[0].restore_group, 1);
        assert_eq!(status.phases[0].members.len(), 2);
        assert_eq!(
            status.phases[0].members[0].state,
            RestoreMemberState::Planned
        );
        assert_eq!(status.phases[0].members[0].source_canister, ROOT);
        assert_eq!(status.phases[0].members[0].target_canister, ROOT);
        assert_eq!(status.phases[0].members[0].snapshot_id, "snap-root");
        assert_eq!(status.phases[0].members[0].artifact_path, "artifacts/root");
        assert_eq!(
            status.phases[0].members[1].state,
            RestoreMemberState::Planned
        );
        assert_eq!(status.phases[0].members[1].source_canister, CHILD);
    }

    // Ensure apply dry-runs render ordered operations without mutating targets.
    #[test]
    fn apply_dry_run_renders_ordered_member_operations() {
        let manifest = valid_manifest(IdentityMode::Relocatable);

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let status = RestoreStatus::from_plan(&plan);
        let dry_run =
            RestoreApplyDryRun::try_from_plan(&plan, Some(&status)).expect("dry-run should build");

        assert_eq!(dry_run.dry_run_version, 1);
        assert_eq!(dry_run.backup_id.as_str(), "fbk_test_001");
        assert!(dry_run.ready);
        assert!(dry_run.status_supplied);
        assert_eq!(dry_run.member_count, 2);
        assert_eq!(dry_run.phase_count, 1);
        assert_eq!(dry_run.planned_snapshot_loads, 2);
        assert_eq!(dry_run.planned_code_reinstalls, 2);
        assert_eq!(dry_run.planned_verification_checks, 2);
        assert_eq!(dry_run.rendered_operations, 8);
        assert_eq!(dry_run.phases.len(), 1);

        let operations = &dry_run.phases[0].operations;
        assert_eq!(operations[0].sequence, 0);
        assert_eq!(
            operations[0].operation,
            RestoreApplyOperationKind::UploadSnapshot
        );
        assert_eq!(operations[0].source_canister, ROOT);
        assert_eq!(operations[0].target_canister, ROOT);
        assert_eq!(operations[0].snapshot_id, Some("snap-root".to_string()));
        assert_eq!(
            operations[0].artifact_path,
            Some("artifacts/root".to_string())
        );
        assert_eq!(
            operations[1].operation,
            RestoreApplyOperationKind::LoadSnapshot
        );
        assert_eq!(
            operations[2].operation,
            RestoreApplyOperationKind::ReinstallCode
        );
        assert_eq!(
            operations[3].operation,
            RestoreApplyOperationKind::VerifyMember
        );
        assert_eq!(operations[3].verification_kind, Some("call".to_string()));
        assert_eq!(
            operations[3].verification_method,
            Some("canic_ready".to_string())
        );
        assert_eq!(operations[4].source_canister, CHILD);
        assert_eq!(
            operations[7].operation,
            RestoreApplyOperationKind::VerifyMember
        );
    }

    // Ensure apply dry-run operation sequences remain unique across phases.
    #[test]
    fn apply_dry_run_sequences_operations_across_phases() {
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        manifest.fleet.members[0].restore_group = 2;

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");

        assert_eq!(dry_run.phases.len(), 2);
        assert_eq!(dry_run.rendered_operations, 8);
        assert_eq!(dry_run.phases[0].operations[0].sequence, 0);
        assert_eq!(dry_run.phases[0].operations[3].sequence, 3);
        assert_eq!(dry_run.phases[1].operations[0].sequence, 4);
        assert_eq!(dry_run.phases[1].operations[3].sequence, 7);
    }

    // Ensure apply dry-runs can prove referenced artifacts exist and match checksums.
    #[test]
    fn apply_dry_run_validates_artifacts_under_backup_root() {
        let root = temp_dir("canic-restore-apply-artifacts-ok");
        fs::create_dir_all(&root).expect("create temp root");
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        set_member_artifact(
            &mut manifest,
            CHILD,
            &root,
            "artifacts/child",
            b"child-snapshot",
        );
        set_member_artifact(
            &mut manifest,
            ROOT,
            &root,
            "artifacts/root",
            b"root-snapshot",
        );

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
            .expect("dry-run should validate artifacts");

        let validation = dry_run
            .artifact_validation
            .expect("artifact validation should be present");
        assert_eq!(validation.checked_members, 2);
        assert!(validation.artifacts_present);
        assert!(validation.checksums_verified);
        assert_eq!(validation.members_with_expected_checksums, 2);
        assert_eq!(validation.checks[0].source_canister, ROOT);
        assert!(validation.checks[0].checksum_verified);

        fs::remove_dir_all(root).expect("remove temp root");
    }

    // Ensure an artifact-validated apply dry-run produces a ready initial journal.
    #[test]
    fn apply_journal_marks_validated_operations_ready() {
        let root = temp_dir("canic-restore-apply-journal-ready");
        fs::create_dir_all(&root).expect("create temp root");
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        set_member_artifact(
            &mut manifest,
            CHILD,
            &root,
            "artifacts/child",
            b"child-snapshot",
        );
        set_member_artifact(
            &mut manifest,
            ROOT,
            &root,
            "artifacts/root",
            b"root-snapshot",
        );

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
            .expect("dry-run should validate artifacts");
        let journal = RestoreApplyJournal::from_dry_run(&dry_run);

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(journal.journal_version, 1);
        assert_eq!(journal.backup_id.as_str(), "fbk_test_001");
        assert!(journal.ready);
        assert!(journal.blocked_reasons.is_empty());
        assert_eq!(journal.operation_count, 8);
        assert_eq!(journal.ready_operations, 8);
        assert_eq!(journal.blocked_operations, 0);
        assert_eq!(journal.operations[0].sequence, 0);
        assert_eq!(
            journal.operations[0].state,
            RestoreApplyOperationState::Ready
        );
        assert!(journal.operations[0].blocking_reasons.is_empty());
    }

    // Ensure apply journals block when artifact validation was not supplied.
    #[test]
    fn apply_journal_blocks_without_artifact_validation() {
        let manifest = valid_manifest(IdentityMode::Relocatable);

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");
        let journal = RestoreApplyJournal::from_dry_run(&dry_run);

        assert!(!journal.ready);
        assert_eq!(journal.ready_operations, 0);
        assert_eq!(journal.blocked_operations, 8);
        assert!(
            journal
                .blocked_reasons
                .contains(&"missing-artifact-validation".to_string())
        );
        assert!(
            journal.operations[0]
                .blocking_reasons
                .contains(&"missing-artifact-validation".to_string())
        );
    }

    // Ensure apply journal status exposes compact readiness and next-operation state.
    #[test]
    fn apply_journal_status_reports_next_ready_operation() {
        let root = temp_dir("canic-restore-apply-journal-status");
        fs::create_dir_all(&root).expect("create temp root");
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        set_member_artifact(
            &mut manifest,
            CHILD,
            &root,
            "artifacts/child",
            b"child-snapshot",
        );
        set_member_artifact(
            &mut manifest,
            ROOT,
            &root,
            "artifacts/root",
            b"root-snapshot",
        );

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
            .expect("dry-run should validate artifacts");
        let journal = RestoreApplyJournal::from_dry_run(&dry_run);
        let status = journal.status();

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(status.status_version, 1);
        assert_eq!(status.backup_id.as_str(), "fbk_test_001");
        assert!(status.ready);
        assert!(!status.complete);
        assert_eq!(status.operation_count, 8);
        assert_eq!(status.ready_operations, 8);
        assert_eq!(status.next_ready_sequence, Some(0));
        assert_eq!(
            status.next_ready_operation,
            Some(RestoreApplyOperationKind::UploadSnapshot)
        );
        assert_eq!(status.next_transition_sequence, Some(0));
        assert_eq!(
            status.next_transition_state,
            Some(RestoreApplyOperationState::Ready)
        );
        assert_eq!(
            status.next_transition_operation,
            Some(RestoreApplyOperationKind::UploadSnapshot)
        );
    }

    // Ensure next-operation output exposes the full next ready journal row.
    #[test]
    fn apply_journal_next_operation_reports_full_ready_row() {
        let root = temp_dir("canic-restore-apply-journal-next");
        fs::create_dir_all(&root).expect("create temp root");
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        set_member_artifact(
            &mut manifest,
            CHILD,
            &root,
            "artifacts/child",
            b"child-snapshot",
        );
        set_member_artifact(
            &mut manifest,
            ROOT,
            &root,
            "artifacts/root",
            b"root-snapshot",
        );

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
            .expect("dry-run should validate artifacts");
        let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
        journal
            .mark_operation_completed(0)
            .expect("mark operation completed");
        let next = journal.next_operation();

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(next.ready);
        assert!(!next.complete);
        assert!(next.operation_available);
        let operation = next.operation.expect("next operation");
        assert_eq!(operation.sequence, 1);
        assert_eq!(operation.state, RestoreApplyOperationState::Ready);
        assert_eq!(operation.operation, RestoreApplyOperationKind::LoadSnapshot);
        assert_eq!(operation.source_canister, ROOT);
    }

    // Ensure blocked journals report no next ready operation.
    #[test]
    fn apply_journal_next_operation_reports_blocked_state() {
        let manifest = valid_manifest(IdentityMode::Relocatable);

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");
        let journal = RestoreApplyJournal::from_dry_run(&dry_run);
        let next = journal.next_operation();

        assert!(!next.ready);
        assert!(!next.operation_available);
        assert!(next.operation.is_none());
        assert!(
            next.blocked_reasons
                .contains(&"missing-artifact-validation".to_string())
        );
    }

    // Ensure command previews expose the dfx upload command without executing it.
    #[test]
    fn apply_journal_command_preview_reports_upload_command() {
        let root = temp_dir("canic-restore-apply-command-upload");
        fs::create_dir_all(&root).expect("create temp root");
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        set_member_artifact(
            &mut manifest,
            CHILD,
            &root,
            "artifacts/child",
            b"child-snapshot",
        );
        set_member_artifact(
            &mut manifest,
            ROOT,
            &root,
            "artifacts/root",
            b"root-snapshot",
        );

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
            .expect("dry-run should validate artifacts");
        let journal = RestoreApplyJournal::from_dry_run(&dry_run);
        let preview = journal.next_command_preview();

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(preview.ready);
        assert!(preview.operation_available);
        assert!(preview.command_available);
        let command = preview.command.expect("command preview");
        assert_eq!(command.program, "dfx");
        assert_eq!(
            command.args,
            vec![
                "canister".to_string(),
                "snapshot".to_string(),
                "upload".to_string(),
                "--dir".to_string(),
                "artifacts/root".to_string(),
                ROOT.to_string(),
            ]
        );
        assert!(command.mutates);
        assert!(!command.requires_stopped_canister);
    }

    // Ensure command previews carry configured dfx program and network.
    #[test]
    fn apply_journal_command_preview_honors_command_config() {
        let root = temp_dir("canic-restore-apply-command-config");
        fs::create_dir_all(&root).expect("create temp root");
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        set_member_artifact(
            &mut manifest,
            CHILD,
            &root,
            "artifacts/child",
            b"child-snapshot",
        );
        set_member_artifact(
            &mut manifest,
            ROOT,
            &root,
            "artifacts/root",
            b"root-snapshot",
        );

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
            .expect("dry-run should validate artifacts");
        let journal = RestoreApplyJournal::from_dry_run(&dry_run);
        let preview = journal.next_command_preview_with_config(&RestoreApplyCommandConfig {
            program: "/tmp/dfx".to_string(),
            network: Some("local".to_string()),
        });

        fs::remove_dir_all(root).expect("remove temp root");
        let command = preview.command.expect("command preview");
        assert_eq!(command.program, "/tmp/dfx");
        assert_eq!(
            command.args,
            vec![
                "canister".to_string(),
                "--network".to_string(),
                "local".to_string(),
                "snapshot".to_string(),
                "upload".to_string(),
                "--dir".to_string(),
                "artifacts/root".to_string(),
                ROOT.to_string(),
            ]
        );
    }

    // Ensure command previews expose stopped-canister hints for snapshot load.
    #[test]
    fn apply_journal_command_preview_reports_load_command() {
        let root = temp_dir("canic-restore-apply-command-load");
        fs::create_dir_all(&root).expect("create temp root");
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        set_member_artifact(
            &mut manifest,
            CHILD,
            &root,
            "artifacts/child",
            b"child-snapshot",
        );
        set_member_artifact(
            &mut manifest,
            ROOT,
            &root,
            "artifacts/root",
            b"root-snapshot",
        );

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
            .expect("dry-run should validate artifacts");
        let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
        journal
            .mark_operation_completed(0)
            .expect("mark upload completed");
        let preview = journal.next_command_preview();

        fs::remove_dir_all(root).expect("remove temp root");
        let command = preview.command.expect("command preview");
        assert_eq!(
            command.args,
            vec![
                "canister".to_string(),
                "snapshot".to_string(),
                "load".to_string(),
                ROOT.to_string(),
                "snap-root".to_string(),
            ]
        );
        assert!(command.mutates);
        assert!(command.requires_stopped_canister);
    }

    // Ensure command previews expose reinstall commands without executing them.
    #[test]
    fn apply_journal_command_preview_reports_reinstall_command() {
        let journal = command_preview_journal(RestoreApplyOperationKind::ReinstallCode, None, None);
        let preview = journal.next_command_preview_with_config(&RestoreApplyCommandConfig {
            program: "dfx".to_string(),
            network: Some("local".to_string()),
        });

        assert!(preview.command_available);
        let command = preview.command.expect("command preview");
        assert_eq!(
            command.args,
            vec![
                "canister".to_string(),
                "--network".to_string(),
                "local".to_string(),
                "install".to_string(),
                "--mode".to_string(),
                "reinstall".to_string(),
                "--yes".to_string(),
                ROOT.to_string(),
            ]
        );
        assert!(command.mutates);
        assert!(!command.requires_stopped_canister);
    }

    // Ensure status verification previews use `dfx canister status`.
    #[test]
    fn apply_journal_command_preview_reports_status_verification_command() {
        let journal = command_preview_journal(
            RestoreApplyOperationKind::VerifyMember,
            Some("status"),
            None,
        );
        let preview = journal.next_command_preview();

        assert!(preview.command_available);
        let command = preview.command.expect("command preview");
        assert_eq!(
            command.args,
            vec![
                "canister".to_string(),
                "status".to_string(),
                ROOT.to_string()
            ]
        );
        assert!(!command.mutates);
        assert!(!command.requires_stopped_canister);
    }

    // Ensure method verification previews use `dfx canister call`.
    #[test]
    fn apply_journal_command_preview_reports_method_verification_command() {
        let journal = command_preview_journal(
            RestoreApplyOperationKind::VerifyMember,
            Some("query"),
            Some("health"),
        );
        let preview = journal.next_command_preview();

        assert!(preview.command_available);
        let command = preview.command.expect("command preview");
        assert_eq!(
            command.args,
            vec![
                "canister".to_string(),
                "call".to_string(),
                ROOT.to_string(),
                "health".to_string(),
            ]
        );
        assert!(!command.mutates);
        assert!(!command.requires_stopped_canister);
    }

    // Ensure unsupported verification rows do not pretend to be runnable.
    #[test]
    fn apply_journal_command_preview_reports_unavailable_for_unknown_verification() {
        let journal =
            command_preview_journal(RestoreApplyOperationKind::VerifyMember, Some("query"), None);
        let preview = journal.next_command_preview();

        assert!(preview.operation_available);
        assert!(!preview.command_available);
        assert!(preview.command.is_none());
    }

    // Ensure apply journal validation rejects inconsistent state counts.
    #[test]
    fn apply_journal_validation_rejects_count_mismatch() {
        let manifest = valid_manifest(IdentityMode::Relocatable);

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");
        let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
        journal.blocked_operations = 0;

        let err = journal.validate().expect_err("count mismatch should fail");

        assert!(matches!(
            err,
            RestoreApplyJournalError::CountMismatch {
                field: "blocked_operations",
                ..
            }
        ));
    }

    // Ensure apply journal validation rejects duplicate operation sequences.
    #[test]
    fn apply_journal_validation_rejects_duplicate_sequences() {
        let manifest = valid_manifest(IdentityMode::Relocatable);

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");
        let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
        journal.operations[1].sequence = journal.operations[0].sequence;

        let err = journal
            .validate()
            .expect_err("duplicate sequence should fail");

        assert!(matches!(
            err,
            RestoreApplyJournalError::DuplicateSequence(0)
        ));
    }

    // Ensure failed journal operations must explain why execution failed.
    #[test]
    fn apply_journal_validation_rejects_failed_without_reason() {
        let manifest = valid_manifest(IdentityMode::Relocatable);

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");
        let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);
        journal.operations[0].state = RestoreApplyOperationState::Failed;
        journal.operations[0].blocking_reasons = Vec::new();
        journal.blocked_operations -= 1;
        journal.failed_operations = 1;

        let err = journal
            .validate()
            .expect_err("failed operation without reason should fail");

        assert!(matches!(
            err,
            RestoreApplyJournalError::FailureReasonRequired(0)
        ));
    }

    // Ensure claiming a ready operation marks it pending and keeps it resumable.
    #[test]
    fn apply_journal_mark_next_operation_pending_claims_first_operation() {
        let mut journal =
            command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None, None);

        journal
            .mark_next_operation_pending_at(Some("2026-05-04T12:00:00Z".to_string()))
            .expect("mark operation pending");
        let status = journal.status();
        let next = journal.next_operation();
        let preview = journal.next_command_preview();

        assert_eq!(journal.pending_operations, 1);
        assert_eq!(journal.ready_operations, 0);
        assert_eq!(
            journal.operations[0].state,
            RestoreApplyOperationState::Pending
        );
        assert_eq!(
            journal.operations[0].state_updated_at.as_deref(),
            Some("2026-05-04T12:00:00Z")
        );
        assert_eq!(status.next_ready_sequence, None);
        assert_eq!(status.next_transition_sequence, Some(0));
        assert_eq!(
            status.next_transition_state,
            Some(RestoreApplyOperationState::Pending)
        );
        assert_eq!(
            status.next_transition_updated_at.as_deref(),
            Some("2026-05-04T12:00:00Z")
        );
        assert!(next.operation_available);
        assert_eq!(
            next.operation.expect("next operation").state,
            RestoreApplyOperationState::Pending
        );
        assert!(preview.operation_available);
        assert!(preview.command_available);
        assert_eq!(
            preview.operation.expect("preview operation").state,
            RestoreApplyOperationState::Pending
        );
    }

    // Ensure a pending claim can be released back to ready for retry.
    #[test]
    fn apply_journal_mark_next_operation_ready_unclaims_pending_operation() {
        let mut journal =
            command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None, None);

        journal
            .mark_next_operation_pending_at(Some("2026-05-04T12:00:00Z".to_string()))
            .expect("mark operation pending");
        journal
            .mark_next_operation_ready_at(Some("2026-05-04T12:01:00Z".to_string()))
            .expect("mark operation ready");
        let status = journal.status();
        let next = journal.next_operation();

        assert_eq!(journal.pending_operations, 0);
        assert_eq!(journal.ready_operations, 1);
        assert_eq!(
            journal.operations[0].state,
            RestoreApplyOperationState::Ready
        );
        assert_eq!(
            journal.operations[0].state_updated_at.as_deref(),
            Some("2026-05-04T12:01:00Z")
        );
        assert_eq!(status.next_ready_sequence, Some(0));
        assert_eq!(status.next_transition_sequence, Some(0));
        assert_eq!(
            status.next_transition_state,
            Some(RestoreApplyOperationState::Ready)
        );
        assert_eq!(
            status.next_transition_updated_at.as_deref(),
            Some("2026-05-04T12:01:00Z")
        );
        assert_eq!(
            next.operation.expect("next operation").state,
            RestoreApplyOperationState::Ready
        );
    }

    // Ensure empty state update markers are rejected during journal validation.
    #[test]
    fn apply_journal_validation_rejects_empty_state_updated_at() {
        let mut journal =
            command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None, None);

        journal.operations[0].state_updated_at = Some(String::new());
        let err = journal
            .validate()
            .expect_err("empty state update marker should fail");

        assert!(matches!(
            err,
            RestoreApplyJournalError::MissingField("operations[].state_updated_at")
        ));
    }

    // Ensure unclaim fails when the next transitionable operation is not pending.
    #[test]
    fn apply_journal_mark_next_operation_ready_rejects_without_pending_operation() {
        let mut journal =
            command_preview_journal(RestoreApplyOperationKind::UploadSnapshot, None, None);

        let err = journal
            .mark_next_operation_ready()
            .expect_err("ready operation should not unclaim");

        assert!(matches!(err, RestoreApplyJournalError::NoPendingOperation));
        assert_eq!(journal.ready_operations, 1);
        assert_eq!(journal.pending_operations, 0);
    }

    // Ensure pending claims cannot skip earlier ready operations.
    #[test]
    fn apply_journal_mark_pending_rejects_out_of_order_operation() {
        let root = temp_dir("canic-restore-apply-journal-pending-out-of-order");
        fs::create_dir_all(&root).expect("create temp root");
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        set_member_artifact(
            &mut manifest,
            CHILD,
            &root,
            "artifacts/child",
            b"child-snapshot",
        );
        set_member_artifact(
            &mut manifest,
            ROOT,
            &root,
            "artifacts/root",
            b"root-snapshot",
        );

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
            .expect("dry-run should validate artifacts");
        let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

        let err = journal
            .mark_operation_pending(1)
            .expect_err("out-of-order pending claim should fail");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(matches!(
            err,
            RestoreApplyJournalError::OutOfOrderOperationTransition {
                requested: 1,
                next: 0
            }
        ));
        assert_eq!(journal.pending_operations, 0);
        assert_eq!(journal.ready_operations, 8);
    }

    // Ensure completing a journal operation updates counts and advances status.
    #[test]
    fn apply_journal_mark_completed_advances_next_ready_operation() {
        let root = temp_dir("canic-restore-apply-journal-completed");
        fs::create_dir_all(&root).expect("create temp root");
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        set_member_artifact(
            &mut manifest,
            CHILD,
            &root,
            "artifacts/child",
            b"child-snapshot",
        );
        set_member_artifact(
            &mut manifest,
            ROOT,
            &root,
            "artifacts/root",
            b"root-snapshot",
        );

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
            .expect("dry-run should validate artifacts");
        let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

        journal
            .mark_operation_completed(0)
            .expect("mark operation completed");
        let status = journal.status();

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(
            journal.operations[0].state,
            RestoreApplyOperationState::Completed
        );
        assert_eq!(journal.completed_operations, 1);
        assert_eq!(journal.ready_operations, 7);
        assert_eq!(status.next_ready_sequence, Some(1));
    }

    // Ensure journal transitions cannot skip earlier ready operations.
    #[test]
    fn apply_journal_mark_completed_rejects_out_of_order_operation() {
        let root = temp_dir("canic-restore-apply-journal-out-of-order");
        fs::create_dir_all(&root).expect("create temp root");
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        set_member_artifact(
            &mut manifest,
            CHILD,
            &root,
            "artifacts/child",
            b"child-snapshot",
        );
        set_member_artifact(
            &mut manifest,
            ROOT,
            &root,
            "artifacts/root",
            b"root-snapshot",
        );

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
            .expect("dry-run should validate artifacts");
        let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

        let err = journal
            .mark_operation_completed(1)
            .expect_err("out-of-order operation should fail");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(matches!(
            err,
            RestoreApplyJournalError::OutOfOrderOperationTransition {
                requested: 1,
                next: 0
            }
        ));
        assert_eq!(journal.completed_operations, 0);
        assert_eq!(journal.ready_operations, 8);
    }

    // Ensure failed journal operations carry a reason and update counts.
    #[test]
    fn apply_journal_mark_failed_records_reason() {
        let root = temp_dir("canic-restore-apply-journal-failed");
        fs::create_dir_all(&root).expect("create temp root");
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        set_member_artifact(
            &mut manifest,
            CHILD,
            &root,
            "artifacts/child",
            b"child-snapshot",
        );
        set_member_artifact(
            &mut manifest,
            ROOT,
            &root,
            "artifacts/root",
            b"root-snapshot",
        );

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
            .expect("dry-run should validate artifacts");
        let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

        journal
            .mark_operation_failed(0, "dfx-load-failed".to_string())
            .expect("mark operation failed");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(
            journal.operations[0].state,
            RestoreApplyOperationState::Failed
        );
        assert_eq!(
            journal.operations[0].blocking_reasons,
            vec!["dfx-load-failed".to_string()]
        );
        assert_eq!(journal.failed_operations, 1);
        assert_eq!(journal.ready_operations, 7);
    }

    // Ensure blocked operations cannot be manually completed before blockers clear.
    #[test]
    fn apply_journal_rejects_blocked_operation_completion() {
        let manifest = valid_manifest(IdentityMode::Relocatable);

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let dry_run = RestoreApplyDryRun::try_from_plan(&plan, None).expect("dry-run should build");
        let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

        let err = journal
            .mark_operation_completed(0)
            .expect_err("blocked operation should not complete");

        assert!(matches!(
            err,
            RestoreApplyJournalError::InvalidOperationTransition { sequence: 0, .. }
        ));
    }

    // Ensure apply dry-runs fail closed when a referenced artifact is missing.
    #[test]
    fn apply_dry_run_rejects_missing_artifacts() {
        let root = temp_dir("canic-restore-apply-artifacts-missing");
        fs::create_dir_all(&root).expect("create temp root");
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        manifest.fleet.members[0].source_snapshot.artifact_path = "missing-child".to_string();

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let err = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
            .expect_err("missing artifact should fail");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(matches!(
            err,
            RestoreApplyDryRunError::ArtifactMissing { .. }
        ));
    }

    // Ensure apply dry-runs reject artifact paths that escape the backup directory.
    #[test]
    fn apply_dry_run_rejects_artifact_path_traversal() {
        let root = temp_dir("canic-restore-apply-artifacts-traversal");
        fs::create_dir_all(&root).expect("create temp root");
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        manifest.fleet.members[1].source_snapshot.artifact_path = "../outside".to_string();

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let err = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, None, &root)
            .expect_err("path traversal should fail");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(matches!(
            err,
            RestoreApplyDryRunError::ArtifactPathEscapesBackup { .. }
        ));
    }

    // Ensure apply dry-runs reject status files that do not match the plan.
    #[test]
    fn apply_dry_run_rejects_mismatched_status() {
        let manifest = valid_manifest(IdentityMode::Relocatable);

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");
        let mut status = RestoreStatus::from_plan(&plan);
        status.backup_id = "other-backup".to_string();

        let err = RestoreApplyDryRun::try_from_plan(&plan, Some(&status))
            .expect_err("mismatched status should fail");

        assert!(matches!(
            err,
            RestoreApplyDryRunError::StatusPlanMismatch {
                field: "backup_id",
                ..
            }
        ));
    }

    // Ensure role-level verification checks are counted once per matching member.
    #[test]
    fn plan_expands_role_verification_checks_per_matching_member() {
        let mut manifest = valid_manifest(IdentityMode::Relocatable);
        manifest.fleet.members.push(fleet_member(
            "app",
            CHILD_TWO,
            Some(ROOT),
            IdentityMode::Relocatable,
            1,
        ));
        manifest
            .verification
            .member_checks
            .push(MemberVerificationChecks {
                role: "app".to_string(),
                checks: vec![VerificationCheck {
                    kind: "app-ready".to_string(),
                    method: Some("ready".to_string()),
                    roles: Vec::new(),
                }],
            });

        let plan = RestorePlanner::plan(&manifest, None).expect("plan should build");

        assert_eq!(plan.verification_summary.fleet_checks, 0);
        assert_eq!(plan.verification_summary.member_check_groups, 1);
        assert_eq!(plan.verification_summary.member_checks, 5);
        assert_eq!(plan.verification_summary.members_with_checks, 3);
        assert_eq!(plan.verification_summary.total_checks, 5);
    }

    // Ensure mapped restores must cover every source member.
    #[test]
    fn mapped_restore_requires_complete_mapping() {
        let manifest = valid_manifest(IdentityMode::Relocatable);
        let mapping = RestoreMapping {
            members: vec![RestoreMappingEntry {
                source_canister: ROOT.to_string(),
                target_canister: ROOT.to_string(),
            }],
        };

        let err = RestorePlanner::plan(&manifest, Some(&mapping))
            .expect_err("incomplete mapping should fail");

        assert!(matches!(err, RestorePlanError::MissingMappingSource(_)));
    }

    // Ensure mappings cannot silently include canisters outside the manifest.
    #[test]
    fn mapped_restore_rejects_unknown_mapping_sources() {
        let manifest = valid_manifest(IdentityMode::Relocatable);
        let unknown = "rdmx6-jaaaa-aaaaa-aaadq-cai";
        let mapping = RestoreMapping {
            members: vec![
                RestoreMappingEntry {
                    source_canister: ROOT.to_string(),
                    target_canister: ROOT.to_string(),
                },
                RestoreMappingEntry {
                    source_canister: CHILD.to_string(),
                    target_canister: TARGET.to_string(),
                },
                RestoreMappingEntry {
                    source_canister: unknown.to_string(),
                    target_canister: unknown.to_string(),
                },
            ],
        };

        let err = RestorePlanner::plan(&manifest, Some(&mapping))
            .expect_err("unknown mapping source should fail");

        assert!(matches!(err, RestorePlanError::UnknownMappingSource(_)));
    }

    // Ensure duplicate target mappings fail before a plan is produced.
    #[test]
    fn duplicate_mapping_targets_fail_validation() {
        let manifest = valid_manifest(IdentityMode::Relocatable);
        let mapping = RestoreMapping {
            members: vec![
                RestoreMappingEntry {
                    source_canister: ROOT.to_string(),
                    target_canister: ROOT.to_string(),
                },
                RestoreMappingEntry {
                    source_canister: CHILD.to_string(),
                    target_canister: ROOT.to_string(),
                },
            ],
        };

        let err = RestorePlanner::plan(&manifest, Some(&mapping))
            .expect_err("duplicate targets should fail");

        assert!(matches!(err, RestorePlanError::DuplicateMappingTarget(_)));
    }

    // Write one artifact and record its path and checksum in the test manifest.
    fn set_member_artifact(
        manifest: &mut FleetBackupManifest,
        canister_id: &str,
        root: &Path,
        artifact_path: &str,
        bytes: &[u8],
    ) {
        let full_path = root.join(artifact_path);
        fs::create_dir_all(full_path.parent().expect("artifact parent")).expect("create parent");
        fs::write(&full_path, bytes).expect("write artifact");
        let checksum = ArtifactChecksum::from_bytes(bytes);
        let member = manifest
            .fleet
            .members
            .iter_mut()
            .find(|member| member.canister_id == canister_id)
            .expect("member should exist");
        member.source_snapshot.artifact_path = artifact_path.to_string();
        member.source_snapshot.checksum = Some(checksum.hash);
    }

    // Return a unique temporary directory for restore tests.
    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        env::temp_dir().join(format!("{name}-{nanos}"))
    }
}
