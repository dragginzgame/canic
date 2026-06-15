use super::super::*;
use super::shared::sorted_unique;

pub(super) fn authority_profile_overlap_findings(plan: &DeploymentPlanV1) -> Vec<SafetyFindingV1> {
    let expected = sorted_unique(plan.authority_profile.expected_controllers.clone());
    let staging = authority_category_overlaps(
        "staging",
        &expected,
        &plan.authority_profile.staging_controllers,
    );
    let emergency = authority_category_overlaps(
        "emergency",
        &expected,
        &plan.authority_profile.emergency_controllers,
    );

    staging.into_iter().chain(emergency).collect()
}

fn authority_category_overlaps(
    category: &str,
    expected_controllers: &[String],
    category_controllers: &[String],
) -> Vec<SafetyFindingV1> {
    let overlaps = sorted_unique(
        category_controllers
            .iter()
            .filter(|controller| {
                expected_controllers
                    .iter()
                    .any(|expected| expected == *controller)
            })
            .cloned()
            .collect(),
    );

    overlaps
        .into_iter()
        .map(|principal| SafetyFindingV1 {
            code: "authority_profile_overlap".to_string(),
            message: format!(
                "{category} authority principal {principal} overlaps the normal expected controller set"
            ),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(principal),
        })
        .collect()
}
