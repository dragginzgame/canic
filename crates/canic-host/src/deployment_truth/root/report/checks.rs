use super::super::super::*;
use crate::deployment_truth::report::UNVERIFIED_DEPLOYMENT_ROOT_CODE;

pub(in crate::deployment_truth) const ROOT_VERIFICATION_CHECK_FAILED_CODE: &str =
    "root_verification_check_failed";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RootVerificationCheckName {
    DeploymentName,
    Environment,
    FleetTemplate,
    RootPrincipal,
    PlanDeploymentName,
    PlanEnvironment,
    PlanFleetTemplate,
    ExplicitObservedRoot,
    RootObservationSource,
    ObservedRootCanisterId,
    SourceCheckId,
    SourceDeploymentPlanId,
    SourceInventoryId,
}

impl RootVerificationCheckName {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::DeploymentName => "deployment_name",
            Self::Environment => "environment",
            Self::FleetTemplate => "fleet_template",
            Self::RootPrincipal => "root_principal",
            Self::PlanDeploymentName => "plan_deployment_name",
            Self::PlanEnvironment => "plan_environment",
            Self::PlanFleetTemplate => "plan_fleet_template",
            Self::ExplicitObservedRoot => "explicit_observed_root",
            Self::RootObservationSource => "root_observation_source",
            Self::ObservedRootCanisterId => "observed_root_canister_id",
            Self::SourceCheckId => "source_check_id",
            Self::SourceDeploymentPlanId => "source_deployment_plan_id",
            Self::SourceInventoryId => "source_inventory_id",
        }
    }
}

pub(super) fn root_verification_identity_checks(
    request: &DeploymentRootVerificationRequestV1,
    check: &DeploymentCheckV1,
    observed_root: Option<&DeploymentRootObservationV1>,
) -> Vec<DeploymentRootVerificationCheckV1> {
    let mut checks = Vec::new();
    push_check(
        &mut checks,
        RootVerificationCheckName::DeploymentName,
        Some(request.deployment_name.as_str()),
        observed_root.map(|root| root.deployment_name.as_str()),
    );
    push_check(
        &mut checks,
        RootVerificationCheckName::Environment,
        Some(request.environment.as_str()),
        observed_root.map(|root| root.environment.as_str()),
    );
    push_check(
        &mut checks,
        RootVerificationCheckName::FleetTemplate,
        Some(request.expected_fleet_template.as_str()),
        observed_root.map(|root| root.fleet_template.as_str()),
    );
    push_check(
        &mut checks,
        RootVerificationCheckName::RootPrincipal,
        Some(request.expected_root_principal.as_str()),
        observed_root.map(|root| root.root_principal.as_str()),
    );
    push_check(
        &mut checks,
        RootVerificationCheckName::PlanDeploymentName,
        Some(request.deployment_name.as_str()),
        Some(check.plan.deployment_identity.deployment_name.as_str()),
    );
    push_check(
        &mut checks,
        RootVerificationCheckName::PlanEnvironment,
        Some(request.environment.as_str()),
        Some(check.plan.deployment_identity.environment.as_str()),
    );
    push_check(
        &mut checks,
        RootVerificationCheckName::PlanFleetTemplate,
        Some(request.expected_fleet_template.as_str()),
        Some(check.plan.fleet_template.as_str()),
    );
    checks
}

pub(super) fn root_verification_evidence_checks(
    request: &DeploymentRootVerificationRequestV1,
    check: &DeploymentCheckV1,
    observed_root: Option<&DeploymentRootObservationV1>,
) -> Vec<DeploymentRootVerificationCheckV1> {
    let mut checks = Vec::new();
    push_check(
        &mut checks,
        RootVerificationCheckName::ExplicitObservedRoot,
        Some("present"),
        observed_root.map(|_| "present"),
    );
    push_check(
        &mut checks,
        RootVerificationCheckName::RootObservationSource,
        Some(DeploymentRootObservationSourceV1::IcpCanisterStatus.label()),
        observed_root.map(root_observation_source_label),
    );
    push_check(
        &mut checks,
        RootVerificationCheckName::ObservedRootCanisterId,
        Some(request.expected_root_principal.as_str()),
        observed_root.map(|root| root.observed_canister_id.as_str()),
    );
    push_check(
        &mut checks,
        RootVerificationCheckName::SourceCheckId,
        Some("present"),
        present_value(check.check_id.as_str()),
    );
    push_check(
        &mut checks,
        RootVerificationCheckName::SourceDeploymentPlanId,
        Some("present"),
        present_value(check.plan.plan_id.as_str()),
    );
    push_check(
        &mut checks,
        RootVerificationCheckName::SourceInventoryId,
        Some("present"),
        present_value(check.inventory.inventory_id.as_str()),
    );
    checks
}

