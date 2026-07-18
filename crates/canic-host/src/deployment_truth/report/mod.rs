use super::*;
use std::collections::{BTreeMap, BTreeSet};

mod artifacts;
mod canisters;
mod config_digests;
mod controllers;
mod module_hashes;
mod pools;
mod receipt_resume;
mod root_subnet;
mod safety;
mod verifier_readiness;

use artifacts::compare_artifacts;
#[cfg(test)]
pub(super) use artifacts::{
    ARTIFACT_DUPLICATE_DIFF_CATEGORY, ARTIFACT_FILE_DIGEST_MISMATCH_CODE,
    ARTIFACT_FILE_SHA256_DIFF_CATEGORY, ARTIFACT_ROLE_CONFLICT_DIFF_CATEGORY,
    DUPLICATE_ARTIFACT_OBSERVED_CODE, DUPLICATE_PLANNED_ARTIFACT_ROLE_CODE,
    PLANNED_ARTIFACT_DUPLICATE_DIFF_CATEGORY, PLANNED_ARTIFACT_ROLE_CONFLICT_DIFF_CATEGORY,
};
pub(in crate::deployment_truth) use artifacts::{
    ARTIFACT_MISSING_CODE, is_artifact_role_failure_code,
};
pub(super) use artifacts::{ARTIFACT_ROLE_CONFLICT_CODE, PLANNED_ARTIFACT_ROLE_CONFLICT_CODE};
#[cfg(test)]
pub(super) use canisters::{
    CANISTER_DUPLICATE_DIFF_CATEGORY, CANISTER_EXTRA_DIFF_CATEGORY,
    CANISTER_ID_ROLE_CONFLICT_DIFF_CATEGORY, CANISTER_ROLE_AMBIGUOUS_CODE,
    CANISTER_ROLE_AMBIGUOUS_DIFF_CATEGORY, CANISTER_ROLE_MISMATCH_CODE, CANISTER_UNOBSERVED_CODE,
    DUPLICATE_CANISTER_OBSERVED_CODE, DUPLICATE_PLANNED_CANISTER_ROLE_CODE,
    EXTRA_CANISTER_OBSERVED_CODE, PLANNED_CANISTER_DUPLICATE_DIFF_CATEGORY,
    PLANNED_CANISTER_ID_CONFLICT_DIFF_CATEGORY, PLANNED_CANISTER_ROLE_CONFLICT_DIFF_CATEGORY,
    ROLE_MISMATCH_DIFF_CATEGORY, SUBNET_REGISTRY_ROLE_MISSING_CODE, UNSAFE_CONTROL_CLASS_CODE,
};
pub(super) use canisters::{
    CANISTER_ID_ROLE_CONFLICT_CODE, PLANNED_CANISTER_ID_CONFLICT_CODE,
    PLANNED_CANISTER_ROLE_CONFLICT_CODE,
};
use canisters::{compare_canisters, compare_observed_canister_id_conflicts};
#[cfg(test)]
pub(super) use config_digests::{RAW_CONFIG_DIGEST_MISMATCH_CODE, RAW_CONFIG_SHA256_DIFF_CATEGORY};
use config_digests::{compare_embedded_config, compare_raw_config};
use controllers::compare_authority_profile;
#[cfg(test)]
pub(super) use controllers::{
    CONTROLLER_AUTHORITY_OVERLAP_CODE, CONTROLLER_EXTRA_DIFF_CATEGORY,
    CONTROLLER_MISSING_DIFF_CATEGORY, CONTROLLERS_UNOBSERVED_CODE,
    EXPECTED_CONTROLLER_MISSING_CODE, EXTRA_CONTROLLER_OBSERVED_CODE,
};
use module_hashes::compare_module_hashes;
#[cfg(test)]
pub(super) use module_hashes::{
    INSTALLED_MODULE_HASH_AMBIGUOUS_CODE, INSTALLED_MODULE_HASH_AMBIGUOUS_DIFF_CATEGORY,
    INSTALLED_MODULE_HASH_DIFF_CATEGORY, INSTALLED_MODULE_HASH_MISMATCH_CODE,
};
pub(super) use pools::{
    CANISTER_POOL_ROLE_CONFLICT_CODE, PLANNED_POOL_CONFLICT_CODE, PLANNED_POOL_ID_CONFLICT_CODE,
    POOL_CANISTER_ID_CONFLICT_CODE,
};
#[cfg(test)]
pub(super) use pools::{
    CANISTER_POOL_ROLE_CONFLICT_DIFF_CATEGORY, DUPLICATE_PLANNED_POOL_CODE,
    DUPLICATE_POOL_CANISTER_OBSERVED_CODE, EXTRA_POOL_CANISTER_OBSERVED_CODE,
    PLANNED_POOL_CONFLICT_DIFF_CATEGORY, PLANNED_POOL_DUPLICATE_DIFF_CATEGORY,
    PLANNED_POOL_ID_CONFLICT_DIFF_CATEGORY, POOL_CANISTER_DIFF_CATEGORY,
    POOL_CANISTER_DUPLICATE_DIFF_CATEGORY, POOL_CANISTER_ID_CONFLICT_DIFF_CATEGORY,
    POOL_CANISTER_ID_DIFF_CATEGORY, POOL_CANISTER_ID_MISMATCH_CODE, POOL_CANISTER_MISSING_CODE,
    POOL_CONTROL_CLASS_DIFF_CATEGORY, POOL_EXTRA_DIFF_CATEGORY, UNSAFE_POOL_CONTROL_CLASS_CODE,
};
use pools::{compare_observed_canister_pool_role_conflicts, compare_pools};
pub use receipt_resume::compare_plan_inventory_and_receipt;
#[cfg(test)]
pub(super) use receipt_resume::{
    DUPLICATE_RECEIPT_PHASE_CODE, DUPLICATE_RECEIPT_ROLE_PHASE_CODE,
    RECEIPT_EXECUTION_STATUS_MISMATCH_CODE, RECEIPT_PLAN_MISMATCH_CODE,
    RECEIPT_POSTCONDITION_UNVERIFIED_CODE,
};
pub(super) use receipt_resume::{RECEIPT_PHASE_CONFLICT_CODE, RECEIPT_ROLE_PHASE_CONFLICT_CODE};
#[cfg(test)]
pub(super) use root_subnet::ROOT_AUTH_CLOUD_ENGINE_SUBNET_CODE;
pub(super) use root_subnet::apply_root_auth_signer_subnet_check;
#[cfg(test)]
pub(super) use root_subnet::{
    RootSubnetEvidence, RootSubnetEvidenceSource, apply_root_auth_signer_subnet_check_with_source,
};
pub use safety::safety_report_from_diff;
pub(in crate::deployment_truth::report) use safety::{resume_safety_reasons, safety_status};
use verifier_readiness::compare_verifier_readiness;
#[cfg(test)]
pub(super) use verifier_readiness::{
    DUPLICATE_PLANNED_VERIFIER_ROLE_EPOCH_CODE, DUPLICATE_VERIFIER_ROLE_EPOCH_OBSERVED_CODE,
    PLANNED_VERIFIER_ROLE_EPOCH_CONFLICT_DIFF_CATEGORY,
    PLANNED_VERIFIER_ROLE_EPOCH_DUPLICATE_DIFF_CATEGORY, VERIFIER_NOT_OBSERVED_LABEL,
    VERIFIER_ROLE_EPOCH_CONFLICT_DIFF_CATEGORY, VERIFIER_ROLE_EPOCH_DIFF_CATEGORY,
    VERIFIER_ROLE_EPOCH_DUPLICATE_DIFF_CATEGORY, VERIFIER_ROLE_EPOCH_STALE_CODE,
    VERIFIER_ROLE_EPOCH_UNOBSERVED_CODE,
};
pub(super) use verifier_readiness::{
    PLANNED_VERIFIER_ROLE_EPOCH_CONFLICT_CODE, VERIFIER_ROLE_EPOCH_CONFLICT_CODE,
};

