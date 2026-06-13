use super::super::{
    ArtifactPromotionExecutionReceiptV1, ArtifactPromotionPlanV1,
    ArtifactPromotionProvenanceReportV1, BuildMaterializationEvidenceV1,
    BuildMaterializationInputV1, BuildMaterializationResultV1, BuildRecipeIdentityV1,
    DeploymentCommandResultV1, DeploymentExecutionPreflightStatusV1,
    DeploymentExecutionPreflightV1, DeploymentExecutionStatusV1, DeploymentExecutorBackendV1,
    DeploymentExecutorCapabilityV1, DeploymentPlanV1, DeploymentReceiptV1,
    PromotionArtifactIdentityGroupV1, PromotionArtifactIdentityReportV1,
    PromotionArtifactIdentitySummaryV1, PromotionMaterializationIdentityReportV1,
    PromotionMaterializationOutputGroupV1, PromotionPlanTransformEvidenceV1,
    PromotionPlanTransformV1, PromotionPolicyCheckV1, PromotionReadinessStatusV1,
    PromotionReadinessV1, PromotionTargetExecutionLineageV1,
    PromotionWasmStoreCatalogVerificationV1, PromotionWasmStoreIdentityReportV1,
    RolePromotionArtifactIdentityV1, RolePromotionExecutionReceiptV1,
    RolePromotionMaterializationIdentityV1, RolePromotionPlanTransformV1,
    RolePromotionPolicyDecisionV1, RolePromotionProvenanceV1, RolePromotionReadinessV1,
    RolePromotionWasmStoreCatalogVerificationV1, RolePromotionWasmStoreIdentityV1, SafetyFindingV1,
    stable_json_sha256_hex,
};
use serde::Serialize;

#[derive(Serialize)]
struct PromotionPlanLineageInput<'a> {
    target_plan_id: &'a str,
    promoted_plan_id: &'a str,
    promoted_plan: &'a DeploymentPlanV1,
    roles: &'a [RolePromotionPlanTransformV1],
}

#[derive(Serialize)]
struct PromotionTargetExecutionLineageInput<'a> {
    promotion_plan_lineage_digest: &'a str,
    promoted_plan_id: &'a str,
    preflight_plan_id: &'a str,
    preflight_safety_report_id: &'a str,
    preflight_authority_plan_id: &'a str,
    preflight_backend: &'a DeploymentExecutorBackendV1,
    preflight_status: DeploymentExecutionPreflightStatusV1,
    planned_phases: &'a [String],
    required_capabilities: &'a [DeploymentExecutorCapabilityV1],
    missing_capabilities: &'a [DeploymentExecutorCapabilityV1],
    execution_attempted: bool,
}

#[derive(Serialize)]
struct PromotionArtifactIdentityReportDigestInput<'a> {
    schema_version: u32,
    report_id: &'a str,
    status: PromotionReadinessStatusV1,
    summary: &'a PromotionArtifactIdentitySummaryV1,
    roles: &'a [RolePromotionArtifactIdentityV1],
    identity_groups: &'a [PromotionArtifactIdentityGroupV1],
    blockers: &'a [SafetyFindingV1],
}

#[derive(Serialize)]
struct PromotionMaterializationIdentityReportDigestInput<'a> {
    schema_version: u32,
    report_id: &'a str,
    status: PromotionReadinessStatusV1,
    roles: &'a [RolePromotionMaterializationIdentityV1],
    output_groups: &'a [PromotionMaterializationOutputGroupV1],
    blockers: &'a [SafetyFindingV1],
}

