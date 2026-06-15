use crate::deployment_truth::{
    PromotionReadinessStatusV1, PromotionWasmStoreCatalogVerificationV1,
    PromotionWasmStoreIdentityReportV1, RolePromotionWasmStoreCatalogVerificationV1,
    RolePromotionWasmStoreIdentityV1, SafetyFindingV1, stable_json_sha256_hex,
};
use serde::Serialize;

#[derive(Serialize)]
struct PromotionWasmStoreIdentityReportDigestInput<'a> {
    schema_version: u32,
    report_id: &'a str,
    status: PromotionReadinessStatusV1,
    roles: &'a [RolePromotionWasmStoreIdentityV1],
    blockers: &'a [SafetyFindingV1],
}

#[derive(Serialize)]
struct PromotionWasmStoreCatalogVerificationDigestInput<'a> {
    schema_version: u32,
    verification_id: &'a str,
    wasm_store_identity_report_id: &'a str,
    status: PromotionReadinessStatusV1,
    roles: &'a [RolePromotionWasmStoreCatalogVerificationV1],
    blockers: &'a [SafetyFindingV1],
}

#[derive(Serialize)]
struct WasmStoreCatalogObservationDigest<'a> {
    role: &'a str,
    wasm_store_locator: &'a str,
    expected_artifact_identity: &'a str,
    observed_artifact_identity: Option<&'a str>,
    expected_published_chunk_count: usize,
    observed_published_chunk_count: Option<usize>,
    catalog_entry_present: bool,
    catalog_matches: bool,
}

pub(in crate::deployment_truth::promotion) fn promotion_wasm_store_identity_report_digest(
    report: &PromotionWasmStoreIdentityReportV1,
) -> String {
    stable_json_sha256_hex(&PromotionWasmStoreIdentityReportDigestInput {
        schema_version: report.schema_version,
        report_id: &report.report_id,
        status: report.status,
        roles: &report.roles,
        blockers: &report.blockers,
    })
}

pub(in crate::deployment_truth::promotion) fn promotion_wasm_store_catalog_verification_digest(
    verification: &PromotionWasmStoreCatalogVerificationV1,
) -> String {
    stable_json_sha256_hex(&PromotionWasmStoreCatalogVerificationDigestInput {
        schema_version: verification.schema_version,
        verification_id: &verification.verification_id,
        wasm_store_identity_report_id: &verification.wasm_store_identity_report_id,
        status: verification.status,
        roles: &verification.roles,
        blockers: &verification.blockers,
    })
}

pub(in crate::deployment_truth::promotion) fn wasm_store_catalog_observation_digest(
    role: &RolePromotionWasmStoreCatalogVerificationV1,
) -> String {
    stable_json_sha256_hex(&WasmStoreCatalogObservationDigest {
        role: &role.role,
        wasm_store_locator: &role.wasm_store_locator,
        expected_artifact_identity: &role.expected_artifact_identity,
        observed_artifact_identity: role.observed_artifact_identity.as_deref(),
        expected_published_chunk_count: role.expected_published_chunk_count,
        observed_published_chunk_count: role.observed_published_chunk_count,
        catalog_entry_present: role.catalog_entry_present,
        catalog_matches: role.catalog_matches,
    })
}
