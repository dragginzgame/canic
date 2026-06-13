mod digest;
mod ensure;
mod error;
mod identity;
mod policy;
mod request;

use super::executor::{
    validate_deployment_execution_preflight, validate_deployment_execution_preflight_for_check,
};
use super::{
    ArtifactPromotionExecutionReceiptV1, ArtifactPromotionPlanV1,
    ArtifactPromotionProvenanceReportV1, ArtifactSourceV1, ArtifactTransportV1,
    BuildMaterializationEvidenceV1, BuildMaterializationInputV1, BuildMaterializationResultV1,
    BuildRecipeIdentityV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION, DeploymentCheckV1, DeploymentPlanV1,
    DeploymentReceiptV1, ObservationStatusV1, PromotionArtifactIdentityGroupV1,
    PromotionArtifactIdentityReportV1, PromotionArtifactLevelV1,
    PromotionMaterializationIdentityReportV1, PromotionMaterializationOutputGroupV1,
    PromotionPlanTransformEvidenceV1, PromotionPlanTransformV1, PromotionReadinessStatusV1,
    PromotionReadinessV1, PromotionTargetExecutionLineageV1, PromotionWasmStoreCatalogEntryV1,
    PromotionWasmStoreCatalogVerificationV1, PromotionWasmStoreIdentityReportV1,
    RoleArtifactSourceKindV1, RoleArtifactSourceV1, RoleArtifactV1,
    RolePromotionArtifactIdentityV1, RolePromotionExecutionReceiptV1, RolePromotionInputV1,
    RolePromotionMaterializationIdentityV1, RolePromotionMaterializationLinkV1,
    RolePromotionPlanTransformV1, RolePromotionPolicyV1, RolePromotionProvenanceV1,
    RolePromotionReadinessV1, RolePromotionWasmStoreCatalogVerificationV1,
    RolePromotionWasmStoreIdentityV1, SafetyFindingV1, SafetySeverityV1, StagingReceiptV1,
};
use digest::{
    artifact_promotion_execution_receipt_digest, artifact_promotion_plan_digest,
    artifact_promotion_provenance_digest, build_materialization_evidence_digest,
    promotion_artifact_identity_report_digest, promotion_materialization_identity_report_digest,
    promotion_plan_transform_evidence_digest, promotion_readiness_digest,
    promotion_wasm_store_catalog_verification_digest, promotion_wasm_store_identity_report_digest,
    wasm_store_catalog_observation_digest,
};
pub use digest::{
    build_materialization_input_digest, promotion_plan_lineage_digest,
    promotion_target_execution_lineage_digest,
};
use ensure::*;
pub use error::*;
use identity::*;
pub use policy::{
    check_promotion_policy, promotion_policy_check_from_inputs, validate_promotion_policy_check,
    validate_role_promotion_policy,
};
pub use request::*;
use std::collections::{BTreeMap, BTreeSet};

pub fn promoted_deployment_plan_from_inputs(
    request: &PromotionPlanTransformRequest,
) -> Result<DeploymentPlanV1, PromotionPlanTransformError> {
    Ok(promoted_deployment_plan_transform_from_inputs(request)?.promoted_plan)
}

pub fn promoted_deployment_plan_transform_from_inputs(
    request: &PromotionPlanTransformRequest,
) -> Result<PromotionPlanTransformV1, PromotionPlanTransformError> {
    ensure_transform_field("promoted_plan_id", &request.promoted_plan_id)?;
    let readiness = promotion_readiness_from_inputs(
        &request.promoted_plan_id,
        &request.target_plan,
        &request.inputs,
    );
    validate_promotion_readiness(&readiness)?;
    if readiness.status == PromotionReadinessStatusV1::Blocked {
        return Err(PromotionPlanTransformError::ReadinessBlocked {
            blocker_count: readiness.blockers.len(),
        });
    }

    let mut promoted_plan = request.target_plan.clone();
    promoted_plan.plan_id.clone_from(&request.promoted_plan_id);
    for input in &request.inputs {
        let Some(role_artifact) = promoted_plan
            .role_artifacts
            .iter_mut()
            .find(|artifact| artifact.role == input.role)
        else {
            return Err(PromotionPlanTransformError::TargetRoleMissing {
                role: input.role.clone(),
            });
        };
        apply_promotion_input_to_role_artifact(role_artifact, input);
    }
    let transform =
        promotion_plan_transform_from_parts(&request.target_plan, promoted_plan, &request.inputs);
    validate_promotion_plan_transform(&transform)?;
    Ok(transform)
}

pub fn promoted_deployment_plan_transform_from_inputs_with_materialization(
    request: &PromotionPlanTransformWithMaterializationRequest,
) -> Result<PromotionPlanTransformV1, PromotionPlanTransformError> {
    let base_request = PromotionPlanTransformRequest {
        promoted_plan_id: request.promoted_plan_id.clone(),
        target_plan: request.target_plan.clone(),
        inputs: request.inputs.clone(),
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&base_request)?;
    attach_source_build_materialization(
        &mut transform,
        &request.inputs,
        &request.materialization_evidence,
    )?;
    refresh_promotion_plan_lineage_digest(&mut transform);
    validate_promotion_plan_transform(&transform)?;
    Ok(transform)
}

pub fn check_promotion_readiness(
    request: &PromotionReadinessRequest,
) -> Result<PromotionReadinessV1, PromotionReadinessError> {
    ensure_readiness_field("readiness_id", &request.readiness_id)?;
    let readiness = promotion_readiness_from_inputs(
        &request.readiness_id,
        &request.target_plan,
        &request.inputs,
    );
    validate_promotion_readiness(&readiness)?;
    Ok(readiness)
}

pub fn check_promotion_readiness_with_policy(
    request: &PromotionReadinessWithPolicyRequest,
) -> Result<PromotionReadinessV1, PromotionReadinessError> {
    ensure_readiness_field("readiness_id", &request.readiness_id)?;
    let readiness = promotion_readiness_from_inputs_with_policy(
        &request.readiness_id,
        &request.target_plan,
        &request.inputs,
        &request.policies,
    );
    validate_promotion_readiness(&readiness)?;
    Ok(readiness)
}

pub fn promotion_artifact_identity_report_from_inputs(
    request: PromotionArtifactIdentityReportRequest,
) -> Result<PromotionArtifactIdentityReportV1, PromotionArtifactIdentityReportError> {
    ensure_identity_report_field("report_id", &request.report_id)?;
    let report = promotion_artifact_identity_report(&request.report_id, &request.inputs);
    validate_promotion_artifact_identity_report(&report)?;
    Ok(report)
}

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
pub fn promotion_artifact_identity_report(
    report_id: impl Into<String>,
    inputs: &[RolePromotionInputV1],
) -> PromotionArtifactIdentityReportV1 {
    let mut roles = Vec::with_capacity(inputs.len());
    let mut blockers = Vec::new();
    for input in inputs {
        if let Err(err) = validate_role_artifact_source(&input.source) {
            blockers.push(promotion_finding(
                "promotion_artifact_source_invalid",
                err.to_string(),
                SafetySeverityV1::HardFailure,
                &input.role,
            ));
        }
        if input.role != input.source.role {
            blockers.push(promotion_finding(
                "promotion_source_role_mismatch",
                format!(
                    "promotion input role {} does not match artifact source role {}",
                    input.role, input.source.role
                ),
                SafetySeverityV1::HardFailure,
                &input.role,
            ));
        }
        roles.push(role_promotion_artifact_identity(input));
    }
    let identity_groups = promotion_artifact_identity_groups(&roles);
    let summary = promotion_artifact_identity_summary(&roles, &identity_groups);

    let mut report = PromotionArtifactIdentityReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: report_id.into(),
        artifact_identity_report_digest: String::new(),
        status: if blockers.is_empty() {
            PromotionReadinessStatusV1::Ready
        } else {
            PromotionReadinessStatusV1::Blocked
        },
        summary,
        identity_groups,
        roles,
        blockers,
    };
    report.artifact_identity_report_digest = promotion_artifact_identity_report_digest(&report);
    report
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

pub fn validate_promotion_artifact_identity_report(
    report: &PromotionArtifactIdentityReportV1,
) -> Result<(), PromotionArtifactIdentityReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            PromotionArtifactIdentityReportError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: report.schema_version,
            },
        );
    }
    ensure_identity_report_field("report_id", &report.report_id)?;
    ensure_identity_report_sha256(
        "artifact_identity_report_digest",
        &report.artifact_identity_report_digest,
    )?;
    ensure_identity_report_status_matches_blockers(report)?;
    ensure_unique_artifact_identity_roles(&report.roles)?;
    for role in &report.roles {
        validate_role_artifact_identity(role)?;
    }
    validate_artifact_identity_groups(&report.roles, &report.identity_groups)?;
    validate_artifact_identity_summary(report)?;
    validate_identity_report_blockers(&report.blockers)?;
    if report.artifact_identity_report_digest != promotion_artifact_identity_report_digest(report) {
        return Err(PromotionArtifactIdentityReportError::LinkageMismatch {
            field: "artifact_identity_report_digest",
        });
    }
    Ok(())
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

pub fn promotion_plan_transform_evidence(
    request: PromotionPlanTransformEvidenceRequest,
) -> Result<PromotionPlanTransformEvidenceV1, PromotionPlanTransformEvidenceError> {
    ensure_evidence_field("evidence_id", &request.evidence_id)?;
    ensure_evidence_field("generated_at", &request.generated_at)?;
    validate_promotion_plan_transform(&request.transform)?;
    let mut evidence = PromotionPlanTransformEvidenceV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        evidence_id: request.evidence_id,
        promotion_plan_transform_evidence_digest: String::new(),
        generated_at: request.generated_at,
        transform: request.transform,
    };
    evidence.promotion_plan_transform_evidence_digest =
        promotion_plan_transform_evidence_digest(&evidence);
    validate_promotion_plan_transform_evidence(&evidence)?;
    Ok(evidence)
}

pub fn artifact_promotion_plan(
    request: ArtifactPromotionPlanRequest,
) -> Result<ArtifactPromotionPlanV1, ArtifactPromotionPlanError> {
    ensure_artifact_promotion_plan_field("plan_id", &request.plan_id)?;
    ensure_artifact_promotion_plan_field("generated_at", &request.generated_at)?;
    validate_promotion_readiness(&request.readiness)?;
    validate_promotion_artifact_identity_report(&request.artifact_identity_report)?;
    validate_promotion_plan_transform(&request.transform)?;
    if let Some(lineage) = &request.target_execution_lineage {
        validate_promotion_target_execution_lineage(lineage)?;
    }

    let blockers =
        artifact_promotion_plan_blockers(&request.readiness, &request.artifact_identity_report);
    let status = if blockers.is_empty() {
        PromotionReadinessStatusV1::Ready
    } else {
        PromotionReadinessStatusV1::Blocked
    };
    let mut plan = ArtifactPromotionPlanV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        plan_id: request.plan_id,
        artifact_promotion_plan_digest: String::new(),
        generated_at: request.generated_at,
        status,
        target_plan_id: request.transform.target_plan_id.clone(),
        promoted_plan_id: request.transform.promoted_plan_id.clone(),
        promotion_plan_lineage_digest: request.transform.promotion_plan_lineage_digest.clone(),
        readiness: request.readiness,
        artifact_identity_report: request.artifact_identity_report,
        transform: request.transform,
        target_execution_lineage: request.target_execution_lineage,
        blockers,
    };
    plan.artifact_promotion_plan_digest = artifact_promotion_plan_digest(&plan);
    validate_artifact_promotion_plan(&plan)?;
    Ok(plan)
}

