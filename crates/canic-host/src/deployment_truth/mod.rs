//! Passive deployment-truth model types for host-side planning and safety checks.

use canic_core::bootstrap::parse_config_model;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fmt::Write as _,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

mod authority;
mod executor;
mod lifecycle;
mod model;
mod observe;
mod plan;
mod promotion;
mod receipt;
mod report;
#[cfg(test)]
mod tests;
mod text;

pub use authority::{
    authority_report_from_check, authority_report_from_check_with_local_id,
    authority_report_from_plan, authority_report_from_plan_with_check_id,
    build_authority_reconciliation_plan,
};
pub use executor::{
    CURRENT_CLI_EXECUTOR_CAPABILITIES, CurrentCliDeploymentExecutor,
    DeploymentExecutionPreflightError, DeploymentExecutor, TESTKIT_PREFLIGHT_CAPABILITIES,
    TestkitPreflightContext, current_cli_execution_context, deployment_execution_preflight,
    deployment_execution_preflight_from_check, has_executor_capabilities,
    missing_executor_capabilities, testkit_execution_context,
    validate_deployment_execution_preflight, validate_deployment_execution_preflight_for_check,
};
pub use lifecycle::{
    ExternalUpgradeReceiptError, external_lifecycle_plan_from_check,
    external_upgrade_proposal_report_from_lifecycle_plan,
    external_upgrade_receipt_from_observation, lifecycle_authority_report_from_check,
    validate_external_upgrade_receipt,
};
pub use model::{
    ArtifactDigestSourceV1, ArtifactPromotionExecutionReceiptV1, ArtifactPromotionPlanV1,
    ArtifactPromotionProvenanceReportV1, ArtifactSourceV1, ArtifactTransportV1,
    AuthorityActionCountV1, AuthorityActionV1, AuthorityApplyBlockerV1, AuthorityApplyReadinessV1,
    AuthorityAttemptedActionV1, AuthorityAutomaticActionV1, AuthorityControlClassCountV1,
    AuthorityControllerDeltaV1, AuthorityControllerObservationV1, AuthorityDryRunEvidenceV1,
    AuthorityExternalActionV1, AuthorityProfileV1, AuthorityReceiptV1,
    AuthorityReconciliationPlanV1, AuthorityReconciliationStateV1, AuthorityReportCountsV1,
    AuthorityReportV1, BuildMaterializationEvidenceV1, BuildMaterializationInputV1,
    BuildMaterializationResultV1, BuildRecipeIdentityV1, CanisterAuthorityActionV1,
    CanisterControlClassV1, ConsentChannelKindV1, ConsentRequirementV1, ConsentSubjectKindV1,
    DeploymentAssumptionV1, DeploymentCheckV1, DeploymentCommandResultV1, DeploymentDiffV1,
    DeploymentExecutionContextV1, DeploymentExecutionPreflightStatusV1,
    DeploymentExecutionPreflightV1, DeploymentExecutionStatusV1, DeploymentExecutorBackendV1,
    DeploymentExecutorCapabilityV1, DeploymentIdentityV1, DeploymentInventoryV1,
    DeploymentObservationGapV1, DeploymentPlanV1, DeploymentReceiptV1, DiffItemV1,
    ExpectedCanisterV1, ExpectedPoolCanisterV1, ExternalLifecyclePlanStatusV1,
    ExternalLifecyclePlanV1, ExternalLifecycleRoleUpgradeV1, ExternalUpgradeAuthorizationModeV1,
    ExternalUpgradeConsentStateV1, ExternalUpgradeProposalReportV1, ExternalUpgradeProposalV1,
    ExternalUpgradeReceiptV1, ExternalUpgradeVerificationResultV1, LifecycleAuthorityReportV1,
    LifecycleAuthorityV1, LifecycleModeV1, LifecycleUpgradeModeV1,
    LifecycleVerificationRequirementV1, LocalDeploymentConfigV1, ObservationStatusV1,
    ObservedArtifactV1, ObservedCanisterV1, ObservedPoolCanisterV1, PhaseReceiptV1,
    PreviousArtifactReceiptKindV1, PromotionArtifactIdentityGroupV1,
    PromotionArtifactIdentityKindV1, PromotionArtifactIdentityReportV1,
    PromotionArtifactIdentitySummaryV1, PromotionArtifactLevelV1,
    PromotionMaterializationIdentityReportV1, PromotionMaterializationOutputGroupV1,
    PromotionPlanTransformEvidenceV1, PromotionPlanTransformV1, PromotionPolicyCheckV1,
    PromotionPolicyClaimV1, PromotionPolicyRequirementV1, PromotionReadinessStatusV1,
    PromotionReadinessV1, PromotionTargetExecutionLineageV1, PromotionWasmStoreCatalogEntryV1,
    PromotionWasmStoreCatalogVerificationV1, PromotionWasmStoreIdentityReportV1, ResumeSafetyV1,
    RoleArtifactManifestV1, RoleArtifactSourceKindV1, RoleArtifactSourceV1, RoleArtifactV1,
    RoleEpochExpectationV1, RoleEpochObservationV1, RolePhaseReceiptV1, RolePhaseResultV1,
    RolePromotionArtifactIdentityV1, RolePromotionExecutionReceiptV1, RolePromotionInputV1,
    RolePromotionMaterializationIdentityV1, RolePromotionMaterializationLinkV1,
    RolePromotionPlanTransformV1, RolePromotionPolicyDecisionV1, RolePromotionPolicyV1,
    RolePromotionProvenanceV1, RolePromotionReadinessV1,
    RolePromotionWasmStoreCatalogVerificationV1, RolePromotionWasmStoreIdentityV1, SafetyFindingV1,
    SafetyReportV1, SafetySeverityV1, SafetyStatusV1, StagingReceiptV1, TrustDomainV1,
    VerifiedPostconditionV1, VerifierReadinessExpectationV1, VerifierReadinessObservationV1,
};
pub use observe::{
    DeploymentTruthError, LocalArtifactManifestRequest, LocalInventoryRequest,
    collect_local_deployment_inventory, collect_local_role_artifact_manifest,
};
pub use plan::{LocalDeploymentPlanRequest, build_local_deployment_plan};
pub use promotion::{
    ArtifactPromotionExecutionReceiptError, ArtifactPromotionExecutionReceiptRequest,
    ArtifactPromotionPlanError, ArtifactPromotionPlanRequest,
    ArtifactPromotionProvenanceReportError, ArtifactPromotionProvenanceReportRequest,
    BuildMaterializationEvidenceRequest, PromotionArtifactIdentityReportError,
    PromotionArtifactIdentityReportRequest, PromotionArtifactSourceError,
    PromotionMaterializationIdentityError, PromotionMaterializationIdentityReportError,
    PromotionMaterializationIdentityReportRequest, PromotionPlanTransformError,
    PromotionPlanTransformEvidenceError, PromotionPlanTransformEvidenceRequest,
    PromotionPlanTransformRequest, PromotionPlanTransformWithMaterializationRequest,
    PromotionPolicyCheckError, PromotionPolicyCheckRequest, PromotionReadinessError,
    PromotionReadinessRequest, PromotionReadinessWithPolicyRequest,
    PromotionTargetExecutionLineageError, PromotionTargetExecutionLineageRequest,
    PromotionWasmStoreCatalogVerificationError, PromotionWasmStoreCatalogVerificationRequest,
    PromotionWasmStoreIdentityReportError, PromotionWasmStoreIdentityReportRequest,
    artifact_promotion_execution_receipt, artifact_promotion_plan,
    artifact_promotion_provenance_report, build_materialization_evidence,
    build_materialization_input_digest, check_promotion_policy, check_promotion_readiness,
    check_promotion_readiness_with_policy, promoted_deployment_plan_from_inputs,
    promoted_deployment_plan_transform_from_inputs,
    promoted_deployment_plan_transform_from_inputs_with_materialization,
    promotion_artifact_identity_report, promotion_artifact_identity_report_from_inputs,
    promotion_materialization_identity_report,
    promotion_materialization_identity_report_from_evidence, promotion_plan_lineage_digest,
    promotion_plan_transform_evidence, promotion_policy_check_from_inputs,
    promotion_readiness_from_inputs, promotion_readiness_from_inputs_with_policy,
    promotion_target_execution_lineage, promotion_target_execution_lineage_digest,
    promotion_wasm_store_catalog_verification, promotion_wasm_store_identity_report,
    promotion_wasm_store_identity_report_from_staging,
    validate_artifact_promotion_execution_receipt, validate_artifact_promotion_plan,
    validate_artifact_promotion_plan_for_check, validate_artifact_promotion_provenance_report,
    validate_build_materialization_evidence, validate_build_materialization_input,
    validate_build_materialization_result, validate_build_recipe_identity,
    validate_promotion_artifact_identity_report,
    validate_promotion_materialization_identity_report, validate_promotion_plan_transform,
    validate_promotion_plan_transform_evidence, validate_promotion_policy_check,
    validate_promotion_readiness, validate_promotion_target_execution_lineage,
    validate_promotion_wasm_store_catalog_verification,
    validate_promotion_wasm_store_identity_report, validate_role_artifact_source,
    validate_role_promotion_policy,
};
pub use receipt::{
    AuthorityEvidenceError, artifact_gate_phase_receipt, artifact_gate_role_phase_receipts,
    authority_dry_run_evidence_from_check, authority_dry_run_evidence_from_check_with_local_ids,
    authority_dry_run_receipt_from_check, authority_dry_run_receipt_from_check_with_local_id,
    authority_dry_run_receipt_from_plan, deployment_execution_status_for_receipt_parts,
    deployment_receipt_from_check, deployment_receipt_from_check_with_status, phase_receipt,
    staging_receipt_evidence, validate_authority_dry_run_evidence,
};
pub use report::{
    LocalDeploymentCheckRequest, check_local_deployment, compare_plan_inventory_and_receipt,
    compare_plan_to_inventory, safety_report_from_diff,
};
pub use text::{
    artifact_promotion_execution_receipt_text, artifact_promotion_plan_text,
    artifact_promotion_provenance_report_text, authority_evidence_text, authority_plan_text,
    authority_receipt_text, authority_report_text, build_materialization_evidence_text,
    deployment_execution_preflight_text, promotion_artifact_identity_report_text,
    promotion_materialization_identity_report_text, promotion_plan_transform_evidence_text,
    promotion_plan_transform_text, promotion_policy_check_text, promotion_readiness_text,
    promotion_target_execution_lineage_text, promotion_wasm_store_catalog_verification_text,
    promotion_wasm_store_identity_report_text,
};

