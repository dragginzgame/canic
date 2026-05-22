use super::*;
use std::collections::{BTreeMap, BTreeSet};

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
    let diff = compare_plan_to_inventory(&plan, &inventory);
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

/// Compare intended deployment state with observed inventory into a machine diff.
#[must_use]
pub fn compare_plan_to_inventory(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
) -> DeploymentDiffV1 {
    let mut artifact_diff = Vec::new();
    let mut controller_diff = Vec::new();
    let pool_diff = Vec::new();
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
    compare_canisters(
        plan,
        inventory,
        &mut controller_diff,
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
    compare_verifier_readiness(plan, inventory, &mut verifier_readiness_diff, &mut warnings);
    for assumption in &plan.unresolved_assumptions {
        warnings.push(SafetyFindingV1 {
            code: "plan_assumption".to_string(),
            message: assumption.description.clone(),
            severity: SafetySeverityV1::Warning,
            subject: Some(assumption.key.clone()),
        });
    }
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
    let observed_by_role = inventory
        .observed_artifacts
        .iter()
        .map(|artifact| (artifact.role.as_str(), artifact))
        .collect::<BTreeMap<_, _>>();

    for expected in &plan.role_artifacts {
        let Some(observed) = observed_by_role.get(expected.role.as_str()) else {
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
            continue;
        };

        compare_artifact_file_sha256(expected, observed, artifact_diff, hard_failures);

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

fn compare_canisters(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    for expected in &plan.expected_canisters {
        let observed = expected.canister_id.as_ref().map_or_else(
            || {
                inventory
                    .observed_canisters
                    .iter()
                    .find(|canister| canister.role.as_deref() == Some(expected.role.as_str()))
            },
            |id| {
                inventory
                    .observed_canisters
                    .iter()
                    .find(|canister| &canister.canister_id == id)
            },
        );
        let Some(observed) = observed else {
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
            continue;
        };
        if matches!(
            observed.control_class,
            CanisterControlClassV1::UnknownUnsafe | CanisterControlClassV1::UserControlled
        ) && expected.control_class == CanisterControlClassV1::DeploymentControlled
        {
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
        compare_role_controllers(plan, observed, controller_diff, hard_failures, warnings);
    }
}

fn compare_role_controllers(
    plan: &DeploymentPlanV1,
    observed: &ObservedCanisterV1,
    controller_diff: &mut Vec<DiffItemV1>,
    hard_failures: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    let role = observed.role.as_deref().unwrap_or("unknown");
    for expected in &plan.authority_profile.expected_controllers {
        if observed
            .controllers
            .iter()
            .any(|controller| controller == expected)
        {
            continue;
        }
        controller_diff.push(diff_item(
            "controller_missing",
            role,
            Some(expected.clone()),
            Some(controller_set_label(&observed.controllers)),
            SafetySeverityV1::HardFailure,
        ));
        hard_failures.push(finding(
            "expected_controller_missing",
            format!("role {role} is missing expected controller {expected}"),
            SafetySeverityV1::HardFailure,
            Some(role.to_string()),
        ));
    }

    for observed_controller in &observed.controllers {
        if is_declared_controller(plan, observed_controller) {
            continue;
        }
        controller_diff.push(diff_item(
            "controller_extra",
            role,
            Some(controller_set_label(
                &plan.authority_profile.expected_controllers,
            )),
            Some(observed_controller.clone()),
            SafetySeverityV1::Warning,
        ));
        warnings.push(finding(
            "extra_controller_observed",
            format!("role {role} has controller outside the expected authority profile"),
            SafetySeverityV1::Warning,
            Some(role.to_string()),
        ));
    }
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
    let observed_by_role = inventory
        .observed_canisters
        .iter()
        .filter_map(|canister| canister.role.as_deref().map(|role| (role, canister)))
        .collect::<BTreeMap<_, _>>();

    for artifact in &plan.role_artifacts {
        let Some(expected) = artifact.installed_module_hash.as_ref() else {
            continue;
        };
        let Some(observed_canister) = observed_by_role.get(artifact.role.as_str()) else {
            continue;
        };
        match observed_canister.module_hash.as_ref() {
            Some(observed) if observed != expected => {
                module_hash_diff.push(diff_item(
                    "installed_module_hash",
                    &artifact.role,
                    Some(expected.clone()),
                    Some(observed.clone()),
                    SafetySeverityV1::HardFailure,
                ));
                hard_failures.push(finding(
                    "installed_module_hash_mismatch",
                    format!("installed module hash differs for role {}", artifact.role),
                    SafetySeverityV1::HardFailure,
                    Some(artifact.role.clone()),
                ));
            }
            None => warnings.push(finding(
                "installed_module_hash_unobserved",
                format!(
                    "installed module hash was not observed for role {}",
                    artifact.role
                ),
                SafetySeverityV1::Warning,
                Some(artifact.role.clone()),
            )),
            _ => {}
        }
    }
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
        embedded_config_diff.push(diff_item(
            "raw_config_sha256",
            "deployment",
            Some((*expected).clone()),
            Some(observed.clone()),
            SafetySeverityV1::HardFailure,
        ));
        hard_failures.push(finding(
            "raw_config_digest_mismatch",
            "raw local config digest changed during deployment truth check",
            SafetySeverityV1::HardFailure,
            Some("local_config.raw_sha256".to_string()),
        ));
    }
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
            embedded_config_diff.push(diff_item(
                "canonical_config",
                "deployment",
                Some(expected.clone()),
                Some(observed.clone()),
                SafetySeverityV1::HardFailure,
            ));
            hard_failures.push(finding(
                "canonical_config_mismatch",
                "canonical runtime config digest differs from the plan",
                SafetySeverityV1::HardFailure,
                Some("local_config".to_string()),
            ));
        }
        None => warnings.push(finding(
            "canonical_config_unobserved",
            "canonical runtime config digest was not observed",
            SafetySeverityV1::Warning,
            Some("local_config".to_string()),
        )),
        _ => {}
    }
}

fn compare_verifier_readiness(
    plan: &DeploymentPlanV1,
    inventory: &DeploymentInventoryV1,
    verifier_readiness_diff: &mut Vec<DiffItemV1>,
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