pub fn promotion_target_execution_lineage(
    request: PromotionTargetExecutionLineageRequest,
) -> Result<PromotionTargetExecutionLineageV1, PromotionTargetExecutionLineageError> {
    ensure_target_execution_lineage_field("lineage_id", &request.lineage_id)?;
    ensure_target_execution_lineage_field("generated_at", &request.generated_at)?;
    validate_promotion_plan_transform(&request.transform)?;
    validate_deployment_execution_preflight(&request.execution_preflight)?;

    let target_execution_lineage_digest = promotion_target_execution_lineage_digest(
        &request.transform,
        &request.execution_preflight,
        false,
    );
    let lineage = PromotionTargetExecutionLineageV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        lineage_id: request.lineage_id,
        generated_at: request.generated_at,
        target_execution_lineage_digest,
        transform: request.transform,
        execution_preflight: request.execution_preflight,
        execution_attempted: false,
    };
    validate_promotion_target_execution_lineage(&lineage)?;
    Ok(lineage)
}

pub fn validate_artifact_promotion_plan(
    plan: &ArtifactPromotionPlanV1,
) -> Result<(), ArtifactPromotionPlanError> {
    if plan.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(ArtifactPromotionPlanError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found: plan.schema_version,
        });
    }
    ensure_artifact_promotion_plan_field("plan_id", &plan.plan_id)?;
    ensure_artifact_promotion_plan_sha256(
        "artifact_promotion_plan_digest",
        &plan.artifact_promotion_plan_digest,
    )?;
    ensure_artifact_promotion_plan_field("generated_at", &plan.generated_at)?;
    ensure_artifact_promotion_plan_field("target_plan_id", &plan.target_plan_id)?;
    ensure_artifact_promotion_plan_field("promoted_plan_id", &plan.promoted_plan_id)?;
    ensure_artifact_promotion_plan_field(
        "promotion_plan_lineage_digest",
        &plan.promotion_plan_lineage_digest,
    )?;
    ensure_artifact_promotion_status_matches_blockers(plan)?;
    validate_promotion_readiness(&plan.readiness)?;
    validate_promotion_artifact_identity_report(&plan.artifact_identity_report)?;
    validate_promotion_plan_transform(&plan.transform)?;
    ensure_artifact_promotion_plan_linkage(plan)?;
    if let Some(lineage) = &plan.target_execution_lineage {
        validate_promotion_target_execution_lineage(lineage)?;
        if lineage.transform != plan.transform {
            return Err(ArtifactPromotionPlanError::LinkageMismatch {
                field: "target_execution_lineage.transform",
            });
        }
    }
    if plan.artifact_promotion_plan_digest != artifact_promotion_plan_digest(plan) {
        return Err(ArtifactPromotionPlanError::LinkageMismatch {
            field: "artifact_promotion_plan_digest",
        });
    }
    Ok(())
}

pub fn validate_artifact_promotion_plan_for_check(
    plan: &ArtifactPromotionPlanV1,
    target_check: &DeploymentCheckV1,
) -> Result<(), ArtifactPromotionPlanError> {
    validate_artifact_promotion_plan(plan)?;
    if target_check.plan != plan.transform.promoted_plan {
        return Err(ArtifactPromotionPlanError::LinkageMismatch {
            field: "target_check.plan",
        });
    }
    let Some(lineage) = &plan.target_execution_lineage else {
        return Err(ArtifactPromotionPlanError::MissingTargetExecutionLineage);
    };
    validate_deployment_execution_preflight_for_check(target_check, &lineage.execution_preflight)
        .map_err(ArtifactPromotionPlanError::TargetCheck)?;
    Ok(())
}

pub fn artifact_promotion_provenance_report(
    request: ArtifactPromotionProvenanceReportRequest,
) -> Result<ArtifactPromotionProvenanceReportV1, ArtifactPromotionProvenanceReportError> {
    ensure_provenance_report_field("report_id", &request.report_id)?;
    validate_artifact_promotion_plan(&request.artifact_promotion_plan)?;
    if let Some(report) = &request.wasm_store_identity_report {
        validate_promotion_wasm_store_identity_report(report)?;
    }
    if let Some(verification) = &request.wasm_store_catalog_verification {
        validate_promotion_wasm_store_catalog_verification(verification)?;
    }
    if let Some(report) = &request.materialization_identity_report {
        validate_promotion_materialization_identity_report(report)?;
    }
    let report = build_artifact_promotion_provenance_report(request);
    validate_artifact_promotion_provenance_report(&report)?;
    Ok(report)
}

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

pub fn artifact_promotion_execution_receipt(
    request: ArtifactPromotionExecutionReceiptRequest,
) -> Result<ArtifactPromotionExecutionReceiptV1, ArtifactPromotionExecutionReceiptError> {
    ensure_execution_receipt_field("receipt_id", &request.receipt_id)?;
    validate_artifact_promotion_provenance_report(&request.provenance_report)?;
    ensure_execution_receipt_provenance_ready(request.provenance_report.status)?;
    validate_deployment_receipt_for_promotion(
        &request.deployment_receipt,
        &request.provenance_report,
    )?;
    let receipt = build_artifact_promotion_execution_receipt(request);
    validate_artifact_promotion_execution_receipt(&receipt)?;
    Ok(receipt)
}

pub fn validate_artifact_promotion_execution_receipt(
    receipt: &ArtifactPromotionExecutionReceiptV1,
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    if receipt.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            ArtifactPromotionExecutionReceiptError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: receipt.schema_version,
            },
        );
    }
    ensure_execution_receipt_field("receipt_id", &receipt.receipt_id)?;
    ensure_execution_receipt_sha256(
        "execution_receipt_digest",
        &receipt.execution_receipt_digest,
    )?;
    ensure_execution_receipt_field(
        "artifact_promotion_plan_id",
        &receipt.artifact_promotion_plan_id,
    )?;
    ensure_execution_receipt_sha256(
        "artifact_promotion_plan_digest",
        &receipt.artifact_promotion_plan_digest,
    )?;
    ensure_execution_receipt_field("provenance_report_id", &receipt.provenance_report_id)?;
    ensure_execution_receipt_sha256(
        "provenance_report_digest",
        &receipt.provenance_report_digest,
    )?;
    ensure_execution_receipt_provenance_ready(receipt.provenance_status)?;
    ensure_execution_receipt_field("promoted_plan_id", &receipt.promoted_plan_id)?;
    ensure_execution_receipt_field(
        "promotion_plan_lineage_digest",
        &receipt.promotion_plan_lineage_digest,
    )?;
    ensure_execution_receipt_field("operation_id", &receipt.operation_id)?;
    ensure_execution_receipt_field("started_at", &receipt.started_at)?;
    if let Some(finished_at) = &receipt.finished_at {
        ensure_execution_receipt_field("finished_at", finished_at)?;
    }
    ensure_execution_receipt_linkage(receipt)?;
    if receipt.execution_receipt_digest != artifact_promotion_execution_receipt_digest(receipt) {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "execution_receipt_digest",
        });
    }
    Ok(())
}

pub fn validate_promotion_plan_transform_evidence(
    evidence: &PromotionPlanTransformEvidenceV1,
) -> Result<(), PromotionPlanTransformEvidenceError> {
    if evidence.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(PromotionPlanTransformEvidenceError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found: evidence.schema_version,
        });
    }
    ensure_evidence_field("evidence_id", &evidence.evidence_id)?;
    ensure_evidence_sha256(
        "promotion_plan_transform_evidence_digest",
        &evidence.promotion_plan_transform_evidence_digest,
    )?;
    ensure_evidence_field("generated_at", &evidence.generated_at)?;
    validate_promotion_plan_transform(&evidence.transform)?;
    if evidence.promotion_plan_transform_evidence_digest
        != promotion_plan_transform_evidence_digest(evidence)
    {
        return Err(PromotionPlanTransformEvidenceError::LinkageMismatch {
            field: "promotion_plan_transform_evidence_digest",
        });
    }
    Ok(())
}

pub fn validate_promotion_target_execution_lineage(
    lineage: &PromotionTargetExecutionLineageV1,
) -> Result<(), PromotionTargetExecutionLineageError> {
    if lineage.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            PromotionTargetExecutionLineageError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: lineage.schema_version,
            },
        );
    }
    ensure_target_execution_lineage_field("lineage_id", &lineage.lineage_id)?;
    ensure_target_execution_lineage_field("generated_at", &lineage.generated_at)?;
    ensure_target_execution_lineage_sha256(
        "target_execution_lineage_digest",
        &lineage.target_execution_lineage_digest,
    )?;
    validate_promotion_plan_transform(&lineage.transform)?;
    validate_deployment_execution_preflight(&lineage.execution_preflight)?;
    if lineage.execution_attempted {
        return Err(PromotionTargetExecutionLineageError::ExecutionAttempted);
    }
    if lineage.execution_preflight.plan_id != lineage.transform.promoted_plan_id {
        return Err(PromotionTargetExecutionLineageError::LinkageMismatch {
            field: "execution_preflight.plan_id",
        });
    }
    let expected = promotion_target_execution_lineage_digest(
        &lineage.transform,
        &lineage.execution_preflight,
        lineage.execution_attempted,
    );
    if expected != lineage.target_execution_lineage_digest {
        return Err(PromotionTargetExecutionLineageError::LinkageMismatch {
            field: "target_execution_lineage_digest",
        });
    }
    Ok(())
}

pub fn validate_promotion_plan_transform(
    transform: &PromotionPlanTransformV1,
) -> Result<(), PromotionPlanTransformError> {
    if transform.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(PromotionPlanTransformError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found: transform.schema_version,
        });
    }
    ensure_transform_field("transform_id", &transform.transform_id)?;
    ensure_transform_field("target_plan_id", &transform.target_plan_id)?;
    ensure_transform_field("promoted_plan_id", &transform.promoted_plan_id)?;
    ensure_transform_field(
        "promotion_plan_lineage_digest",
        &transform.promotion_plan_lineage_digest,
    )?;
    ensure_transform_field("promoted_plan.plan_id", &transform.promoted_plan.plan_id)?;
    if transform.promoted_plan.plan_id != transform.promoted_plan_id {
        return Err(PromotionPlanTransformError::PromotedPlanIdMismatch {
            expected: transform.promoted_plan_id.clone(),
            found: transform.promoted_plan.plan_id.clone(),
        });
    }
    ensure_unique_transform_roles(&transform.roles)?;
    for role in &transform.roles {
        validate_role_plan_transform(role, &transform.promoted_plan)?;
    }
    let expected = promotion_plan_lineage_digest(
        &transform.target_plan_id,
        &transform.promoted_plan_id,
        &transform.promoted_plan,
        &transform.roles,
    );
    if expected != transform.promotion_plan_lineage_digest {
        return Err(PromotionPlanTransformError::RoleStateMismatch {
            role: "promotion_plan_lineage".to_string(),
            field: "promotion_plan_lineage_digest",
        });
    }
    Ok(())
}