pub const DEPLOYMENT_TRUTH_SCHEMA_VERSION: u32 = 1;
const ROOT_ROLE: &str = "root";
const IMPLICIT_WASM_STORE_ROLE: &str = "wasm_store";

fn deployment_truth_roles_with_implicit_wasm_store(mut roles: Vec<String>) -> Vec<String> {
    if !roles.iter().any(|role| role == IMPLICIT_WASM_STORE_ROLE) {
        roles.push(IMPLICIT_WASM_STORE_ROLE.to_string());
    }
    roles.sort_by(|left, right| {
        deployment_truth_role_rank(left)
            .cmp(&deployment_truth_role_rank(right))
            .then_with(|| left.cmp(right))
    });
    roles.dedup();
    roles
}

fn deployment_truth_role_rank(role: &str) -> u8 {
    match role {
        ROOT_ROLE => 0,
        IMPLICIT_WASM_STORE_ROLE => 1,
        _ => 2,
    }
}

fn deployment_truth_artifact_source(role: &str) -> ArtifactSourceV1 {
    match role {
        IMPLICIT_WASM_STORE_ROLE => ArtifactSourceV1::WasmStore,
        _ => ArtifactSourceV1::LocalBuild,
    }
}

fn deployment_config_path(workspace_root: &Path, config_path: Option<&Path>) -> PathBuf {
    config_path.map_or_else(
        || crate::release_set::config_path(workspace_root),
        |path| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                workspace_root.join(path)
            }
        },
    )
}

fn file_sha256_hex(path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut hex, "{byte:02x}").expect("writing to a String cannot fail");
    }
    Ok(hex)
}

fn canonical_runtime_config_sha256_hex(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    let config = parse_config_model(&source).map_err(|err| err.to_string())?;
    Ok(bytes_sha256_hex(&serde_json::to_vec(&config)?))
}

fn bytes_sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut hex, "{byte:02x}").expect("writing to a String cannot fail");
    }
    hex
}

fn stable_json_sha256_hex<T: Serialize>(value: &T) -> String {
    bytes_sha256_hex(
        &serde_json::to_vec(value)
            .expect("deployment truth identity inputs must JSON-encode deterministically"),
    )
}

fn local_authority_artifact_id(check: &DeploymentCheckV1, suffix: &str) -> String {
    format!(
        "local:{}:{}:{suffix}",
        check.plan.runtime_variant, check.plan.deployment_identity.deployment_name
    )
}
