use super::super::super::{
    digest::artifact_promotion_execution_receipt_digest,
    ensure::{ensure_execution_receipt_field, ensure_execution_receipt_sha256},
    error::ArtifactPromotionExecutionReceiptError,
};
use super::linkage::{ensure_execution_receipt_linkage, ensure_execution_receipt_provenance_ready};
use crate::deployment_truth::{
    ArtifactPromotionExecutionReceiptV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION,
};

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