#[must_use]
pub fn promotion_readiness_from_inputs(
    readiness_id: impl Into<String>,
    target_plan: &DeploymentPlanV1,
    inputs: &[RolePromotionInputV1],
) -> PromotionReadinessV1 {
    let mut roles = Vec::with_capacity(inputs.len());
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    for input in inputs {
        let target_artifact = target_plan
            .role_artifacts
            .iter()
            .find(|artifact| artifact.role == input.role);
        let Some(target_artifact) = target_artifact else {
            blockers.push(promotion_finding(
                "promotion_target_role_missing",
                format!("target plan does not contain role {}", input.role),
                SafetySeverityV1::HardFailure,
                &input.role,
            ));
            continue;
        };

        let role_readiness = role_promotion_readiness(input, target_artifact);
        collect_role_findings(input, &role_readiness, &mut blockers, &mut warnings);
        roles.push(role_readiness);
    }

    let status = if blockers.is_empty() {
        PromotionReadinessStatusV1::Ready
    } else {
        PromotionReadinessStatusV1::Blocked
    };

    let mut readiness = PromotionReadinessV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        readiness_id: readiness_id.into(),
        promotion_readiness_digest: String::new(),
        target_plan_id: target_plan.plan_id.clone(),
        status,
        roles,
        blockers,
        warnings,
    };
    readiness.promotion_readiness_digest = promotion_readiness_digest(&readiness);
    readiness
}

#[must_use]
pub fn promotion_readiness_from_inputs_with_policy(
    readiness_id: impl Into<String>,
    target_plan: &DeploymentPlanV1,
    inputs: &[RolePromotionInputV1],
    policies: &[RolePromotionPolicyV1],
) -> PromotionReadinessV1 {
    let readiness_id = readiness_id.into();
    let policy_check =
        promotion_policy_check_from_inputs(format!("{readiness_id}:policy"), inputs, policies);
    let mut readiness = promotion_readiness_from_inputs(readiness_id, target_plan, inputs);
    readiness.blockers.extend(policy_check.blockers);
    readiness.status = if readiness.blockers.is_empty() {
        PromotionReadinessStatusV1::Ready
    } else {
        PromotionReadinessStatusV1::Blocked
    };
    readiness.promotion_readiness_digest = promotion_readiness_digest(&readiness);
    readiness
}

pub fn validate_promotion_readiness(
    readiness: &PromotionReadinessV1,
) -> Result<(), PromotionReadinessError> {
    if readiness.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(PromotionReadinessError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found: readiness.schema_version,
        });
    }
    ensure_readiness_field("readiness_id", &readiness.readiness_id)?;
    ensure_readiness_sha256(
        "promotion_readiness_digest",
        &readiness.promotion_readiness_digest,
    )?;
    ensure_readiness_field("target_plan_id", &readiness.target_plan_id)?;
    ensure_readiness_status_matches_blockers(readiness)?;
    ensure_unique_readiness_roles(&readiness.roles)?;
    for role in &readiness.roles {
        validate_role_readiness(role)?;
    }
    validate_readiness_findings(
        "blockers",
        &readiness.blockers,
        SafetySeverityV1::HardFailure,
    )?;
    validate_readiness_findings("warnings", &readiness.warnings, SafetySeverityV1::Warning)?;
    if readiness.promotion_readiness_digest != promotion_readiness_digest(readiness) {
        return Err(PromotionReadinessError::LinkageMismatch {
            field: "promotion_readiness_digest",
        });
    }
    Ok(())
}

pub fn validate_role_artifact_source(
    source: &RoleArtifactSourceV1,
) -> Result<(), PromotionArtifactSourceError> {
    ensure_field("role", &source.role)?;
    ensure_locator_requirement(source)?;
    ensure_previous_receipt_requirement(source)?;
    ensure_digest_requirement(source)?;
    ensure_previous_receipt_lineage_digest_requirement(source)?;
    ensure_optional_sha256(
        "expected_wasm_sha256",
        source.expected_wasm_sha256.as_deref(),
    )?;
    ensure_optional_sha256(
        "expected_wasm_gz_sha256",
        source.expected_wasm_gz_sha256.as_deref(),
    )?;
    ensure_optional_sha256(
        "expected_candid_sha256",
        source.expected_candid_sha256.as_deref(),
    )?;
    ensure_optional_sha256(
        "expected_canonical_embedded_config_sha256",
        source.expected_canonical_embedded_config_sha256.as_deref(),
    )?;
    ensure_optional_sha256(
        "previous_receipt_lineage_digest",
        source.previous_receipt_lineage_digest.as_deref(),
    )?;
    Ok(())
}

pub fn build_materialization_evidence(
    request: BuildMaterializationEvidenceRequest,
) -> Result<BuildMaterializationEvidenceV1, PromotionMaterializationIdentityError> {
    ensure_materialization_field("evidence_id", &request.evidence_id)?;
    validate_build_recipe_identity(&request.recipe)?;
    validate_build_materialization_input(&request.materialization_input)?;
    validate_build_materialization_result(&request.materialization_result)?;
    let computed_materialization_input_digest =
        build_materialization_input_digest(&request.materialization_input);
    let mut evidence = BuildMaterializationEvidenceV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        evidence_id: request.evidence_id,
        materialization_evidence_digest: String::new(),
        recipe_id_matches_input: request.recipe.recipe_id
            == request.materialization_input.build_recipe_id,
        recipe_id_matches_result: request.recipe.recipe_id
            == request.materialization_result.build_recipe_id,
        materialization_input_digest_matches_result: computed_materialization_input_digest
            == request.materialization_result.materialization_input_digest,
        computed_materialization_input_digest,
        recipe: request.recipe,
        materialization_input: request.materialization_input,
        materialization_result: request.materialization_result,
    };
    evidence.materialization_evidence_digest = build_materialization_evidence_digest(&evidence);
    validate_build_materialization_evidence(&evidence)?;
    Ok(evidence)
}

pub fn validate_build_materialization_evidence(
    evidence: &BuildMaterializationEvidenceV1,
) -> Result<(), PromotionMaterializationIdentityError> {
    if evidence.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            PromotionMaterializationIdentityError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: evidence.schema_version,
            },
        );
    }
    ensure_materialization_field("evidence_id", &evidence.evidence_id)?;
    ensure_materialization_sha256(
        "materialization_evidence_digest",
        &evidence.materialization_evidence_digest,
    )?;
    validate_build_recipe_identity(&evidence.recipe)?;
    validate_build_materialization_input(&evidence.materialization_input)?;
    validate_build_materialization_result(&evidence.materialization_result)?;
    ensure_materialization_sha256(
        "computed_materialization_input_digest",
        &evidence.computed_materialization_input_digest,
    )?;
    ensure_materialization_link(
        "recipe_id_matches_input",
        evidence.recipe_id_matches_input
            == (evidence.recipe.recipe_id == evidence.materialization_input.build_recipe_id),
    )?;
    ensure_materialization_link("recipe_id_matches_input", evidence.recipe_id_matches_input)?;
    ensure_materialization_link(
        "recipe_id_matches_result",
        evidence.recipe_id_matches_result
            == (evidence.recipe.recipe_id == evidence.materialization_result.build_recipe_id),
    )?;
    ensure_materialization_link(
        "recipe_id_matches_result",
        evidence.recipe_id_matches_result,
    )?;
    let computed = build_materialization_input_digest(&evidence.materialization_input);
    if computed != evidence.computed_materialization_input_digest {
        return Err(PromotionMaterializationIdentityError::DigestMismatch {
            field: "computed_materialization_input_digest",
            expected: computed,
            found: evidence.computed_materialization_input_digest.clone(),
        });
    }
    ensure_materialization_link(
        "materialization_input_digest_matches_result",
        evidence.materialization_input_digest_matches_result
            == (evidence.computed_materialization_input_digest
                == evidence.materialization_result.materialization_input_digest),
    )?;
    ensure_materialization_link(
        "materialization_input_digest_matches_result",
        evidence.materialization_input_digest_matches_result,
    )?;
    if evidence.materialization_evidence_digest != build_materialization_evidence_digest(evidence) {
        return Err(PromotionMaterializationIdentityError::LinkageMismatch {
            field: "materialization_evidence_digest",
        });
    }
    Ok(())
}

pub fn promotion_materialization_identity_report_from_evidence(
    request: PromotionMaterializationIdentityReportRequest,
) -> Result<PromotionMaterializationIdentityReportV1, PromotionMaterializationIdentityReportError> {
    ensure_materialization_report_field("report_id", &request.report_id)?;
    for evidence in &request.evidence {
        validate_build_materialization_evidence(evidence)?;
    }
    let report = promotion_materialization_identity_report(&request.report_id, &request.evidence);
    validate_promotion_materialization_identity_report(&report)?;
    Ok(report)
}

#[must_use]
pub fn promotion_materialization_identity_report(
    report_id: impl Into<String>,
    evidence: &[BuildMaterializationEvidenceV1],
) -> PromotionMaterializationIdentityReportV1 {
    let roles = evidence
        .iter()
        .map(role_materialization_identity_from_evidence)
        .collect::<Vec<_>>();
    let output_groups = promotion_materialization_output_groups(&roles);
    let blockers = Vec::new();
    let mut report = PromotionMaterializationIdentityReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: report_id.into(),
        materialization_identity_report_digest: String::new(),
        status: PromotionReadinessStatusV1::Ready,
        roles,
        output_groups,
        blockers,
    };
    report.materialization_identity_report_digest =
        promotion_materialization_identity_report_digest(&report);
    report
}

pub fn validate_promotion_materialization_identity_report(
    report: &PromotionMaterializationIdentityReportV1,
) -> Result<(), PromotionMaterializationIdentityReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            PromotionMaterializationIdentityReportError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: report.schema_version,
            },
        );
    }
    ensure_materialization_report_field("report_id", &report.report_id)?;
    ensure_materialization_report_sha256(
        "materialization_identity_report_digest",
        &report.materialization_identity_report_digest,
    )?;
    ensure_materialization_report_status_matches_blockers(report)?;
    ensure_unique_materialization_report_roles(&report.roles)?;
    for role in &report.roles {
        validate_role_materialization_identity(role)?;
    }
    validate_materialization_output_groups(&report.roles, &report.output_groups)?;
    let expected_blockers = Vec::<SafetyFindingV1>::new();
    if report.blockers != expected_blockers {
        return Err(PromotionMaterializationIdentityReportError::BlockerMismatch);
    }
    validate_materialization_report_blockers(&report.blockers)?;
    if report.materialization_identity_report_digest
        != promotion_materialization_identity_report_digest(report)
    {
        return Err(
            PromotionMaterializationIdentityReportError::LinkageMismatch {
                field: "materialization_identity_report_digest",
            },
        );
    }
    Ok(())
}

pub fn validate_build_recipe_identity(
    recipe: &BuildRecipeIdentityV1,
) -> Result<(), PromotionMaterializationIdentityError> {
    ensure_materialization_field("recipe_id", &recipe.recipe_id)?;
    ensure_materialization_field("source_revision", &recipe.source_revision)?;
    ensure_materialization_field("package_or_role_selector", &recipe.package_or_role_selector)?;
    ensure_materialization_field("cargo_profile", &recipe.cargo_profile)?;
    ensure_materialization_sha256("cargo_features_digest", &recipe.cargo_features_digest)?;
    ensure_materialization_sha256("cargo_lock_digest", &recipe.cargo_lock_digest)?;
    ensure_materialization_field("rust_toolchain", &recipe.rust_toolchain)?;
    ensure_materialization_field("builder_version", &recipe.builder_version)?;
    ensure_materialization_field("target_triple", &recipe.target_triple)?;
    ensure_materialization_field("linker_identity", &recipe.linker_identity)?;
    ensure_materialization_field("deterministic_build_mode", &recipe.deterministic_build_mode)?;
    ensure_materialization_field("wasm_opt_version", &recipe.wasm_opt_version)?;
    ensure_materialization_field("compression_identity", &recipe.compression_identity)?;
    Ok(())
}

