use crate::deployment_truth::ArtifactPromotionProvenanceReportV1;

use super::super::super::{
    ensure::ensure_provenance_report_field, error::ArtifactPromotionProvenanceReportError,
    request::ArtifactPromotionProvenanceReportRequest,
};
use super::build::build_artifact_promotion_provenance_report;
use super::validation::validate_artifact_promotion_provenance_report;

pub fn artifact_promotion_provenance_report(
    request: ArtifactPromotionProvenanceReportRequest,
) -> Result<ArtifactPromotionProvenanceReportV1, ArtifactPromotionProvenanceReportError> {
    ensure_provenance_report_field("report_id", &request.report_id)?;
    super::super::super::validate_artifact_promotion_plan(&request.artifact_promotion_plan)?;
    if let Some(report) = &request.wasm_store_identity_report {
        super::super::super::validate_promotion_wasm_store_identity_report(report)?;
    }
    if let Some(verification) = &request.wasm_store_catalog_verification {
        super::super::super::validate_promotion_wasm_store_catalog_verification(verification)?;
    }
    if let Some(report) = &request.materialization_identity_report {
        super::super::super::validate_promotion_materialization_identity_report(report)?;
    }
    let report = build_artifact_promotion_provenance_report(request);
    validate_artifact_promotion_provenance_report(&report)?;
    Ok(report)
}
