//! Module: restore::apply::validation
//!
//! Responsibility: validate serialized restore apply dry-runs before durable conversion.
//! Does not own: artifact filesystem verification, journal transitions, or command execution.
//! Boundary: rejects inconsistent dry-run versions and projections before journal creation.

use crate::{
    artifacts::{ArtifactChecksum, ArtifactChecksumError},
    restore::apply::{
        RestoreApplyArtifactCheck, RestoreApplyDryRun, RestoreApplyDryRunOperation,
        RestoreApplyOperationKind, RestoreApplyOperationKindCounts,
    },
};

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error as ThisError;

const SUPPORTED_DRY_RUN_VERSION: u16 = 1;

///
/// RestoreApplyDryRunValidationError
///
/// Typed failure returned before a serialized dry-run can create durable state.
/// Owned by restore apply validation and preserved by journal construction.
///

#[derive(Debug, ThisError)]
pub enum RestoreApplyDryRunValidationError {
    #[error("restore apply dry-run field {field} has invalid artifact checksum")]
    ArtifactChecksum {
        field: &'static str,
        #[source]
        source: ArtifactChecksumError,
    },

    #[error("restore apply dry-run artifact check duplicates source canister {0}")]
    DuplicateArtifactSource(String),

    #[error("restore apply dry-run has duplicate operation sequence {0}")]
    DuplicateSequence(usize),