pub fn validate_build_materialization_input(
    input: &BuildMaterializationInputV1,
) -> Result<(), PromotionMaterializationIdentityError> {
    ensure_materialization_field("materialization_input_id", &input.materialization_input_id)?;
    ensure_materialization_field("build_recipe_id", &input.build_recipe_id)?;
    ensure_materialization_sha256(
        "canonical_embedded_config_sha256",
        &input.canonical_embedded_config_sha256,
    )?;
    ensure_materialization_field("network", &input.network)?;
    ensure_materialization_field("root_trust_anchor", &input.root_trust_anchor)?;
    ensure_materialization_field("runtime_variant", &input.runtime_variant)?;
    Ok(())
}

pub fn validate_build_materialization_result(
    result: &BuildMaterializationResultV1,
) -> Result<(), PromotionMaterializationIdentityError> {
    ensure_materialization_field(
        "materialization_result_id",
        &result.materialization_result_id,
    )?;
    ensure_materialization_field("build_recipe_id", &result.build_recipe_id)?;
    ensure_materialization_sha256(
        "materialization_input_digest",
        &result.materialization_input_digest,
    )?;
    ensure_materialization_sha256("wasm_sha256", &result.wasm_sha256)?;
    ensure_materialization_sha256("wasm_gz_sha256", &result.wasm_gz_sha256)?;
    ensure_materialization_sha256("installed_module_hash", &result.installed_module_hash)?;
    ensure_materialization_sha256("candid_sha256", &result.candid_sha256)?;
    Ok(())
}

fn apply_promotion_input_to_role_artifact(
    role_artifact: &mut RoleArtifactV1,
    input: &RolePromotionInputV1,
) {
    match input.promotion_level {
        PromotionArtifactLevelV1::SealedWasm => {
            role_artifact.source = artifact_source_for_promotion_source(input.source.kind);
            apply_promotion_source_locator(role_artifact, &input.source);
            role_artifact
                .wasm_sha256
                .clone_from(&input.source.expected_wasm_sha256);
            role_artifact
                .wasm_gz_sha256
                .clone_from(&input.source.expected_wasm_gz_sha256);
            role_artifact
                .candid_sha256
                .clone_from(&input.source.expected_candid_sha256);
            role_artifact
                .canonical_embedded_config_sha256
                .clone_from(&input.source.expected_canonical_embedded_config_sha256);
        }
        PromotionArtifactLevelV1::SourceBuild => {}
    }
}

const fn artifact_source_for_promotion_source(kind: RoleArtifactSourceKindV1) -> ArtifactSourceV1 {
    match kind {
        RoleArtifactSourceKindV1::WorkspacePackage => ArtifactSourceV1::LocalBuild,
        RoleArtifactSourceKindV1::CanonicalWasmStoreDefault => ArtifactSourceV1::WasmStore,
        RoleArtifactSourceKindV1::PublishedPackage
        | RoleArtifactSourceKindV1::LocalWasm
        | RoleArtifactSourceKindV1::LocalWasmGz
        | RoleArtifactSourceKindV1::PreviousReceiptArtifact => ArtifactSourceV1::External,
    }
}

fn apply_promotion_source_locator(
    role_artifact: &mut RoleArtifactV1,
    source: &RoleArtifactSourceV1,
) {
    match source.kind {
        RoleArtifactSourceKindV1::LocalWasm => {
            role_artifact.wasm_path.clone_from(&source.locator);
        }
        RoleArtifactSourceKindV1::LocalWasmGz => {
            role_artifact.wasm_gz_path.clone_from(&source.locator);
        }
        _ => {}
    }
}

fn promotion_plan_transform_from_parts(
    target_plan: &DeploymentPlanV1,
    promoted_plan: DeploymentPlanV1,
    inputs: &[RolePromotionInputV1],
) -> PromotionPlanTransformV1 {
    let roles = inputs
        .iter()
        .filter_map(|input| {
            let before = target_plan
                .role_artifacts
                .iter()
                .find(|artifact| artifact.role == input.role)?;
            let after = promoted_plan
                .role_artifacts
                .iter()
                .find(|artifact| artifact.role == input.role)?;
            Some(role_plan_transform(input, before, after))
        })
        .collect::<Vec<_>>();
    let promotion_plan_lineage_digest = promotion_plan_lineage_digest(
        &target_plan.plan_id,
        &promoted_plan.plan_id,
        &promoted_plan,
        &roles,
    );

    PromotionPlanTransformV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        transform_id: format!("promotion-transform:{}", promoted_plan.plan_id),
        target_plan_id: target_plan.plan_id.clone(),
        promoted_plan_id: promoted_plan.plan_id.clone(),
        promotion_plan_lineage_digest,
        promoted_plan,
        roles,
    }
}

fn role_plan_transform(
    input: &RolePromotionInputV1,
    before: &RoleArtifactV1,
    after: &RoleArtifactV1,
) -> RolePromotionPlanTransformV1 {
    RolePromotionPlanTransformV1 {
        role: input.role.clone(),
        promotion_level: input.promotion_level,
        source_kind: input.source.kind,
        source_locator: input.source.locator.clone(),
        artifact_source_before: before.source,
        artifact_source_after: after.source,
        wasm_sha256_before: before.wasm_sha256.clone(),
        wasm_sha256_after: after.wasm_sha256.clone(),
        wasm_gz_sha256_before: before.wasm_gz_sha256.clone(),
        wasm_gz_sha256_after: after.wasm_gz_sha256.clone(),
        candid_sha256_before: before.candid_sha256.clone(),
        candid_sha256_after: after.candid_sha256.clone(),
        canonical_embedded_config_sha256_before: before.canonical_embedded_config_sha256.clone(),
        canonical_embedded_config_sha256_after: after.canonical_embedded_config_sha256.clone(),
        artifact_identity_changed: artifact_identity_changed(before, after),
        embedded_config_changed: before.canonical_embedded_config_sha256
            != after.canonical_embedded_config_sha256,
        target_materialization_preserved: input.promotion_level
            == PromotionArtifactLevelV1::SourceBuild
            && role_materialization_identity_matches(before, after),
        source_build_materialization: None,
    }
}

fn attach_source_build_materialization(
    transform: &mut PromotionPlanTransformV1,
    inputs: &[RolePromotionInputV1],
    evidence: &[BuildMaterializationEvidenceV1],
) -> Result<(), PromotionPlanTransformError> {
    let input_roles = inputs
        .iter()
        .map(|input| input.role.as_str())
        .collect::<BTreeSet<_>>();
    let mut links = BTreeMap::new();
    for item in evidence {
        validate_build_materialization_evidence(item)?;
        let role = item.recipe.package_or_role_selector.as_str();
        if !input_roles.contains(role) {
            return Err(PromotionPlanTransformError::UnexpectedMaterializationRole {
                role: role.to_string(),
            });
        }
        if links
            .insert(role.to_string(), materialization_link_from_evidence(item))
            .is_some()
        {
            return Err(PromotionPlanTransformError::DuplicateMaterializationRole {
                role: role.to_string(),
            });
        }
    }

    for role in &mut transform.roles {
        match role.promotion_level {
            PromotionArtifactLevelV1::SourceBuild => {
                let Some(link) = links.remove(&role.role) else {
                    return Err(PromotionPlanTransformError::MaterializationRoleMissing {
                        role: role.role.clone(),
                    });
                };
                role.source_build_materialization = Some(link);
            }
            PromotionArtifactLevelV1::SealedWasm => {
                if links.remove(&role.role).is_some() {
                    return Err(PromotionPlanTransformError::UnexpectedMaterializationRole {
                        role: role.role.clone(),
                    });
                }
            }
        }
    }

    if let Some(role) = links.keys().next() {
        return Err(PromotionPlanTransformError::UnexpectedMaterializationRole {
            role: role.clone(),
        });
    }
    Ok(())
}

fn materialization_link_from_evidence(
    evidence: &BuildMaterializationEvidenceV1,
) -> RolePromotionMaterializationLinkV1 {
    RolePromotionMaterializationLinkV1 {
        role: evidence.recipe.package_or_role_selector.clone(),
        evidence_id: evidence.evidence_id.clone(),
        materialization_evidence_digest: evidence.materialization_evidence_digest.clone(),
        recipe_id: evidence.recipe.recipe_id.clone(),
        materialization_input_id: evidence
            .materialization_input
            .materialization_input_id
            .clone(),
        materialization_result_id: evidence
            .materialization_result
            .materialization_result_id
            .clone(),
        materialization_input_digest: evidence.computed_materialization_input_digest.clone(),
        wasm_sha256: evidence.materialization_result.wasm_sha256.clone(),
        wasm_gz_sha256: evidence.materialization_result.wasm_gz_sha256.clone(),
        installed_module_hash: evidence
            .materialization_result
            .installed_module_hash
            .clone(),
        candid_sha256: evidence.materialization_result.candid_sha256.clone(),
    }
}

fn artifact_promotion_plan_blockers(
    readiness: &PromotionReadinessV1,
    artifact_identity_report: &PromotionArtifactIdentityReportV1,
) -> Vec<SafetyFindingV1> {
    let mut blockers =
        Vec::with_capacity(readiness.blockers.len() + artifact_identity_report.blockers.len());
    blockers.extend(readiness.blockers.clone());
    blockers.extend(artifact_identity_report.blockers.clone());
    blockers
}

fn build_artifact_promotion_provenance_report(
    request: ArtifactPromotionProvenanceReportRequest,
) -> ArtifactPromotionProvenanceReportV1 {
    let plan = request.artifact_promotion_plan;
    let mut roles = plan
        .transform
        .roles
        .iter()
        .map(role_promotion_provenance_from_transform)
        .collect::<Vec<_>>();
    attach_wasm_store_provenance(&mut roles, request.wasm_store_identity_report.as_ref());
    attach_wasm_store_catalog_provenance(
        &mut roles,
        request.wasm_store_catalog_verification.as_ref(),
    );
    attach_materialization_provenance(&mut roles, request.materialization_identity_report.as_ref());
    let blockers = artifact_promotion_provenance_blockers(
        &plan,
        request.wasm_store_identity_report.as_ref(),
        request.wasm_store_catalog_verification.as_ref(),
        request.materialization_identity_report.as_ref(),
        &roles,
    );
    let status = if blockers.is_empty() {
        PromotionReadinessStatusV1::Ready
    } else {
        PromotionReadinessStatusV1::Blocked
    };
    let mut report = ArtifactPromotionProvenanceReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: request.report_id,
        status,
        artifact_promotion_plan_id: plan.plan_id,
        artifact_promotion_plan_digest: plan.artifact_promotion_plan_digest,
        target_plan_id: plan.target_plan_id,
        promoted_plan_id: plan.promoted_plan_id,
        promotion_plan_lineage_digest: plan.promotion_plan_lineage_digest,
        provenance_report_digest: String::new(),
        readiness_id: plan.readiness.readiness_id,
        artifact_identity_report_id: plan.artifact_identity_report.report_id,
        transform_id: plan.transform.transform_id,
        target_execution_lineage_id: plan
            .target_execution_lineage
            .map(|lineage| lineage.lineage_id),
        wasm_store_identity_report_id: request
            .wasm_store_identity_report
            .as_ref()
            .map(|report| report.report_id.clone()),
        wasm_store_identity_report_digest: request
            .wasm_store_identity_report
            .map(|report| report.wasm_store_identity_report_digest),
        wasm_store_catalog_verification_id: request
            .wasm_store_catalog_verification
            .as_ref()
            .map(|verification| verification.verification_id.clone()),
        wasm_store_catalog_verification_digest: request
            .wasm_store_catalog_verification
            .map(|verification| verification.wasm_store_catalog_verification_digest),
        materialization_identity_report_id: request
            .materialization_identity_report
            .as_ref()
            .map(|report| report.report_id.clone()),
        materialization_identity_report_digest: request
            .materialization_identity_report
            .map(|report| report.materialization_identity_report_digest),
        execution_attempted: false,
        roles,
        blockers,
    };
    report.provenance_report_digest = artifact_promotion_provenance_digest(&report);
    report
}

