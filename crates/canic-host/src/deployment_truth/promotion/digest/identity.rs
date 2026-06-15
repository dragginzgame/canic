use crate::deployment_truth::{
    PromotionArtifactIdentityGroupV1, PromotionArtifactIdentityReportV1,
    PromotionArtifactIdentitySummaryV1, PromotionReadinessStatusV1,
    RolePromotionArtifactIdentityV1, SafetyFindingV1, stable_json_sha256_hex,
};
use serde::Serialize;

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

pub(in crate::deployment_truth::promotion) fn promotion_artifact_identity_report_digest(
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