#[derive(Serialize)]
struct BuildMaterializationEvidenceDigestInput<'a> {
    schema_version: u32,
    evidence_id: &'a str,
    recipe: &'a BuildRecipeIdentityV1,
    materialization_input: &'a BuildMaterializationInputV1,
    materialization_result: &'a BuildMaterializationResultV1,
    computed_materialization_input_digest: &'a str,
    recipe_id_matches_input: bool,
    recipe_id_matches_result: bool,
    materialization_input_digest_matches_result: bool,
}

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
struct PromotionPolicyCheckDigestInput<'a> {
    schema_version: u32,
    check_id: &'a str,
    status: PromotionReadinessStatusV1,
    roles: &'a [RolePromotionPolicyDecisionV1],
    blockers: &'a [SafetyFindingV1],
}

#[derive(Serialize)]
struct PromotionReadinessDigestInput<'a> {
    schema_version: u32,
    readiness_id: &'a str,
    target_plan_id: &'a str,
    status: PromotionReadinessStatusV1,
    roles: &'a [RolePromotionReadinessV1],
    blockers: &'a [SafetyFindingV1],
    warnings: &'a [SafetyFindingV1],
}

#[derive(Serialize)]
struct PromotionPlanTransformEvidenceDigestInput<'a> {
    schema_version: u32,
    evidence_id: &'a str,
    generated_at: &'a str,
    transform: &'a PromotionPlanTransformV1,
}

#[derive(Serialize)]
struct ArtifactPromotionPlanDigestInput<'a> {
    schema_version: u32,
    plan_id: &'a str,
    generated_at: &'a str,
    status: PromotionReadinessStatusV1,
    target_plan_id: &'a str,
    promoted_plan_id: &'a str,
    promotion_plan_lineage_digest: &'a str,
    readiness: &'a PromotionReadinessV1,
    artifact_identity_report: &'a PromotionArtifactIdentityReportV1,
    transform: &'a PromotionPlanTransformV1,
    target_execution_lineage: Option<&'a PromotionTargetExecutionLineageV1>,
    blockers: &'a [SafetyFindingV1],
}

#[derive(Serialize)]
struct ArtifactPromotionProvenanceDigestInput<'a> {
    schema_version: u32,
    report_id: &'a str,
    status: PromotionReadinessStatusV1,
    artifact_promotion_plan_id: &'a str,
    artifact_promotion_plan_digest: &'a str,
    target_plan_id: &'a str,
    promoted_plan_id: &'a str,
    promotion_plan_lineage_digest: &'a str,
    readiness_id: &'a str,
    artifact_identity_report_id: &'a str,
    transform_id: &'a str,
    target_execution_lineage_id: Option<&'a str>,
    wasm_store_identity_report_id: Option<&'a str>,
    wasm_store_identity_report_digest: Option<&'a str>,
    wasm_store_catalog_verification_id: Option<&'a str>,
    wasm_store_catalog_verification_digest: Option<&'a str>,
    materialization_identity_report_id: Option<&'a str>,
    materialization_identity_report_digest: Option<&'a str>,
    execution_attempted: bool,
    roles: &'a [RolePromotionProvenanceV1],
    blockers: &'a [SafetyFindingV1],
}

#[derive(Serialize)]
struct ArtifactPromotionExecutionReceiptDigestInput<'a> {
    schema_version: u32,
    receipt_id: &'a str,
    artifact_promotion_plan_id: &'a str,
    artifact_promotion_plan_digest: &'a str,
    provenance_report_id: &'a str,
    provenance_report_digest: &'a str,
    provenance_status: PromotionReadinessStatusV1,
    promoted_plan_id: &'a str,
    promotion_plan_lineage_digest: &'a str,
    operation_id: &'a str,
    operation_status: DeploymentExecutionStatusV1,
    command_result: &'a DeploymentCommandResultV1,
    started_at: &'a str,
    finished_at: Option<&'a str>,
    deployment_receipt: &'a DeploymentReceiptV1,
    roles: &'a [RolePromotionExecutionReceiptV1],
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

#[must_use]
pub fn build_materialization_input_digest(input: &BuildMaterializationInputV1) -> String {
    stable_json_sha256_hex(input)
}

