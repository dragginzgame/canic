use super::super::*;
use super::{diff_item, finding};

pub(in crate::deployment_truth) const INSTALLED_MODULE_HASH_DIFF_CATEGORY: &str =
    "installed_module_hash";
pub(in crate::deployment_truth) const INSTALLED_MODULE_HASH_MISMATCH_CODE: &str =
    "installed_module_hash_mismatch";
const INSTALLED_MODULE_HASH_UNOBSERVED_CODE: &str = "installed_module_hash_unobserved";
pub(in crate::deployment_truth) const INSTALLED_MODULE_HASH_AMBIGUOUS_DIFF_CATEGORY: &str =
    "installed_module_hash_ambiguous";
pub(in crate::deployment_truth) const INSTALLED_MODULE_HASH_AMBIGUOUS_CODE: &str =
    "installed_module_hash_ambiguous";

pub(super) fn compare_module_hashes(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
    module_hash_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    for artifact in &plan.role_artifacts {
        let Some(expected) = artifact.installed_module_hash.as_ref() else {
            continue;
        };
        let Some(observed_canister) = observed_canister_for_module_hash(
            plan,
            inventory,
            &artifact.role,
            module_hash_diff,
            hard_failures,
        ) else {
            continue;
        };
        match observed_canister.module_hash.as_ref() {
            Some(observed) if observed != expected => record_module_hash_mismatch(
                &artifact.role,
                expected,
                observed,
                module_hash_diff,
                hard_failures,
            ),
            None => record_module_hash_unobserved(&artifact.role, warnings),
            _ => {}
        }
    }
}

fn observed_canister_for_module_hash<'a>(
    plan: &DeploymentPlanV1,
    inventory: &'a DeploymentInventoryV1,
    role: &str,
    module_hash_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) -> Option<&'a ObservedCanisterV1> {
    if let Some(expected_id) = expected_canister_id_for_role(plan, role) {
        return inventory
            .observed_canisters
            .iter()
            .find(|canister| canister.canister_id == expected_id);
    }

    let role_matches = inventory
        .observed_canisters
        .iter()
        .filter(|canister| canister.role.as_deref() == Some(role))
        .collect::<Vec<_>>();
    if role_matches.len() > 1 {
        record_ambiguous_module_hash_role(role, &role_matches, module_hash_diff, hard_failures);
        return None;
    }

    role_matches.into_iter().next()
}

fn record_module_hash_mismatch(
    role: &str,
    expected: &str,
    observed: &str,
    module_hash_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    module_hash_diff.push(diff_item(
        INSTALLED_MODULE_HASH_DIFF_CATEGORY,
        role,
        Some(expected.to_string()),
        Some(observed.to_string()),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        INSTALLED_MODULE_HASH_MISMATCH_CODE,
        format!("installed module hash differs for role {role}"),
        SafetySeverityV1::HardFailure,
        Some(role.to_string()),
    ));
}

fn record_module_hash_unobserved(role: &str, warnings: &mut Vec<SafetyFindingV1>) {
    warnings.push(finding(
        INSTALLED_MODULE_HASH_UNOBSERVED_CODE,
        format!("installed module hash was not observed for role {role}"),
        SafetySeverityV1::Warning,
        Some(role.to_string()),
    ));
}

fn record_ambiguous_module_hash_role(
    role: &str,
    role_matches: &[&ObservedCanisterV1],
    module_hash_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    let observed_ids = role_matches
        .iter()
        .map(|canister| canister.canister_id.as_str())
        .collect::<Vec<_>>()
        .join(",");
    module_hash_diff.push(diff_item(
        INSTALLED_MODULE_HASH_AMBIGUOUS_DIFF_CATEGORY,
        role,
        Some("one observed canister".to_string()),
        Some(observed_ids.clone()),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        INSTALLED_MODULE_HASH_AMBIGUOUS_CODE,
        format!(
            "installed module hash for role {role} has multiple observed canisters: {observed_ids}"
        ),
        SafetySeverityV1::HardFailure,
        Some(role.to_string()),
    ));
}

fn expected_canister_id_for_role<'a>(plan: &'a DeploymentPlanV1, role: &str) -> Option<&'a str> {
    plan.expected_canisters
        .iter()
        .find(|canister| canister.role == role)
        .and_then(|canister| canister.canister_id.as_deref())
}
