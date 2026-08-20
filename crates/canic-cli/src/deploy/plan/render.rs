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

use std::path::Path;

use canic_host::{
    durable_io::create_new_bytes,
    fleet_install_plan::{
        FreshFleetDeploymentPlanV1, FreshFleetFundingPayerV1, PlannedCanisterCreationFunding,
    },
};

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
    let mut data = render_json(report)?.into_bytes();
    data.push(b'\n');
    create_new_bytes(path, &data).map_err(plan_output_error)
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
        format!("fleet: {}", report.fleet),
        format!("app: {}", report.app),
        format!("environment: {}", report.environment),
        format!("fleet_input: {}", report.fleet_input_path),
        format!("config: {}", report.config_path),
        format!("build_profile: {}", report.build_profile),
        format!(
            "release_build: {}",
            report.release_build_id.as_deref().unwrap_or("workspace")
        ),
        format!(
            "no_effects_started: {}",
            report
                .fresh_fleet_plan
                .as_ref()
                .is_some_and(|plan| plan.preflight.effects.no_effects_started())
        ),
        String::new(),
    ];

    if let Some(plan) = &report.fresh_fleet_plan {
        append_fresh_fleet_decision(&mut lines, plan);
    }

    append_diagnostics(&mut lines, "blockers", &report.blockers);
    append_diagnostics(&mut lines, "warnings", &report.warnings);
    append_diagnostics(&mut lines, "assumptions", &report.assumptions);
    append_diagnostics(&mut lines, "verified facts", &report.verified_facts);
    append_operations(&mut lines, &report.proposed_operations);
    append_next_actions(&mut lines, &report.next_actions);

    lines.join("\n")
}

fn append_fresh_fleet_decision(lines: &mut Vec<String>, plan: &FreshFleetDeploymentPlanV1) {
    let counts = plan.counts;
    let operator = &plan.authority.operator;
    lines.push("canonical fresh-Fleet decision".to_string());
    lines.push(format!("  plan_digest: {}", plan.plan_digest));
    lines.push(format!("  operator_principal: {}", operator.principal));
    lines.push(format!(
        "  operator_funding_account: {}",
        operator.funding_account
    ));
    lines.push(format!(
        "  operator_balance: {}",
        render_funding(&operator.balance)
    ));
    lines.push(format!(
        "  operator_balance_evidence: source={} observed_at={} valid_until={} fresh={} sufficient={}",
        operator.source,
        operator.observed_at_unix_secs,
        operator.valid_until_unix_secs,
        operator.balance_fresh,
        plan.operator_balance_sufficient,
    ));
    lines.push(format!(
        "  maximum_operator_debit: {}",
        render_funding(&plan.maximum_operator_debit)
    ));
    lines.push(format!(
        "  canister_counts: coordinator={} root={} store={} component={} ready_pool={} role={} total={}",
        counts.coordinator_canisters,
        counts.root_canisters,
        counts.wasm_store_canisters,
        counts.component_canisters,
        counts.ready_pool_canisters,
        counts.role_canisters,
        counts.total_canisters,
    ));
    for root in &plan.preflight.fleet_subnet_roots {
        lines.push(format!(
            "  root: subnet={} component={} initial_pool={} pool_creations={} ready_pool={} admissions={}",
            root.placement_subnet,
            root.initial_component_canisters,
            root.initial_pool_canisters,
            root.pool_canister_creations,
            root.remaining_pool_canisters,
            root.component_admissions.len(),
        ));
    }
    for requirement in &plan.funding_requirements {
        let payer = match requirement.payer {
            FreshFleetFundingPayerV1::Operator => "operator",
            FreshFleetFundingPayerV1::FleetSubnetRoot => "fleet_subnet_root",
        };
        lines.push(format!(
            "  funding: category={} owner={} payer={} count={} per_canister={} maximum={}",
            requirement.category,
            requirement.owner,
            payer,
            requirement.canister_count,
            render_funding(&requirement.per_canister),
            render_funding(&requirement.maximum),
        ));
    }
    lines.push(String::new());
}

fn render_funding(funding: &PlannedCanisterCreationFunding) -> String {
    match funding {
        PlannedCanisterCreationFunding::Cycles { cycles } => format!("{cycles} cycles"),
        PlannedCanisterCreationFunding::Icp { e8s } => format!("{e8s} ICP e8s"),
    }
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
