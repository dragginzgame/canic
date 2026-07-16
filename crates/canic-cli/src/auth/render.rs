//! Module: canic_cli::auth::render
//!
//! Responsibility: render delegated-auth command results for operators.
//! Does not own: command parsing, transport, or response decoding.

use super::{
    AuthCommandError, AuthIssuerObservation, AuthRenewalBatchStatus, AuthRenewalStatusCode,
    AuthRenewalStatusResult, AuthRenewalTemplateStatus,
};

pub(super) fn write_renewal_status_result(
    json: bool,
    result: &AuthRenewalStatusResult,
) -> Result<(), AuthCommandError> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
    } else {
        println!("{}", render_renewal_status_result(result));
    }
    Ok(())
}

pub(super) fn render_renewal_status_result(result: &AuthRenewalStatusResult) -> String {
    let mut lines = vec![
        format!("Auth renewal status: {}", result.issuer_pid),
        format!("Deployment: {}", result.deployment),
        format!("Root: {}", result.target.canister_id),
        format!("Status: {}", result.status.label()),
        format!(
            "Template: {}",
            render_template_status(&result.renewal.template)
        ),
    ];
    append_state_lines(&mut lines, result);
    append_batch_lines(&mut lines, result);
    append_issuer_observation_lines(&mut lines, result);
    lines.join("\n")
}

fn append_state_lines(lines: &mut Vec<String>, result: &AuthRenewalStatusResult) {
    if result.renewal.state.present {
        lines.push(format!(
            "Last installed expires: {}",
            result
                .renewal
                .state
                .last_installed_expires_at_ns
                .as_deref()
                .unwrap_or("-")
        ));
        lines.push(format!(
            "Refresh after: {}",
            result
                .renewal
                .state
                .last_installed_refresh_after_ns
                .as_deref()
                .unwrap_or("-")
        ));
        lines.push(format!(
            "Next attempt after: {}",
            result
                .renewal
                .state
                .next_attempt_after_ns
                .as_deref()
                .unwrap_or("-")
        ));
    }
}

fn append_batch_lines(lines: &mut Vec<String>, result: &AuthRenewalStatusResult) {
    lines.push(format!(
        "Latest batch: {}",
        render_batch_status(&result.renewal.latest_batch)
    ));
    if result.renewal.latest_batch.present {
        lines.push(format!(
            "Batch ID: {}",
            result
                .renewal
                .latest_batch
                .batch_id
                .as_deref()
                .unwrap_or("-")
        ));
        lines.push(format!(
            "Batch proof epoch: {}",
            result
                .renewal
                .latest_batch
                .proof_epoch
                .map_or_else(|| "-".to_string(), |value| value.to_string())
        ));
        lines.push(format!(
            "Batch expires: {}",
            result
                .renewal
                .latest_batch
                .expires_at_ns
                .as_deref()
                .unwrap_or("-")
        ));
        if let Some(failure) = &result.renewal.latest_batch.failure {
            lines.push(format!("Failure: {failure}"));
        }
    }
}

fn append_issuer_observation_lines(lines: &mut Vec<String>, result: &AuthRenewalStatusResult) {
    lines.push(format!(
        "Issuer observation: {}",
        render_issuer_observation(&result.issuer_observation)
    ));
    if result.issuer_observation.available {
        lines.push(format!(
            "Issuer cert hash: {}",
            result
                .issuer_observation
                .cert_hash
                .as_deref()
                .unwrap_or("-")
        ));
        lines.push(format!(
            "Issuer expires: {}",
            result
                .issuer_observation
                .expires_at_ns
                .as_deref()
                .unwrap_or("-")
        ));
    } else if let Some(reason) = &result.issuer_observation.reason {
        lines.push(format!("Issuer observation reason: {reason}"));
    }
}

const fn render_template_status(template: &AuthRenewalTemplateStatus) -> &'static str {
    match (template.present, template.enabled) {
        (false, _) => "missing",
        (true, Some(true)) => "enabled",
        (true, Some(false)) => "disabled",
        (true, None) => "configured",
    }
}

fn render_batch_status(batch: &AuthRenewalBatchStatus) -> &str {
    if batch.present {
        batch.status.as_deref().unwrap_or("present")
    } else {
        "none"
    }
}

pub(super) fn render_issuer_observation(observation: &AuthIssuerObservation) -> String {
    if observation.drift_detected {
        format!(
            "{} ({})",
            AuthRenewalStatusCode::DriftDetected.label(),
            observation.status
        )
    } else {
        observation.status.clone()
    }
}
