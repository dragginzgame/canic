use super::{RestorePhase, RestorePlan, RestorePlanMember, RestoreStatus};
use crate::{
    artifacts::{ArtifactChecksum, ArtifactChecksumError},
    manifest::VerificationCheck,
};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use thiserror::Error as ThisError;

mod journal;

pub use journal::{
    RestoreApplyCommandConfig, RestoreApplyCommandOutput, RestoreApplyCommandOutputPair,
    RestoreApplyCommandPreview, RestoreApplyJournal, RestoreApplyJournalError,
    RestoreApplyJournalOperation, RestoreApplyJournalReport, RestoreApplyJournalStatus,
    RestoreApplyNextOperation, RestoreApplyOperationKind, RestoreApplyOperationKindCounts,
    RestoreApplyOperationReceipt, RestoreApplyOperationReceiptOutcome, RestoreApplyOperationState,
    RestoreApplyPendingSummary, RestoreApplyProgressSummary, RestoreApplyReportOperation,
    RestoreApplyReportOutcome, RestoreApplyRunnerCommand,
};

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
