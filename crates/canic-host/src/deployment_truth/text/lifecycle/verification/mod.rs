use super::super::super::*;
use super::super::{append_string_items, optional_text};
use super::shared::{
    append_verification_check_requirement_items, append_verification_policy_requirement_items,
};

/// Render an external-upgrade verification report as passive operator text.
#[must_use]
pub fn external_upgrade_verification_report_text(
    report: &ExternalUpgradeVerificationReportV1,
) -> String {
    let mut lines = vec![
        "External upgrade verification report".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("report_id: {}", report.report_id),
        format!("report_digest: {}", report.report_digest),
        format!("proposal_id: {}", report.proposal_id),
        format!("proposal_digest: {}", report.proposal_digest),
        format!("receipt_id: {}", report.receipt_id),
        format!("receipt_digest: {}", report.receipt_digest),
        format!("subject: {}", report.subject),
        format!("role: {}", optional_text(report.role.as_deref())),
        format!(
            "canister_id: {}",
            optional_text(report.canister_id.as_deref())
        ),
        format!(
            "verification_result: {}",
            report.verification_result.label()
        ),
        format!(
            "live_inventory_required: {}",
            report.live_inventory_required
        ),
        format!("status_summary: {}", report.status_summary),
        format!("verification_notes: {}", report.verification_notes.len()),
    ];
    append_string_items(&mut lines, "verification_notes", &report.verification_notes);
    lines.join("\n")
}

/// Render an external-upgrade verification policy as passive operator text.
#[must_use]
pub fn external_upgrade_verification_policy_text(
    policy: &ExternalUpgradeVerificationPolicyV1,
) -> String {
    let mut lines = vec![
        "External upgrade verification policy".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("policy_id: {}", policy.policy_id),
        format!("policy_digest: {}", policy.policy_digest),
        format!("proposal_id: {}", policy.proposal_id),
        format!("proposal_digest: {}", policy.proposal_digest),
        format!("subject: {}", policy.subject),
        format!("role: {}", optional_text(policy.role.as_deref())),
        format!(
            "canister_id: {}",
            optional_text(policy.canister_id.as_deref())
        ),
        format!("summary: {}", policy.status_summary),
        String::new(),
        format!(
            "max_observation_age_seconds: {}",
            policy
                .max_observation_age_seconds
                .map_or_else(|| "none".to_string(), |value| value.to_string())
        ),
    ];
    append_verification_policy_requirement_items(&mut lines, &policy.verification_requirements);
    lines.join("\n")
}

/// Render an external-upgrade verification check as passive operator text.
#[must_use]
pub fn external_upgrade_verification_check_text(
    check: &ExternalUpgradeVerificationCheckV1,
) -> String {
    let mut lines = vec![
        "External upgrade verification check".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        "live_lookup: none".to_string(),
        format!("check_id: {}", check.check_id),
        format!("check_digest: {}", check.check_digest),
        format!("policy_id: {}", check.policy_id),
        format!("policy_digest: {}", check.policy_digest),
        format!("proposal_id: {}", check.proposal_id),
        format!("proposal_digest: {}", check.proposal_digest),
        format!("subject: {}", check.subject),
        format!("role: {}", optional_text(check.role.as_deref())),
        format!(
            "canister_id: {}",
            optional_text(check.canister_id.as_deref())
        ),
        format!("verification_result: {}", check.verification_result.label()),
        format!("summary: {}", check.status_summary),
        String::new(),
        format!("observation.source: {}", check.observation.source.label()),
        format!(
            "observation.deployment_check_id: {}",
            optional_text(check.observation.deployment_check_id.as_deref())
        ),
        format!(
            "observation.deployment_check_digest: {}",
            optional_text(check.observation.deployment_check_digest.as_deref())
        ),
        format!(
            "observation.inventory_id: {}",
            optional_text(check.observation.inventory_id.as_deref())
        ),
        format!(
            "observation.observed_at: {}",
            optional_text(check.observation.observed_at.as_deref())
        ),
        format!(
            "observation.observed_control_class: {}",
            check
                .observation
                .observed_control_class
                .map_or_else(|| "none".to_string(), |value| value.label().to_string())
        ),
    ];
    append_verification_check_requirement_items(&mut lines, &check.requirement_results);
    lines.join("\n")
}
