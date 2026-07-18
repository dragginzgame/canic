use super::super::SafetyFindingV1;
use super::source::{PromotionReadinessStatusV1, RoleArtifactSourceKindV1};
use serde::{Deserialize, Serialize};

///
/// BuildRecipeIdentityV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BuildRecipeIdentityV1 {
    pub recipe_id: String,
    pub source_kind: RoleArtifactSourceKindV1,
    pub source_revision: String,
    pub source_tree_clean: bool,
    pub package_or_role_selector: String,
    pub cargo_profile: String,
    pub cargo_features_digest: String,
    pub cargo_lock_digest: String,
    pub rust_toolchain: String,
    pub builder_version: String,
    pub target_triple: String,
    pub linker_identity: String,
    pub deterministic_build_mode: String,
    pub wasm_opt_version: String,
    pub compression_identity: String,
}

///
/// BuildMaterializationInputV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BuildMaterializationInputV1 {
    pub materialization_input_id: String,
    pub build_recipe_id: String,
    pub canonical_embedded_config_sha256: String,
    pub environment: String,
    pub root_trust_anchor: String,
    pub runtime_variant: String,
}

///
/// BuildMaterializationResultV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BuildMaterializationResultV1 {
    pub materialization_result_id: String,
    pub build_recipe_id: String,
    pub materialization_input_digest: String,
    pub wasm_sha256: String,
    pub wasm_gz_sha256: String,
    pub installed_module_hash: String,
    pub candid_sha256: String,
}

///
/// BuildMaterializationEvidenceV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BuildMaterializationEvidenceV1 {
    pub schema_version: u32,
    pub evidence_id: String,
    pub materialization_evidence_digest: String,
    pub recipe: BuildRecipeIdentityV1,
    pub materialization_input: BuildMaterializationInputV1,
    pub materialization_result: BuildMaterializationResultV1,
    pub computed_materialization_input_digest: String,
    pub recipe_id_matches_input: bool,
    pub recipe_id_matches_result: bool,
    pub materialization_input_digest_matches_result: bool,
}

///
/// PromotionMaterializationIdentityReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionMaterializationIdentityReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub materialization_identity_report_digest: String,
    pub status: PromotionReadinessStatusV1,
    pub roles: Vec<RolePromotionMaterializationIdentityV1>,
    pub output_groups: Vec<PromotionMaterializationOutputGroupV1>,
    pub blockers: Vec<SafetyFindingV1>,
}

///
/// RolePromotionMaterializationIdentityV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePromotionMaterializationIdentityV1 {
    pub role: String,
    pub evidence_id: String,
    pub materialization_evidence_digest: String,
    pub recipe_id: String,
    pub materialization_input_id: String,
    pub materialization_result_id: String,
    pub materialization_input_digest: String,
    pub canonical_embedded_config_sha256: String,
    pub environment: String,
    pub root_trust_anchor: String,
    pub runtime_variant: String,
    pub wasm_sha256: String,
    pub wasm_gz_sha256: String,
    pub installed_module_hash: String,
    pub candid_sha256: String,
}

///
/// PromotionMaterializationOutputGroupV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionMaterializationOutputGroupV1 {
    pub output_identity_key: String,
    pub roles: Vec<String>,
    pub wasm_sha256: String,
    pub wasm_gz_sha256: String,
    pub installed_module_hash: String,
    pub candid_sha256: String,
}

///
/// RolePromotionMaterializationLinkV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePromotionMaterializationLinkV1 {
    pub role: String,
    pub evidence_id: String,
    pub materialization_evidence_digest: String,
    pub recipe_id: String,
    pub materialization_input_id: String,
    pub materialization_result_id: String,
    pub materialization_input_digest: String,
    pub wasm_sha256: String,
    pub wasm_gz_sha256: String,
    pub installed_module_hash: String,
    pub candid_sha256: String,
}