pub(in crate::deployment_truth) const DEPLOYMENT_MANIFEST_MISMATCH_CODE: &str =
    "deployment_manifest_mismatch";
pub(in crate::deployment_truth) const OBSERVATION_GAP_CODE: &str = "observation_gap";
pub(in crate::deployment_truth) const UNVERIFIED_DEPLOYMENT_ROOT_CODE: &str =
    "unverified_deployment_root";
pub(in crate::deployment_truth) const PLAN_ASSUMPTION_CODE: &str = "plan_assumption";
pub(in crate::deployment_truth) const IDENTITY_UNOBSERVED_CODE: &str = "identity_unobserved";
pub(in crate::deployment_truth) const ENVIRONMENT_MISMATCH_CODE: &str = "environment_mismatch";
pub(in crate::deployment_truth) const ROOT_TRUST_ANCHOR_MISMATCH_CODE: &str =
    "root_trust_anchor_mismatch";
pub(in crate::deployment_truth) const DEPLOYMENT_MANIFEST_UNOBSERVED_CODE: &str =
    "deployment_manifest_unobserved";

#[must_use]
pub fn is_evidence_conflict_finding_code(code: &str) -> bool {
    matches!(
        code,
        PLANNED_ARTIFACT_ROLE_CONFLICT_CODE
            | ARTIFACT_ROLE_CONFLICT_CODE
            | CANISTER_ID_ROLE_CONFLICT_CODE
            | PLANNED_CANISTER_ROLE_CONFLICT_CODE
            | PLANNED_CANISTER_ID_CONFLICT_CODE
            | CANISTER_POOL_ROLE_CONFLICT_CODE
            | PLANNED_POOL_CONFLICT_CODE
            | PLANNED_POOL_ID_CONFLICT_CODE
            | POOL_CANISTER_ID_CONFLICT_CODE
            | RECEIPT_PHASE_CONFLICT_CODE
            | RECEIPT_ROLE_PHASE_CONFLICT_CODE
            | PLANNED_VERIFIER_ROLE_EPOCH_CONFLICT_CODE
            | VERIFIER_ROLE_EPOCH_CONFLICT_CODE
    )
}

