use super::super::{
    ArtifactTransportV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION, ObservationStatusV1,
    PromotionReadinessStatusV1, PromotionWasmStoreCatalogEntryV1,
    PromotionWasmStoreCatalogVerificationV1, PromotionWasmStoreIdentityReportV1,
    RolePromotionWasmStoreCatalogVerificationV1, RolePromotionWasmStoreIdentityV1, SafetyFindingV1,
    SafetySeverityV1, StagingReceiptV1,
};
use super::digest::{
    promotion_wasm_store_catalog_verification_digest, promotion_wasm_store_identity_report_digest,
    wasm_store_catalog_observation_digest,
};
use super::ensure::{
    ensure_wasm_store_catalog_verification_field, ensure_wasm_store_catalog_verification_sha256,
    ensure_wasm_store_identity_report_field, ensure_wasm_store_identity_report_sha256,
};
use super::error::{
    PromotionWasmStoreCatalogVerificationError, PromotionWasmStoreIdentityReportError,
};
use super::request::{
    PromotionWasmStoreCatalogVerificationRequest, PromotionWasmStoreIdentityReportRequest,
};
use std::collections::{BTreeMap, BTreeSet};

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

pub fn promotion_wasm_store_catalog_verification(
    request: PromotionWasmStoreCatalogVerificationRequest,
) -> Result<PromotionWasmStoreCatalogVerificationV1, PromotionWasmStoreCatalogVerificationError> {
    ensure_wasm_store_catalog_verification_field("verification_id", &request.verification_id)?;
    validate_promotion_wasm_store_identity_report(&request.wasm_store_identity_report)?;
    ensure_unique_wasm_store_catalog_entries(&request.catalog_entries)?;
    let verification = build_wasm_store_catalog_verification(request);
    validate_promotion_wasm_store_catalog_verification(&verification)?;
    Ok(verification)
}