pub(super) fn root_verification_blockers(
    identity_checks: &[DeploymentRootVerificationCheckV1],
    evidence_checks: &[DeploymentRootVerificationCheckV1],
    check: &DeploymentCheckV1,
) -> Vec<SafetyFindingV1> {
    let mut blockers = failed_checks("identity", identity_checks);
    blockers.extend(failed_checks("evidence", evidence_checks));
    blockers.extend(source_check_consistency_blockers(check));
    blockers.extend(source_check_blockers(check));
    blockers
}

fn push_check(
    checks: &mut Vec<DeploymentRootVerificationCheckV1>,
    name: RootVerificationCheckName,
    expected: Option<&str>,
    observed: Option<&str>,
) {
    checks.push(DeploymentRootVerificationCheckV1 {
        name: name.label().to_string(),
        expected: expected.map(str::to_string),
        observed: observed.map(str::to_string),
        satisfied: expected == observed,
    });
}

pub(super) const fn present_value(value: &str) -> Option<&'static str> {
    if value.is_empty() {
        None
    } else {
        Some("present")
    }
}

const fn root_observation_source_label(root: &DeploymentRootObservationV1) -> &str {
    root.observation_source.label()
}

fn failed_checks(
    category: &'static str,
    checks: &[DeploymentRootVerificationCheckV1],
) -> Vec<SafetyFindingV1> {
    checks
        .iter()
        .filter(|check| !check.satisfied)
        .map(|check| SafetyFindingV1 {
            code: ROOT_VERIFICATION_CHECK_FAILED_CODE.to_string(),
            message: format!("{category} check {} did not match", check.name),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(check.name.clone()),
        })
        .collect()
}

pub(in crate::deployment_truth) const ROOT_VERIFICATION_SOURCE_CHECK_SCHEMA_MISMATCH_CODE: &str =
    "root_verification_source_check_schema_mismatch";
pub(in crate::deployment_truth) const ROOT_VERIFICATION_SOURCE_CHECK_DIFF_STALE_CODE: &str =
    "root_verification_source_check_diff_stale";
pub(in crate::deployment_truth) const ROOT_VERIFICATION_SOURCE_CHECK_REPORT_STALE_CODE: &str =
    "root_verification_source_check_report_stale";

fn source_check_blockers(check: &DeploymentCheckV1) -> Vec<SafetyFindingV1> {
    let hard_failures = &check.report.hard_failures;
    if hard_failures.is_empty() {
        return Vec::new();
    }
    if hard_failures.len() == 1 && is_expected_unverified_root_finding(&hard_failures[0]) {
        return Vec::new();
    }
    hard_failures.clone()
}

fn source_check_consistency_blockers(check: &DeploymentCheckV1) -> Vec<SafetyFindingV1> {
    let mut blockers = Vec::new();
    if check.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        blockers.push(SafetyFindingV1 {
            code: ROOT_VERIFICATION_SOURCE_CHECK_SCHEMA_MISMATCH_CODE.to_string(),
            message: "source deployment check schema version is unsupported".to_string(),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(check.check_id.clone()),
        });
        return blockers;
    }

    let expected_diff = compare_plan_to_inventory(&check.plan, &check.inventory);
    if check.diff != expected_diff {
        blockers.push(SafetyFindingV1 {
            code: ROOT_VERIFICATION_SOURCE_CHECK_DIFF_STALE_CODE.to_string(),
            message: "source deployment check diff does not match its plan and inventory"
                .to_string(),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(check.check_id.clone()),
        });
        return blockers;
    }

    let expected_report = safety_report_from_diff(
        &check.report.report_id,
        check.report.diff_id.clone(),
        &check.diff,
    );
    if check.report != expected_report {
        blockers.push(SafetyFindingV1 {
            code: ROOT_VERIFICATION_SOURCE_CHECK_REPORT_STALE_CODE.to_string(),
            message: "source deployment check report does not match its diff".to_string(),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(check.check_id.clone()),
        });
    }
    blockers
}

fn is_expected_unverified_root_finding(finding: &SafetyFindingV1) -> bool {
    finding.code == UNVERIFIED_DEPLOYMENT_ROOT_CODE
        && finding.subject.as_deref() == Some("local_state.unverified_root_canister_id")
}