fn artifact_promotion_provenance_blockers(
    plan: &ArtifactPromotionPlanV1,
    wasm_store_report: Option<&PromotionWasmStoreIdentityReportV1>,
    wasm_store_catalog: Option<&PromotionWasmStoreCatalogVerificationV1>,
    materialization_report: Option<&PromotionMaterializationIdentityReportV1>,
    roles: &[RolePromotionProvenanceV1],
) -> Vec<SafetyFindingV1> {
    let mut blockers = plan.blockers.clone();
    let role_names = roles
        .iter()
        .map(|role| role.role.as_str())
        .collect::<BTreeSet<_>>();
    if let Some(report) = wasm_store_report {
        blockers.extend(report.blockers.iter().cloned());
    }
    append_wasm_store_catalog_provenance_blockers(
        &mut blockers,
        wasm_store_report,
        wasm_store_catalog,
        &role_names,
    );
    if let Some(report) = materialization_report {
        blockers.extend(report.blockers.iter().cloned());
    }
    append_optional_report_unknown_role_blockers(
        &mut blockers,
        wasm_store_report,
        wasm_store_catalog,
        materialization_report,
        &role_names,
    );
    blockers
}

fn append_wasm_store_catalog_provenance_blockers(
    blockers: &mut Vec<SafetyFindingV1>,
    wasm_store_report: Option<&PromotionWasmStoreIdentityReportV1>,
    wasm_store_catalog: Option<&PromotionWasmStoreCatalogVerificationV1>,
    role_names: &BTreeSet<&str>,
) {
    let Some(verification) = wasm_store_catalog else {
        return;
    };
    blockers.extend(verification.blockers.iter().cloned());
    match wasm_store_report {
        Some(report) if verification.wasm_store_identity_report_id == report.report_id => {}
        Some(report) => blockers.push(promotion_finding(
            "promotion_provenance_wasm_store_catalog_identity_mismatch",
            format!(
                "wasm-store catalog verification references identity report {}, but provenance uses {}",
                verification.wasm_store_identity_report_id, report.report_id
            ),
            SafetySeverityV1::HardFailure,
            "wasm_store_catalog",
        )),
        None => blockers.push(promotion_finding(
            "promotion_provenance_wasm_store_catalog_identity_missing",
            "wasm-store catalog verification requires the referenced wasm-store identity report",
            SafetySeverityV1::HardFailure,
            "wasm_store_catalog",
        )),
    }
    if let Some(report) = wasm_store_report {
        append_wasm_store_catalog_locator_blockers(blockers, report, verification, role_names);
    }
}

fn append_wasm_store_catalog_locator_blockers(
    blockers: &mut Vec<SafetyFindingV1>,
    report: &PromotionWasmStoreIdentityReportV1,
    verification: &PromotionWasmStoreCatalogVerificationV1,
    role_names: &BTreeSet<&str>,
) {
    for catalog_role in &verification.roles {
        if !role_names.contains(catalog_role.role.as_str()) {
            continue;
        }
        match report.roles.iter().find(|role| role.role == catalog_role.role) {
            Some(identity_role)
                if identity_role.wasm_store_locator.as_deref()
                    == Some(catalog_role.wasm_store_locator.as_str()) => {}
            Some(identity_role) => blockers.push(promotion_finding(
                "promotion_provenance_wasm_store_catalog_locator_mismatch",
                format!(
                    "wasm-store catalog verification role {} uses locator {}, but identity report uses {}",
                    catalog_role.role,
                    catalog_role.wasm_store_locator,
                    identity_role.wasm_store_locator.as_deref().unwrap_or("none")
                ),
                SafetySeverityV1::HardFailure,
                &catalog_role.role,
            )),
            None => blockers.push(promotion_finding(
                "promotion_provenance_wasm_store_catalog_role_identity_missing",
                format!(
                    "wasm-store catalog verification role {} is missing from the wasm-store identity report",
                    catalog_role.role
                ),
                SafetySeverityV1::HardFailure,
                &catalog_role.role,
            )),
        }
    }
}

fn append_optional_report_unknown_role_blockers(
    blockers: &mut Vec<SafetyFindingV1>,
    wasm_store_report: Option<&PromotionWasmStoreIdentityReportV1>,
    wasm_store_catalog: Option<&PromotionWasmStoreCatalogVerificationV1>,
    materialization_report: Option<&PromotionMaterializationIdentityReportV1>,
    role_names: &BTreeSet<&str>,
) {
    if let Some(report) = wasm_store_report {
        for role in &report.roles {
            if !role_names.contains(role.role.as_str()) {
                blockers.push(promotion_finding(
                    "promotion_provenance_unknown_wasm_store_role",
                    format!(
                        "wasm-store identity report contains unknown role {}",
                        role.role
                    ),
                    SafetySeverityV1::HardFailure,
                    &role.role,
                ));
            }
        }
    }
    if let Some(verification) = wasm_store_catalog {
        for role in &verification.roles {
            if !role_names.contains(role.role.as_str()) {
                blockers.push(promotion_finding(
                    "promotion_provenance_unknown_wasm_store_catalog_role",
                    format!(
                        "wasm-store catalog verification contains unknown role {}",
                        role.role
                    ),
                    SafetySeverityV1::HardFailure,
                    &role.role,
                ));
            }
        }
    }
    if let Some(report) = materialization_report {
        for role in &report.roles {
            if !role_names.contains(role.role.as_str()) {
                blockers.push(promotion_finding(
                    "promotion_provenance_unknown_materialization_role",
                    format!(
                        "materialization identity report contains unknown role {}",
                        role.role
                    ),
                    SafetySeverityV1::HardFailure,
                    &role.role,
                ));
            }
        }
    }
}

fn role_promotion_provenance_from_transform(
    role: &RolePromotionPlanTransformV1,
) -> RolePromotionProvenanceV1 {
    RolePromotionProvenanceV1 {
        role: role.role.clone(),
        promotion_level: role.promotion_level,
        source_kind: role.source_kind,
        artifact_identity_changed: role.artifact_identity_changed,
        embedded_config_changed: role.embedded_config_changed,
        target_materialization_preserved: role.target_materialization_preserved,
        materialization_evidence_id: role
            .source_build_materialization
            .as_ref()
            .map(|materialization| materialization.evidence_id.clone()),
        materialization_evidence_digest: role
            .source_build_materialization
            .as_ref()
            .map(|materialization| materialization.materialization_evidence_digest.clone()),
        wasm_store_locator: None,
        wasm_store_catalog_observation_digest: None,
    }
}

fn attach_wasm_store_provenance(
    roles: &mut [RolePromotionProvenanceV1],
    report: Option<&PromotionWasmStoreIdentityReportV1>,
) {
    let Some(report) = report else {
        return;
    };
    for role in roles {
        if let Some(wasm_store_role) = report.roles.iter().find(|item| item.role == role.role) {
            role.wasm_store_locator = wasm_store_role.wasm_store_locator.clone();
        }
    }
}

fn attach_wasm_store_catalog_provenance(
    roles: &mut [RolePromotionProvenanceV1],
    verification: Option<&PromotionWasmStoreCatalogVerificationV1>,
) {
    let Some(verification) = verification else {
        return;
    };
    for role in roles {
        if let Some(catalog_role) = verification
            .roles
            .iter()
            .find(|item| item.role == role.role)
        {
            role.wasm_store_catalog_observation_digest =
                Some(catalog_role.catalog_observation_digest.clone());
        }
    }
}

fn attach_materialization_provenance(
    roles: &mut [RolePromotionProvenanceV1],
    report: Option<&PromotionMaterializationIdentityReportV1>,
) {
    let Some(report) = report else {
        return;
    };
    for role in roles {
        if let Some(materialization_role) = report.roles.iter().find(|item| item.role == role.role)
        {
            role.materialization_evidence_id = Some(materialization_role.evidence_id.clone());
            role.materialization_evidence_digest =
                Some(materialization_role.materialization_evidence_digest.clone());
        }
    }
}

fn build_artifact_promotion_execution_receipt(
    request: ArtifactPromotionExecutionReceiptRequest,
) -> ArtifactPromotionExecutionReceiptV1 {
    let roles = request
        .provenance_report
        .roles
        .iter()
        .map(|role| role_promotion_execution_receipt(role, &request.deployment_receipt))
        .collect::<Vec<_>>();
    let mut receipt = ArtifactPromotionExecutionReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        receipt_id: request.receipt_id,
        execution_receipt_digest: String::new(),
        artifact_promotion_plan_id: request.provenance_report.artifact_promotion_plan_id.clone(),
        artifact_promotion_plan_digest: request
            .provenance_report
            .artifact_promotion_plan_digest
            .clone(),
        provenance_report_id: request.provenance_report.report_id.clone(),
        provenance_report_digest: request.provenance_report.provenance_report_digest,
        provenance_status: request.provenance_report.status,
        promoted_plan_id: request.provenance_report.promoted_plan_id.clone(),
        promotion_plan_lineage_digest: request.provenance_report.promotion_plan_lineage_digest,
        operation_id: request.deployment_receipt.operation_id.clone(),
        operation_status: request.deployment_receipt.operation_status,
        command_result: request.deployment_receipt.command_result.clone(),
        started_at: request.deployment_receipt.started_at.clone(),
        finished_at: request.deployment_receipt.finished_at.clone(),
        deployment_receipt: request.deployment_receipt,
        roles,
    };
    receipt.execution_receipt_digest = artifact_promotion_execution_receipt_digest(&receipt);
    receipt
}

fn role_promotion_execution_receipt(
    role: &RolePromotionProvenanceV1,
    deployment_receipt: &DeploymentReceiptV1,
) -> RolePromotionExecutionReceiptV1 {
    let role_receipt = deployment_receipt
        .role_phase_receipts
        .iter()
        .rev()
        .find(|receipt| receipt.role == role.role);
    RolePromotionExecutionReceiptV1 {
        role: role.role.clone(),
        promotion_level: role.promotion_level,
        materialization_evidence_id: role.materialization_evidence_id.clone(),
        materialization_evidence_digest: role.materialization_evidence_digest.clone(),
        wasm_store_locator: role.wasm_store_locator.clone(),
        wasm_store_catalog_observation_digest: role.wasm_store_catalog_observation_digest.clone(),
        role_phase_result: role_receipt.map(|receipt| receipt.result),
        artifact_digest: role_receipt.and_then(|receipt| receipt.artifact_digest.clone()),
        observed_module_hash_after: role_receipt
            .and_then(|receipt| receipt.observed_module_hash_after.clone()),
        canonical_embedded_config_sha256: role_receipt
            .and_then(|receipt| receipt.canonical_embedded_config_sha256.clone()),
    }
}