pub fn validate_promotion_wasm_store_catalog_verification(
    verification: &PromotionWasmStoreCatalogVerificationV1,
) -> Result<(), PromotionWasmStoreCatalogVerificationError> {
    if verification.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            PromotionWasmStoreCatalogVerificationError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: verification.schema_version,
            },
        );
    }
    ensure_wasm_store_catalog_verification_field("verification_id", &verification.verification_id)?;
    ensure_wasm_store_catalog_verification_sha256(
        "wasm_store_catalog_verification_digest",
        &verification.wasm_store_catalog_verification_digest,
    )?;
    ensure_wasm_store_catalog_verification_field(
        "wasm_store_identity_report_id",
        &verification.wasm_store_identity_report_id,
    )?;
    ensure_wasm_store_catalog_status_matches_blockers(verification)?;
    ensure_unique_wasm_store_catalog_verification_roles(&verification.roles)?;
    let expected_blockers = wasm_store_catalog_verification_blockers(&verification.roles);
    if expected_blockers != verification.blockers {
        return Err(PromotionWasmStoreCatalogVerificationError::BlockerMismatch);
    }
    validate_wasm_store_catalog_verification_blockers(&verification.blockers)?;
    if verification.wasm_store_catalog_verification_digest
        != promotion_wasm_store_catalog_verification_digest(verification)
    {
        return Err(
            PromotionWasmStoreCatalogVerificationError::LinkageMismatch {
                field: "wasm_store_catalog_verification_digest",
            },
        );
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
            blockers.push(super::promotion_finding(
                "promotion_wasm_store_transport_mismatch",
                format!("role {} was not staged through wasm_store", role.role),
                SafetySeverityV1::HardFailure,
                &role.role,
            ));
        }
        if role.wasm_store_locator.as_deref().is_none_or(str::is_empty) {
            blockers.push(super::promotion_finding(
                "promotion_wasm_store_locator_missing",
                format!("role {} does not record a wasm_store locator", role.role),
                SafetySeverityV1::HardFailure,
                &role.role,
            ));
        }
        if role.verified_postcondition.status != ObservationStatusV1::Observed {
            blockers.push(super::promotion_finding(
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
            blockers.push(super::promotion_finding(
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

fn build_wasm_store_catalog_verification(
    request: PromotionWasmStoreCatalogVerificationRequest,
) -> PromotionWasmStoreCatalogVerificationV1 {
    let catalog = request
        .catalog_entries
        .iter()
        .map(|entry| (entry.locator.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let roles = request
        .wasm_store_identity_report
        .roles
        .iter()
        .map(|role| role_wasm_store_catalog_verification(role, &catalog))
        .collect::<Vec<_>>();
    let blockers = wasm_store_catalog_verification_blockers(&roles);
    let mut verification = PromotionWasmStoreCatalogVerificationV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        verification_id: request.verification_id,
        wasm_store_catalog_verification_digest: String::new(),
        wasm_store_identity_report_id: request.wasm_store_identity_report.report_id,
        status: if blockers.is_empty() {
            PromotionReadinessStatusV1::Ready
        } else {
            PromotionReadinessStatusV1::Blocked
        },
        roles,
        blockers,
    };
    verification.wasm_store_catalog_verification_digest =
        promotion_wasm_store_catalog_verification_digest(&verification);
    verification
}

fn role_wasm_store_catalog_verification(
    role: &RolePromotionWasmStoreIdentityV1,
    catalog: &BTreeMap<&str, &PromotionWasmStoreCatalogEntryV1>,
) -> RolePromotionWasmStoreCatalogVerificationV1 {
    let locator = role.wasm_store_locator.clone().unwrap_or_default();
    let entry = catalog.get(locator.as_str()).copied();
    let mut verification = RolePromotionWasmStoreCatalogVerificationV1 {
        role: role.role.clone(),
        wasm_store_locator: locator,
        expected_artifact_identity: role.artifact_identity.clone(),
        observed_artifact_identity: entry.map(|entry| entry.artifact_identity.clone()),
        expected_published_chunk_count: role.published_chunk_count,
        observed_published_chunk_count: entry.map(|entry| entry.published_chunk_count),
        catalog_entry_present: entry.is_some(),
        catalog_matches: entry.is_some_and(|entry| {
            entry.artifact_identity == role.artifact_identity
                && entry.published_chunk_count == role.published_chunk_count
        }),
        catalog_observation_digest: String::new(),
    };
    verification.catalog_observation_digest = wasm_store_catalog_observation_digest(&verification);
    verification
}

fn wasm_store_catalog_verification_blockers(
    roles: &[RolePromotionWasmStoreCatalogVerificationV1],
) -> Vec<SafetyFindingV1> {
    let mut blockers = Vec::new();
    for role in roles {
        if role.wasm_store_locator.is_empty() {
            blockers.push(super::promotion_finding(
                "promotion_wasm_store_catalog_locator_missing",
                format!("role {} does not record a wasm_store locator", role.role),
                SafetySeverityV1::HardFailure,
                &role.role,
            ));
        } else if !role.catalog_entry_present {
            blockers.push(super::promotion_finding(
                "promotion_wasm_store_catalog_entry_missing",
                format!(
                    "role {} locator {} was not present in the wasm_store catalog observation",
                    role.role, role.wasm_store_locator
                ),
                SafetySeverityV1::HardFailure,
                &role.role,
            ));
        }
        if let Some(observed) = &role.observed_artifact_identity
            && observed != &role.expected_artifact_identity
        {
            blockers.push(super::promotion_finding(
                "promotion_wasm_store_catalog_artifact_mismatch",
                format!(
                    "role {} expected artifact {} at {}, observed {}",
                    role.role, role.expected_artifact_identity, role.wasm_store_locator, observed
                ),
                SafetySeverityV1::HardFailure,
                &role.role,
            ));
        }
        if let Some(observed) = role.observed_published_chunk_count
            && observed != role.expected_published_chunk_count
        {
            blockers.push(super::promotion_finding(
                "promotion_wasm_store_catalog_chunk_count_mismatch",
                format!(
                    "role {} expected {} published chunk(s) at {}, observed {}",
                    role.role,
                    role.expected_published_chunk_count,
                    role.wasm_store_locator,
                    observed
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

const fn ensure_wasm_store_catalog_status_matches_blockers(
    verification: &PromotionWasmStoreCatalogVerificationV1,
) -> Result<(), PromotionWasmStoreCatalogVerificationError> {
    match (verification.status, verification.blockers.is_empty()) {
        (PromotionReadinessStatusV1::Ready, false)
        | (PromotionReadinessStatusV1::Blocked, true) => Err(
            PromotionWasmStoreCatalogVerificationError::StatusBlockerMismatch {
                status: verification.status,
                blocker_count: verification.blockers.len(),
            },
        ),
        _ => Ok(()),
    }
}

fn ensure_unique_wasm_store_catalog_entries(
    entries: &[PromotionWasmStoreCatalogEntryV1],
) -> Result<(), PromotionWasmStoreCatalogVerificationError> {
    let mut seen = BTreeSet::new();
    for entry in entries {
        ensure_wasm_store_catalog_verification_field("catalog.locator", &entry.locator)?;
        ensure_wasm_store_catalog_verification_field(
            "catalog.artifact_identity",
            &entry.artifact_identity,
        )?;
        if !seen.insert(entry.locator.as_str()) {
            return Err(
                PromotionWasmStoreCatalogVerificationError::DuplicateLocator {
                    locator: entry.locator.clone(),
                },
            );
        }
    }
    Ok(())
}

fn ensure_unique_wasm_store_catalog_verification_roles(
    roles: &[RolePromotionWasmStoreCatalogVerificationV1],
) -> Result<(), PromotionWasmStoreCatalogVerificationError> {
    let mut seen = BTreeSet::new();
    for role in roles {
        ensure_wasm_store_catalog_verification_field("role", &role.role)?;
        ensure_wasm_store_catalog_verification_field(
            "expected_artifact_identity",
            &role.expected_artifact_identity,
        )?;
        ensure_wasm_store_catalog_verification_field(
            "catalog_observation_digest",
            &role.catalog_observation_digest,
        )?;
        if !seen.insert(role.role.as_str()) {
            return Err(PromotionWasmStoreCatalogVerificationError::DuplicateRole {
                role: role.role.clone(),
            });
        }
        if role.catalog_entry_present
            != (role.observed_artifact_identity.is_some()
                && role.observed_published_chunk_count.is_some())
        {
            return Err(PromotionWasmStoreCatalogVerificationError::RoleMismatch {
                role: role.role.clone(),
                field: "catalog_entry_present",
            });
        }
        if role.catalog_matches
            != (role.catalog_entry_present
                && role.observed_artifact_identity.as_ref()
                    == Some(&role.expected_artifact_identity)
                && role.observed_published_chunk_count == Some(role.expected_published_chunk_count))
        {
            return Err(PromotionWasmStoreCatalogVerificationError::RoleMismatch {
                role: role.role.clone(),
                field: "catalog_matches",
            });
        }
        if role.catalog_observation_digest != wasm_store_catalog_observation_digest(role) {
            return Err(PromotionWasmStoreCatalogVerificationError::RoleMismatch {
                role: role.role.clone(),
                field: "catalog_observation_digest",
            });
        }
    }
    Ok(())
}

fn validate_wasm_store_catalog_verification_blockers(
    blockers: &[SafetyFindingV1],
) -> Result<(), PromotionWasmStoreCatalogVerificationError> {
    for blocker in blockers {
        ensure_wasm_store_catalog_verification_field("blocker.code", &blocker.code)?;
        ensure_wasm_store_catalog_verification_field("blocker.message", &blocker.message)?;
        if blocker.severity != SafetySeverityV1::HardFailure {
            return Err(
                PromotionWasmStoreCatalogVerificationError::BlockerSeverityMismatch {
                    severity: blocker.severity,
                },
            );
        }
    }
    Ok(())
}