pub(super) fn build_materialization_evidence_digest(
    evidence: &BuildMaterializationEvidenceV1,
) -> String {
    stable_json_sha256_hex(&BuildMaterializationEvidenceDigestInput {
        schema_version: evidence.schema_version,
        evidence_id: &evidence.evidence_id,
        recipe: &evidence.recipe,
        materialization_input: &evidence.materialization_input,
        materialization_result: &evidence.materialization_result,
        computed_materialization_input_digest: &evidence.computed_materialization_input_digest,
        recipe_id_matches_input: evidence.recipe_id_matches_input,
        recipe_id_matches_result: evidence.recipe_id_matches_result,
        materialization_input_digest_matches_result: evidence
            .materialization_input_digest_matches_result,
    })
}

pub(super) fn promotion_artifact_identity_report_digest(
    report: &PromotionArtifactIdentityReportV1,
) -> String {
    stable_json_sha256_hex(&PromotionArtifactIdentityReportDigestInput {
        schema_version: report.schema_version,
        report_id: &report.report_id,
        status: report.status,
        summary: &report.summary,
        roles: &report.roles,
        identity_groups: &report.identity_groups,
        blockers: &report.blockers,
    })
}

pub(super) fn promotion_materialization_identity_report_digest(
    report: &PromotionMaterializationIdentityReportV1,
) -> String {
    stable_json_sha256_hex(&PromotionMaterializationIdentityReportDigestInput {
        schema_version: report.schema_version,
        report_id: &report.report_id,
        status: report.status,
        roles: &report.roles,
        output_groups: &report.output_groups,
        blockers: &report.blockers,
    })
}

