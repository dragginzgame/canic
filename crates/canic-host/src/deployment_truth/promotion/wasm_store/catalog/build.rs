use super::super::super::{
    digest::{
        promotion_wasm_store_catalog_verification_digest, wasm_store_catalog_observation_digest,
    },
    request::PromotionWasmStoreCatalogVerificationRequest,
};
use super::blockers::wasm_store_catalog_verification_blockers;
use crate::deployment_truth::{
    DEPLOYMENT_TRUTH_SCHEMA_VERSION, PromotionReadinessStatusV1, PromotionWasmStoreCatalogEntryV1,
    PromotionWasmStoreCatalogVerificationV1, RolePromotionWasmStoreCatalogVerificationV1,
    RolePromotionWasmStoreIdentityV1,
};
use std::collections::BTreeMap;

pub(super) fn build_wasm_store_catalog_verification(
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
