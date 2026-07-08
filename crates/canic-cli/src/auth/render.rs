//! Module: canic_cli::auth::render
//!
//! Responsibility: render delegated-auth command results for operators.
//! Does not own: command parsing, transport, or response decoding.

use super::{
    AuthCommandError, AuthIssuerObservation, AuthRenewalActiveAttemptStatus, AuthRenewalStatusCode,
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
    if result.renewal.state.present {
        lines.push(format!(
            "Last outcome: {}",
            result.renewal.state.last_outcome.as_deref().unwrap_or("-")
        ));
        lines.push(format!(
            "Consecutive failures: {}",
            result
                .renewal
                .state
                .consecutive_failures
                .map_or_else(|| "-".to_string(), |value| value.to_string())
        ));
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
    lines.push(format!(
        "Active attempt: {}",
        render_active_attempt_status(&result.renewal.active_attempt)
    ));
    if result.renewal.active_attempt.present {
        lines.push(format!(
            "Batch: {}",
            result
                .renewal
                .active_attempt
                .batch_id
                .as_deref()
                .unwrap_or("-")
        ));
        if let Some(failure) = &result.renewal.active_attempt.failure {
            lines.push(format!("Failure: {failure}"));
        }
    }
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
    lines.join("\n")
}

const fn render_template_status(template: &AuthRenewalTemplateStatus) -> &'static str {
    match (template.present, template.enabled) {
        (false, _) => "missing",
        (true, Some(true)) => "enabled",
        (true, Some(false)) => "disabled",
        (true, None) => "configured",
    }
}

fn render_active_attempt_status(attempt: &AuthRenewalActiveAttemptStatus) -> &str {
    if attempt.present {
        attempt.status.as_deref().unwrap_or("present")
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
