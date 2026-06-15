use super::super::{SafetyFindingV1, VerifiedPostconditionV1};
use super::source::{
    ArtifactTransportV1, PromotionArtifactLevelV1, PromotionReadinessStatusV1,
    RoleArtifactSourceKindV1,
};
use serde::{Deserialize, Serialize};

///
/// PromotionArtifactIdentityReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionArtifactIdentityReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub artifact_identity_report_digest: String,
    pub status: PromotionReadinessStatusV1,
    pub summary: PromotionArtifactIdentitySummaryV1,
    pub roles: Vec<RolePromotionArtifactIdentityV1>,
    pub identity_groups: Vec<PromotionArtifactIdentityGroupV1>,
    pub blockers: Vec<SafetyFindingV1>,
}

///
/// PromotionArtifactIdentitySummaryV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionArtifactIdentitySummaryV1 {
    pub role_count: usize,
    pub identity_group_count: usize,
    pub shared_identity_group_count: usize,
    pub digest_pinned_role_count: usize,
    pub source_build_role_count: usize,
    pub deferred_identity_role_count: usize,
}

///
/// PromotionWasmStoreIdentityReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionWasmStoreIdentityReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub wasm_store_identity_report_digest: String,
    pub status: PromotionReadinessStatusV1,
    pub roles: Vec<RolePromotionWasmStoreIdentityV1>,
    pub blockers: Vec<SafetyFindingV1>,
}

///
/// RolePromotionWasmStoreIdentityV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePromotionWasmStoreIdentityV1 {
    pub role: String,
    pub artifact_identity: String,
    pub transport: ArtifactTransportV1,
    pub wasm_store_locator: Option<String>,
    pub prepared_chunk_hashes: Vec<String>,
    pub published_chunk_count: usize,
    pub verified_postcondition: VerifiedPostconditionV1,
}

///
/// PromotionWasmStoreCatalogEntryV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionWasmStoreCatalogEntryV1 {
    pub locator: String,
    pub artifact_identity: String,
    pub published_chunk_count: usize,
}

///
/// PromotionWasmStoreCatalogVerificationV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionWasmStoreCatalogVerificationV1 {
    pub schema_version: u32,
    pub verification_id: String,
    pub wasm_store_catalog_verification_digest: String,
    pub wasm_store_identity_report_id: String,
    pub status: PromotionReadinessStatusV1,
    pub roles: Vec<RolePromotionWasmStoreCatalogVerificationV1>,
    pub blockers: Vec<SafetyFindingV1>,
}

///
/// RolePromotionWasmStoreCatalogVerificationV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePromotionWasmStoreCatalogVerificationV1 {
    pub role: String,
    pub wasm_store_locator: String,
    pub expected_artifact_identity: String,
    pub observed_artifact_identity: Option<String>,
    pub expected_published_chunk_count: usize,
    pub observed_published_chunk_count: Option<usize>,
    pub catalog_entry_present: bool,
    pub catalog_matches: bool,
    pub catalog_observation_digest: String,
}

///
/// PromotionArtifactIdentityGroupV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionArtifactIdentityGroupV1 {
    pub identity_key: String,
    pub identity_kind: PromotionArtifactIdentityKindV1,
    pub roles: Vec<String>,
    pub source_kinds: Vec<RoleArtifactSourceKindV1>,
    pub source_locators: Vec<String>,
    pub digest_pinned: bool,
    pub wasm_sha256: Option<String>,
    pub wasm_gz_sha256: Option<String>,
    pub candid_sha256: Option<String>,
    pub canonical_embedded_config_sha256: Option<String>,
}

///
/// RolePromotionArtifactIdentityV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePromotionArtifactIdentityV1 {
    pub role: String,
    pub promotion_level: PromotionArtifactLevelV1,
    pub source_kind: RoleArtifactSourceKindV1,
    pub source_locator: Option<String>,
    pub identity_kind: PromotionArtifactIdentityKindV1,
    pub digest_pinned: bool,
    pub wasm_sha256: Option<String>,
    pub wasm_gz_sha256: Option<String>,
    pub candid_sha256: Option<String>,
    pub canonical_embedded_config_sha256: Option<String>,
}

///
/// PromotionArtifactIdentityKindV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PromotionArtifactIdentityKindV1 {
    SealedWasm,
    SealedCompressedWasm,
    SealedWasmAndCompressedWasm,
    SourceBuild,
    Deferred,
}
