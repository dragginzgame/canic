use super::super::super::*;
use serde::Serialize;

#[derive(Serialize)]
struct ExternalLifecyclePendingReportDigestInput<'a> {
    report_id: &'a str,
    lifecycle_plan_id: &'a str,
    lifecycle_plan_digest: &'a str,
    proposal_report_id: &'a str,
    proposal_report_digest: &'a str,
    deployment_plan_id: &'a str,
    deployment_plan_digest: &'a str,
    inventory_id: &'a str,
    direct_upgrade_count: usize,
    pending_external_count: usize,
    blocked_count: usize,
    pending_external_actions: &'a [ExternalLifecyclePendingActionV1],
    blocked_subjects: &'a [String],
    residual_exposure: &'a [String],
    status: ExternalLifecyclePlanStatusV1,
}

#[derive(Serialize)]
struct ExternalLifecycleCheckDigestInput<'a> {
    check_id: &'a str,
    lifecycle_plan_id: &'a str,
    lifecycle_plan_digest: &'a str,
    proposal_report_id: &'a str,
    proposal_report_digest: &'a str,
    pending_report_id: &'a str,
    pending_report_digest: &'a str,
    deployment_plan_id: &'a str,
    deployment_plan_digest: &'a str,
    inventory_id: &'a str,
    status: ExternalLifecyclePlanStatusV1,
    direct_upgrade_count: usize,
    pending_external_count: usize,
    blocked_count: usize,
    residual_exposure_count: usize,
    summary: &'a str,
    next_actions: &'a [String],
}

#[derive(Serialize)]
struct ExternalLifecycleHandoffDigestInput<'a> {
    handoff_id: &'a str,
    lifecycle_check_id: &'a str,
    lifecycle_check_digest: &'a str,
    pending_report_id: &'a str,
    pending_report_digest: &'a str,
    proposal_report_id: &'a str,
    proposal_report_digest: &'a str,
    deployment_plan_id: &'a str,
    deployment_plan_digest: &'a str,
    inventory_id: &'a str,
    status: ExternalLifecyclePlanStatusV1,
    handoff_actions: &'a [ExternalLifecycleHandoffActionV1],
    blocked_subjects: &'a [String],
    residual_exposure: &'a [String],
    operator_summary: &'a str,
}

#[derive(Serialize)]
struct CriticalExternalFixReportDigestInput<'a> {
    report_id: &'a str,
    fix_id: &'a str,
    severity: &'a str,
    lifecycle_plan_id: &'a str,
    lifecycle_plan_digest: &'a str,
    pending_report_id: &'a str,
    pending_report_digest: &'a str,
    deployment_plan_id: &'a str,
    deployment_plan_digest: &'a str,
    inventory_id: &'a str,
    affected_roles: &'a [String],
    affected_canisters: &'a [String],
    directly_patchable_roles: &'a [String],
    externally_blocked_roles: &'a [String],
    dependency_blocked_roles: &'a [String],
    required_external_actions: &'a [String],
    protected_call_implications: &'a [String],
    residual_exposure: &'a [String],
    operator_next_steps: &'a [String],
}

pub(in crate::deployment_truth::lifecycle) fn external_lifecycle_pending_report_digest(
    report: &ExternalLifecyclePendingReportV1,
) -> String {
    stable_json_sha256_hex(&ExternalLifecyclePendingReportDigestInput {
        report_id: &report.report_id,
        lifecycle_plan_id: &report.lifecycle_plan_id,
        lifecycle_plan_digest: &report.lifecycle_plan_digest,
        proposal_report_id: &report.proposal_report_id,
        proposal_report_digest: &report.proposal_report_digest,
        deployment_plan_id: &report.deployment_plan_id,
        deployment_plan_digest: &report.deployment_plan_digest,
        inventory_id: &report.inventory_id,
        direct_upgrade_count: report.direct_upgrade_count,
        pending_external_count: report.pending_external_count,
        blocked_count: report.blocked_count,
        pending_external_actions: &report.pending_external_actions,
        blocked_subjects: &report.blocked_subjects,
        residual_exposure: &report.residual_exposure,
        status: report.status,
    })
}

pub(in crate::deployment_truth::lifecycle) fn external_lifecycle_check_digest(
    check: &ExternalLifecycleCheckV1,
) -> String {
    stable_json_sha256_hex(&ExternalLifecycleCheckDigestInput {
        check_id: &check.check_id,
        lifecycle_plan_id: &check.lifecycle_plan_id,
        lifecycle_plan_digest: &check.lifecycle_plan_digest,
        proposal_report_id: &check.proposal_report_id,
        proposal_report_digest: &check.proposal_report_digest,
        pending_report_id: &check.pending_report_id,
        pending_report_digest: &check.pending_report_digest,
        deployment_plan_id: &check.deployment_plan_id,
        deployment_plan_digest: &check.deployment_plan_digest,
        inventory_id: &check.inventory_id,
        status: check.status,
        direct_upgrade_count: check.direct_upgrade_count,
        pending_external_count: check.pending_external_count,
        blocked_count: check.blocked_count,
        residual_exposure_count: check.residual_exposure_count,
        summary: &check.summary,
        next_actions: &check.next_actions,
    })
}

pub(in crate::deployment_truth::lifecycle) fn external_lifecycle_handoff_digest(
    handoff: &ExternalLifecycleHandoffV1,
) -> String {
    stable_json_sha256_hex(&ExternalLifecycleHandoffDigestInput {
        handoff_id: &handoff.handoff_id,
        lifecycle_check_id: &handoff.lifecycle_check_id,
        lifecycle_check_digest: &handoff.lifecycle_check_digest,
        pending_report_id: &handoff.pending_report_id,
        pending_report_digest: &handoff.pending_report_digest,
        proposal_report_id: &handoff.proposal_report_id,
        proposal_report_digest: &handoff.proposal_report_digest,
        deployment_plan_id: &handoff.deployment_plan_id,
        deployment_plan_digest: &handoff.deployment_plan_digest,
        inventory_id: &handoff.inventory_id,
        status: handoff.status,
        handoff_actions: &handoff.handoff_actions,
        blocked_subjects: &handoff.blocked_subjects,
        residual_exposure: &handoff.residual_exposure,
        operator_summary: &handoff.operator_summary,
    })
}

pub(in crate::deployment_truth::lifecycle) fn critical_external_fix_report_digest(
    report: &CriticalExternalFixReportV1,
) -> String {
    stable_json_sha256_hex(&CriticalExternalFixReportDigestInput {
        report_id: &report.report_id,
        fix_id: &report.fix_id,
        severity: &report.severity,
        lifecycle_plan_id: &report.lifecycle_plan_id,
        lifecycle_plan_digest: &report.lifecycle_plan_digest,
        pending_report_id: &report.pending_report_id,
        pending_report_digest: &report.pending_report_digest,
        deployment_plan_id: &report.deployment_plan_id,
        deployment_plan_digest: &report.deployment_plan_digest,
        inventory_id: &report.inventory_id,
        affected_roles: &report.affected_roles,
        affected_canisters: &report.affected_canisters,
        directly_patchable_roles: &report.directly_patchable_roles,
        externally_blocked_roles: &report.externally_blocked_roles,
        dependency_blocked_roles: &report.dependency_blocked_roles,
        required_external_actions: &report.required_external_actions,
        protected_call_implications: &report.protected_call_implications,
        residual_exposure: &report.residual_exposure,
        operator_next_steps: &report.operator_next_steps,
    })
}
