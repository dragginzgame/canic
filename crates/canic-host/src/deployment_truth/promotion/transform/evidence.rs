use super::super::{
    digest::promotion_plan_transform_evidence_digest,
    ensure::{ensure_evidence_field, ensure_evidence_sha256},
    error::PromotionPlanTransformEvidenceError,
    request::PromotionPlanTransformEvidenceRequest,
};
use super::plan::validate_promotion_plan_transform;
use crate::deployment_truth::{DEPLOYMENT_TRUTH_SCHEMA_VERSION, PromotionPlanTransformEvidenceV1};

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
