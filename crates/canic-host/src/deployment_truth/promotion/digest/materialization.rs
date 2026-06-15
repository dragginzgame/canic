use crate::deployment_truth::{
    BuildMaterializationEvidenceV1, BuildMaterializationInputV1, BuildMaterializationResultV1,
    BuildRecipeIdentityV1, PromotionMaterializationIdentityReportV1,
    PromotionMaterializationOutputGroupV1, PromotionReadinessStatusV1,
    RolePromotionMaterializationIdentityV1, SafetyFindingV1, stable_json_sha256_hex,
};
use serde::Serialize;

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

#[must_use]
pub fn build_materialization_input_digest(input: &BuildMaterializationInputV1) -> String {
    stable_json_sha256_hex(input)
}

pub(in crate::deployment_truth::promotion) fn build_materialization_evidence_digest(
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

pub(in crate::deployment_truth::promotion) fn promotion_materialization_identity_report_digest(
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
