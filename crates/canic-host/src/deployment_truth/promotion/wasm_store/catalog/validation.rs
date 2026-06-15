use super::super::super::{
    digest::{
        promotion_wasm_store_catalog_verification_digest, wasm_store_catalog_observation_digest,
    },
    ensure::{
        ensure_wasm_store_catalog_verification_field, ensure_wasm_store_catalog_verification_sha256,
    },
    error::PromotionWasmStoreCatalogVerificationError,
};
use super::blockers::wasm_store_catalog_verification_blockers;
use crate::deployment_truth::{
    DEPLOYMENT_TRUTH_SCHEMA_VERSION, PromotionReadinessStatusV1, PromotionWasmStoreCatalogEntryV1,
    PromotionWasmStoreCatalogVerificationV1, RolePromotionWasmStoreCatalogVerificationV1,
    SafetyFindingV1, SafetySeverityV1,
};
use std::collections::BTreeSet;

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

pub(super) fn ensure_unique_wasm_store_catalog_entries(
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
