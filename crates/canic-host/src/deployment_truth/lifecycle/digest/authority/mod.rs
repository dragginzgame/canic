use super::super::super::*;
use serde::Serialize;

#[derive(Serialize)]
struct LifecycleAuthorityReportDigestInput<'a> {
    report_id: &'a str,
    check_id: &'a str,
    plan_id: &'a str,
    inventory_id: &'a str,
    authorities: &'a [LifecycleAuthorityV1],
    external_action_required_count: usize,
    blocked_count: usize,
}

#[derive(Serialize)]
struct ExternalLifecyclePlanDigestInput<'a> {
    lifecycle_authority_report_id: &'a str,
    deployment_plan_id: &'a str,
    deployment_plan_digest: &'a str,
    inventory_id: &'a str,
    lifecycle_authority_rows: &'a [LifecycleAuthorityV1],
    directly_executable_role_upgrades: &'a [ExternalLifecycleRoleUpgradeV1],
    proposed_external_role_upgrades: &'a [ExternalLifecycleRoleUpgradeV1],
    blocked_role_upgrades: &'a [ExternalLifecycleRoleUpgradeV1],
    dependency_blockers: &'a [String],
    protected_call_implications: &'a [String],
    residual_exposure: &'a [String],
    status: ExternalLifecyclePlanStatusV1,
}

pub(in crate::deployment_truth::lifecycle) fn external_lifecycle_plan_digest(
    plan: &ExternalLifecyclePlanV1,
) -> String {
    stable_json_sha256_hex(&ExternalLifecyclePlanDigestInput {
        lifecycle_authority_report_id: &plan.lifecycle_authority_report_id,
        deployment_plan_id: &plan.deployment_plan_id,
        deployment_plan_digest: &plan.deployment_plan_digest,
        inventory_id: &plan.inventory_id,
        lifecycle_authority_rows: &plan.lifecycle_authority_rows,
        directly_executable_role_upgrades: &plan.directly_executable_role_upgrades,
        proposed_external_role_upgrades: &plan.proposed_external_role_upgrades,
        blocked_role_upgrades: &plan.blocked_role_upgrades,
        dependency_blockers: &plan.dependency_blockers,
        protected_call_implications: &plan.protected_call_implications,
        residual_exposure: &plan.residual_exposure,
        status: plan.status,
    })
}

pub(in crate::deployment_truth::lifecycle) fn lifecycle_authority_report_digest(
    report: &LifecycleAuthorityReportV1,
) -> String {
    stable_json_sha256_hex(&LifecycleAuthorityReportDigestInput {
        report_id: &report.report_id,
        check_id: &report.check_id,
        plan_id: &report.plan_id,
        inventory_id: &report.inventory_id,
        authorities: &report.authorities,
        external_action_required_count: report.external_action_required_count,
        blocked_count: report.blocked_count,
    })
}