///
/// DuplicateEvidenceGroup
///
struct DuplicateEvidenceGroup {
    subject: String,
    count: usize,
    evidence_label: String,
    is_conflict: bool,
}

///
/// LocalDeploymentCheckRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalDeploymentCheckRequest {
    pub deployment_name: String,
    pub environment: String,
    pub artifact_environment: String,
    pub workspace_root: std::path::PathBuf,
    pub icp_root: std::path::PathBuf,
    pub config_path: Option<std::path::PathBuf>,
    pub observed_at: String,
    pub runtime_variant: String,
    pub build_profile: String,
}

/// Build local plan and inventory, then return the passive safety check bundle.
pub fn check_local_deployment(
    request: &LocalDeploymentCheckRequest,
) -> Result<DeploymentCheckV1, DeploymentTruthError> {
    let plan = build_local_deployment_plan(&LocalDeploymentPlanRequest {
        deployment_name: request.deployment_name.clone(),
        environment: request.environment.clone(),
        artifact_environment: request.artifact_environment.clone(),
        workspace_root: request.workspace_root.clone(),
        icp_root: request.icp_root.clone(),
        config_path: request.config_path.clone(),
        runtime_variant: request.runtime_variant.clone(),
        build_profile: request.build_profile.clone(),
    });
    let inventory = collect_local_deployment_inventory(&LocalInventoryRequest {
        deployment_name: request.deployment_name.clone(),
        environment: request.environment.clone(),
        artifact_environment: request.artifact_environment.clone(),
        workspace_root: request.workspace_root.clone(),
        icp_root: request.icp_root.clone(),
        config_path: request.config_path.clone(),
        observed_at: request.observed_at.clone(),
    })?;
    let mut diff = compare_plan_to_inventory(&plan, &inventory);
    apply_root_auth_signer_subnet_check(
        &mut diff,
        &inventory,
        &request.environment,
        &request.icp_root,
    );
    let report = safety_report_from_diff(
        format!(
            "local:{}:{}:report",
            request.environment, request.deployment_name
        ),
        Some(format!(
            "local:{}:{}:diff",
            request.environment, request.deployment_name
        )),
        &diff,
    );

    Ok(DeploymentCheckV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        check_id: format!(
            "local:{}:{}:check",
            request.environment, request.deployment_name
        ),
        plan,
        inventory,
        diff,
        report,
    })
}

fn refresh_resume_safety(diff: &mut DeploymentDiffV1) {
    diff.resume_safety.status = safety_status(&diff.hard_failures, &diff.warnings);
    diff.resume_safety.reasons = resume_safety_reasons(&diff.hard_failures, &diff.warnings);
}

