use crate::deployment_truth::{
    ArtifactPromotionProvenanceReportV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION,
    PromotionReadinessStatusV1, RolePromotionProvenanceV1, SafetyFindingV1, SafetySeverityV1,
};
use std::collections::BTreeSet;

use super::super::super::{
    digest::artifact_promotion_provenance_digest,
    ensure::{ensure_provenance_report_field, ensure_provenance_report_sha256},
    error::ArtifactPromotionProvenanceReportError,
};

pub fn validate_artifact_promotion_provenance_report(
    report: &ArtifactPromotionProvenanceReportV1,
) -> Result<(), ArtifactPromotionProvenanceReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            ArtifactPromotionProvenanceReportError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: report.schema_version,
            },
        );
    }
    ensure_provenance_report_field("report_id", &report.report_id)?;
    ensure_provenance_report_field(
        "artifact_promotion_plan_id",
        &report.artifact_promotion_plan_id,
    )?;
    ensure_provenance_report_sha256(
        "artifact_promotion_plan_digest",
        &report.artifact_promotion_plan_digest,
    )?;
    ensure_provenance_report_field("target_plan_id", &report.target_plan_id)?;
    ensure_provenance_report_field("promoted_plan_id", &report.promoted_plan_id)?;
    ensure_provenance_report_field(
        "promotion_plan_lineage_digest",
        &report.promotion_plan_lineage_digest,
    )?;
    ensure_provenance_report_sha256("provenance_report_digest", &report.provenance_report_digest)?;
    ensure_provenance_report_field("readiness_id", &report.readiness_id)?;
    ensure_provenance_report_field(
        "artifact_identity_report_id",
        &report.artifact_identity_report_id,
    )?;
    ensure_provenance_report_field("transform_id", &report.transform_id)?;
    if let Some(lineage_id) = &report.target_execution_lineage_id {
        ensure_provenance_report_field("target_execution_lineage_id", lineage_id)?;
    }
    if let Some(report_id) = &report.wasm_store_identity_report_id {
        ensure_provenance_report_field("wasm_store_identity_report_id", report_id)?;
    }
    if let Some(digest) = &report.wasm_store_identity_report_digest {
        ensure_provenance_report_sha256("wasm_store_identity_report_digest", digest)?;
        if report.wasm_store_identity_report_id.is_none() {
            return Err(ArtifactPromotionProvenanceReportError::LinkageMismatch {
                field: "wasm_store_identity_report_digest",
            });
        }
    }
    if let Some(verification_id) = &report.wasm_store_catalog_verification_id {
        ensure_provenance_report_field("wasm_store_catalog_verification_id", verification_id)?;
        if report.wasm_store_identity_report_id.is_none() {
            return Err(ArtifactPromotionProvenanceReportError::LinkageMismatch {
                field: "wasm_store_catalog_verification_id",
            });
        }
    }
    if let Some(digest) = &report.wasm_store_catalog_verification_digest {
        ensure_provenance_report_sha256("wasm_store_catalog_verification_digest", digest)?;
        if report.wasm_store_catalog_verification_id.is_none() {
            return Err(ArtifactPromotionProvenanceReportError::LinkageMismatch {
                field: "wasm_store_catalog_verification_digest",
            });
        }
    }
    if let Some(report_id) = &report.materialization_identity_report_id {
        ensure_provenance_report_field("materialization_identity_report_id", report_id)?;
    }
    if let Some(digest) = &report.materialization_identity_report_digest {
        ensure_provenance_report_sha256("materialization_identity_report_digest", digest)?;
        if report.materialization_identity_report_id.is_none() {
            return Err(ArtifactPromotionProvenanceReportError::LinkageMismatch {
                field: "materialization_identity_report_digest",
            });
        }
    }
    ensure_provenance_report_status_matches_blockers(report)?;
    ensure_unique_provenance_roles(&report.roles)?;
    for role in &report.roles {
        validate_role_promotion_provenance(role)?;
    }
    validate_provenance_report_blockers(&report.blockers)?;
    if report.provenance_report_digest != artifact_promotion_provenance_digest(report) {
        return Err(ArtifactPromotionProvenanceReportError::LinkageMismatch {
            field: "provenance_report_digest",
        });
    }
    Ok(())
}

const fn ensure_provenance_report_status_matches_blockers(
    report: &ArtifactPromotionProvenanceReportV1,
) -> Result<(), ArtifactPromotionProvenanceReportError> {
    match (report.status, report.blockers.is_empty()) {
        (PromotionReadinessStatusV1::Ready, false)
        | (PromotionReadinessStatusV1::Blocked, true) => Err(
            ArtifactPromotionProvenanceReportError::StatusBlockerMismatch {
                status: report.status,
                blocker_count: report.blockers.len(),
            },
        ),
        _ => Ok(()),
    }
}

fn ensure_unique_provenance_roles(
    roles: &[RolePromotionProvenanceV1],
) -> Result<(), ArtifactPromotionProvenanceReportError> {
    let mut seen = BTreeSet::new();
    for role in roles {
        if !seen.insert(role.role.as_str()) {
            return Err(ArtifactPromotionProvenanceReportError::DuplicateRole {
                role: role.role.clone(),
            });
        }
    }
    Ok(())
}

fn validate_role_promotion_provenance(
    role: &RolePromotionProvenanceV1,
) -> Result<(), ArtifactPromotionProvenanceReportError> {
    ensure_provenance_report_field("role", &role.role)?;
    if let Some(evidence_id) = &role.materialization_evidence_id {
        ensure_provenance_report_field("materialization_evidence_id", evidence_id)?;
    }
    if let Some(digest) = &role.materialization_evidence_digest {
        ensure_provenance_report_sha256("materialization_evidence_digest", digest)?;
    }
    if let Some(locator) = &role.wasm_store_locator {
        ensure_provenance_report_field("wasm_store_locator", locator)?;
    }
    if let Some(digest) = &role.wasm_store_catalog_observation_digest {
        ensure_provenance_report_sha256("wasm_store_catalog_observation_digest", digest)?;
    }
    Ok(())
}

fn validate_provenance_report_blockers(
    blockers: &[SafetyFindingV1],
) -> Result<(), ArtifactPromotionProvenanceReportError> {
    for blocker in blockers {
        ensure_provenance_report_field("blocker.code", &blocker.code)?;
        ensure_provenance_report_field("blocker.message", &blocker.message)?;
        if blocker.severity != SafetySeverityV1::HardFailure {
            return Err(
                ArtifactPromotionProvenanceReportError::BlockerSeverityMismatch {
                    severity: blocker.severity,
                },
            );
        }
    }
    Ok(())
}