pub(super) fn promotion_wasm_store_identity_report_digest(
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

pub(super) fn promotion_wasm_store_catalog_verification_digest(
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

pub(super) fn promotion_policy_check_digest(check: &PromotionPolicyCheckV1) -> String {
    stable_json_sha256_hex(&PromotionPolicyCheckDigestInput {
        schema_version: check.schema_version,
        check_id: &check.check_id,
        status: check.status,
        roles: &check.roles,
        blockers: &check.blockers,
    })
}

pub(super) fn promotion_readiness_digest(readiness: &PromotionReadinessV1) -> String {
    stable_json_sha256_hex(&PromotionReadinessDigestInput {
        schema_version: readiness.schema_version,
        readiness_id: &readiness.readiness_id,
        target_plan_id: &readiness.target_plan_id,
        status: readiness.status,
        roles: &readiness.roles,
        blockers: &readiness.blockers,
        warnings: &readiness.warnings,
    })
}

pub(super) fn promotion_plan_transform_evidence_digest(
    evidence: &PromotionPlanTransformEvidenceV1,
) -> String {
    stable_json_sha256_hex(&PromotionPlanTransformEvidenceDigestInput {
        schema_version: evidence.schema_version,
        evidence_id: &evidence.evidence_id,
        generated_at: &evidence.generated_at,
        transform: &evidence.transform,
    })
}

#[must_use]
pub fn promotion_plan_lineage_digest(
    target_plan_id: &str,
    promoted_plan_id: &str,
    promoted_plan: &DeploymentPlanV1,
    roles: &[RolePromotionPlanTransformV1],
) -> String {
    stable_json_sha256_hex(&PromotionPlanLineageInput {
        target_plan_id,
        promoted_plan_id,
        promoted_plan,
        roles,
    })
}

#[must_use]
pub fn promotion_target_execution_lineage_digest(
    transform: &PromotionPlanTransformV1,
    preflight: &DeploymentExecutionPreflightV1,
    execution_attempted: bool,
) -> String {
    stable_json_sha256_hex(&PromotionTargetExecutionLineageInput {
        promotion_plan_lineage_digest: &transform.promotion_plan_lineage_digest,
        promoted_plan_id: &transform.promoted_plan_id,
        preflight_plan_id: &preflight.plan_id,
        preflight_safety_report_id: &preflight.safety_report_id,
        preflight_authority_plan_id: &preflight.authority_plan_id,
        preflight_backend: &preflight.backend,
        preflight_status: preflight.status,
        planned_phases: &preflight.planned_phases,
        required_capabilities: &preflight.required_capabilities,
        missing_capabilities: &preflight.missing_capabilities,
        execution_attempted,
    })
}

pub(super) fn artifact_promotion_provenance_digest(
    report: &ArtifactPromotionProvenanceReportV1,
) -> String {
    stable_json_sha256_hex(&ArtifactPromotionProvenanceDigestInput {
        schema_version: report.schema_version,
        report_id: &report.report_id,
        status: report.status,
        artifact_promotion_plan_id: &report.artifact_promotion_plan_id,
        artifact_promotion_plan_digest: &report.artifact_promotion_plan_digest,
        target_plan_id: &report.target_plan_id,
        promoted_plan_id: &report.promoted_plan_id,
        promotion_plan_lineage_digest: &report.promotion_plan_lineage_digest,
        readiness_id: &report.readiness_id,
        artifact_identity_report_id: &report.artifact_identity_report_id,
        transform_id: &report.transform_id,
        target_execution_lineage_id: report.target_execution_lineage_id.as_deref(),
        wasm_store_identity_report_id: report.wasm_store_identity_report_id.as_deref(),
        wasm_store_identity_report_digest: report.wasm_store_identity_report_digest.as_deref(),
        wasm_store_catalog_verification_id: report.wasm_store_catalog_verification_id.as_deref(),
        wasm_store_catalog_verification_digest: report
            .wasm_store_catalog_verification_digest
            .as_deref(),
        materialization_identity_report_id: report.materialization_identity_report_id.as_deref(),
        materialization_identity_report_digest: report
            .materialization_identity_report_digest
            .as_deref(),
        execution_attempted: report.execution_attempted,
        roles: &report.roles,
        blockers: &report.blockers,
    })
}

pub(super) fn artifact_promotion_plan_digest(plan: &ArtifactPromotionPlanV1) -> String {
    stable_json_sha256_hex(&ArtifactPromotionPlanDigestInput {
        schema_version: plan.schema_version,
        plan_id: &plan.plan_id,
        generated_at: &plan.generated_at,
        status: plan.status,
        target_plan_id: &plan.target_plan_id,
        promoted_plan_id: &plan.promoted_plan_id,
        promotion_plan_lineage_digest: &plan.promotion_plan_lineage_digest,
        readiness: &plan.readiness,
        artifact_identity_report: &plan.artifact_identity_report,
        transform: &plan.transform,
        target_execution_lineage: plan.target_execution_lineage.as_ref(),
        blockers: &plan.blockers,
    })
}

pub(super) fn artifact_promotion_execution_receipt_digest(
    receipt: &ArtifactPromotionExecutionReceiptV1,
) -> String {
    stable_json_sha256_hex(&ArtifactPromotionExecutionReceiptDigestInput {
        schema_version: receipt.schema_version,
        receipt_id: &receipt.receipt_id,
        artifact_promotion_plan_id: &receipt.artifact_promotion_plan_id,
        artifact_promotion_plan_digest: &receipt.artifact_promotion_plan_digest,
        provenance_report_id: &receipt.provenance_report_id,
        provenance_report_digest: &receipt.provenance_report_digest,
        provenance_status: receipt.provenance_status,
        promoted_plan_id: &receipt.promoted_plan_id,
        promotion_plan_lineage_digest: &receipt.promotion_plan_lineage_digest,
        operation_id: &receipt.operation_id,
        operation_status: receipt.operation_status,
        command_result: &receipt.command_result,
        started_at: &receipt.started_at,
        finished_at: receipt.finished_at.as_deref(),
        deployment_receipt: &receipt.deployment_receipt,
        roles: &receipt.roles,
    })
}

pub(super) fn wasm_store_catalog_observation_digest(
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
