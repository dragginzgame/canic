use super::{DeployCommandError, value_arg};
use crate::{
    cli::{
        clap::{
            flag_arg, parse_matches, path_option, render_usage, required_string,
            string_option_or_else, typed_option,
        },
        defaults::local_network,
        globals::internal_network_arg,
        help::print_help_or_version,
    },
    version_text,
};
use canic_host::{
    canister_build::CanisterBuildProfile,
    deployment_truth::{DeploymentAssumptionV1, DeploymentPlanV1, LocalDeploymentPlanRequest},
    release_set::{
        configured_fleet_name, icp_root as resolve_icp_root,
        workspace_root as resolve_workspace_root,
    },
};
use clap::Command as ClapCommand;
use serde::Serialize;
use std::{
    ffi::OsString,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

const REPORT_SCHEMA_VERSION: u16 = 1;
const DEPLOYMENT_ARG: &str = "deployment";
const JSON_ARG: &str = "json";
const OUT_ARG: &str = "out";
const CONFIG_ARG: &str = "config";
const BUILD_PROFILE_ARG: &str = "build-profile";

const DEPLOY_PLAN_HELP_AFTER: &str = "\
Examples:
  canic deploy plan demo-local
  canic deploy plan demo-local --json
  canic deploy plan demo-local --out deployment-plan.json
  canic deploy plan demo-local --config fleets/demo/canic.toml

Builds a deterministic planning report from local project config. The command
does not install, upgrade, create canisters, write deployment truth, update
installed deployment records, or call live IC state. --out writes JSON only and
fails if the requested path already exists or its parent directory is missing.";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeployPlanOptions {
    pub(super) deployment: String,
    pub(super) network: String,
    pub(super) json: bool,
    pub(super) out: Option<PathBuf>,
    pub(super) config: Option<PathBuf>,
    pub(super) build_profile: CanisterBuildProfile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeployPlanRoots {
    pub(super) workspace_root: PathBuf,
    pub(super) icp_root: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct DeploymentPlanReport {
    schema_version: u16,
    command: &'static str,
    target: String,
    network: String,
    build_profile: String,
    config_path: String,
    status: PlanStatus,
    comparison_status: ComparisonStatus,
    plan: DeploymentPlanV1,
    blockers: Vec<PlanDiagnostic>,
    warnings: Vec<PlanDiagnostic>,
    assumptions: Vec<PlanDiagnostic>,
    verified_facts: Vec<PlanDiagnostic>,
    proposed_operations: Vec<ProposedOperationLabel>,
    next_actions: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
enum PlanStatus {
    Planned,
    Warning,
    Blocked,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ComparisonStatus {
    NotRequested,
    NotAvailable,
    Compared,
    ComparedWithWarnings,
    ComparedWithDrift,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct PlanDiagnostic {
    category: &'static str,
    code: String,
    severity: &'static str,
    subject: String,
    detail: String,
    next: Option<String>,
    source: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct ProposedOperationLabel {
    phase: &'static str,
    label: &'static str,
    subject: String,
    status: &'static str,
}

pub(super) fn run<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = DeployPlanOptions::parse(args)?;
    let roots = DeployPlanRoots::discover()?;
    let report = build_report(&options, &roots);
    write_report(&options, &report)?;
    command_exit_result(&report)
}

pub(super) fn build_report(
    options: &DeployPlanOptions,
    roots: &DeployPlanRoots,
) -> DeploymentPlanReport {
    let config_path = plan_config_path(&roots.workspace_root, options);
    let plan = build_plan(options, roots, &config_path);
    let mut blockers = target_resolution_blockers(options, &config_path);
    let target_resolved = blockers.is_empty();
    if target_resolved {
        blockers.extend(plan_blockers(&plan));
    }
    let mut assumptions = plan_assumptions(&plan);
    let mut warnings = plan_warnings(&plan);
    let mut verified_facts = verified_facts(options, &config_path, target_resolved, &plan);
    let mut proposed_operations = proposed_operations(&plan);
    let mut next_actions = next_actions(options, &blockers, &warnings, &assumptions);

    sort_diagnostics(&mut blockers);
    sort_diagnostics(&mut warnings);
    sort_diagnostics(&mut assumptions);
    sort_diagnostics(&mut verified_facts);
    proposed_operations.sort_by(|left, right| {
        left.phase
            .cmp(right.phase)
            .then_with(|| left.label.cmp(right.label))
            .then_with(|| left.subject.cmp(&right.subject))
    });
    next_actions.sort();
    next_actions.dedup();

    let status = aggregate_status(&blockers, &warnings, &assumptions);
    let comparison_status = comparison_status(&plan, &blockers, &warnings, &assumptions);

    DeploymentPlanReport {
        schema_version: REPORT_SCHEMA_VERSION,
        command: "canic deploy plan",
        target: options.deployment.clone(),
        network: options.network.clone(),
        build_profile: build_profile_name(options),
        config_path: display_path(&config_path),
        status,
        comparison_status,
        plan,
        blockers,
        warnings,
        assumptions,
        verified_facts,
        proposed_operations,
        next_actions,
    }
}

fn build_plan(
    options: &DeployPlanOptions,
    roots: &DeployPlanRoots,
    config_path: &Path,
) -> DeploymentPlanV1 {
    canic_host::deployment_truth::build_local_deployment_plan(&LocalDeploymentPlanRequest {
        deployment_name: options.deployment.clone(),
        network: options.network.clone(),
        workspace_root: roots.workspace_root.clone(),
        icp_root: roots.icp_root.clone(),
        config_path: Some(config_path.to_path_buf()),
        runtime_variant: options.network.clone(),
        build_profile: build_profile_name(options),
    })
}

fn target_resolution_blockers(
    options: &DeployPlanOptions,
    config_path: &Path,
) -> Vec<PlanDiagnostic> {
    if let Err(err) = validate_deployment_target_name(&options.deployment) {
        return vec![PlanDiagnostic {
            category: "deployment_identity",
            code: "deployment_target_invalid".to_string(),
            severity: "blocked",
            subject: options.deployment.clone(),
            detail: err,
            next: Some("use letters, numbers, '-' or '_' for deployment target names".to_string()),
            source: "cli_arg",
        }];
    }

    match configured_fleet_name(config_path) {
        Ok(_) => Vec::new(),
        Err(err) => vec![PlanDiagnostic {
            category: "config",
            code: "deployment_target_unresolved".to_string(),
            severity: "blocked",
            subject: options.deployment.clone(),
            detail: format!(
                "deployment target {} could not be resolved from {}: {err}",
                options.deployment,
                config_path.display()
            ),
            next: Some(
                "provide --config with a readable fleet config for this deployment".to_string(),
            ),
            source: "deployment_config",
        }],
    }
}

fn validate_deployment_target_name(name: &str) -> Result<(), String> {
    let valid = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
    if valid {
        Ok(())
    } else {
        Err(format!(
            "invalid deployment target name {name:?}; use letters, numbers, '-' or '_'"
        ))
    }
}

fn verified_facts(
    options: &DeployPlanOptions,
    config_path: &Path,
    target_resolved: bool,
    plan: &DeploymentPlanV1,
) -> Vec<PlanDiagnostic> {
    if !target_resolved {
        return Vec::new();
    }

    let mut facts = vec![PlanDiagnostic {
        category: "config",
        code: "deployment_target_resolved".to_string(),
        severity: "info",
        subject: options.deployment.clone(),
        detail: format!(
            "deployment target {} resolved from {}",
            options.deployment,
            config_path.display()
        ),
        next: None,
        source: "fleet_config",
    }];

    if let Some(root) = &plan.trust_domain.root_trust_anchor {
        facts.push(PlanDiagnostic {
            category: "observation",
            code: "installed_root_canister_id_resolved".to_string(),
            severity: "info",
            subject: options.deployment.clone(),
            detail: format!("installed deployment state resolves root canister {root}"),
            next: None,
            source: "installed_deployment",
        });
    }

    facts
}

fn plan_assumptions(plan: &DeploymentPlanV1) -> Vec<PlanDiagnostic> {
    plan.unresolved_assumptions
        .iter()
        .filter(|assumption| !is_blocking_plan_assumption(&assumption.key))
        .filter(|assumption| !is_warning_plan_assumption(&assumption.key))
        .map(assumption_diagnostic)
        .collect()
}

fn plan_blockers(plan: &DeploymentPlanV1) -> Vec<PlanDiagnostic> {
    plan.unresolved_assumptions
        .iter()
        .filter(|assumption| is_blocking_plan_assumption(&assumption.key))
        .map(blocking_assumption_diagnostic)
        .collect()
}

fn is_blocking_plan_assumption(key: &str) -> bool {
    key.starts_with("local_config.") || key == "local_state.unverified_root_canister_id"
}

fn is_warning_plan_assumption(key: &str) -> bool {
    key.starts_with("local_state.") && !is_blocking_plan_assumption(key)
}

fn blocking_assumption_diagnostic(assumption: &DeploymentAssumptionV1) -> PlanDiagnostic {
    PlanDiagnostic {
        category: assumption_category(&assumption.key),
        code: diagnostic_code(&assumption.key),
        severity: "blocked",
        subject: assumption.key.clone(),
        detail: assumption.description.clone(),
        next: Some(blocking_assumption_next(&assumption.key)),
        source: "deployment_plan_builder",
    }
}

fn blocking_assumption_next(key: &str) -> String {
    if key == "local_state.unverified_root_canister_id" {
        "run canic deploy check and verify the registered root before planning apply".to_string()
    } else {
        "repair the local fleet config before planning apply".to_string()
    }
}

fn plan_warnings(plan: &DeploymentPlanV1) -> Vec<PlanDiagnostic> {
    plan.unresolved_assumptions
        .iter()
        .filter(|assumption| is_warning_plan_assumption(&assumption.key))
        .map(|assumption| PlanDiagnostic {
            category: "observation",
            code: local_state_warning_code(assumption),
            severity: "warning",
            subject: plan.deployment_identity.deployment_name.clone(),
            detail: assumption.description.clone(),
            next: Some(
                "run canic deploy check after installation or provide saved evidence".to_string(),
            ),
            source: "installed_deployment",
        })
        .collect()
}

fn local_state_warning_code(assumption: &DeploymentAssumptionV1) -> String {
    if is_observed_state_drift_assumption(assumption) {
        "observed_inventory_drift".to_string()
    } else if assumption.key == "local_state.root_canister_id" {
        "observed_inventory_unavailable".to_string()
    } else {
        diagnostic_code(&assumption.key)
    }
}

fn assumption_diagnostic(assumption: &DeploymentAssumptionV1) -> PlanDiagnostic {
    PlanDiagnostic {
        category: assumption_category(&assumption.key),
        code: diagnostic_code(&assumption.key),
        severity: "warning",
        subject: assumption.key.clone(),
        detail: assumption.description.clone(),
        next: assumption_next(&assumption.key),
        source: "deployment_plan_builder",
    }
}

fn assumption_category(key: &str) -> &'static str {
    if key.contains("artifact") || key.contains("manifest") {
        "artifact"
    } else if key.contains("state") {
        "observation"
    } else if key.contains("controller") {
        "authority"
    } else if key.contains("pool") {
        "topology"
    } else {
        "config"
    }
}

fn assumption_next(key: &str) -> Option<String> {
    if key.contains("artifact") || key.contains("manifest") {
        Some("run canic build or provide a build profile with resolved artifacts".to_string())
    } else if key.contains("local_state") {
        Some("compare after first deployment or provide deployment-check evidence".to_string())
    } else {
        None
    }
}

fn diagnostic_code(key: &str) -> String {
    let mut code = String::new();
    for ch in key.chars() {
        if ch.is_ascii_alphanumeric() {
            code.push(ch.to_ascii_lowercase());
        } else if !code.ends_with('_') {
            code.push('_');
        }
    }
    code.trim_matches('_').to_string()
}

fn proposed_operations(plan: &DeploymentPlanV1) -> Vec<ProposedOperationLabel> {
    let mut operations = Vec::new();
    for canister in &plan.expected_canisters {
        if canister.canister_id.is_none() {
            operations.push(operation("create_canister", &canister.role));
        }
    }
    for artifact in &plan.role_artifacts {
        operations.push(operation(
            wasm_operation_label(plan, &artifact.role),
            &artifact.role,
        ));
    }
    operations.push(operation(
        "verify_topology",
        &plan.deployment_identity.deployment_name,
    ));
    operations
}

fn wasm_operation_label(plan: &DeploymentPlanV1, role: &str) -> &'static str {
    if plan
        .expected_canisters
        .iter()
        .any(|canister| canister.role == role && canister.canister_id.is_some())
    {
        "upgrade_wasm"
    } else {
        "install_wasm"
    }
}

fn operation(label: &'static str, subject: &str) -> ProposedOperationLabel {
    ProposedOperationLabel {
        phase: "future_apply_preview",
        label,
        subject: subject.to_string(),
        status: "not_executed",
    }
}

fn next_actions(
    options: &DeployPlanOptions,
    blockers: &[PlanDiagnostic],
    warnings: &[PlanDiagnostic],
    assumptions: &[PlanDiagnostic],
) -> Vec<String> {
    let mut actions = Vec::new();
    if !blockers.is_empty() {
        actions.push("fix blocker diagnostics before designing apply".to_string());
    }
    if !warnings.is_empty() || !assumptions.is_empty() {
        actions.push("resolve warnings before designing apply".to_string());
    }
    actions.push(format!(
        "run canic medic deployment {} if operator readiness is uncertain",
        options.deployment
    ));
    actions
}

fn aggregate_status(
    blockers: &[PlanDiagnostic],
    warnings: &[PlanDiagnostic],
    assumptions: &[PlanDiagnostic],
) -> PlanStatus {
    if blockers
        .iter()
        .any(|diagnostic| diagnostic.severity == "unsupported")
    {
        return PlanStatus::Unsupported;
    }
    if !blockers.is_empty() {
        return PlanStatus::Blocked;
    }
    if !warnings.is_empty() || !assumptions.is_empty() {
        return PlanStatus::Warning;
    }
    PlanStatus::Planned
}

fn comparison_status(
    plan: &DeploymentPlanV1,
    blockers: &[PlanDiagnostic],
    warnings: &[PlanDiagnostic],
    assumptions: &[PlanDiagnostic],
) -> ComparisonStatus {
    if !blockers.is_empty() {
        return ComparisonStatus::NotRequested;
    }

    if has_observed_state_drift(plan) {
        return ComparisonStatus::ComparedWithDrift;
    }

    if has_missing_observed_state(plan) {
        return ComparisonStatus::NotAvailable;
    }

    if plan.trust_domain.root_trust_anchor.is_none() {
        return ComparisonStatus::NotRequested;
    }

    if !warnings.is_empty() || !assumptions.is_empty() {
        ComparisonStatus::ComparedWithWarnings
    } else {
        ComparisonStatus::Compared
    }
}

fn has_observed_state_drift(plan: &DeploymentPlanV1) -> bool {
    plan.unresolved_assumptions
        .iter()
        .any(is_observed_state_drift_assumption)
}

fn has_missing_observed_state(plan: &DeploymentPlanV1) -> bool {
    plan.unresolved_assumptions.iter().any(|assumption| {
        assumption.key == "local_state.root_canister_id"
            && !is_observed_state_drift_assumption(assumption)
    })
}

fn is_observed_state_drift_assumption(assumption: &DeploymentAssumptionV1) -> bool {
    assumption.key == "local_state.root_canister_id"
        && assumption.description.contains(" has network ")
}

fn sort_diagnostics(diagnostics: &mut [PlanDiagnostic]) {
    diagnostics.sort_by(|left, right| {
        left.severity
            .cmp(right.severity)
            .then_with(|| left.category.cmp(right.category))
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.subject.cmp(&right.subject))
            .then_with(|| left.source.cmp(right.source))
    });
}

pub(super) fn write_report(
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

pub(super) fn command_exit_result(report: &DeploymentPlanReport) -> Result<(), DeployCommandError> {
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

pub(super) fn render_json(report: &DeploymentPlanReport) -> Result<String, DeployCommandError> {
    serde_json::to_string_pretty(report).map_err(plan_output_error)
}

fn plan_output_error(err: impl std::error::Error + 'static) -> DeployCommandError {
    DeployCommandError::PlanOutput(Box::new(err))
}

pub(super) fn render_text(report: &DeploymentPlanReport) -> String {
    let mut lines = vec![
        "Deployment plan".to_string(),
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
            diagnostic.severity, diagnostic.category, diagnostic.code
        ));
        lines.push(format!("    subject: {}", diagnostic.subject));
        lines.push(format!("    detail: {}", diagnostic.detail));
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

    lines.push("future apply preview".to_string());
    for operation in operations {
        lines.push(format!(
            "  - {} {} ({})",
            operation.label, operation.subject, operation.status
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

fn plan_config_path(workspace_root: &Path, options: &DeployPlanOptions) -> PathBuf {
    let config = options.config.clone().unwrap_or_else(|| {
        PathBuf::from("fleets")
            .join(&options.deployment)
            .join("canic.toml")
    });
    if config.is_absolute() {
        config
    } else {
        workspace_root.join(config)
    }
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

fn build_profile_name(options: &DeployPlanOptions) -> String {
    options.build_profile.target_dir_name().to_string()
}

impl DeployPlanOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| DeployCommandError::Usage(usage()))?;
        Ok(Self {
            deployment: required_string(&matches, DEPLOYMENT_ARG),
            network: string_option_or_else(&matches, "network", local_network),
            json: matches.get_flag(JSON_ARG),
            out: path_option(&matches, OUT_ARG),
            config: path_option(&matches, CONFIG_ARG),
            build_profile: typed_option(&matches, BUILD_PROFILE_ARG)
                .unwrap_or_else(CanisterBuildProfile::current),
        })
    }
}

impl DeployPlanRoots {
    fn discover() -> Result<Self, DeployCommandError> {
        Ok(Self {
            workspace_root: resolve_workspace_root().map_err(DeployCommandError::from)?,
            icp_root: resolve_icp_root().map_err(DeployCommandError::from)?,
        })
    }
}

pub(super) fn command() -> ClapCommand {
    ClapCommand::new("plan")
        .bin_name("canic deploy plan")
        .about("Explain the deterministic deployment plan without mutation")
        .disable_help_flag(true)
        .override_usage("canic deploy plan <deployment>")
        .arg(deployment_arg())
        .arg(json_arg())
        .arg(out_arg())
        .arg(config_arg())
        .arg(build_profile_arg())
        .arg(internal_network_arg())
        .after_help(DEPLOY_PLAN_HELP_AFTER)
}

fn deployment_arg() -> clap::Arg {
    value_arg(DEPLOYMENT_ARG)
        .value_name(DEPLOYMENT_ARG)
        .required(true)
        .help("Deployment target name to plan")
}

fn json_arg() -> clap::Arg {
    flag_arg(JSON_ARG)
        .long(JSON_ARG)
        .help("Print JSON DeploymentPlanReport to stdout")
}

fn out_arg() -> clap::Arg {
    value_arg(OUT_ARG)
        .long(OUT_ARG)
        .value_name("path")
        .num_args(1)
        .help("Write JSON DeploymentPlanReport to a new file")
}

fn config_arg() -> clap::Arg {
    value_arg(CONFIG_ARG)
        .long(CONFIG_ARG)
        .value_name("path")
        .num_args(1)
        .help("Fleet config path used to build the desired plan")
}

fn build_profile_arg() -> clap::Arg {
    value_arg(BUILD_PROFILE_ARG)
        .long(BUILD_PROFILE_ARG)
        .value_name("debug|fast|release")
        .num_args(1)
        .value_parser(clap::value_parser!(CanisterBuildProfile))
        .help("Expected canister wasm build profile")
}

pub(super) fn usage() -> String {
    render_usage(command)
}

impl PlanStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Warning => "warning",
            Self::Blocked => "blocked",
            Self::Unsupported => "unsupported",
        }
    }
}

impl ComparisonStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::NotRequested => "not_requested",
            Self::NotAvailable => "not_available",
            Self::Compared => "compared",
            Self::ComparedWithWarnings => "compared_with_warnings",
            Self::ComparedWithDrift => "compared_with_drift",
        }
    }
}
