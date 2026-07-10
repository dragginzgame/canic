use super::super::*;
use super::{diff_item, duplicate_evidence_groups, finding};
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::deployment_truth) const PLANNED_ARTIFACT_ROLE_CONFLICT_CODE: &str =
    "planned_artifact_role_conflict";
pub(in crate::deployment_truth) const PLANNED_ARTIFACT_ROLE_CONFLICT_DIFF_CATEGORY: &str =
    "planned_artifact_role_conflict";
pub(in crate::deployment_truth) const PLANNED_ARTIFACT_DUPLICATE_DIFF_CATEGORY: &str =
    "planned_artifact_duplicate";
pub(in crate::deployment_truth) const DUPLICATE_PLANNED_ARTIFACT_ROLE_CODE: &str =
    "duplicate_planned_artifact_role";
pub(in crate::deployment_truth) const ARTIFACT_ROLE_CONFLICT_CODE: &str = "artifact_role_conflict";
pub(in crate::deployment_truth) const ARTIFACT_ROLE_CONFLICT_DIFF_CATEGORY: &str =
    "artifact_role_conflict";
pub(in crate::deployment_truth) const ARTIFACT_DUPLICATE_DIFF_CATEGORY: &str = "artifact_duplicate";
pub(in crate::deployment_truth) const DUPLICATE_ARTIFACT_OBSERVED_CODE: &str =
    "duplicate_artifact_observed";
const ARTIFACT_DIFF_CATEGORY: &str = "artifact";
pub(in crate::deployment_truth) const ARTIFACT_MISSING_CODE: &str = "artifact_missing";
pub(in crate::deployment_truth) const ARTIFACT_FILE_SHA256_DIFF_CATEGORY: &str =
    "artifact_file_sha256";
pub(in crate::deployment_truth) const ARTIFACT_FILE_DIGEST_MISMATCH_CODE: &str =
    "artifact_file_digest_mismatch";
const ARTIFACT_SHA256_DIFF_CATEGORY: &str = "artifact_sha256";
const ARTIFACT_DIGEST_MISMATCH_CODE: &str = "artifact_digest_mismatch";
const ARTIFACT_DIGEST_UNOBSERVED_CODE: &str = "artifact_digest_unobserved";

pub(in crate::deployment_truth) fn is_artifact_role_failure_code(code: &str) -> bool {
    matches!(
        code,
        ARTIFACT_ROLE_CONFLICT_CODE
            | ARTIFACT_MISSING_CODE
            | ARTIFACT_FILE_DIGEST_MISMATCH_CODE
            | ARTIFACT_DIGEST_MISMATCH_CODE
    )
}

pub(super) fn compare_artifacts(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
    artifact_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    let planned_conflicting_roles =
        compare_planned_artifact_role_conflicts(plan, artifact_diff, hard_failures, warnings);
    let conflicting_roles =
        compare_observed_artifact_role_conflicts(inventory, artifact_diff, hard_failures, warnings);
    let mut observed_by_role = BTreeMap::new();
    for artifact in &inventory.observed_artifacts {
        if conflicting_roles.contains(&artifact.role) {
            continue;
        }
        observed_by_role
            .entry(artifact.role.as_str())
            .or_insert(artifact);
    }

    let mut compared_roles = BTreeSet::new();
    for expected in &plan.role_artifacts {
        if planned_conflicting_roles.contains(&expected.role)
            || conflicting_roles.contains(&expected.role)
            || !compared_roles.insert(expected.role.as_str())
        {
            continue;
        }
        let Some(observed) = observed_by_role.get(expected.role.as_str()) else {
            record_missing_artifact(expected, artifact_diff, hard_failures);
            continue;
        };

        compare_artifact_file_sha256(expected, observed, artifact_diff, hard_failures);
        compare_artifact_payload_sha256(expected, observed, artifact_diff, hard_failures, warnings);
    }
}

