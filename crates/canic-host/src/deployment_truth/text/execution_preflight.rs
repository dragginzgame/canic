use super::super::*;
use super::{append_hard_failure_items, append_string_items};

/// Render an execution preflight as operator text.
#[must_use]
pub fn deployment_execution_preflight_text(preflight: &DeploymentExecutionPreflightV1) -> String {
    let mut lines = vec![
        "Deployment execution preflight".to_string(),
        "mode: passive".to_string(),
        format!(
            "status: {}",
            deployment_execution_preflight_status_label(preflight.status)
        ),
        format!("plan_id: {}", preflight.plan_id),
        format!("safety_report_id: {}", preflight.safety_report_id),
        format!("authority_plan_id: {}", preflight.authority_plan_id),
        format!("backend: {:?}", preflight.backend),
        String::new(),
        "counts:".to_string(),
        format!("  planned_phases: {}", preflight.planned_phases.len()),
        format!(
            "  required_capabilities: {}",
            preflight.required_capabilities.len()
        ),
        format!(
            "  missing_capabilities: {}",
            preflight.missing_capabilities.len()
        ),
        format!("  blockers: {}", preflight.blockers.len()),
    ];

    append_string_items(&mut lines, "planned_phases", &preflight.planned_phases);
    append_capability_items(
        &mut lines,
        "required_capabilities",
        &preflight.required_capabilities,
    );
    append_capability_items(
        &mut lines,
        "missing_capabilities",
        &preflight.missing_capabilities,
    );
    append_hard_failure_items(&mut lines, "blockers", &preflight.blockers);
    lines.join("\n")
}

fn append_capability_items(
    lines: &mut Vec<String>,
    label: &str,
    capabilities: &[DeploymentExecutorCapabilityV1],
) {
    if capabilities.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{label}:"));
    for capability in capabilities {
        lines.push(format!("  - {capability:?}"));
    }
}

const fn deployment_execution_preflight_status_label(
    status: DeploymentExecutionPreflightStatusV1,
) -> &'static str {
    match status {
        DeploymentExecutionPreflightStatusV1::Ready => "ready",
        DeploymentExecutionPreflightStatusV1::Blocked => "blocked",
    }
}
