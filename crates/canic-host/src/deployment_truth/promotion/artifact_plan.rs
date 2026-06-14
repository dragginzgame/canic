use super::super::executor::{
    validate_deployment_execution_preflight, validate_deployment_execution_preflight_for_check,
};
use super::super::{
    ArtifactPromotionPlanV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION, DeploymentCheckV1,
    PromotionArtifactIdentityReportV1, PromotionReadinessStatusV1, PromotionReadinessV1,
    PromotionTargetExecutionLineageV1, SafetyFindingV1,
};
use super::digest::{artifact_promotion_plan_digest, promotion_target_execution_lineage_digest};
use super::ensure::{
    ensure_artifact_promotion_plan_field, ensure_artifact_promotion_plan_sha256,
    ensure_target_execution_lineage_field, ensure_target_execution_lineage_sha256,
};
use super::error::{ArtifactPromotionPlanError, PromotionTargetExecutionLineageError};
use super::request::{ArtifactPromotionPlanRequest, PromotionTargetExecutionLineageRequest};

pub fn artifact_promotion_plan(
    request: ArtifactPromotionPlanRequest,
) -> Result<ArtifactPromotionPlanV1, ArtifactPromotionPlanError> {
    ensure_artifact_promotion_plan_field("plan_id", &request.plan_id)?;
    ensure_artifact_promotion_plan_field("generated_at", &request.generated_at)?;
    super::validate_promotion_readiness(&request.readiness)?;
    super::validate_promotion_artifact_identity_report(&request.artifact_identity_report)?;
    super::validate_promotion_plan_transform(&request.transform)?;
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
    super::validate_promotion_plan_transform(&request.transform)?;
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
    super::validate_promotion_readiness(&plan.readiness)?;
    super::validate_promotion_artifact_identity_report(&plan.artifact_identity_report)?;
    super::validate_promotion_plan_transform(&plan.transform)?;
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
    super::validate_promotion_plan_transform(&lineage.transform)?;
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
