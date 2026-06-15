use crate::deployment_truth::{
    BuildMaterializationEvidenceV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION,
    PromotionMaterializationIdentityReportV1, PromotionReadinessStatusV1,
};

use super::super::digest::promotion_materialization_identity_report_digest;
use super::super::ensure::ensure_materialization_report_field;
use super::super::error::PromotionMaterializationIdentityReportError;
use super::super::identity::{
    promotion_materialization_output_groups, role_materialization_identity_from_evidence,
};
use super::super::request::PromotionMaterializationIdentityReportRequest;
use super::evidence::validate_build_materialization_evidence;
use super::validation::validate_promotion_materialization_identity_report;

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
