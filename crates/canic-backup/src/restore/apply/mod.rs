use super::{RestorePlan, RestorePlanMember};
use crate::{
    artifacts::{ArtifactChecksum, ArtifactChecksumError},
    manifest::VerificationCheck,
    persistence::resolve_backup_artifact_path,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error as ThisError;

mod journal;

pub(in crate::restore) use journal::RestoreApplyCommandOutputPair;
pub(in crate::restore) use journal::RestoreApplyJournalReport;
pub use journal::{
    RestoreApplyCommandConfig, RestoreApplyCommandOutput, RestoreApplyCommandPreview,
    RestoreApplyJournal, RestoreApplyJournalError, RestoreApplyJournalOperation,
    RestoreApplyOperationKind, RestoreApplyOperationKindCounts, RestoreApplyOperationReceipt,
    RestoreApplyOperationReceiptOutcome, RestoreApplyOperationState, RestoreApplyPendingSummary,
    RestoreApplyProgressSummary, RestoreApplyReportOperation, RestoreApplyReportOutcome,
    RestoreApplyRunnerCommand,
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
    pub planned_snapshot_uploads: usize,
    pub planned_snapshot_loads: usize,
    pub planned_verification_checks: usize,
    pub planned_operations: usize,
    pub rendered_operations: usize,
    #[serde(default)]
    pub operation_counts: RestoreApplyOperationKindCounts,
    pub artifact_validation: Option<RestoreApplyArtifactValidation>,
    pub operations: Vec<RestoreApplyDryRunOperation>,
}

impl RestoreApplyDryRun {
    /// Build a no-mutation apply dry-run from a restore plan.
    #[must_use]
    pub fn from_plan(plan: &RestorePlan) -> Self {
        Self::from_validated_plan(plan)
    }

    /// Build an apply dry-run and verify all referenced artifacts under a backup root.
    pub fn try_from_plan_with_artifacts(
        plan: &RestorePlan,
        backup_root: &Path,
    ) -> Result<Self, RestoreApplyDryRunError> {
        let mut dry_run = Self::from_plan(plan);
        dry_run.artifact_validation = Some(validate_restore_apply_artifacts(plan, backup_root)?);
        Ok(dry_run)
    }

    // Build a no-mutation apply dry-run from a restore plan.
    fn from_validated_plan(plan: &RestorePlan) -> Self {
        let mut next_sequence = 0;
        let mut operations = plan
            .members
            .iter()
            .flat_map(|member| member_operations(member, &mut next_sequence))
            .collect::<Vec<_>>();
        append_fleet_verification_operations(plan, &mut operations, &mut next_sequence);
        let rendered_operations = operations.len();
        let operation_counts =
            RestoreApplyOperationKindCounts::from_dry_run_operations(&operations);

        Self {
            dry_run_version: 1,
            backup_id: plan.backup_id.clone(),
            ready: plan.readiness_summary.ready,
            readiness_reasons: plan.readiness_summary.reasons.clone(),
            member_count: plan.member_count,
            planned_snapshot_uploads: plan.operation_summary.planned_snapshot_uploads,
            planned_snapshot_loads: plan.operation_summary.planned_snapshot_loads,
            planned_verification_checks: plan.operation_summary.planned_verification_checks,
            planned_operations: plan.operation_summary.planned_operations,
            rendered_operations,
            operation_counts,
            artifact_validation: None,
            operations,
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
    let resolved_path = safe_restore_artifact_path(
        backup_root,
        &member.source_canister,
        &member.source_snapshot.artifact_path,
    )?;

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
    backup_root: &Path,
    source_canister: &str,
    artifact_path: &str,
) -> Result<PathBuf, RestoreApplyDryRunError> {
    if let Some(path) = resolve_backup_artifact_path(backup_root, artifact_path) {
        return Ok(path);
    }

    Err(RestoreApplyDryRunError::ArtifactPathEscapesBackup {
        source_canister: source_canister.to_string(),
        artifact_path: artifact_path.to_string(),
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

// Build upload, load, and verification operations for one restore member.
fn member_operations(
    member: &RestorePlanMember,
    next_sequence: &mut usize,
) -> Vec<RestoreApplyDryRunOperation> {
    let mut operations = Vec::new();
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

    operations
}

// Append one member-level dry-run operation using the member's restore order.
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
        member_order: member.member_order,
        source_canister: member.source_canister.clone(),
        target_canister: member.target_canister.clone(),
        role: member.role.clone(),
        snapshot_id: Some(member.source_snapshot.snapshot_id.clone()),
        artifact_path: Some(member.source_snapshot.artifact_path.clone()),
        verification_kind: check.map(|check| check.kind.clone()),
    });
}

// Append fleet-level verification checks after all member operations.
fn append_fleet_verification_operations(
    plan: &RestorePlan,
    operations: &mut Vec<RestoreApplyDryRunOperation>,
    next_sequence: &mut usize,
) {
    if plan.fleet_verification_checks.is_empty() {
        return;
    }

    let root = plan
        .members
        .iter()
        .find(|member| member.source_canister == plan.source_root_canister);
    let source_canister = root.map_or_else(
        || plan.source_root_canister.clone(),
        |member| member.source_canister.clone(),
    );
    let target_canister = root.map_or_else(
        || plan.source_root_canister.clone(),
        |member| member.target_canister.clone(),
    );
    for check in &plan.fleet_verification_checks {
        push_fleet_operation(
            operations,
            next_sequence,
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
    source_canister: &str,
    target_canister: &str,
    check: &VerificationCheck,
) {
    let sequence = *next_sequence;
    *next_sequence += 1;
    let member_order = operations.len();

    operations.push(RestoreApplyDryRunOperation {
        sequence,
        operation: RestoreApplyOperationKind::VerifyFleet,
        member_order,
        source_canister: source_canister.to_string(),
        target_canister: target_canister.to_string(),
        role: "fleet".to_string(),
        snapshot_id: None,
        artifact_path: None,
        verification_kind: Some(check.kind.clone()),
    });
}

///
/// RestoreApplyDryRunOperation
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RestoreApplyDryRunOperation {
    pub sequence: usize,
    pub operation: RestoreApplyOperationKind,
    pub member_order: usize,
    pub source_canister: String,
    pub target_canister: String,
    pub role: String,
    pub snapshot_id: Option<String>,
    pub artifact_path: Option<String>,
    pub verification_kind: Option<String>,
}

///
/// RestoreApplyDryRunError
///

#[derive(Debug, ThisError)]
pub enum RestoreApplyDryRunError {
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
