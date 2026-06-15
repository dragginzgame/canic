use crate::deployment_truth::{
    ArtifactTransportV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION, ObservationStatusV1,
    PromotionReadinessStatusV1, PromotionWasmStoreIdentityReportV1,
    RolePromotionWasmStoreIdentityV1, SafetyFindingV1, SafetySeverityV1, StagingReceiptV1,
};
use std::collections::BTreeSet;

use super::super::digest::promotion_wasm_store_identity_report_digest;
use super::super::ensure::{
    ensure_wasm_store_identity_report_field, ensure_wasm_store_identity_report_sha256,
};
use super::super::error::PromotionWasmStoreIdentityReportError;
use super::super::request::PromotionWasmStoreIdentityReportRequest;

pub fn promotion_wasm_store_identity_report_from_staging(
    request: PromotionWasmStoreIdentityReportRequest,
) -> Result<PromotionWasmStoreIdentityReportV1, PromotionWasmStoreIdentityReportError> {
    ensure_wasm_store_identity_report_field("report_id", &request.report_id)?;
    ensure_wasm_store_identity_staging_receipts(&request.staging_receipts)?;
    let report =
        promotion_wasm_store_identity_report(&request.report_id, &request.staging_receipts);
    validate_promotion_wasm_store_identity_report(&report)?;
    Ok(report)
}

#[must_use]
pub fn promotion_wasm_store_identity_report(
    report_id: impl Into<String>,
    staging_receipts: &[StagingReceiptV1],
) -> PromotionWasmStoreIdentityReportV1 {
    let roles = staging_receipts
        .iter()
        .map(role_wasm_store_identity_from_staging)
        .collect::<Vec<_>>();
    let blockers = wasm_store_identity_blockers(&roles);
    let mut report = PromotionWasmStoreIdentityReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: report_id.into(),
        wasm_store_identity_report_digest: String::new(),
        status: if blockers.is_empty() {
            PromotionReadinessStatusV1::Ready
        } else {
            PromotionReadinessStatusV1::Blocked
        },
        roles,
        blockers,
    };
    report.wasm_store_identity_report_digest = promotion_wasm_store_identity_report_digest(&report);
    report
}

pub fn validate_promotion_wasm_store_identity_report(
    report: &PromotionWasmStoreIdentityReportV1,
) -> Result<(), PromotionWasmStoreIdentityReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            PromotionWasmStoreIdentityReportError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: report.schema_version,
            },
        );
    }
    ensure_wasm_store_identity_report_field("report_id", &report.report_id)?;
    ensure_wasm_store_identity_report_sha256(
        "wasm_store_identity_report_digest",
        &report.wasm_store_identity_report_digest,
    )?;
    ensure_wasm_store_identity_status_matches_blockers(report)?;
    ensure_unique_wasm_store_identity_roles(&report.roles)?;
    for role in &report.roles {
        validate_role_wasm_store_identity(role)?;
    }
    let expected_blockers = wasm_store_identity_blockers(&report.roles);
    if expected_blockers != report.blockers {
        return Err(PromotionWasmStoreIdentityReportError::BlockerMismatch);
    }
    validate_wasm_store_identity_blockers(&report.blockers)?;
    if report.wasm_store_identity_report_digest
        != promotion_wasm_store_identity_report_digest(report)
    {
        return Err(PromotionWasmStoreIdentityReportError::LinkageMismatch {
            field: "wasm_store_identity_report_digest",
        });
    }
    Ok(())
}

fn role_wasm_store_identity_from_staging(
    receipt: &StagingReceiptV1,
) -> RolePromotionWasmStoreIdentityV1 {
    RolePromotionWasmStoreIdentityV1 {
        role: receipt.role.clone(),
        artifact_identity: receipt.artifact_identity.clone(),
        transport: receipt.transport,
        wasm_store_locator: receipt.wasm_store_locator.clone(),
        prepared_chunk_hashes: receipt.prepared_chunk_hashes.clone(),
        published_chunk_count: receipt.published_chunk_count,
        verified_postcondition: receipt.verified_postcondition.clone(),
    }
}