fn validate_deployment_receipt_for_promotion(
    receipt: &DeploymentReceiptV1,
    provenance: &ArtifactPromotionProvenanceReportV1,
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    if receipt.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            ArtifactPromotionExecutionReceiptError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: receipt.schema_version,
            },
        );
    }
    ensure_execution_receipt_field("deployment_receipt.operation_id", &receipt.operation_id)?;
    ensure_execution_receipt_field("deployment_receipt.started_at", &receipt.started_at)?;
    if receipt.plan_id != provenance.promoted_plan_id {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "deployment_receipt.plan_id",
        });
    }
    if let Some(finished_at) = &receipt.finished_at {
        ensure_execution_receipt_field("deployment_receipt.finished_at", finished_at)?;
    }
    Ok(())
}

fn wasm_store_identity_blockers(
    roles: &[RolePromotionWasmStoreIdentityV1],
) -> Vec<SafetyFindingV1> {
    let mut blockers = Vec::new();
    for role in roles {
        if role.transport != ArtifactTransportV1::WasmStore {
            blockers.push(promotion_finding(
                "promotion_wasm_store_transport_mismatch",
                format!("role {} was not staged through wasm_store", role.role),
                SafetySeverityV1::HardFailure,
                &role.role,
            ));
        }
        if role.wasm_store_locator.as_deref().is_none_or(str::is_empty) {
            blockers.push(promotion_finding(
                "promotion_wasm_store_locator_missing",
                format!("role {} does not record a wasm_store locator", role.role),
                SafetySeverityV1::HardFailure,
                &role.role,
            ));
        }
        if role.verified_postcondition.status != ObservationStatusV1::Observed {
            blockers.push(promotion_finding(
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
            blockers.push(promotion_finding(
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
            blockers.push(promotion_finding(
                "promotion_wasm_store_catalog_locator_missing",
                format!("role {} does not record a wasm_store locator", role.role),
                SafetySeverityV1::HardFailure,
                &role.role,
            ));
        } else if !role.catalog_entry_present {
            blockers.push(promotion_finding(
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
            blockers.push(promotion_finding(
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
            blockers.push(promotion_finding(
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

fn refresh_promotion_plan_lineage_digest(transform: &mut PromotionPlanTransformV1) {
    transform.promotion_plan_lineage_digest = promotion_plan_lineage_digest(
        &transform.target_plan_id,
        &transform.promoted_plan_id,
        &transform.promoted_plan,
        &transform.roles,
    );
}

fn validate_role_artifact_identity(
    role: &RolePromotionArtifactIdentityV1,
) -> Result<(), PromotionArtifactIdentityReportError> {
    ensure_identity_report_field("role", &role.role)?;
    ensure_identity_optional_sha256("wasm_sha256", role.wasm_sha256.as_deref())?;
    ensure_identity_optional_sha256("wasm_gz_sha256", role.wasm_gz_sha256.as_deref())?;
    ensure_identity_optional_sha256("candid_sha256", role.candid_sha256.as_deref())?;
    ensure_identity_optional_sha256(
        "canonical_embedded_config_sha256",
        role.canonical_embedded_config_sha256.as_deref(),
    )?;
    Ok(())
}

fn validate_artifact_identity_groups(
    roles: &[RolePromotionArtifactIdentityV1],
    groups: &[PromotionArtifactIdentityGroupV1],
) -> Result<(), PromotionArtifactIdentityReportError> {
    let role_names = roles
        .iter()
        .map(|role| role.role.as_str())
        .collect::<BTreeSet<_>>();
    let mut grouped_roles = BTreeSet::new();
    let mut group_keys = BTreeSet::new();
    for group in groups {
        validate_artifact_identity_group(group)?;
        if !group_keys.insert(group.identity_key.as_str()) {
            return Err(
                PromotionArtifactIdentityReportError::DuplicateIdentityGroup {
                    identity_key: group.identity_key.clone(),
                },
            );
        }
        if group.roles.is_empty() {
            return Err(PromotionArtifactIdentityReportError::EmptyIdentityGroup {
                identity_key: group.identity_key.clone(),
            });
        }
        for role in &group.roles {
            if !role_names.contains(role.as_str()) {
                return Err(PromotionArtifactIdentityReportError::UnknownGroupedRole {
                    role: role.clone(),
                });
            }
            if !grouped_roles.insert(role.as_str()) {
                return Err(PromotionArtifactIdentityReportError::DuplicateGroupedRole {
                    role: role.clone(),
                });
            }
            let role_identity = roles
                .iter()
                .find(|candidate| candidate.role == *role)
                .expect("known role should be present");
            let expected = artifact_identity_key_for_role(role_identity);
            if expected != group.identity_key {
                return Err(
                    PromotionArtifactIdentityReportError::IdentityGroupRoleMismatch {
                        role: role.clone(),
                        expected,
                        found: group.identity_key.clone(),
                    },
                );
            }
        }
    }
    for role in roles {
        if !grouped_roles.contains(role.role.as_str()) {
            return Err(PromotionArtifactIdentityReportError::MissingGroupedRole {
                role: role.role.clone(),
            });
        }
    }
    Ok(())
}

fn validate_artifact_identity_summary(
    report: &PromotionArtifactIdentityReportV1,
) -> Result<(), PromotionArtifactIdentityReportError> {
    let expected = promotion_artifact_identity_summary(&report.roles, &report.identity_groups);
    if report.summary.role_count != expected.role_count {
        return Err(PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "role_count",
        });
    }
    if report.summary.identity_group_count != expected.identity_group_count {
        return Err(PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "identity_group_count",
        });
    }
    if report.summary.shared_identity_group_count != expected.shared_identity_group_count {
        return Err(PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "shared_identity_group_count",
        });
    }
    if report.summary.digest_pinned_role_count != expected.digest_pinned_role_count {
        return Err(PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "digest_pinned_role_count",
        });
    }
    if report.summary.source_build_role_count != expected.source_build_role_count {
        return Err(PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "source_build_role_count",
        });
    }
    if report.summary.deferred_identity_role_count != expected.deferred_identity_role_count {
        return Err(PromotionArtifactIdentityReportError::SummaryMismatch {
            field: "deferred_identity_role_count",
        });
    }
    Ok(())
}

fn validate_artifact_identity_group(
    group: &PromotionArtifactIdentityGroupV1,
) -> Result<(), PromotionArtifactIdentityReportError> {
    ensure_identity_report_field("identity_group.identity_key", &group.identity_key)?;
    if group.source_kinds.is_empty() {
        return Err(PromotionArtifactIdentityReportError::MissingRequiredField {
            field: "identity_group.source_kinds",
        });
    }
    ensure_identity_optional_sha256("identity_group.wasm_sha256", group.wasm_sha256.as_deref())?;
    ensure_identity_optional_sha256(
        "identity_group.wasm_gz_sha256",
        group.wasm_gz_sha256.as_deref(),
    )?;
    ensure_identity_optional_sha256(
        "identity_group.candid_sha256",
        group.candid_sha256.as_deref(),
    )?;
    ensure_identity_optional_sha256(
        "identity_group.canonical_embedded_config_sha256",
        group.canonical_embedded_config_sha256.as_deref(),
    )?;
    let expected = artifact_identity_key_for_group(group);
    if expected != group.identity_key {
        return Err(
            PromotionArtifactIdentityReportError::IdentityGroupKeyMismatch {
                expected,
                found: group.identity_key.clone(),
            },
        );
    }
    Ok(())
}

fn validate_materialization_output_groups(
    roles: &[RolePromotionMaterializationIdentityV1],
    groups: &[PromotionMaterializationOutputGroupV1],
) -> Result<(), PromotionMaterializationIdentityReportError> {
    let role_names = roles
        .iter()
        .map(|role| role.role.as_str())
        .collect::<BTreeSet<_>>();
    let mut grouped_roles = BTreeSet::new();
    let mut group_keys = BTreeSet::new();
    for group in groups {
        validate_materialization_output_group(group)?;
        if !group_keys.insert(group.output_identity_key.as_str()) {
            return Err(
                PromotionMaterializationIdentityReportError::DuplicateOutputGroup {
                    output_identity_key: group.output_identity_key.clone(),
                },
            );
        }
        if group.roles.is_empty() {
            return Err(
                PromotionMaterializationIdentityReportError::EmptyOutputGroup {
                    output_identity_key: group.output_identity_key.clone(),
                },
            );
        }
        for role in &group.roles {
            if !role_names.contains(role.as_str()) {
                return Err(
                    PromotionMaterializationIdentityReportError::UnknownGroupedRole {
                        role: role.clone(),
                    },
                );
            }
            if !grouped_roles.insert(role.as_str()) {
                return Err(
                    PromotionMaterializationIdentityReportError::DuplicateGroupedRole {
                        role: role.clone(),
                    },
                );
            }
            let role_identity = roles
                .iter()
                .find(|candidate| candidate.role == *role)
                .expect("known role should be present");
            let expected = materialization_output_key_for_role(role_identity);
            if expected != group.output_identity_key {
                return Err(
                    PromotionMaterializationIdentityReportError::OutputGroupRoleMismatch {
                        role: role.clone(),
                        expected,
                        found: group.output_identity_key.clone(),
                    },
                );
            }
        }
    }
    for role in roles {
        if !grouped_roles.contains(role.role.as_str()) {
            return Err(
                PromotionMaterializationIdentityReportError::MissingGroupedRole {
                    role: role.role.clone(),
                },
            );
        }
    }
    Ok(())
}

fn validate_materialization_output_group(
    group: &PromotionMaterializationOutputGroupV1,
) -> Result<(), PromotionMaterializationIdentityReportError> {
    ensure_materialization_report_field(
        "output_group.output_identity_key",
        &group.output_identity_key,
    )?;
    ensure_materialization_report_sha256("output_group.wasm_sha256", &group.wasm_sha256)?;
    ensure_materialization_report_sha256("output_group.wasm_gz_sha256", &group.wasm_gz_sha256)?;
    ensure_materialization_report_sha256(
        "output_group.installed_module_hash",
        &group.installed_module_hash,
    )?;
    ensure_materialization_report_sha256("output_group.candid_sha256", &group.candid_sha256)?;
    let expected = materialization_output_key_for_group(group);
    if expected != group.output_identity_key {
        return Err(
            PromotionMaterializationIdentityReportError::OutputGroupKeyMismatch {
                expected,
                found: group.output_identity_key.clone(),
            },
        );
    }
    Ok(())
}

fn validate_role_plan_transform(
    role: &RolePromotionPlanTransformV1,
    promoted_plan: &DeploymentPlanV1,
) -> Result<(), PromotionPlanTransformError> {
    ensure_transform_field("role", &role.role)?;
    let Some(promoted_role) = promoted_plan
        .role_artifacts
        .iter()
        .find(|artifact| artifact.role == role.role)
    else {
        return Err(PromotionPlanTransformError::PromotedRoleMissing {
            role: role.role.clone(),
        });
    };
    ensure_role_matches_promoted_artifact(role, promoted_role)?;
    ensure_role_transform_flags_are_consistent(role)?;
    validate_role_materialization_link(role, promoted_role)?;
    Ok(())
}

fn ensure_role_matches_promoted_artifact(
    role: &RolePromotionPlanTransformV1,
    promoted_role: &RoleArtifactV1,
) -> Result<(), PromotionPlanTransformError> {
    ensure_role_field_matches(
        role,
        "artifact_source_after",
        role.artifact_source_after == promoted_role.source,
    )?;
    ensure_role_field_matches(
        role,
        "wasm_sha256_after",
        role.wasm_sha256_after == promoted_role.wasm_sha256,
    )?;
    ensure_role_field_matches(
        role,
        "wasm_gz_sha256_after",
        role.wasm_gz_sha256_after == promoted_role.wasm_gz_sha256,
    )?;
    ensure_role_field_matches(
        role,
        "candid_sha256_after",
        role.candid_sha256_after == promoted_role.candid_sha256,
    )?;
    ensure_role_field_matches(
        role,
        "canonical_embedded_config_sha256_after",
        role.canonical_embedded_config_sha256_after
            == promoted_role.canonical_embedded_config_sha256,
    )
}

fn ensure_role_transform_flags_are_consistent(
    role: &RolePromotionPlanTransformV1,
) -> Result<(), PromotionPlanTransformError> {
    ensure_role_field_matches(
        role,
        "artifact_identity_changed",
        role.artifact_identity_changed == role_summary_artifact_identity_changed(role),
    )?;
    ensure_role_field_matches(
        role,
        "embedded_config_changed",
        role.embedded_config_changed
            == (role.canonical_embedded_config_sha256_before
                != role.canonical_embedded_config_sha256_after),
    )?;
    if role.target_materialization_preserved {
        ensure_role_field_matches(
            role,
            "target_materialization_preserved",
            role.promotion_level == PromotionArtifactLevelV1::SourceBuild
                && !role.artifact_identity_changed
                && !role.embedded_config_changed,
        )?;
    }
    Ok(())
}

fn validate_role_materialization_link(
    role: &RolePromotionPlanTransformV1,
    promoted_role: &RoleArtifactV1,
) -> Result<(), PromotionPlanTransformError> {
    let Some(link) = &role.source_build_materialization else {
        return Ok(());
    };
    ensure_role_field_matches(
        role,
        "source_build_materialization",
        role.promotion_level == PromotionArtifactLevelV1::SourceBuild,
    )?;
    ensure_role_field_matches(
        role,
        "source_build_materialization.role",
        link.role == role.role,
    )?;
    ensure_transform_field(
        "source_build_materialization.evidence_id",
        &link.evidence_id,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.materialization_evidence_digest",
        &link.materialization_evidence_digest,
    )?;
    ensure_transform_field("source_build_materialization.recipe_id", &link.recipe_id)?;
    ensure_transform_field(
        "source_build_materialization.materialization_input_id",
        &link.materialization_input_id,
    )?;
    ensure_transform_field(
        "source_build_materialization.materialization_result_id",
        &link.materialization_result_id,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.materialization_input_digest",
        &link.materialization_input_digest,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.wasm_sha256",
        &link.wasm_sha256,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.wasm_gz_sha256",
        &link.wasm_gz_sha256,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.installed_module_hash",
        &link.installed_module_hash,
    )?;
    ensure_materialization_sha256(
        "source_build_materialization.candid_sha256",
        &link.candid_sha256,
    )?;
    ensure_role_field_matches(
        role,
        "source_build_materialization.wasm_sha256",
        promoted_role.wasm_sha256.as_deref() == Some(link.wasm_sha256.as_str()),
    )?;
    ensure_role_field_matches(
        role,
        "source_build_materialization.wasm_gz_sha256",
        promoted_role.wasm_gz_sha256.as_deref() == Some(link.wasm_gz_sha256.as_str()),
    )?;
    ensure_role_field_matches(
        role,
        "source_build_materialization.installed_module_hash",
        promoted_role.installed_module_hash.as_deref() == Some(link.installed_module_hash.as_str()),
    )?;
    ensure_role_field_matches(
        role,
        "source_build_materialization.candid_sha256",
        promoted_role.candid_sha256.as_deref() == Some(link.candid_sha256.as_str()),
    )
}

fn ensure_role_field_matches(
    role: &RolePromotionPlanTransformV1,
    field: &'static str,
    matches: bool,
) -> Result<(), PromotionPlanTransformError> {
    if matches {
        Ok(())
    } else {
        Err(PromotionPlanTransformError::RoleStateMismatch {
            role: role.role.clone(),
            field,
        })
    }
}

fn validate_role_readiness(role: &RolePromotionReadinessV1) -> Result<(), PromotionReadinessError> {
    ensure_readiness_field("role", &role.role)?;
    ensure_readiness_optional_sha256("source_wasm_sha256", role.source_wasm_sha256.as_deref())?;
    ensure_readiness_optional_sha256(
        "source_wasm_gz_sha256",
        role.source_wasm_gz_sha256.as_deref(),
    )?;
    ensure_readiness_optional_sha256("target_wasm_sha256", role.target_wasm_sha256.as_deref())?;
    ensure_readiness_optional_sha256(
        "target_wasm_gz_sha256",
        role.target_wasm_gz_sha256.as_deref(),
    )?;
    ensure_readiness_optional_sha256(
        "source_canonical_embedded_config_sha256",
        role.source_canonical_embedded_config_sha256.as_deref(),
    )?;
    ensure_readiness_optional_sha256(
        "target_canonical_embedded_config_sha256",
        role.target_canonical_embedded_config_sha256.as_deref(),
    )?;
    if role.restage_required != (role.target_store_has_artifact == Some(false)) {
        return Err(PromotionReadinessError::RestageStateMismatch {
            role: role.role.clone(),
        });
    }
    Ok(())
}

fn role_promotion_readiness(
    input: &RolePromotionInputV1,
    target_artifact: &RoleArtifactV1,
) -> RolePromotionReadinessV1 {
    let source_wasm_sha256 = input.source.expected_wasm_sha256.clone();
    let source_wasm_gz_sha256 = input.source.expected_wasm_gz_sha256.clone();
    let target_wasm_sha256 = target_artifact.wasm_sha256.clone();
    let target_wasm_gz_sha256 = target_artifact.wasm_gz_sha256.clone();
    let byte_identical_wasm =
        matching_optional_digest(source_wasm_sha256.as_ref(), target_wasm_sha256.as_ref()).or_else(
            || {
                matching_optional_digest(
                    source_wasm_gz_sha256.as_ref(),
                    target_wasm_gz_sha256.as_ref(),
                )
            },
        );
    let embedded_config_identical = matching_optional_digest(
        input
            .source
            .expected_canonical_embedded_config_sha256
            .as_ref(),
        target_artifact.canonical_embedded_config_sha256.as_ref(),
    );

    RolePromotionReadinessV1 {
        role: input.role.clone(),
        promotion_level: input.promotion_level,
        source_kind: input.source.kind,
        source_locator: input.source.locator.clone(),
        source_wasm_sha256,
        source_wasm_gz_sha256,
        target_wasm_sha256,
        target_wasm_gz_sha256,
        source_canonical_embedded_config_sha256: input
            .source
            .expected_canonical_embedded_config_sha256
            .clone(),
        target_canonical_embedded_config_sha256: target_artifact
            .canonical_embedded_config_sha256
            .clone(),
        byte_identical_wasm,
        embedded_config_identical,
        target_store_has_artifact: input.target_store_has_artifact,
        restage_required: input.target_store_has_artifact == Some(false),
    }
}

fn collect_role_findings(
    input: &RolePromotionInputV1,
    readiness: &RolePromotionReadinessV1,
    blockers: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    if let Err(err) = validate_role_artifact_source(&input.source) {
        blockers.push(promotion_finding(
            "promotion_artifact_source_invalid",
            err.to_string(),
            SafetySeverityV1::HardFailure,
            &input.role,
        ));
    }

    if input.role != input.source.role {
        blockers.push(promotion_finding(
            "promotion_source_role_mismatch",
            format!(
                "promotion input role {} does not match artifact source role {}",
                input.role, input.source.role
            ),
            SafetySeverityV1::HardFailure,
            &input.role,
        ));
    }

    if input.require_byte_identical_wasm && readiness.byte_identical_wasm != Some(true) {
        blockers.push(promotion_finding(
            "promotion_wasm_digest_mismatch",
            "promotion requires byte-identical wasm but source and target digests differ or are incomplete",
            SafetySeverityV1::HardFailure,
            &input.role,
        ));
    }

    if input.require_target_embedded_config
        && readiness
            .target_canonical_embedded_config_sha256
            .as_deref()
            .is_none_or(str::is_empty)
    {
        blockers.push(promotion_finding(
            "promotion_target_embedded_config_missing",
            "promotion requires target canonical embedded config but target plan has no digest",
            SafetySeverityV1::HardFailure,
            &input.role,
        ));
    }

    if input.promotion_level == PromotionArtifactLevelV1::SealedWasm
        && readiness.embedded_config_identical != Some(true)
    {
        blockers.push(promotion_finding(
            "promotion_sealed_wasm_embedded_config_mismatch",
            "sealed wasm promotion requires embedded config identity to be acceptable for the target",
            SafetySeverityV1::HardFailure,
            &input.role,
        ));
    }

    if readiness.restage_required {
        warnings.push(promotion_finding(
            "promotion_target_store_restage_required",
            "target artifact store does not already contain the artifact; restaging is required",
            SafetySeverityV1::Warning,
            &input.role,
        ));
    }
}

fn matching_optional_digest(left: Option<&String>, right: Option<&String>) -> Option<bool> {
    match (left.map(String::as_str), right.map(String::as_str)) {
        (Some(left), Some(right)) => Some(left == right),
        _ => None,
    }
}

fn promotion_finding(
    code: impl Into<String>,
    message: impl Into<String>,
    severity: SafetySeverityV1,
    role: &str,
) -> SafetyFindingV1 {
    SafetyFindingV1 {
        code: code.into(),
        message: message.into(),
        severity,
        subject: Some(role.to_string()),
    }
}

const fn ensure_readiness_status_matches_blockers(
    readiness: &PromotionReadinessV1,
) -> Result<(), PromotionReadinessError> {
    match (readiness.status, readiness.blockers.is_empty()) {
        (PromotionReadinessStatusV1::Ready, false)
        | (PromotionReadinessStatusV1::Blocked, true) => {
            Err(PromotionReadinessError::StatusBlockerMismatch {
                status: readiness.status,
                blocker_count: readiness.blockers.len(),
            })
        }
        _ => Ok(()),
    }
}

fn ensure_unique_readiness_roles(
    roles: &[RolePromotionReadinessV1],
) -> Result<(), PromotionReadinessError> {
    let mut seen = std::collections::BTreeSet::new();
    for role in roles {
        if !seen.insert(role.role.as_str()) {
            return Err(PromotionReadinessError::DuplicateRole {
                role: role.role.clone(),
            });
        }
    }
    Ok(())
}

fn ensure_unique_transform_roles(
    roles: &[RolePromotionPlanTransformV1],
) -> Result<(), PromotionPlanTransformError> {
    let mut seen = std::collections::BTreeSet::new();
    for role in roles {
        if !seen.insert(role.role.as_str()) {
            return Err(PromotionPlanTransformError::DuplicateRole {
                role: role.role.clone(),
            });
        }
    }
    Ok(())
}

const fn ensure_identity_report_status_matches_blockers(
    report: &PromotionArtifactIdentityReportV1,
) -> Result<(), PromotionArtifactIdentityReportError> {
    match (report.status, report.blockers.is_empty()) {
        (PromotionReadinessStatusV1::Ready, false)
        | (PromotionReadinessStatusV1::Blocked, true) => Err(
            PromotionArtifactIdentityReportError::StatusBlockerMismatch {
                status: report.status,
                blocker_count: report.blockers.len(),
            },
        ),
        _ => Ok(()),
    }
}

fn ensure_unique_artifact_identity_roles(
    roles: &[RolePromotionArtifactIdentityV1],
) -> Result<(), PromotionArtifactIdentityReportError> {
    let mut seen = std::collections::BTreeSet::new();
    for role in roles {
        if !seen.insert(role.role.as_str()) {
            return Err(PromotionArtifactIdentityReportError::DuplicateRole {
                role: role.role.clone(),
            });
        }
    }
    Ok(())
}

fn validate_identity_report_blockers(
    blockers: &[SafetyFindingV1],
) -> Result<(), PromotionArtifactIdentityReportError> {
    for blocker in blockers {
        ensure_identity_report_field("blocker.code", &blocker.code)?;
        ensure_identity_report_field("blocker.message", &blocker.message)?;
        if blocker.severity != SafetySeverityV1::HardFailure {
            return Err(
                PromotionArtifactIdentityReportError::BlockerSeverityMismatch {
                    severity: blocker.severity,
                },
            );
        }
    }
    Ok(())
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

const fn ensure_materialization_report_status_matches_blockers(
    report: &PromotionMaterializationIdentityReportV1,
) -> Result<(), PromotionMaterializationIdentityReportError> {
    match (report.status, report.blockers.is_empty()) {
        (PromotionReadinessStatusV1::Ready, false)
        | (PromotionReadinessStatusV1::Blocked, true) => Err(
            PromotionMaterializationIdentityReportError::StatusBlockerMismatch {
                status: report.status,
                blocker_count: report.blockers.len(),
            },
        ),
        _ => Ok(()),
    }
}

fn ensure_unique_materialization_report_roles(
    roles: &[RolePromotionMaterializationIdentityV1],
) -> Result<(), PromotionMaterializationIdentityReportError> {
    let mut seen_roles = BTreeSet::new();
    let mut seen_evidence = BTreeSet::new();
    for role in roles {
        if !seen_roles.insert(role.role.as_str()) {
            return Err(PromotionMaterializationIdentityReportError::DuplicateRole {
                role: role.role.clone(),
            });
        }
        if !seen_evidence.insert(role.evidence_id.as_str()) {
            return Err(
                PromotionMaterializationIdentityReportError::DuplicateEvidence {
                    evidence_id: role.evidence_id.clone(),
                },
            );
        }
    }
    Ok(())
}

fn validate_role_materialization_identity(
    role: &RolePromotionMaterializationIdentityV1,
) -> Result<(), PromotionMaterializationIdentityReportError> {
    ensure_materialization_report_field("role", &role.role)?;
    ensure_materialization_report_field("evidence_id", &role.evidence_id)?;
    ensure_materialization_report_sha256(
        "materialization_evidence_digest",
        &role.materialization_evidence_digest,
    )?;
    ensure_materialization_report_field("recipe_id", &role.recipe_id)?;
    ensure_materialization_report_field(
        "materialization_input_id",
        &role.materialization_input_id,
    )?;
    ensure_materialization_report_field(
        "materialization_result_id",
        &role.materialization_result_id,
    )?;
    ensure_materialization_report_sha256(
        "materialization_input_digest",
        &role.materialization_input_digest,
    )?;
    ensure_materialization_report_sha256(
        "canonical_embedded_config_sha256",
        &role.canonical_embedded_config_sha256,
    )?;
    ensure_materialization_report_field("network", &role.network)?;
    ensure_materialization_report_field("root_trust_anchor", &role.root_trust_anchor)?;
    ensure_materialization_report_field("runtime_variant", &role.runtime_variant)?;
    ensure_materialization_report_sha256("wasm_sha256", &role.wasm_sha256)?;
    ensure_materialization_report_sha256("wasm_gz_sha256", &role.wasm_gz_sha256)?;
    ensure_materialization_report_sha256("installed_module_hash", &role.installed_module_hash)?;
    ensure_materialization_report_sha256("candid_sha256", &role.candid_sha256)?;
    Ok(())
}

fn validate_materialization_report_blockers(
    blockers: &[SafetyFindingV1],
) -> Result<(), PromotionMaterializationIdentityReportError> {
    for blocker in blockers {
        ensure_materialization_report_field("blocker.code", &blocker.code)?;
        ensure_materialization_report_field("blocker.message", &blocker.message)?;
        if blocker.severity != SafetySeverityV1::HardFailure {
            return Err(
                PromotionMaterializationIdentityReportError::BlockerSeverityMismatch {
                    severity: blocker.severity,
                },
            );
        }
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

fn ensure_execution_receipt_linkage(
    receipt: &ArtifactPromotionExecutionReceiptV1,
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    if receipt.deployment_receipt.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            ArtifactPromotionExecutionReceiptError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: receipt.deployment_receipt.schema_version,
            },
        );
    }
    if receipt.deployment_receipt.plan_id != receipt.promoted_plan_id {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "deployment_receipt.plan_id",
        });
    }
    if receipt.deployment_receipt.operation_id != receipt.operation_id {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "operation_id",
        });
    }
    if receipt.deployment_receipt.operation_status != receipt.operation_status {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "operation_status",
        });
    }
    if receipt.deployment_receipt.command_result != receipt.command_result {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "command_result",
        });
    }
    if receipt.deployment_receipt.started_at != receipt.started_at {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "started_at",
        });
    }
    if receipt.deployment_receipt.finished_at != receipt.finished_at {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "finished_at",
        });
    }
    ensure_execution_receipt_roles_match_deployment_receipt(
        &receipt.roles,
        &receipt.deployment_receipt,
    )?;
    ensure_unique_execution_receipt_roles(&receipt.roles)
}