    #[error("restore apply dry-run field {0} is required")]
    MissingField(&'static str),

    #[error("restore apply dry-run is missing operation sequence {0}")]
    MissingSequence(usize),

    #[error("restore apply dry-run projection {0} does not match its concrete operations")]
    ProjectionMismatch(&'static str),

    #[error("restore apply dry-run ready state does not match readiness reasons")]
    ReadinessMismatch,

    #[error("unsupported restore apply dry-run version {0}")]
    UnsupportedVersion(u16),
}

impl RestoreApplyDryRun {
    /// Validate one serialized dry-run before it creates a durable journal.
    pub fn validate(&self) -> Result<(), RestoreApplyDryRunValidationError> {
        if self.dry_run_version != SUPPORTED_DRY_RUN_VERSION {
            return Err(RestoreApplyDryRunValidationError::UnsupportedVersion(
                self.dry_run_version,
            ));
        }
        validate_nonempty("backup_id", &self.backup_id)?;
        validate_readiness(self)?;
        validate_operation_projections(self)?;
        validate_operation_sequences(&self.operations)?;
        for operation in &self.operations {
            validate_operation_identity(operation)?;
        }
        validate_artifact_projection(self)
    }
}

fn validate_readiness(
    dry_run: &RestoreApplyDryRun,
) -> Result<(), RestoreApplyDryRunValidationError> {
    if dry_run.ready != dry_run.readiness_reasons.is_empty() {
        return Err(RestoreApplyDryRunValidationError::ReadinessMismatch);
    }

    let mut reasons = BTreeSet::new();
    for reason in &dry_run.readiness_reasons {
        validate_nonempty("readiness_reasons[]", reason)?;
        if !reasons.insert(reason) {
            return Err(RestoreApplyDryRunValidationError::ProjectionMismatch(
                "readiness_reasons",
            ));
        }
    }
    Ok(())
}

fn validate_operation_projections(
    dry_run: &RestoreApplyDryRun,
) -> Result<(), RestoreApplyDryRunValidationError> {
    let counts = RestoreApplyOperationKindCounts::from_dry_run_operations(&dry_run.operations);
    require_projection("operation_counts", dry_run.operation_counts == counts)?;
    require_count(
        "member_count.canister_stops",
        dry_run.member_count,
        counts.canister_stops,
    )?;
    require_count(
        "member_count.canister_starts",
        dry_run.member_count,
        counts.canister_starts,
    )?;
    require_count(
        "member_count.snapshot_uploads",
        dry_run.member_count,
        counts.snapshot_uploads,
    )?;
    require_count(
        "member_count.snapshot_loads",
        dry_run.member_count,
        counts.snapshot_loads,
    )?;
    require_count(
        "planned_canister_stops",
        dry_run.planned_canister_stops,
        counts.canister_stops,
    )?;
    require_count(
        "planned_canister_starts",
        dry_run.planned_canister_starts,
        counts.canister_starts,
    )?;
    require_count(
        "planned_snapshot_uploads",
        dry_run.planned_snapshot_uploads,
        counts.snapshot_uploads,
    )?;
    require_count(
        "planned_snapshot_loads",
        dry_run.planned_snapshot_loads,
        counts.snapshot_loads,
    )?;
    require_count(
        "planned_verification_checks",
        dry_run.planned_verification_checks,
        counts.verification_operations,
    )?;
    require_count(
        "planned_operations",
        dry_run.planned_operations,
        dry_run.operations.len(),
    )?;
    require_count(
        "rendered_operations",
        dry_run.rendered_operations,
        dry_run.operations.len(),
    )
}

fn validate_operation_sequences(
    operations: &[RestoreApplyDryRunOperation],
) -> Result<(), RestoreApplyDryRunValidationError> {
    let mut sequences = BTreeSet::new();
    for operation in operations {
        if !sequences.insert(operation.sequence) {
            return Err(RestoreApplyDryRunValidationError::DuplicateSequence(
                operation.sequence,
            ));
        }
    }
    for expected in 0..operations.len() {
        if !sequences.contains(&expected) {
            return Err(RestoreApplyDryRunValidationError::MissingSequence(expected));
        }
    }
    Ok(())
}

fn validate_operation_identity(
    operation: &RestoreApplyDryRunOperation,
) -> Result<(), RestoreApplyDryRunValidationError> {
    validate_nonempty("operations[].source_canister", &operation.source_canister)?;
    validate_nonempty("operations[].target_canister", &operation.target_canister)?;
    validate_nonempty("operations[].role", &operation.role)
}

fn validate_artifact_projection(
    dry_run: &RestoreApplyDryRun,
) -> Result<(), RestoreApplyDryRunValidationError> {
    let Some(validation) = &dry_run.artifact_validation else {
        return Ok(());
    };

    validate_nonempty("artifact_validation.backup_root", &validation.backup_root)?;
    require_count(
        "artifact_validation.checked_members",
        validation.checked_members,
        validation.checks.len(),
    )?;
    require_count(
        "artifact_validation.checked_members",
        validation.checked_members,
        dry_run.member_count,
    )?;

    let uploads = upload_operations(&dry_run.operations)?;
    require_count(
        "artifact_validation.upload_operations",
        uploads.len(),
        dry_run.member_count,
    )?;

    let mut sources = BTreeSet::new();
    for check in &validation.checks {
        validate_artifact_check(check)?;
        if !sources.insert(check.source_canister.as_str()) {
            return Err(RestoreApplyDryRunValidationError::DuplicateArtifactSource(
                check.source_canister.clone(),
            ));
        }
        let upload = uploads.get(check.source_canister.as_str()).ok_or(
            RestoreApplyDryRunValidationError::ProjectionMismatch(
                "artifact_validation.checks[].source_canister",
            ),
        )?;
        require_projection(
            "artifact_validation.checks[]",
            artifact_check_matches_upload(check, upload),
        )?;
    }

    let artifacts_present = validation.checks.iter().all(|check| check.exists);
    require_projection(
        "artifact_validation.artifacts_present",
        validation.artifacts_present == artifacts_present,
    )?;
    let expected_checksums = validation
        .checks
        .iter()
        .filter(|check| check.checksum_expected.is_some())
        .count();
    require_count(
        "artifact_validation.members_with_expected_checksums",
        validation.members_with_expected_checksums,
        expected_checksums,
    )?;
    let checksums_verified = expected_checksums == dry_run.member_count
        && validation
            .checks
            .iter()
            .all(|check| check.checksum_verified);
    require_projection(
        "artifact_validation.checksums_verified",
        validation.checksums_verified == checksums_verified,
    )
}

fn upload_operations(
    operations: &[RestoreApplyDryRunOperation],
) -> Result<BTreeMap<&str, &RestoreApplyDryRunOperation>, RestoreApplyDryRunValidationError> {
    let mut uploads = BTreeMap::new();
    for operation in operations
        .iter()
        .filter(|operation| operation.operation == RestoreApplyOperationKind::UploadSnapshot)
    {
        if uploads
            .insert(operation.source_canister.as_str(), operation)
            .is_some()
        {
            return Err(RestoreApplyDryRunValidationError::ProjectionMismatch(
                "operations[].source_canister",
            ));
        }
    }
    Ok(uploads)
}

fn validate_artifact_check(
    check: &RestoreApplyArtifactCheck,
) -> Result<(), RestoreApplyDryRunValidationError> {
    validate_nonempty(
        "artifact_validation.checks[].source_canister",
        &check.source_canister,
    )?;
    validate_nonempty(
        "artifact_validation.checks[].target_canister",
        &check.target_canister,
    )?;
    validate_nonempty(
        "artifact_validation.checks[].snapshot_id",
        &check.snapshot_id,
    )?;
    validate_nonempty(
        "artifact_validation.checks[].artifact_path",
        &check.artifact_path,
    )?;
    validate_nonempty(
        "artifact_validation.checks[].resolved_path",
        &check.resolved_path,
    )?;
    validate_checksum_algorithm(
        "artifact_validation.checks[].checksum_algorithm",
        &check.checksum_algorithm,
    )?;
    if let Some(expected) = &check.checksum_expected {
        validate_checksum_hash("artifact_validation.checks[].checksum_expected", expected)?;
    }
    if let Some(actual) = &check.checksum_actual {
        validate_checksum_hash("artifact_validation.checks[].checksum_actual", actual)?;
    }
    let checksum_verified = check.exists
        && check.checksum_expected.is_some()
        && check.checksum_expected == check.checksum_actual;
    require_projection(
        "artifact_validation.checks[].checksum_verified",
        check.checksum_verified == checksum_verified,
    )
}

fn artifact_check_matches_upload(
    check: &RestoreApplyArtifactCheck,
    upload: &RestoreApplyDryRunOperation,
) -> bool {
    let checksum_matches = match &upload.artifact_checksum {
        Some(checksum) => {
            checksum.algorithm == check.checksum_algorithm
                && Some(checksum.hash.as_str()) == check.checksum_expected.as_deref()
        }
        None => check.checksum_expected.is_none(),
    };

    check.target_canister == upload.target_canister
        && upload.snapshot_id.as_deref() == Some(check.snapshot_id.as_str())
        && upload.artifact_path.as_deref() == Some(check.artifact_path.as_str())
        && checksum_matches
}

fn validate_nonempty(
    field: &'static str,
    value: &str,
) -> Result<(), RestoreApplyDryRunValidationError> {
    require_projection(field, !value.trim().is_empty())
        .map_err(|_| RestoreApplyDryRunValidationError::MissingField(field))
}

fn validate_checksum_algorithm(
    field: &'static str,
    algorithm: &str,
) -> Result<(), RestoreApplyDryRunValidationError> {
    ArtifactChecksum::validate_algorithm(algorithm)
        .map_err(|source| RestoreApplyDryRunValidationError::ArtifactChecksum { field, source })
}

fn validate_checksum_hash(
    field: &'static str,
    hash: &str,
) -> Result<(), RestoreApplyDryRunValidationError> {
    ArtifactChecksum::validate_hash(hash)
        .map_err(|source| RestoreApplyDryRunValidationError::ArtifactChecksum { field, source })
}

const fn require_count(
    field: &'static str,
    reported: usize,
    actual: usize,
) -> Result<(), RestoreApplyDryRunValidationError> {
    require_projection(field, reported == actual)
}

const fn require_projection(
    field: &'static str,
    matches: bool,
) -> Result<(), RestoreApplyDryRunValidationError> {
    if matches {
        Ok(())
    } else {
        Err(RestoreApplyDryRunValidationError::ProjectionMismatch(field))
    }
}