/// Compare intended deployment state with observed inventory into a machine diff.
#[must_use]
pub fn compare_plan_to_inventory(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
) -> DeploymentDiffV1 {
    let mut artifact_diff = Vec::new();
    let mut controller_diff = Vec::new();
    let mut pool_diff = Vec::new();
    let mut embedded_config_diff = Vec::new();
    let mut module_hash_diff = Vec::new();
    let mut verifier_readiness_diff = Vec::new();
    let mut hard_failures = Vec::new();
    let mut warnings = Vec::new();

    compare_identity(plan, inventory, &mut hard_failures);
    compare_authority_profile(plan, &mut controller_diff, &mut hard_failures);
    compare_artifacts(
        plan,
        inventory,
        &mut artifact_diff,
        &mut hard_failures,
        &mut warnings,
    );
    compare_observed_canister_id_conflicts(
        inventory,
        &mut controller_diff,
        &mut hard_failures,
        &mut warnings,
    );
    compare_observed_canister_pool_role_conflicts(inventory, &mut pool_diff, &mut hard_failures);
    compare_canisters(
        plan,
        inventory,
        &mut controller_diff,
        &mut hard_failures,
        &mut warnings,
    );
    compare_pools(
        plan,
        inventory,
        &mut pool_diff,
        &mut hard_failures,
        &mut warnings,
    );
    compare_module_hashes(
        plan,
        inventory,
        &mut module_hash_diff,
        &mut hard_failures,
        &mut warnings,
    );
    compare_raw_config(
        plan,
        inventory,
        &mut embedded_config_diff,
        &mut hard_failures,
    );
    compare_embedded_config(
        plan,
        inventory,
        &mut embedded_config_diff,
        &mut hard_failures,
        &mut warnings,
    );
    compare_verifier_readiness(
        plan,
        inventory,
        &mut verifier_readiness_diff,
        &mut hard_failures,
        &mut warnings,
    );
    record_plan_assumptions(plan, &mut hard_failures, &mut warnings);
    for gap in &inventory.unresolved_observations {
        warnings.push(SafetyFindingV1 {
            code: OBSERVATION_GAP_CODE.to_string(),
            message: gap.description.clone(),
            severity: SafetySeverityV1::Warning,
            subject: Some(gap.key.clone()),
        });
    }

    let status = safety_status(&hard_failures, &warnings);
    DeploymentDiffV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        plan_identity: plan.deployment_identity.clone(),
        observed_identity: inventory.observed_identity.clone(),
        artifact_diff,
        controller_diff,
        pool_diff,
        embedded_config_diff,
        module_hash_diff,
        verifier_readiness_diff,
        resume_safety: ResumeSafetyV1 {
            status,
            reasons: resume_safety_reasons(&hard_failures, &warnings),
        },
        hard_failures,
        warnings,
        resumable_phases: Vec::new(),
    }
}

fn record_plan_assumptions(
    plan: &DeploymentPlanV1,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    for assumption in &plan.unresolved_assumptions {
        if assumption.key == "local_state.unverified_root_canister_id" {
            hard_failures.push(SafetyFindingV1 {
                code: UNVERIFIED_DEPLOYMENT_ROOT_CODE.to_string(),
                message: assumption.description.clone(),
                severity: SafetySeverityV1::HardFailure,
                subject: Some(assumption.key.clone()),
            });
        } else {
            warnings.push(SafetyFindingV1 {
                code: PLAN_ASSUMPTION_CODE.to_string(),
                message: assumption.description.clone(),
                severity: SafetySeverityV1::Warning,
                subject: Some(assumption.key.clone()),
            });
        }
    }
}

fn compare_identity(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    let Some(observed) = &inventory.observed_identity else {
        hard_failures.push(finding(
            IDENTITY_UNOBSERVED_CODE,
            "deployment identity was not observed",
            SafetySeverityV1::HardFailure,
            None,
        ));
        return;
    };

    if observed.environment != plan.deployment_identity.environment {
        hard_failures.push(finding(
            ENVIRONMENT_MISMATCH_CODE,
            format!(
                "plan environment {} differs from observed environment {}",
                plan.deployment_identity.environment, observed.environment
            ),
            SafetySeverityV1::HardFailure,
            Some("deployment_identity.environment".to_string()),
        ));
    }
    if let (Some(expected), Some(actual)) = (
        plan.deployment_identity.root_principal.as_ref(),
        observed.root_principal.as_ref(),
    ) && expected != actual
    {
        hard_failures.push(finding(
            ROOT_TRUST_ANCHOR_MISMATCH_CODE,
            format!("plan root {expected} differs from observed root {actual}"),
            SafetySeverityV1::HardFailure,
            Some("deployment_identity.root_principal".to_string()),
        ));
    }
    match (
        plan.deployment_identity.deployment_manifest_digest.as_ref(),
        observed.deployment_manifest_digest.as_ref(),
    ) {
        (Some(expected), Some(actual)) if expected != actual => {
            hard_failures.push(finding(
                DEPLOYMENT_MANIFEST_MISMATCH_CODE,
                "deployment manifest digest differs from the observed local config",
                SafetySeverityV1::HardFailure,
                Some("deployment_identity.deployment_manifest_digest".to_string()),
            ));
        }
        (Some(_), None) => {
            hard_failures.push(finding(
                DEPLOYMENT_MANIFEST_UNOBSERVED_CODE,
                "deployment manifest digest was not observed",
                SafetySeverityV1::HardFailure,
                Some("deployment_identity.deployment_manifest_digest".to_string()),
            ));
        }
        _ => {}
    }
}