const fn ensure_execution_receipt_provenance_ready(
    status: PromotionReadinessStatusV1,
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    if matches!(status, PromotionReadinessStatusV1::Ready) {
        Ok(())
    } else {
        Err(ArtifactPromotionExecutionReceiptError::ProvenanceNotReady { status })
    }
}

fn ensure_execution_receipt_roles_match_deployment_receipt(
    roles: &[RolePromotionExecutionReceiptV1],
    deployment_receipt: &DeploymentReceiptV1,
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    let promotion_roles = roles
        .iter()
        .map(|role| role.role.as_str())
        .collect::<BTreeSet<_>>();
    let deployment_roles = deployment_receipt
        .role_phase_receipts
        .iter()
        .map(|receipt| receipt.role.as_str())
        .collect::<BTreeSet<_>>();
    for role in &promotion_roles {
        if !deployment_roles.contains(role) {
            return Err(
                ArtifactPromotionExecutionReceiptError::MissingDeploymentRole {
                    role: (*role).to_string(),
                },
            );
        }
    }
    for role in &deployment_roles {
        if !promotion_roles.contains(role) {
            return Err(
                ArtifactPromotionExecutionReceiptError::UnknownDeploymentRole {
                    role: (*role).to_string(),
                },
            );
        }
    }
    for role in roles {
        let role_receipt = deployment_receipt
            .role_phase_receipts
            .iter()
            .rev()
            .find(|receipt| receipt.role == role.role);
        if role.role_phase_result != role_receipt.map(|receipt| receipt.result) {
            return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
                field: "role_phase_result",
            });
        }
        if role.artifact_digest != role_receipt.and_then(|receipt| receipt.artifact_digest.clone())
        {
            return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
                field: "artifact_digest",
            });
        }
        if role.observed_module_hash_after
            != role_receipt.and_then(|receipt| receipt.observed_module_hash_after.clone())
        {
            return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
                field: "observed_module_hash_after",
            });
        }
        if role.canonical_embedded_config_sha256
            != role_receipt.and_then(|receipt| receipt.canonical_embedded_config_sha256.clone())
        {
            return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
                field: "canonical_embedded_config_sha256",
            });
        }
    }
    Ok(())
}