fn compare_planned_artifact_role_conflicts(
    plan: &DeploymentPlanV1,
    artifact_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) -> BTreeSet<String> {
    let mut conflicting_roles = BTreeSet::new();
    for group in duplicate_evidence_groups(
        &plan.role_artifacts,
        |planned| planned.role.as_str().to_string(),
        planned_artifact_evidence_label,
        " | ",
    ) {
        if group.is_conflict {
            conflicting_roles.insert(group.subject.clone());
            artifact_diff.push(diff_item(
                PLANNED_ARTIFACT_ROLE_CONFLICT_DIFF_CATEGORY,
                &group.subject,
                Some("one planned artifact".to_string()),
                Some(group.evidence_label.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                PLANNED_ARTIFACT_ROLE_CONFLICT_CODE,
                format!(
                    "planned artifact role {} has conflicting evidence: {}",
                    group.subject, group.evidence_label
                ),
                SafetySeverityV1::HardFailure,
                Some(group.subject),
            ));
        } else {
            artifact_diff.push(diff_item(
                PLANNED_ARTIFACT_DUPLICATE_DIFF_CATEGORY,
                &group.subject,
                Some(group.evidence_label.clone()),
                Some(group.count.to_string()),
                SafetySeverityV1::Warning,
            ));
            warnings.push(finding(
                DUPLICATE_PLANNED_ARTIFACT_ROLE_CODE,
                format!(
                    "planned artifact role {} was declared {} times with identical evidence",
                    group.subject, group.count
                ),
                SafetySeverityV1::Warning,
                Some(group.subject),
            ));
        }
    }
    conflicting_roles
}

fn planned_artifact_evidence_label(planned: &RoleArtifactV1) -> String {
    format!(
        "wasm_gz_path={};wasm_gz={};file={};module={};raw_config={};canonical={}",
        planned.wasm_gz_path.as_deref().unwrap_or("<none>"),
        planned.wasm_gz_sha256.as_deref().unwrap_or("<none>"),
        planned
            .observed_wasm_gz_file_sha256
            .as_deref()
            .unwrap_or("<none>"),
        planned.installed_module_hash.as_deref().unwrap_or("<none>"),
        planned.raw_config_sha256.as_deref().unwrap_or("<none>"),
        planned
            .canonical_embedded_config_sha256
            .as_deref()
            .unwrap_or("<none>")
    )
}

fn compare_observed_artifact_role_conflicts(
    inventory: &DeploymentInventoryV1,
    artifact_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) -> BTreeSet<String> {
    let mut conflicting_roles = BTreeSet::new();
    for group in duplicate_evidence_groups(
        &inventory.observed_artifacts,
        |observed| observed.role.as_str().to_string(),
        observed_artifact_evidence_label,
        " | ",
    ) {
        if group.is_conflict {
            conflicting_roles.insert(group.subject.clone());
            artifact_diff.push(diff_item(
                ARTIFACT_ROLE_CONFLICT_DIFF_CATEGORY,
                &group.subject,
                Some("one artifact observation".to_string()),
                Some(group.evidence_label.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                ARTIFACT_ROLE_CONFLICT_CODE,
                format!(
                    "observed artifact role {} has conflicting evidence: {}",
                    group.subject, group.evidence_label
                ),
                SafetySeverityV1::HardFailure,
                Some(group.subject),
            ));
        } else {
            artifact_diff.push(diff_item(
                ARTIFACT_DUPLICATE_DIFF_CATEGORY,
                &group.subject,
                Some(group.evidence_label.clone()),
                Some(group.count.to_string()),
                SafetySeverityV1::Warning,
            ));
            warnings.push(finding(
                DUPLICATE_ARTIFACT_OBSERVED_CODE,
                format!(
                    "observed artifact role {} was reported {} times with identical evidence",
                    group.subject, group.count
                ),
                SafetySeverityV1::Warning,
                Some(group.subject),
            ));
        }
    }
    conflicting_roles
}

fn observed_artifact_evidence_label(observed: &ObservedArtifactV1) -> String {
    format!(
        "path={};file={};payload={};size={};source={:?}",
        observed.artifact_path,
        observed.file_sha256.as_deref().unwrap_or("<none>"),
        observed.payload_sha256.as_deref().unwrap_or("<none>"),
        observed
            .payload_size_bytes
            .map_or_else(|| "<none>".to_string(), |size| size.to_string()),
        observed.source
    )
}

fn record_missing_artifact(
    expected: &RoleArtifactV1,
    artifact_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    artifact_diff.push(diff_item(
        ARTIFACT_DIFF_CATEGORY,
        &expected.role,
        expected.wasm_gz_path.clone(),
        None,
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        ARTIFACT_MISSING_CODE,
        format!("missing observed artifact for role {}", expected.role),
        SafetySeverityV1::HardFailure,
        Some(expected.role.clone()),
    ));
}

fn compare_artifact_file_sha256(
    expected: &RoleArtifactV1,
    observed: &ObservedArtifactV1,
    artifact_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    match (
        expected.observed_wasm_gz_file_sha256.as_ref(),
        observed.file_sha256.as_ref(),
    ) {
        (Some(want), Some(got)) if want != got => {
            artifact_diff.push(diff_item(
                ARTIFACT_FILE_SHA256_DIFF_CATEGORY,
                &expected.role,
                Some(want.clone()),
                Some(got.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                ARTIFACT_FILE_DIGEST_MISMATCH_CODE,
                format!(
                    "observed artifact file digest changed during deployment truth check for role {}",
                    expected.role
                ),
                SafetySeverityV1::HardFailure,
                Some(expected.role.clone()),
            ));
        }
        (_, Some(got)) => {
            artifact_diff.push(diff_item(
                ARTIFACT_FILE_SHA256_DIFF_CATEGORY,
                &expected.role,
                expected.observed_wasm_gz_file_sha256.clone(),
                Some(got.clone()),
                SafetySeverityV1::Info,
            ));
        }
        _ => {}
    }
}

fn compare_artifact_payload_sha256(
    expected: &RoleArtifactV1,
    observed: &ObservedArtifactV1,
    artifact_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    match (
        expected.wasm_gz_sha256.as_ref(),
        observed.payload_sha256.as_ref(),
    ) {
        (Some(want), Some(got)) if want != got => {
            artifact_diff.push(diff_item(
                ARTIFACT_SHA256_DIFF_CATEGORY,
                &expected.role,
                Some(want.clone()),
                Some(got.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                ARTIFACT_DIGEST_MISMATCH_CODE,
                format!("artifact digest mismatch for role {}", expected.role),
                SafetySeverityV1::HardFailure,
                Some(expected.role.clone()),
            ));
        }
        (Some(want), None) => warnings.push(finding(
            ARTIFACT_DIGEST_UNOBSERVED_CODE,
            format!(
                "expected artifact digest {want} for role {} was not observed",
                expected.role
            ),
            SafetySeverityV1::Warning,
            Some(expected.role.clone()),
        )),
        _ => {}
    }
}
