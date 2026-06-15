use super::group::{promotion_artifact_identity_groups, promotion_artifact_identity_summary};
use super::roles::role_promotion_artifact_identity;
use super::validation::validate_promotion_artifact_identity_report;
use crate::deployment_truth::{
    DEPLOYMENT_TRUTH_SCHEMA_VERSION, PromotionArtifactIdentityReportV1, PromotionReadinessStatusV1,
    RolePromotionInputV1, SafetySeverityV1,
};

use super::super::digest::promotion_artifact_identity_report_digest;
use super::super::ensure::ensure_identity_report_field;
use super::super::error::PromotionArtifactIdentityReportError;
use super::super::request::PromotionArtifactIdentityReportRequest;

pub fn promotion_artifact_identity_report_from_inputs(
    request: PromotionArtifactIdentityReportRequest,
) -> Result<PromotionArtifactIdentityReportV1, PromotionArtifactIdentityReportError> {
    ensure_identity_report_field("report_id", &request.report_id)?;
    let report = promotion_artifact_identity_report(&request.report_id, &request.inputs);
    validate_promotion_artifact_identity_report(&report)?;
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
        if let Err(err) = super::super::transform::validate_role_artifact_source(&input.source) {
            blockers.push(super::super::promotion_finding(
                "promotion_artifact_source_invalid",
                err.to_string(),
                SafetySeverityV1::HardFailure,
                &input.role,
            ));
        }
        if input.role != input.source.role {
            blockers.push(super::super::promotion_finding(
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
