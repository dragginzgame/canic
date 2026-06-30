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
use canisters::{compare_canisters, compare_observed_canister_id_conflicts};
use config_digests::{compare_embedded_config, compare_raw_config};
use controllers::compare_authority_profile;
use module_hashes::compare_module_hashes;
use pools::{compare_observed_canister_pool_role_conflicts, compare_pools};
pub use receipt_resume::compare_plan_inventory_and_receipt;
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
    pub network: String,
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
        network: request.network.clone(),
        workspace_root: request.workspace_root.clone(),
        icp_root: request.icp_root.clone(),
        config_path: request.config_path.clone(),
        runtime_variant: request.runtime_variant.clone(),
        build_profile: request.build_profile.clone(),
    });
    let inventory = collect_local_deployment_inventory(&LocalInventoryRequest {
        deployment_name: request.deployment_name.clone(),
        network: request.network.clone(),
        workspace_root: request.workspace_root.clone(),
        icp_root: request.icp_root.clone(),
        config_path: request.config_path.clone(),
        observed_at: request.observed_at.clone(),
    })?;
    let mut diff = compare_plan_to_inventory(&plan, &inventory);
    apply_root_auth_signer_subnet_check(&mut diff, &inventory, &request.network, &request.icp_root);
    let report = safety_report_from_diff(
        format!(
            "local:{}:{}:report",
            request.network, request.deployment_name
        ),
        Some(format!(
            "local:{}:{}:diff",
            request.network, request.deployment_name
        )),
        &diff,
    );

    Ok(DeploymentCheckV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        check_id: format!(
            "local:{}:{}:check",
            request.network, request.deployment_name
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
            code: "observation_gap".to_string(),
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
                code: "unverified_deployment_root".to_string(),
                message: assumption.description.clone(),
                severity: SafetySeverityV1::HardFailure,
                subject: Some(assumption.key.clone()),
            });
        } else {
            warnings.push(SafetyFindingV1 {
                code: "plan_assumption".to_string(),
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
            "identity_unobserved",
            "deployment identity was not observed",
            SafetySeverityV1::HardFailure,
            None,
        ));
        return;
    };

    if observed.network != plan.deployment_identity.network {
        hard_failures.push(finding(
            "network_mismatch",
            format!(
                "plan network {} differs from observed network {}",
                plan.deployment_identity.network, observed.network
            ),
            SafetySeverityV1::HardFailure,
            Some("deployment_identity.network".to_string()),
        ));
    }
    if let (Some(expected), Some(actual)) = (
        plan.deployment_identity.root_principal.as_ref(),
        observed.root_principal.as_ref(),
    ) && expected != actual
    {
        hard_failures.push(finding(
            "root_trust_anchor_mismatch",
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
                "deployment_manifest_mismatch",
                "deployment manifest digest differs from the observed local config",
                SafetySeverityV1::HardFailure,
                Some("deployment_identity.deployment_manifest_digest".to_string()),
            ));
        }
        (Some(_), None) => {
            hard_failures.push(finding(
                "deployment_manifest_unobserved",
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