fn ensure_unique_execution_receipt_roles(
    roles: &[RolePromotionExecutionReceiptV1],
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    let mut seen = BTreeSet::new();
    for role in roles {
        ensure_execution_receipt_field("role", &role.role)?;
        if !seen.insert(role.role.as_str()) {
            return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch { field: "roles" });
        }
        if let Some(evidence_id) = &role.materialization_evidence_id {
            ensure_execution_receipt_field("materialization_evidence_id", evidence_id)?;
        }
        if let Some(digest) = &role.materialization_evidence_digest {
            ensure_execution_receipt_sha256("materialization_evidence_digest", digest)?;
        }
        if let Some(locator) = &role.wasm_store_locator {
            ensure_execution_receipt_field("wasm_store_locator", locator)?;
        }
        if let Some(digest) = &role.wasm_store_catalog_observation_digest {
            ensure_execution_receipt_sha256("wasm_store_catalog_observation_digest", digest)?;
        }
        if let Some(digest) = &role.artifact_digest {
            ensure_execution_receipt_field("artifact_digest", digest)?;
        }
        if let Some(hash) = &role.observed_module_hash_after {
            ensure_execution_receipt_field("observed_module_hash_after", hash)?;
        }
        if let Some(digest) = &role.canonical_embedded_config_sha256 {
            ensure_execution_receipt_field("canonical_embedded_config_sha256", digest)?;
        }
    }
    Ok(())
}

fn validate_readiness_findings(
    field: &'static str,
    findings: &[SafetyFindingV1],
    expected_severity: SafetySeverityV1,
) -> Result<(), PromotionReadinessError> {
    for finding in findings {
        ensure_readiness_field("finding.code", &finding.code)?;
        ensure_readiness_field("finding.message", &finding.message)?;
        if finding.severity != expected_severity {
            return Err(PromotionReadinessError::FindingSeverityMismatch {
                field,
                severity: finding.severity,
            });
        }
    }
    Ok(())
}

const fn ensure_artifact_promotion_status_matches_blockers(
    plan: &ArtifactPromotionPlanV1,
) -> Result<(), ArtifactPromotionPlanError> {
    match (plan.status, plan.blockers.is_empty()) {
        (PromotionReadinessStatusV1::Ready, false)
        | (PromotionReadinessStatusV1::Blocked, true) => {
            Err(ArtifactPromotionPlanError::StatusBlockerMismatch {
                status: plan.status,
                blocker_count: plan.blockers.len(),
            })
        }
        _ => Ok(()),
    }
}

fn ensure_artifact_promotion_plan_linkage(
    plan: &ArtifactPromotionPlanV1,
) -> Result<(), ArtifactPromotionPlanError> {
    let expected_blockers =
        artifact_promotion_plan_blockers(&plan.readiness, &plan.artifact_identity_report);
    if expected_blockers != plan.blockers {
        return Err(ArtifactPromotionPlanError::LinkageMismatch { field: "blockers" });
    }
    if plan.readiness.target_plan_id != plan.target_plan_id {
        return Err(ArtifactPromotionPlanError::LinkageMismatch {
            field: "readiness.target_plan_id",
        });
    }
    if plan.transform.target_plan_id != plan.target_plan_id {
        return Err(ArtifactPromotionPlanError::LinkageMismatch {
            field: "transform.target_plan_id",
        });
    }
    if plan.transform.promoted_plan_id != plan.promoted_plan_id {
        return Err(ArtifactPromotionPlanError::LinkageMismatch {
            field: "transform.promoted_plan_id",
        });
    }
    if plan.transform.promotion_plan_lineage_digest != plan.promotion_plan_lineage_digest {
        return Err(ArtifactPromotionPlanError::LinkageMismatch {
            field: "promotion_plan_lineage_digest",
        });
    }
    Ok(())
}