fn wasm_store_identity_blockers(
    roles: &[RolePromotionWasmStoreIdentityV1],
) -> Vec<SafetyFindingV1> {
    let mut blockers = Vec::new();
    for role in roles {
        if role.transport != ArtifactTransportV1::WasmStore {
            blockers.push(super::super::promotion_finding(
                "promotion_wasm_store_transport_mismatch",
                format!("role {} was not staged through wasm_store", role.role),
                SafetySeverityV1::HardFailure,
                &role.role,
            ));
        }
        if role.wasm_store_locator.as_deref().is_none_or(str::is_empty) {
            blockers.push(super::super::promotion_finding(
                "promotion_wasm_store_locator_missing",
                format!("role {} does not record a wasm_store locator", role.role),
                SafetySeverityV1::HardFailure,
                &role.role,
            ));
        }
        if role.verified_postcondition.status != ObservationStatusV1::Observed {
            blockers.push(super::super::promotion_finding(
                "promotion_wasm_store_postcondition_not_observed",
                format!(
                    "role {} wasm_store postcondition is {:?}",
                    role.role, role.verified_postcondition.status
                ),
                SafetySeverityV1::HardFailure,
                &role.role,
            ));
        }
        if role.published_chunk_count != role.prepared_chunk_hashes.len() {
            blockers.push(super::super::promotion_finding(
                "promotion_wasm_store_chunk_count_mismatch",
                format!(
                    "role {} published {} chunk(s) for {} prepared chunk hash(es)",
                    role.role,
                    role.published_chunk_count,
                    role.prepared_chunk_hashes.len()
                ),
                SafetySeverityV1::HardFailure,
                &role.role,
            ));
        }
    }
    blockers
}

const fn ensure_wasm_store_identity_status_matches_blockers(
    report: &PromotionWasmStoreIdentityReportV1,
) -> Result<(), PromotionWasmStoreIdentityReportError> {
    match (report.status, report.blockers.is_empty()) {
        (PromotionReadinessStatusV1::Ready, false)
        | (PromotionReadinessStatusV1::Blocked, true) => Err(
            PromotionWasmStoreIdentityReportError::StatusBlockerMismatch {
                status: report.status,
                blocker_count: report.blockers.len(),
            },
        ),
        _ => Ok(()),
    }
}

fn ensure_unique_wasm_store_identity_roles(
    roles: &[RolePromotionWasmStoreIdentityV1],
) -> Result<(), PromotionWasmStoreIdentityReportError> {
    let mut seen = BTreeSet::new();
    for role in roles {
        if !seen.insert(role.role.as_str()) {
            return Err(PromotionWasmStoreIdentityReportError::DuplicateRole {
                role: role.role.clone(),
            });
        }
    }
    Ok(())
}

fn ensure_wasm_store_identity_staging_receipts(
    receipts: &[StagingReceiptV1],
) -> Result<(), PromotionWasmStoreIdentityReportError> {
    for receipt in receipts {
        if receipt.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
            return Err(
                PromotionWasmStoreIdentityReportError::StagingReceiptSchemaVersionMismatch {
                    role: receipt.role.clone(),
                    expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                    found: receipt.schema_version,
                },
            );
        }
        ensure_wasm_store_identity_report_field("role", &receipt.role)?;
        ensure_wasm_store_identity_report_field("artifact_identity", &receipt.artifact_identity)?;
    }
    Ok(())
}

fn validate_role_wasm_store_identity(
    role: &RolePromotionWasmStoreIdentityV1,
) -> Result<(), PromotionWasmStoreIdentityReportError> {
    ensure_wasm_store_identity_report_field("role", &role.role)?;
    ensure_wasm_store_identity_report_field("artifact_identity", &role.artifact_identity)?;
    if let Some(locator) = &role.wasm_store_locator {
        ensure_wasm_store_identity_report_field("wasm_store_locator", locator)?;
    }
    for chunk_hash in &role.prepared_chunk_hashes {
        ensure_wasm_store_identity_report_field("prepared_chunk_hash", chunk_hash)?;
    }
    Ok(())
}

fn validate_wasm_store_identity_blockers(
    blockers: &[SafetyFindingV1],
) -> Result<(), PromotionWasmStoreIdentityReportError> {
    for blocker in blockers {
        ensure_wasm_store_identity_report_field("blocker.code", &blocker.code)?;
        ensure_wasm_store_identity_report_field("blocker.message", &blocker.message)?;
        if blocker.severity != SafetySeverityV1::HardFailure {
            return Err(
                PromotionWasmStoreIdentityReportError::BlockerSeverityMismatch {
                    severity: blocker.severity,
                },
            );
        }
    }
    Ok(())
}