fn finding(
    code: impl Into<String>,
    message: impl Into<String>,
    severity: SafetySeverityV1,
    subject: Option<String>,
) -> SafetyFindingV1 {
    SafetyFindingV1 {
        code: code.into(),
        message: message.into(),
        severity,
        subject,
    }
}

fn diff_item(
    category: impl Into<String>,
    subject: impl Into<String>,
    expected: Option<String>,
    observed: Option<String>,
    severity: SafetySeverityV1,
) -> DiffItemV1 {
    DiffItemV1 {
        category: category.into(),
        subject: subject.into(),
        expected,
        observed,
        severity,
    }
}

fn duplicate_evidence_groups<T>(
    items: &[T],
    subject: impl Fn(&T) -> String,
    evidence: impl Fn(&T) -> String,
    evidence_separator: &str,
) -> Vec<DuplicateEvidenceGroup> {
    let mut groups = Vec::new();
    for (subject, entries) in group_by_subject(items, |item| Some(subject(item))) {
        if entries.len() <= 1 {
            continue;
        }
        let evidence_values = entries
            .iter()
            .map(|entry| evidence(entry))
            .collect::<BTreeSet<_>>();
        groups.push(DuplicateEvidenceGroup {
            subject,
            count: entries.len(),
            evidence_label: evidence_values
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(evidence_separator),
            is_conflict: evidence_values.len() > 1,
        });
    }
    groups
}

fn duplicate_evidence_groups_by<T, K>(
    items: &[T],
    subject: impl Fn(&T) -> String,
    evidence_key: impl Fn(&T) -> K,
    evidence_label: impl Fn(&T) -> String,
    evidence_separator: &str,
) -> Vec<DuplicateEvidenceGroup>
where
    K: Ord,
{
    let mut groups = Vec::new();
    for (subject, entries) in group_by_subject(items, |item| Some(subject(item))) {
        if entries.len() <= 1 {
            continue;
        }
        let evidence_values = entries
            .iter()
            .map(|entry| (evidence_key(entry), evidence_label(entry)))
            .collect::<BTreeMap<_, _>>();
        groups.push(DuplicateEvidenceGroup {
            subject,
            count: entries.len(),
            evidence_label: evidence_values
                .values()
                .cloned()
                .collect::<Vec<_>>()
                .join(evidence_separator),
            is_conflict: evidence_values.len() > 1,
        });
    }
    groups
}

fn conflicting_assignment_groups<T>(
    items: &[T],
    subject: impl Fn(&T) -> Option<String>,
    value: impl Fn(&T) -> String,
    value_separator: &str,
) -> Vec<DuplicateEvidenceGroup> {
    let mut groups = Vec::new();
    for (subject, entries) in group_by_subject(items, subject) {
        if entries.len() <= 1 {
            continue;
        }
        let values = entries
            .iter()
            .map(|entry| value(entry))
            .collect::<BTreeSet<_>>();
        if values.len() <= 1 {
            continue;
        }
        groups.push(DuplicateEvidenceGroup {
            subject,
            count: entries.len(),
            evidence_label: values
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(value_separator),
            is_conflict: true,
        });
    }
    groups
}

fn group_by_subject<T>(
    items: &[T],
    subject: impl Fn(&T) -> Option<String>,
) -> BTreeMap<String, Vec<&T>> {
    let mut by_subject = BTreeMap::<String, Vec<&T>>::new();
    for item in items {
        if let Some(subject) = subject(item) {
            by_subject.entry(subject).or_default().push(item);
        }
    }
    by_subject
}

#[cfg(test)]
mod classification_tests {
    use super::*;

    #[test]
    fn finding_classification_uses_exact_owner_codes() {
        assert!(is_evidence_conflict_finding_code(
            ARTIFACT_ROLE_CONFLICT_CODE
        ));
        assert!(is_evidence_conflict_finding_code(
            RECEIPT_PHASE_CONFLICT_CODE
        ));
        assert!(!is_evidence_conflict_finding_code("artifact_conflict"));
        assert!(!is_evidence_conflict_finding_code("conflict"));

        assert!(is_artifact_role_failure_code(ARTIFACT_MISSING_CODE));
        assert!(!is_artifact_role_failure_code(
            PLANNED_ARTIFACT_ROLE_CONFLICT_CODE
        ));
    }
}
