//! Module: canic_cli::deploy::plan::render
//!
//! Responsibility: render and persist deterministic deployment-plan reports.
//! Does not own: plan construction, diagnostic policy, or command parsing.
//! Boundary: consumes the assembled report and emits text or JSON without mutation.

use crate::deploy::{
    DeployCommandError,
    plan::{
        command::DeployPlanOptions,
        report::{DeploymentPlanReport, PlanDiagnostic, PlanStatus, ProposedOperationLabel},
    },
};
use std::{fs::OpenOptions, io::Write, path::Path};

pub(in crate::deploy) fn write_report(
    options: &DeployPlanOptions,
    report: &DeploymentPlanReport,
) -> Result<(), DeployCommandError> {
    if let Some(out) = &options.out {
        write_json_new(out, report)?;
    }

    if options.json {
        print_json(report)
    } else {
        println!("{}", render_text(report));
        Ok(())
    }
}

pub(in crate::deploy) fn command_exit_result(
    report: &DeploymentPlanReport,
) -> Result<(), DeployCommandError> {
    match report.status {
        PlanStatus::Planned | PlanStatus::Warning => Ok(()),
        PlanStatus::Blocked | PlanStatus::Unsupported => Err(DeployCommandError::PlanBlocked(
            report.status.as_str().to_string(),
        )),
    }
}

fn write_json_new(path: &Path, report: &DeploymentPlanReport) -> Result<(), DeployCommandError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(plan_output_error)?;
    let data = render_json(report)?;
    file.write_all(data.as_bytes()).map_err(plan_output_error)?;
    file.write_all(b"\n").map_err(plan_output_error)?;
    Ok(())
}

fn print_json(report: &DeploymentPlanReport) -> Result<(), DeployCommandError> {
    let json = render_json(report)?;
    println!("{json}");
    Ok(())
}

pub(in crate::deploy) fn render_json(
    report: &DeploymentPlanReport,
) -> Result<String, DeployCommandError> {
    serde_json::to_string_pretty(report).map_err(plan_output_error)
}

fn plan_output_error(err: impl std::error::Error + 'static) -> DeployCommandError {
    DeployCommandError::PlanOutput(Box::new(err))
}

pub(in crate::deploy) fn render_text(report: &DeploymentPlanReport) -> String {
    let mut lines = vec![
        "Deployment plan".to_string(),
        format!("schema_version: {}", report.schema_version),
        format!("command: {}", report.command),
        format!("status: {}", report.status.as_str()),
        format!("comparison: {}", report.comparison_status.as_str()),
        format!("target: {}", report.target),
        format!("network: {}", report.network),
        format!("config: {}", report.config_path),
        format!("build_profile: {}", report.build_profile),
        String::new(),
    ];

    append_diagnostics(&mut lines, "blockers", &report.blockers);
    append_diagnostics(&mut lines, "warnings", &report.warnings);
    append_diagnostics(&mut lines, "assumptions", &report.assumptions);
    append_diagnostics(&mut lines, "verified facts", &report.verified_facts);
    append_operations(&mut lines, &report.proposed_operations);
    append_next_actions(&mut lines, &report.next_actions);

    lines.join("\n")
}

fn append_diagnostics(lines: &mut Vec<String>, label: &str, diagnostics: &[PlanDiagnostic]) {
    if diagnostics.is_empty() {
        return;
    }

    lines.push(label.to_string());
    for diagnostic in diagnostics {
        lines.push(format!(
            "  [{}] {} {}",
            diagnostic.severity.label(),
            diagnostic.category.label(),
            diagnostic.code
        ));
        lines.push(format!("    subject: {}", diagnostic.subject));
        lines.push(format!("    detail: {}", diagnostic.detail));
        lines.push(format!("    source: {}", diagnostic.source.label()));
        if let Some(next) = &diagnostic.next {
            lines.push(format!("    next: {next}"));
        }
    }
    lines.push(String::new());
}

fn append_operations(lines: &mut Vec<String>, operations: &[ProposedOperationLabel]) {
    if operations.is_empty() {
        return;
    }

    lines.push("future apply preview (proposed operation labels; not executed)".to_string());
    for operation in operations {
        lines.push(format!(
            "  - phase: {} label: {} subject: {} status: {}",
            operation.phase.label(),
            operation.label.label(),
            operation.subject,
            operation.status.label()
        ));
    }
    lines.push(String::new());
}

fn append_next_actions(lines: &mut Vec<String>, actions: &[String]) {
    if actions.is_empty() {
        return;
    }

    lines.push("next actions".to_string());
    for action in actions {
        lines.push(format!("  - {action}"));
    }
}
