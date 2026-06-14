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

use artifacts::compare_artifacts;
use canisters::{compare_canisters, compare_observed_canister_id_conflicts};
use config_digests::{compare_embedded_config, compare_raw_config};
use controllers::compare_authority_profile;
use module_hashes::compare_module_hashes;
use pools::{compare_observed_canister_pool_role_conflicts, compare_pools};
pub use receipt_resume::compare_plan_inventory_and_receipt;
pub(super) use root_subnet::apply_root_canister_signature_subnet_check;
#[cfg(test)]
pub(super) use root_subnet::{
    RootSubnetEvidence, RootSubnetEvidenceSource,
    apply_root_canister_signature_subnet_check_with_source,
};

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
    apply_root_canister_signature_subnet_check(
        &mut diff,
        &inventory,
        &request.network,
        &request.icp_root,
    );
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

/// Render an operator-facing safety report from a machine deployment diff.
#[must_use]
pub fn safety_report_from_diff(
    report_id: impl Into<String>,
    diff_id: Option<String>,
    diff: &DeploymentDiffV1,
) -> SafetyReportV1 {
    let status = safety_status(&diff.hard_failures, &diff.warnings);
    SafetyReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: report_id.into(),
        diff_id,
        status,
        summary: safety_summary(status, diff.hard_failures.len(), diff.warnings.len()),
        hard_failures: diff.hard_failures.clone(),
        warnings: diff.warnings.clone(),
        next_actions: safety_next_actions(status),
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

fn compare_verifier_readiness(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
    verifier_readiness_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    if !plan.expected_verifier_readiness.required {
        return;
    }
    if inventory.observed_verifier_readiness.status == ObservationStatusV1::NotObserved {
        verifier_readiness_diff.push(diff_item(
            "verifier_readiness",
            "deployment",
            Some("required".to_string()),
            Some("not_observed".to_string()),
            SafetySeverityV1::Warning,
        ));
        warnings.push(finding(
            "verifier_readiness_unobserved",
            "verifier readiness was required but not observed",
            SafetySeverityV1::Warning,
            Some("verifier_readiness".to_string()),
        ));
    }

    let planned_conflicting_roles = compare_planned_verifier_epoch_conflicts(
        plan,
        verifier_readiness_diff,
        hard_failures,
        warnings,
    );
    let conflicting_roles = compare_observed_verifier_epoch_conflicts(
        inventory,
        verifier_readiness_diff,
        hard_failures,
        warnings,
    );
    let mut observed_by_role = BTreeMap::new();
    for epoch in &inventory.observed_verifier_readiness.role_epochs {
        if conflicting_roles.contains(&epoch.role) {
            continue;
        }
        observed_by_role.entry(epoch.role.as_str()).or_insert(epoch);
    }
    let mut compared_roles = BTreeSet::new();
    for expected in &plan.expected_verifier_readiness.expected_role_epochs {
        if planned_conflicting_roles.contains(&expected.role)
            || conflicting_roles.contains(&expected.role)
            || !compared_roles.insert(expected.role.as_str())
        {
            continue;
        }
        let observed = observed_by_role.get(expected.role.as_str());
        if let Some(observed_epoch) = observed.and_then(|observed| {
            (observed.status == ObservationStatusV1::Observed)
                .then_some(observed.observed_epoch)
                .flatten()
        }) {
            if observed_epoch < expected.minimum_epoch {
                record_stale_verifier_role_epoch(
                    expected,
                    observed_epoch,
                    verifier_readiness_diff,
                    hard_failures,
                );
            }
        } else {
            record_unobserved_verifier_role_epoch(expected, verifier_readiness_diff, warnings);
        }
    }
}

fn record_stale_verifier_role_epoch(
    expected: &RoleEpochExpectationV1,
    observed_epoch: u64,
    verifier_readiness_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    verifier_readiness_diff.push(diff_item(
        "verifier_role_epoch",
        &expected.role,
        Some(expected.minimum_epoch.to_string()),
        Some(observed_epoch.to_string()),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        "verifier_role_epoch_stale",
        format!(
            "verifier role {} has epoch {observed_epoch}, expected at least {}",
            expected.role, expected.minimum_epoch
        ),
        SafetySeverityV1::HardFailure,
        Some(expected.role.clone()),
    ));
}

fn record_unobserved_verifier_role_epoch(
    expected: &RoleEpochExpectationV1,
    verifier_readiness_diff: &mut Vec<DiffItemV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    verifier_readiness_diff.push(diff_item(
        "verifier_role_epoch",
        &expected.role,
        Some(expected.minimum_epoch.to_string()),
        Some("not_observed".to_string()),
        SafetySeverityV1::Warning,
    ));
    warnings.push(finding(
        "verifier_role_epoch_unobserved",
        format!("verifier role {} epoch was not observed", expected.role),
        SafetySeverityV1::Warning,
        Some(expected.role.clone()),
    ));
}

fn compare_planned_verifier_epoch_conflicts(
    plan: &DeploymentPlanV1,
    verifier_readiness_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) -> BTreeSet<String> {
    let mut conflicting_roles = BTreeSet::new();
    for group in duplicate_evidence_groups(
        &plan.expected_verifier_readiness.expected_role_epochs,
        |expected| expected.role.as_str().to_string(),
        |expected| expected.minimum_epoch.to_string(),
        ",",
    ) {
        if group.is_conflict {
            conflicting_roles.insert(group.subject.clone());
            verifier_readiness_diff.push(diff_item(
                "planned_verifier_role_epoch_conflict",
                &group.subject,
                Some("one minimum epoch".to_string()),
                Some(group.evidence_label.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                "planned_verifier_role_epoch_conflict",
                format!(
                    "planned verifier role {} has conflicting minimum epochs: {}",
                    group.subject, group.evidence_label
                ),
                SafetySeverityV1::HardFailure,
                Some(group.subject),
            ));
        } else {
            verifier_readiness_diff.push(diff_item(
                "planned_verifier_role_epoch_duplicate",
                &group.subject,
                Some(group.evidence_label.clone()),
                Some(group.count.to_string()),
                SafetySeverityV1::Warning,
            ));
            warnings.push(finding(
                "duplicate_planned_verifier_role_epoch",
                format!(
                    "planned verifier role {} epoch was declared {} times with identical evidence",
                    group.subject, group.count
                ),
                SafetySeverityV1::Warning,
                Some(group.subject),
            ));
        }
    }
    conflicting_roles
}

fn compare_observed_verifier_epoch_conflicts(
    inventory: &DeploymentInventoryV1,
    verifier_readiness_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) -> BTreeSet<String> {
    let mut conflicting_roles = BTreeSet::new();
    for group in duplicate_evidence_groups(
        &inventory.observed_verifier_readiness.role_epochs,
        |observed| observed.role.as_str().to_string(),
        verifier_epoch_evidence_label,
        ",",
    ) {
        if group.is_conflict {
            conflicting_roles.insert(group.subject.clone());
            verifier_readiness_diff.push(diff_item(
                "verifier_role_epoch_conflict",
                &group.subject,
                Some("one epoch observation".to_string()),
                Some(group.evidence_label.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                "verifier_role_epoch_conflict",
                format!(
                    "verifier role {} has conflicting epoch observations: {}",
                    group.subject, group.evidence_label
                ),
                SafetySeverityV1::HardFailure,
                Some(group.subject),
            ));
        } else {
            verifier_readiness_diff.push(diff_item(
                "verifier_role_epoch_duplicate",
                &group.subject,
                Some(group.evidence_label.clone()),
                Some(group.count.to_string()),
                SafetySeverityV1::Warning,
            ));
            warnings.push(finding(
                "duplicate_verifier_role_epoch_observed",
                format!(
                    "verifier role {} epoch was reported {} times with identical evidence",
                    group.subject, group.count
                ),
                SafetySeverityV1::Warning,
                Some(group.subject),
            ));
        }
    }
    conflicting_roles
}

fn verifier_epoch_evidence_label(observed: &RoleEpochObservationV1) -> String {
    format!(
        "epoch={};status={:?}",
        observed
            .observed_epoch
            .map_or_else(|| "<none>".to_string(), |epoch| epoch.to_string()),
        observed.status
    )
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

const fn safety_status(
    hard_failures: &[SafetyFindingV1],
    warnings: &[SafetyFindingV1],
) -> SafetyStatusV1 {
    if !hard_failures.is_empty() {
        SafetyStatusV1::Blocked
    } else if !warnings.is_empty() {
        SafetyStatusV1::Warning
    } else {
        SafetyStatusV1::Safe
    }
}

fn resume_safety_reasons(
    hard_failures: &[SafetyFindingV1],
    warnings: &[SafetyFindingV1],
) -> Vec<String> {
    if !hard_failures.is_empty() {
        return hard_failures
            .iter()
            .map(|finding| finding.message.clone())
            .collect();
    }
    if !warnings.is_empty() {
        return warnings
            .iter()
            .map(|finding| finding.message.clone())
            .collect();
    }
    vec!["no blocking deployment truth differences were found".to_string()]
}

fn safety_summary(
    status: SafetyStatusV1,
    hard_failure_count: usize,
    warning_count: usize,
) -> String {
    match status {
        SafetyStatusV1::Safe => "deployment inventory matches the checked plan".to_string(),
        SafetyStatusV1::Warning => {
            format!("deployment inventory has {warning_count} warning(s)")
        }
        SafetyStatusV1::Blocked => {
            format!(
                "deployment inventory has {hard_failure_count} blocking issue(s) and {warning_count} warning(s)"
            )
        }
        SafetyStatusV1::NotEvaluated => "deployment safety has not been evaluated".to_string(),
    }
}

fn safety_next_actions(status: SafetyStatusV1) -> Vec<String> {
    match status {
        SafetyStatusV1::Safe => Vec::new(),
        SafetyStatusV1::Warning => {
            vec!["review deployment warnings before continuing".to_string()]
        }
        SafetyStatusV1::Blocked => {
            vec!["resolve blocking deployment truth differences before mutation".to_string()]
        }
        SafetyStatusV1::NotEvaluated => vec!["collect deployment inventory".to_string()],
    }
}
