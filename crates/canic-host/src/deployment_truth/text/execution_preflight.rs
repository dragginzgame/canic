use super::super::*;
use super::{append_hard_failure_items, append_string_items};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExecutionPreflightTextLabel(&'static str);

impl ExecutionPreflightTextLabel {
    const AUTHORITY_PLAN_ID: Self = Self("authority_plan_id");
    const BACKEND: Self = Self("backend");
    const BLOCKERS: Self = Self("blockers");
    const COUNTS: Self = Self("counts");
    const MISSING_CAPABILITIES: Self = Self("missing_capabilities");
    const MODE_PASSIVE: Self = Self("mode: passive");
    const PLAN_ID: Self = Self("plan_id");
    const PLANNED_PHASES: Self = Self("planned_phases");
    const REQUIRED_CAPABILITIES: Self = Self("required_capabilities");
    const SAFETY_REPORT_ID: Self = Self("safety_report_id");
    const STATUS: Self = Self("status");
    const TITLE: Self = Self("Deployment execution preflight");

    #[must_use]
    const fn as_str(self) -> &'static str {
        self.0
    }
}

/// Render an execution preflight as operator text.
#[must_use]
pub fn deployment_execution_preflight_text(preflight: &DeploymentExecutionPreflightV1) -> String {
    let mut lines = vec![
        ExecutionPreflightTextLabel::TITLE.as_str().to_string(),
        ExecutionPreflightTextLabel::MODE_PASSIVE
            .as_str()
            .to_string(),
        format!(
            "{}: {}",
            ExecutionPreflightTextLabel::STATUS.as_str(),
            preflight.status.label()
        ),
        format!(
            "{}: {}",
            ExecutionPreflightTextLabel::PLAN_ID.as_str(),
            preflight.plan_id
        ),
        format!(
            "{}: {}",
            ExecutionPreflightTextLabel::SAFETY_REPORT_ID.as_str(),
            preflight.safety_report_id
        ),
        format!(
            "{}: {}",
            ExecutionPreflightTextLabel::AUTHORITY_PLAN_ID.as_str(),
            preflight.authority_plan_id
        ),
        format!(
            "{}: {:?}",
            ExecutionPreflightTextLabel::BACKEND.as_str(),
            preflight.backend
        ),
        String::new(),
        format!("{}:", ExecutionPreflightTextLabel::COUNTS.as_str()),
        format!(
            "  {}: {}",
            ExecutionPreflightTextLabel::PLANNED_PHASES.as_str(),
            preflight.planned_phases.len()
        ),
        format!(
            "  {}: {}",
            ExecutionPreflightTextLabel::REQUIRED_CAPABILITIES.as_str(),
            preflight.required_capabilities.len()
        ),
        format!(
            "  {}: {}",
            ExecutionPreflightTextLabel::MISSING_CAPABILITIES.as_str(),
            preflight.missing_capabilities.len()
        ),
        format!(
            "  {}: {}",
            ExecutionPreflightTextLabel::BLOCKERS.as_str(),
            preflight.blockers.len()
        ),
    ];

    append_string_items(
        &mut lines,
        ExecutionPreflightTextLabel::PLANNED_PHASES.as_str(),
        &preflight.planned_phases,
    );
    append_capability_items(
        &mut lines,
        ExecutionPreflightTextLabel::REQUIRED_CAPABILITIES,
        &preflight.required_capabilities,
    );
    append_capability_items(
        &mut lines,
        ExecutionPreflightTextLabel::MISSING_CAPABILITIES,
        &preflight.missing_capabilities,
    );
    append_hard_failure_items(
        &mut lines,
        ExecutionPreflightTextLabel::BLOCKERS.as_str(),
        &preflight.blockers,
    );
    lines.join("\n")
}

fn append_capability_items(
    lines: &mut Vec<String>,
    label: ExecutionPreflightTextLabel,
    capabilities: &[DeploymentExecutorCapabilityV1],
) {
    if capabilities.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push(format!("{}:", label.as_str()));
    for capability in capabilities {
        lines.push(format!("  - {capability:?}"));
    }
}
