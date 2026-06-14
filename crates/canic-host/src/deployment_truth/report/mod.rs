use super::*;
use std::collections::{BTreeMap, BTreeSet};

mod receipt_resume;
mod root_subnet;

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

fn compare_authority_profile(
    plan: &DeploymentPlanV1,
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    let mut reported = BTreeSet::new();
    for controller in &plan.authority_profile.expected_controllers {
        if !is_staging_or_emergency_controller(plan, controller) {
            continue;
        }
        if !reported.insert(controller.as_str()) {
            continue;
        }
        controller_diff.push(diff_item(
            "controller_authority_overlap",
            "authority_profile",
            Some("expected-only".to_string()),
            Some(controller.clone()),
            SafetySeverityV1::HardFailure,
        ));
        hard_failures.push(finding(
            "controller_authority_overlap",
            format!(
                "controller {controller} appears in both expected and staging/emergency authority"
            ),
            SafetySeverityV1::HardFailure,
            Some("authority_profile".to_string()),
        ));
    }
}

fn compare_artifacts(
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
                "planned_artifact_role_conflict",
                &group.subject,
                Some("one planned artifact".to_string()),
                Some(group.evidence_label.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                "planned_artifact_role_conflict",
                format!(
                    "planned artifact role {} has conflicting evidence: {}",
                    group.subject, group.evidence_label
                ),
                SafetySeverityV1::HardFailure,
                Some(group.subject),
            ));
        } else {
            artifact_diff.push(diff_item(
                "planned_artifact_duplicate",
                &group.subject,
                Some(group.evidence_label.clone()),
                Some(group.count.to_string()),
                SafetySeverityV1::Warning,
            ));
            warnings.push(finding(
                "duplicate_planned_artifact_role",
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
                "artifact_role_conflict",
                &group.subject,
                Some("one artifact observation".to_string()),
                Some(group.evidence_label.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                "artifact_role_conflict",
                format!(
                    "observed artifact role {} has conflicting evidence: {}",
                    group.subject, group.evidence_label
                ),
                SafetySeverityV1::HardFailure,
                Some(group.subject),
            ));
        } else {
            artifact_diff.push(diff_item(
                "artifact_duplicate",
                &group.subject,
                Some(group.evidence_label.clone()),
                Some(group.count.to_string()),
                SafetySeverityV1::Warning,
            ));
            warnings.push(finding(
                "duplicate_artifact_observed",
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
        "artifact",
        &expected.role,
        expected.wasm_gz_path.clone(),
        None,
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        "artifact_missing",
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
                "artifact_file_sha256",
                &expected.role,
                Some(want.clone()),
                Some(got.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                "artifact_file_digest_mismatch",
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
                "artifact_file_sha256",
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
                "artifact_sha256",
                &expected.role,
                Some(want.clone()),
                Some(got.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                "artifact_digest_mismatch",
                format!("artifact digest mismatch for role {}", expected.role),
                SafetySeverityV1::HardFailure,
                Some(expected.role.clone()),
            ));
        }
        (Some(want), None) => warnings.push(finding(
            "artifact_digest_unobserved",
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

fn compare_observed_canister_id_conflicts(
    inventory: &DeploymentInventoryV1,
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    for group in duplicate_evidence_groups(
        &inventory.observed_canisters,
        |observed| observed.canister_id.as_str().to_string(),
        observed_role_label,
        ",",
    ) {
        if group.is_conflict {
            controller_diff.push(diff_item(
                "canister_id_role_conflict",
                &group.subject,
                None,
                Some(group.evidence_label.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                "canister_id_role_conflict",
                format!(
                    "observed canister {} has conflicting roles {}",
                    group.subject, group.evidence_label
                ),
                SafetySeverityV1::HardFailure,
                Some(group.subject),
            ));
        } else {
            controller_diff.push(diff_item(
                "canister_duplicate",
                &group.subject,
                Some(group.evidence_label.clone()),
                Some(group.count.to_string()),
                SafetySeverityV1::Warning,
            ));
            warnings.push(finding(
                "duplicate_canister_observed",
                format!(
                    "observed canister {} was reported {} times for role {}",
                    group.subject, group.count, group.evidence_label
                ),
                SafetySeverityV1::Warning,
                Some(group.subject),
            ));
        }
    }
}

fn observed_role_label(observed: &ObservedCanisterV1) -> String {
    observed
        .role
        .clone()
        .unwrap_or_else(|| "<unknown>".to_string())
}

fn compare_canisters(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    let planned_conflicts =
        compare_planned_canister_conflicts(plan, controller_diff, hard_failures, warnings);
    let mut matched_observed = BTreeSet::new();
    let mut compared_planned = BTreeSet::new();
    for expected in &plan.expected_canisters {
        if planned_conflicts.role_conflicts.contains(&expected.role)
            || expected
                .canister_id
                .as_ref()
                .is_some_and(|id| planned_conflicts.id_conflicts.contains(id))
            || !compared_planned.insert(planned_canister_evidence_label(expected))
        {
            continue;
        }
        let observed = expected.canister_id.as_ref().map_or_else(
            || {
                let role_matches = inventory
                    .observed_canisters
                    .iter()
                    .filter(|canister| canister.role.as_deref() == Some(expected.role.as_str()))
                    .collect::<Vec<_>>();
                if role_matches.len() > 1 {
                    record_ambiguous_canister_role(
                        expected,
                        &role_matches,
                        controller_diff,
                        hard_failures,
                    );
                    None
                } else {
                    role_matches.into_iter().next()
                }
            },
            |id| {
                inventory
                    .observed_canisters
                    .iter()
                    .find(|canister| &canister.canister_id == id)
            },
        );
        let Some(observed) = observed else {
            record_missing_canister(expected, controller_diff, hard_failures, warnings);
            continue;
        };
        matched_observed.insert(observed.canister_id.as_str());
        compare_observed_role(expected, observed, controller_diff, hard_failures);
        record_unsafe_canister_control_class(expected, observed, controller_diff, hard_failures);
        compare_role_controllers(plan, observed, controller_diff, hard_failures, warnings);
    }
    warn_extra_observed_canisters(
        plan,
        inventory,
        controller_diff,
        warnings,
        &matched_observed,
    );
}

struct PlannedCanisterConflicts {
    role_conflicts: BTreeSet<String>,
    id_conflicts: BTreeSet<String>,
}

fn compare_planned_canister_conflicts(
    plan: &DeploymentPlanV1,
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) -> PlannedCanisterConflicts {
    let mut role_conflicts = BTreeSet::new();
    let mut id_conflicts = BTreeSet::new();

    for group in duplicate_evidence_groups(
        &plan.expected_canisters,
        |planned| planned.role.as_str().to_string(),
        planned_canister_evidence_label,
        " | ",
    ) {
        if group.is_conflict {
            role_conflicts.insert(group.subject.clone());
            controller_diff.push(diff_item(
                "planned_canister_role_conflict",
                &group.subject,
                Some("one planned canister".to_string()),
                Some(group.evidence_label.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                "planned_canister_role_conflict",
                format!(
                    "planned canister role {} has conflicting evidence: {}",
                    group.subject, group.evidence_label
                ),
                SafetySeverityV1::HardFailure,
                Some(group.subject),
            ));
        } else {
            controller_diff.push(diff_item(
                "planned_canister_duplicate",
                &group.subject,
                Some(group.evidence_label.clone()),
                Some(group.count.to_string()),
                SafetySeverityV1::Warning,
            ));
            warnings.push(finding(
                "duplicate_planned_canister_role",
                format!(
                    "planned canister role {} was declared {} times with identical evidence",
                    group.subject, group.count
                ),
                SafetySeverityV1::Warning,
                Some(group.subject),
            ));
        }
    }

    for group in conflicting_assignment_groups(
        &plan.expected_canisters,
        |planned| planned.canister_id.clone(),
        |planned| planned.role.clone(),
        ",",
    ) {
        id_conflicts.insert(group.subject.clone());
        controller_diff.push(diff_item(
            "planned_canister_id_conflict",
            &group.subject,
            Some("one planned role".to_string()),
            Some(group.evidence_label.clone()),
            SafetySeverityV1::HardFailure,
        ));
        hard_failures.push(finding(
            "planned_canister_id_conflict",
            format!(
                "planned canister id {} is assigned to conflicting roles {}",
                group.subject, group.evidence_label
            ),
            SafetySeverityV1::HardFailure,
            Some(group.subject),
        ));
    }

    PlannedCanisterConflicts {
        role_conflicts,
        id_conflicts,
    }
}

fn planned_canister_evidence_label(planned: &ExpectedCanisterV1) -> String {
    format!(
        "role={};id={};control={:?}",
        planned.role,
        planned.canister_id.as_deref().unwrap_or("<none>"),
        planned.control_class
    )
}

fn record_missing_canister(
    expected: &ExpectedCanisterV1,
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    let severity = if expected.canister_id.is_some() {
        SafetySeverityV1::HardFailure
    } else {
        SafetySeverityV1::Warning
    };
    controller_diff.push(diff_item(
        "canister",
        &expected.role,
        expected.canister_id.clone(),
        None,
        severity,
    ));
    let finding = finding(
        if expected.canister_id.is_some() {
            "canister_missing"
        } else {
            "canister_unobserved"
        },
        format!("missing observed canister for role {}", expected.role),
        severity,
        Some(expected.role.clone()),
    );
    if expected.canister_id.is_some() {
        hard_failures.push(finding);
    } else {
        warnings.push(finding);
    }
}

fn record_unsafe_canister_control_class(
    expected: &ExpectedCanisterV1,
    observed: &ObservedCanisterV1,
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    if !matches!(
        observed.control_class,
        CanisterControlClassV1::UnknownUnsafe | CanisterControlClassV1::UserControlled
    ) || expected.control_class != CanisterControlClassV1::DeploymentControlled
    {
        return;
    }
    controller_diff.push(diff_item(
        "control_class",
        &expected.role,
        Some("DeploymentControlled".to_string()),
        Some(format!("{:?}", observed.control_class)),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        "unsafe_control_class",
        format!("role {} has unsafe observed control class", expected.role),
        SafetySeverityV1::HardFailure,
        Some(expected.role.clone()),
    ));
}

fn record_ambiguous_canister_role(
    expected: &ExpectedCanisterV1,
    observed_matches: &[&ObservedCanisterV1],
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    let observed_ids = observed_matches
        .iter()
        .map(|canister| canister.canister_id.as_str())
        .collect::<Vec<_>>()
        .join(",");
    controller_diff.push(diff_item(
        "canister_role_ambiguous",
        &expected.role,
        Some("one observed canister".to_string()),
        Some(observed_ids.clone()),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        "canister_role_ambiguous",
        format!(
            "expected role {} has multiple observed canisters: {observed_ids}",
            expected.role
        ),
        SafetySeverityV1::HardFailure,
        Some(expected.role.clone()),
    ));
}

fn compare_observed_role(
    expected: &ExpectedCanisterV1,
    observed: &ObservedCanisterV1,
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    let Some(observed_role) = observed.role.as_deref() else {
        return;
    };
    if observed_role == expected.role {
        return;
    }
    controller_diff.push(diff_item(
        "role_mismatch",
        &expected.role,
        Some(expected.role.clone()),
        Some(observed_role.to_string()),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        "canister_role_mismatch",
        format!(
            "expected canister {} to have role {}, observed role {observed_role}",
            observed.canister_id, expected.role
        ),
        SafetySeverityV1::HardFailure,
        Some(expected.role.clone()),
    ));
}

fn warn_extra_observed_canisters(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
    controller_diff: &mut Vec<DiffItemV1>,
    warnings: &mut Vec<SafetyFindingV1>,
    matched_observed: &BTreeSet<&str>,
) {
    let expected_pool_roles = plan
        .expected_pool
        .iter()
        .filter_map(|pool| pool.role.as_deref())
        .collect::<BTreeSet<_>>();

    for observed in &inventory.observed_canisters {
        if matched_observed.contains(observed.canister_id.as_str()) {
            continue;
        }
        if let Some(role) = observed.role.as_deref()
            && expected_pool_roles.contains(role)
        {
            continue;
        }
        let subject = observed_canister_subject(observed);
        controller_diff.push(diff_item(
            "canister_extra",
            &subject,
            None,
            Some(observed.canister_id.clone()),
            SafetySeverityV1::Warning,
        ));
        warnings.push(finding(
            "extra_canister_observed",
            format!("observed undeclared canister {subject}"),
            SafetySeverityV1::Warning,
            Some(subject),
        ));
    }
}

fn observed_canister_subject(observed: &ObservedCanisterV1) -> String {
    observed
        .role
        .clone()
        .unwrap_or_else(|| observed.canister_id.clone())
}

fn compare_observed_canister_pool_role_conflicts(
    inventory: &DeploymentInventoryV1,
    pool_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    let mut pools_by_id = BTreeMap::<&str, Vec<&ObservedPoolCanisterV1>>::new();
    for observed_pool in &inventory.observed_pool {
        pools_by_id
            .entry(observed_pool.canister_id.as_str())
            .or_default()
            .push(observed_pool);
    }

    for observed_canister in &inventory.observed_canisters {
        let Some(canister_role) = observed_canister.role.as_deref() else {
            continue;
        };
        let Some(observed_pools) = pools_by_id.get(observed_canister.canister_id.as_str()) else {
            continue;
        };
        for observed_pool in observed_pools {
            let Some(pool_role) = observed_pool.role.as_deref() else {
                continue;
            };
            if pool_role == canister_role {
                continue;
            }
            let observed_label = format!(
                "canister={};pool={}",
                observed_canister_subject(observed_canister),
                observed_pool_subject(observed_pool)
            );
            pool_diff.push(diff_item(
                "canister_pool_role_conflict",
                &observed_canister.canister_id,
                None,
                Some(observed_label.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                "canister_pool_role_conflict",
                format!(
                    "observed canister {} has conflicting canister/pool roles {observed_label}",
                    observed_canister.canister_id
                ),
                SafetySeverityV1::HardFailure,
                Some(observed_canister.canister_id.clone()),
            ));
        }
    }
}

fn compare_pools(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
    pool_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    let planned_conflicts =
        compare_planned_pool_conflicts(plan, pool_diff, hard_failures, warnings);
    compare_observed_pool_id_conflicts(inventory, pool_diff, hard_failures, warnings);
    let mut matched_observed = BTreeSet::new();
    let mut compared_planned = BTreeSet::new();
    for expected in &plan.expected_pool {
        if planned_conflicts
            .subject_conflicts
            .contains(&expected_pool_subject(expected))
            || expected
                .canister_id
                .as_ref()
                .is_some_and(|id| planned_conflicts.id_conflicts.contains(id))
            || !compared_planned.insert(planned_pool_evidence_label(expected))
        {
            continue;
        }
        compare_expected_pool(
            expected,
            inventory,
            pool_diff,
            hard_failures,
            warnings,
            &mut matched_observed,
        );
    }

    for observed in &inventory.observed_pool {
        warn_extra_observed_pool(plan, observed, pool_diff, warnings, &matched_observed);
    }
}

struct PlannedPoolConflicts {
    subject_conflicts: BTreeSet<String>,
    id_conflicts: BTreeSet<String>,
}

fn compare_planned_pool_conflicts(
    plan: &DeploymentPlanV1,
    pool_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) -> PlannedPoolConflicts {
    let mut subject_conflicts = BTreeSet::new();
    let mut id_conflicts = BTreeSet::new();

    for group in duplicate_evidence_groups(
        &plan.expected_pool,
        expected_pool_subject,
        planned_pool_evidence_label,
        " | ",
    ) {
        if group.is_conflict {
            subject_conflicts.insert(group.subject.clone());
            pool_diff.push(diff_item(
                "planned_pool_conflict",
                &group.subject,
                Some("one planned pool canister".to_string()),
                Some(group.evidence_label.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                "planned_pool_conflict",
                format!(
                    "planned pool {} has conflicting evidence: {}",
                    group.subject, group.evidence_label
                ),
                SafetySeverityV1::HardFailure,
                Some(group.subject),
            ));
        } else {
            pool_diff.push(diff_item(
                "planned_pool_duplicate",
                &group.subject,
                Some(group.evidence_label.clone()),
                Some(group.count.to_string()),
                SafetySeverityV1::Warning,
            ));
            warnings.push(finding(
                "duplicate_planned_pool",
                format!(
                    "planned pool {} was declared {} times with identical evidence",
                    group.subject, group.count
                ),
                SafetySeverityV1::Warning,
                Some(group.subject),
            ));
        }
    }

    for group in conflicting_assignment_groups(
        &plan.expected_pool,
        |planned| planned.canister_id.clone(),
        expected_pool_subject,
        ",",
    ) {
        id_conflicts.insert(group.subject.clone());
        pool_diff.push(diff_item(
            "planned_pool_id_conflict",
            &group.subject,
            Some("one planned pool identity".to_string()),
            Some(group.evidence_label.clone()),
            SafetySeverityV1::HardFailure,
        ));
        hard_failures.push(finding(
            "planned_pool_id_conflict",
            format!(
                "planned pool id {} is assigned to conflicting identities {}",
                group.subject, group.evidence_label
            ),
            SafetySeverityV1::HardFailure,
            Some(group.subject),
        ));
    }

    PlannedPoolConflicts {
        subject_conflicts,
        id_conflicts,
    }
}

fn planned_pool_evidence_label(planned: &ExpectedPoolCanisterV1) -> String {
    format!(
        "pool={};role={};id={}",
        planned.pool,
        planned.role.as_deref().unwrap_or("<none>"),
        planned.canister_id.as_deref().unwrap_or("<none>")
    )
}

fn compare_observed_pool_id_conflicts(
    inventory: &DeploymentInventoryV1,
    pool_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    for group in duplicate_evidence_groups(
        &inventory.observed_pool,
        |observed| observed.canister_id.as_str().to_string(),
        observed_pool_subject,
        ",",
    ) {
        if group.is_conflict {
            pool_diff.push(diff_item(
                "pool_canister_id_conflict",
                &group.subject,
                None,
                Some(group.evidence_label.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                "pool_canister_id_conflict",
                format!(
                    "observed pool canister {} has conflicting pool identities {}",
                    group.subject, group.evidence_label
                ),
                SafetySeverityV1::HardFailure,
                Some(group.subject),
            ));
        } else {
            pool_diff.push(diff_item(
                "pool_canister_duplicate",
                &group.subject,
                Some(group.evidence_label.clone()),
                Some(group.count.to_string()),
                SafetySeverityV1::Warning,
            ));
            warnings.push(finding(
                "duplicate_pool_canister_observed",
                format!(
                    "observed pool canister {} was reported {} times for {}",
                    group.subject, group.count, group.evidence_label
                ),
                SafetySeverityV1::Warning,
                Some(group.subject),
            ));
        }
    }
}

fn compare_expected_pool<'a>(
    expected: &ExpectedPoolCanisterV1,
    inventory: &'a DeploymentInventoryV1,
    pool_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
    matched_observed: &mut BTreeSet<&'a str>,
) {
    let observed = expected
        .canister_id
        .as_ref()
        .and_then(|id| {
            inventory
                .observed_pool
                .iter()
                .find(|pool| &pool.canister_id == id)
        })
        .or_else(|| {
            inventory
                .observed_pool
                .iter()
                .find(|pool| pool_matches_expected_pool(pool, expected))
        });
    let Some(observed) = observed else {
        record_missing_pool(expected, pool_diff, hard_failures, warnings);
        return;
    };

    matched_observed.insert(observed.canister_id.as_str());
    record_pool_id_mismatch(expected, observed, pool_diff, hard_failures);
    record_unsafe_pool_control_class(observed, pool_diff, hard_failures);
}

fn record_missing_pool(
    expected: &ExpectedPoolCanisterV1,
    pool_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    let severity = if expected.canister_id.is_some() {
        SafetySeverityV1::HardFailure
    } else {
        SafetySeverityV1::Warning
    };
    let subject = expected_pool_subject(expected);
    pool_diff.push(diff_item(
        "pool_canister",
        &subject,
        expected.canister_id.clone(),
        None,
        severity,
    ));
    let finding = finding(
        if expected.canister_id.is_some() {
            "pool_canister_missing"
        } else {
            "pool_canister_unobserved"
        },
        format!("missing observed pool canister for {subject}"),
        severity,
        Some(subject),
    );
    if expected.canister_id.is_some() {
        hard_failures.push(finding);
    } else {
        warnings.push(finding);
    }
}

fn record_pool_id_mismatch(
    expected: &ExpectedPoolCanisterV1,
    observed: &ObservedPoolCanisterV1,
    pool_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    if let Some(expected_id) = expected.canister_id.as_ref()
        && observed.canister_id != *expected_id
    {
        let subject = observed_pool_subject(observed);
        pool_diff.push(diff_item(
            "pool_canister_id",
            &subject,
            Some(expected_id.clone()),
            Some(observed.canister_id.clone()),
            SafetySeverityV1::HardFailure,
        ));
        hard_failures.push(finding(
            "pool_canister_id_mismatch",
            format!(
                "pool canister {subject} has observed id {}, expected {expected_id}",
                observed.canister_id
            ),
            SafetySeverityV1::HardFailure,
            Some(subject),
        ));
    }
}

fn record_unsafe_pool_control_class(
    observed: &ObservedPoolCanisterV1,
    pool_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    if !matches!(
        observed.control_class,
        CanisterControlClassV1::UnknownUnsafe | CanisterControlClassV1::UserControlled
    ) {
        return;
    }
    let subject = observed_pool_subject(observed);
    pool_diff.push(diff_item(
        "pool_control_class",
        &subject,
        Some("CanicManagedPool".to_string()),
        Some(format!("{:?}", observed.control_class)),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        "unsafe_pool_control_class",
        format!("pool canister {subject} has unsafe observed control class"),
        SafetySeverityV1::HardFailure,
        Some(subject),
    ));
}

fn warn_extra_observed_pool(
    plan: &DeploymentPlanV1,
    observed: &ObservedPoolCanisterV1,
    pool_diff: &mut Vec<DiffItemV1>,
    warnings: &mut Vec<SafetyFindingV1>,
    matched_observed: &BTreeSet<&str>,
) {
    if matched_observed.contains(observed.canister_id.as_str())
        || plan.expected_pool.iter().any(|expected| {
            expected.canister_id.as_ref() == Some(&observed.canister_id)
                || pool_matches_expected_pool(observed, expected)
        })
    {
        return;
    }
    let subject = observed_pool_subject(observed);
    pool_diff.push(diff_item(
        "pool_extra",
        &subject,
        None,
        Some(observed.canister_id.clone()),
        SafetySeverityV1::Warning,
    ));
    warnings.push(finding(
        "extra_pool_canister_observed",
        format!("observed undeclared pool canister {subject}"),
        SafetySeverityV1::Warning,
        Some(subject),
    ));
}

fn pool_matches_expected_pool(
    observed: &ObservedPoolCanisterV1,
    expected: &ExpectedPoolCanisterV1,
) -> bool {
    observed.pool == expected.pool
        && expected
            .role
            .as_ref()
            .is_none_or(|role| observed.role.as_ref() == Some(role))
}

fn expected_pool_subject(expected: &ExpectedPoolCanisterV1) -> String {
    expected.role.as_ref().map_or_else(
        || expected.pool.clone(),
        |role| format!("{}:{role}", expected.pool),
    )
}

fn observed_pool_subject(observed: &ObservedPoolCanisterV1) -> String {
    observed.role.as_ref().map_or_else(
        || observed.pool.clone(),
        |role| format!("{}:{role}", observed.pool),
    )
}

fn compare_role_controllers(
    plan: &DeploymentPlanV1,
    observed: &ObservedCanisterV1,
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    let role = observed.role.as_deref().unwrap_or("unknown");
    if observed.controllers.is_empty() && !observed_source_includes_live_status(observed) {
        warnings.push(finding(
            "controllers_unobserved",
            format!("controllers were not observed for role {role}"),
            SafetySeverityV1::Warning,
            Some(role.to_string()),
        ));
        return;
    }
    for expected in &plan.authority_profile.expected_controllers {
        if observed
            .controllers
            .iter()
            .any(|controller| controller == expected)
        {
            continue;
        }
        record_missing_expected_controller(
            role,
            expected,
            &observed.controllers,
            controller_diff,
            hard_failures,
        );
    }

    for observed_controller in &observed.controllers {
        if is_declared_controller(plan, observed_controller) {
            continue;
        }
        record_extra_controller(role, observed_controller, plan, controller_diff, warnings);
    }
}

fn record_missing_expected_controller(
    role: &str,
    expected: &str,
    observed_controllers: &[String],
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    controller_diff.push(diff_item(
        "controller_missing",
        role,
        Some(expected.to_string()),
        Some(controller_set_label(observed_controllers)),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        "expected_controller_missing",
        format!("role {role} is missing expected controller {expected}"),
        SafetySeverityV1::HardFailure,
        Some(role.to_string()),
    ));
}

fn record_extra_controller(
    role: &str,
    observed_controller: &str,
    plan: &DeploymentPlanV1,
    controller_diff: &mut Vec<DiffItemV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    controller_diff.push(diff_item(
        "controller_extra",
        role,
        Some(controller_set_label(
            &plan.authority_profile.expected_controllers,
        )),
        Some(observed_controller.to_string()),
        SafetySeverityV1::Warning,
    ));
    warnings.push(finding(
        "extra_controller_observed",
        format!("role {role} has controller outside the expected authority profile"),
        SafetySeverityV1::Warning,
        Some(role.to_string()),
    ));
}

fn observed_source_includes_live_status(observed: &ObservedCanisterV1) -> bool {
    observed
        .role_assignment_source
        .as_deref()
        .is_some_and(|source| source.contains("icp_canister_status"))
}

fn is_declared_controller(plan: &DeploymentPlanV1, controller: &str) -> bool {
    plan.authority_profile
        .expected_controllers
        .iter()
        .chain(plan.authority_profile.staging_controllers.iter())
        .chain(plan.authority_profile.emergency_controllers.iter())
        .any(|expected| expected == controller)
}

fn is_staging_or_emergency_controller(plan: &DeploymentPlanV1, controller: &str) -> bool {
    plan.authority_profile
        .staging_controllers
        .iter()
        .chain(plan.authority_profile.emergency_controllers.iter())
        .any(|declared| declared == controller)
}

fn controller_set_label(controllers: &[String]) -> String {
    if controllers.is_empty() {
        return "<none>".to_string();
    }
    controllers.join(",")
}

fn compare_module_hashes(
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
        "installed_module_hash",
        role,
        Some(expected.to_string()),
        Some(observed.to_string()),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        "installed_module_hash_mismatch",
        format!("installed module hash differs for role {role}"),
        SafetySeverityV1::HardFailure,
        Some(role.to_string()),
    ));
}

fn record_module_hash_unobserved(role: &str, warnings: &mut Vec<SafetyFindingV1>) {
    warnings.push(finding(
        "installed_module_hash_unobserved",
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
        "installed_module_hash_ambiguous",
        role,
        Some("one observed canister".to_string()),
        Some(observed_ids.clone()),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        "installed_module_hash_ambiguous",
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

fn compare_raw_config(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
    embedded_config_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    let mut expected = plan
        .role_artifacts
        .iter()
        .filter_map(|artifact| artifact.raw_config_sha256.as_ref())
        .collect::<Vec<_>>();
    expected.sort_unstable();
    expected.dedup();
    let [expected] = expected.as_slice() else {
        if expected.len() > 1 {
            hard_failures.push(finding(
                "raw_config_plan_inconsistent",
                "planned role artifacts disagree on raw config digest",
                SafetySeverityV1::HardFailure,
                Some("role_artifacts.raw_config_sha256".to_string()),
            ));
        }
        return;
    };

    if let Some(observed) = &inventory.local_config.raw_config_sha256
        && observed != *expected
    {
        record_raw_config_mismatch(expected, observed, embedded_config_diff, hard_failures);
    }
}

fn record_raw_config_mismatch(
    expected: &str,
    observed: &str,
    embedded_config_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    embedded_config_diff.push(diff_item(
        "raw_config_sha256",
        "deployment",
        Some(expected.to_string()),
        Some(observed.to_string()),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        "raw_config_digest_mismatch",
        "raw local config digest changed during deployment truth check",
        SafetySeverityV1::HardFailure,
        Some("local_config.raw_sha256".to_string()),
    ));
}

fn compare_embedded_config(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
    embedded_config_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    let Some(expected) = &plan.deployment_identity.canonical_runtime_config_digest else {
        return;
    };
    match &inventory.local_config.canonical_embedded_config_sha256 {
        Some(observed) if observed != expected => {
            record_canonical_config_mismatch(
                expected,
                observed,
                embedded_config_diff,
                hard_failures,
            );
        }
        None => record_canonical_config_unobserved(warnings),
        _ => {}
    }
}

fn record_canonical_config_mismatch(
    expected: &str,
    observed: &str,
    embedded_config_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
) {
    embedded_config_diff.push(diff_item(
        "canonical_config",
        "deployment",
        Some(expected.to_string()),
        Some(observed.to_string()),
        SafetySeverityV1::HardFailure,
    ));
    hard_failures.push(finding(
        "canonical_config_mismatch",
        "canonical runtime config digest differs from the plan",
        SafetySeverityV1::HardFailure,
        Some("local_config".to_string()),
    ));
}

fn record_canonical_config_unobserved(warnings: &mut Vec<SafetyFindingV1>) {
    warnings.push(finding(
        "canonical_config_unobserved",
        "canonical runtime config digest was not observed",
        SafetySeverityV1::Warning,
        Some("local_config".to_string()),
    ));
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
