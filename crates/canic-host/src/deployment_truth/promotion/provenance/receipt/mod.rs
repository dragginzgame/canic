mod build;
mod linkage;
mod validation;

use super::super::{
    ensure::ensure_execution_receipt_field, error::ArtifactPromotionExecutionReceiptError,
    request::ArtifactPromotionExecutionReceiptRequest,
};
use super::report::validate_artifact_promotion_provenance_report;
use crate::deployment_truth::ArtifactPromotionExecutionReceiptV1;

use build::build_artifact_promotion_execution_receipt;
use linkage::{
    ensure_execution_receipt_provenance_ready, validate_deployment_receipt_for_promotion,
};

pub use validation::validate_artifact_promotion_execution_receipt;

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
