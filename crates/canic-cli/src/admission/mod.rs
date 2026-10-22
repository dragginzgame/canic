//! Module: canic_cli::admission
//!
//! Responsibility: parse and render protected Fleet-admission commands.
//! Does not own: plans, live authority checks, transport, or participant convergence.
//! Boundary: Host workflows own plan, apply and status operations.

#[cfg(test)]
mod tests;

use crate::{
    cli::{
        clap::{
            flag_arg, parse_matches, parse_required_subcommand, passthrough_subcommand,
            render_usage, required_path, required_string, string_option, value_arg,
        },
        globals::{internal_environment_arg, internal_icp_arg},
        help::print_help_or_version,
    },
    output,
    support::icp_target::IcpTargetOptions,
    version_text,
};
use canic_contracts::{
    dto::fleet_admission::{FleetAdmissionMutationAction, FleetAdmissionMutationOutcome},
    ids::{ComponentInstanceId, FleetAdmissionSelector, SubnetId},
};
use canic_core::cdk::utils::hash::hex_bytes;
use canic_host::admission::{
    AdmissionError, AdmissionTarget, model::AdmissionPlanRecord, view::AdmissionStatusReport,
    workflow,
};
use clap::{ArgGroup, Command as ClapCommand};
use ic_host_fs::durable::{PublicationMode, WriteOptions, write_bytes};
use ic_platform::principal::ExternalPrincipal as Principal;
use std::{ffi::OsString, fs, path::PathBuf};
use thiserror::Error as ThisError;

const FLEET_ARG: &str = "fleet";
const ADD_ARG: &str = "add";
const APPLY_COMMAND: &str = "apply";
const COMPONENT_INSTANCE_ARG: &str = "component-instance";
const COMPONENT_SPEC_ARG: &str = "component-spec";
const FLEET_SELECTOR_ARG: &str = "fleet-selector";
const FLEET_SUBNET_ROOT_ARG: &str = "fleet-subnet-root";
const JSON_ARG: &str = "json";
const OUT_ARG: &str = "out";
const PLAN_COMMAND: &str = "plan";
const PLAN_FILE_ARG: &str = "plan-file";
const REMOVE_ARG: &str = "remove";
const STATUS_COMMAND: &str = "status";

/// CLI argument, rendering and Host admission failures.
#[derive(Debug, ThisError)]
pub enum AdmissionCommandError {
    #[error("{0}")]
    Usage(String),
    #[error("invalid admission Principal {value:?}: {reason}")]
    InvalidPrincipal { value: String, reason: String },
    #[error("invalid admission selector: {0}")]
    InvalidSelector(String),
    #[error(transparent)]
    Host(#[from] AdmissionError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AdmissionPlanOptions {
    target: IcpTargetOptions,
    fleet: String,
    action: FleetAdmissionMutationAction,
    principal: Principal,
    selector: FleetAdmissionSelector,
    out: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AdmissionApplyOptions {
    target: IcpTargetOptions,
    fleet: String,
    plan_file: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AdmissionStatusOptions {
    target: IcpTargetOptions,
    fleet: String,
    json: bool,
}

pub fn run<I>(args: I) -> Result<(), AdmissionCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, admission_usage, version_text()) {
        return Ok(());
    }
    let (command, args) = parse_required_subcommand(admission_command(), args)
        .map_err(|_| AdmissionCommandError::Usage(admission_usage()))?;
    match command.as_str() {
        APPLY_COMMAND => {
            if print_help_or_version(&args, apply_usage, version_text()) {
                Ok(())
            } else {
                run_apply(AdmissionApplyOptions::parse(args)?)
            }
        }
        PLAN_COMMAND => {
            if print_help_or_version(&args, plan_usage, version_text()) {
                Ok(())
            } else {
                run_plan(AdmissionPlanOptions::parse(args)?)
            }
        }
        STATUS_COMMAND => {
            if print_help_or_version(&args, status_usage, version_text()) {
                Ok(())
            } else {
                run_status(AdmissionStatusOptions::parse(args)?)
            }
        }
        _ => unreachable!("admission command declares only known subcommands"),
    }
}

fn run_plan(options: AdmissionPlanOptions) -> Result<(), AdmissionCommandError> {
    let plan = workflow::plan(
        &options.fleet,
        &host_target(&options.target),
        options.action,
        options.selector,
        options.principal,
    )?;
    let mut bytes = serde_json::to_vec_pretty(&plan)?;
    bytes.push(b'\n');
    write_bytes(
        &crate::output::resolve_operator_path(&options.out)?,
        WriteOptions {
            mode: PublicationMode::Replace,
            permissions: 0o666,
        },
        &bytes,
    )
    .map_err(canic_host::publication::ops::io_error)?;
    println!(
        "Fleet-admission plan written to {}\nOperation: {}\nGeneration: {} -> {}\nRegistry revision: {}\nParticipant Roots: {}\nManaged participants: {}",
        options.out.display(),
        hex_bytes(plan.request.operation_id),
        plan.predecessor_policy.generation,
        plan.successor_policy.generation,
        plan.predecessor_registry.revision,
        plan.participant_catalogs.len(),
        plan.participant_catalogs
            .iter()
            .map(|catalog| catalog.participants.len())
            .sum::<usize>(),
    );
    Ok(())
}

fn run_apply(options: AdmissionApplyOptions) -> Result<(), AdmissionCommandError> {
    let plan: AdmissionPlanRecord = serde_json::from_slice(&fs::read(&options.plan_file)?)?;
    let response = workflow::apply(&options.fleet, &host_target(&options.target), &plan)?;
    println!(
        "Fleet-admission mutation accepted\nOperation: {}\nOutcome: {}\nGeneration: {}\nPolicy digest: {}",
        hex_bytes(response.operation_id),
        mutation_outcome_label(response.outcome),
        response.generation,
        hex_bytes(response.policy_digest),
    );
    Ok(())
}

fn run_status(options: AdmissionStatusOptions) -> Result<(), AdmissionCommandError> {
    let report = workflow::collect_status(&options.fleet, &host_target(&options.target))?;
    if options.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", render_status(&report));
    }
    Ok(())
}

impl AdmissionPlanOptions {
    fn parse(args: Vec<OsString>) -> Result<Self, AdmissionCommandError> {
        let matches = parse_matches(plan_command(), args)
            .map_err(|_| AdmissionCommandError::Usage(plan_usage()))?;
        let (action, principal_text) = match (
            string_option(&matches, ADD_ARG),
            string_option(&matches, REMOVE_ARG),
        ) {
            (Some(principal), None) => (FleetAdmissionMutationAction::Add, principal),
            (None, Some(principal)) => (FleetAdmissionMutationAction::Remove, principal),
            _ => unreachable!("Clap requires exactly one admission action"),
        };
        let principal = parse_principal(&principal_text)?;
        Ok(Self {
            target: IcpTargetOptions::parse(&matches),
            fleet: required_string(&matches, FLEET_ARG),
            action,
            principal,
            selector: parse_selector(&matches)?,
            out: required_path(&matches, OUT_ARG),
        })
    }
}

impl AdmissionApplyOptions {
    fn parse(args: Vec<OsString>) -> Result<Self, AdmissionCommandError> {
        let matches = parse_matches(apply_command(), args)
            .map_err(|_| AdmissionCommandError::Usage(apply_usage()))?;
        Ok(Self {
            target: IcpTargetOptions::parse(&matches),
            fleet: required_string(&matches, FLEET_ARG),
            plan_file: required_path(&matches, PLAN_FILE_ARG),
        })
    }
}

impl AdmissionStatusOptions {
    fn parse(args: Vec<OsString>) -> Result<Self, AdmissionCommandError> {
        let matches = parse_matches(status_command(), args)
            .map_err(|_| AdmissionCommandError::Usage(status_usage()))?;
        Ok(Self {
            target: IcpTargetOptions::parse(&matches),
            fleet: required_string(&matches, FLEET_ARG),
            json: matches.get_flag(JSON_ARG),
        })
    }
}

fn parse_principal(value: &str) -> Result<Principal, AdmissionCommandError> {
    Principal::from_text(value).map_err(|error| AdmissionCommandError::InvalidPrincipal {
        value: value.to_string(),
        reason: error.to_string(),
    })
}

fn parse_selector(
    matches: &clap::ArgMatches,
) -> Result<FleetAdmissionSelector, AdmissionCommandError> {
    if matches.get_flag(FLEET_SELECTOR_ARG) {
        return Ok(FleetAdmissionSelector::Fleet);
    }
    if let Some(value) = string_option(matches, COMPONENT_SPEC_ARG) {
        return value
            .parse()
            .map(FleetAdmissionSelector::ComponentSpec)
            .map_err(|error: canic_contracts::ids::ComponentSpecIdParseError| {
                AdmissionCommandError::InvalidSelector(error.to_string())
            });
    }
    if let Some(value) = string_option(matches, COMPONENT_INSTANCE_ARG) {
        return value
            .parse::<ComponentInstanceId>()
            .map(FleetAdmissionSelector::ComponentInstance)
            .map_err(|error| AdmissionCommandError::InvalidSelector(error.to_string()));
    }
    if let Some(value) = string_option(matches, FLEET_SUBNET_ROOT_ARG) {
        return Principal::from_text(&value)
            .map(SubnetId::from_principal)
            .map(FleetAdmissionSelector::FleetSubnetRoot)
            .map_err(|error| AdmissionCommandError::InvalidSelector(error.to_string()));
    }
    unreachable!("Clap requires one admission selector")
}

const fn mutation_outcome_label(outcome: FleetAdmissionMutationOutcome) -> &'static str {
    match outcome {
        FleetAdmissionMutationOutcome::Planned => "planned",
        FleetAdmissionMutationOutcome::Converged => "converged",
        FleetAdmissionMutationOutcome::CatalogChanged => "catalog_changed",
        FleetAdmissionMutationOutcome::AlreadyPresent => "already_present",
        FleetAdmissionMutationOutcome::AlreadyAbsent => "already_absent",
    }
}

fn render_status(report: &AdmissionStatusReport) -> String {
    let mut lines = vec![
        format!("Fleet admission: {}/{}", report.environment, report.fleet),
        format!("  coordinator: {}", report.coordinator),
        format!("  Registry revision: {}", report.registry_revision),
        format!("  generation: {}", report.generation),
        format!("  policy digest: {}", report.policy_digest),
        format!("  Fleet Principals: {}", report.fleet_principal_count),
        format!(
            "  narrower rules: {} ({} Principal references)",
            report.narrower_rule_count, report.narrower_principal_reference_count
        ),
        format!(
            "  current: {}",
            report.current_phase.as_deref().unwrap_or("none")
        ),
    ];
    for root in &report.roots {
        lines.push(format!(
            "  root {}: generation={} phase={} participants={} pending={} prepared={} activated={} open={}{}",
            root.root,
            root.active_generation,
            root.phase.as_deref().unwrap_or("idle"),
            root.participant_count,
            root.pending_count,
            root.prepared_count,
            root.activated_count,
            root.open_count,
            root.first_unresolved
                .map(|target| format!(" first_unresolved={target}"))
                .unwrap_or_default(),
        ));
    }
    output::render_terminal_lines(&lines)
}

fn admission_command() -> ClapCommand {
    [APPLY_COMMAND, PLAN_COMMAND, STATUS_COMMAND]
        .into_iter()
        .fold(
            ClapCommand::new("admission").bin_name("canic admission"),
            |command, name| {
                command.subcommand(passthrough_subcommand(
                    ClapCommand::new(name).disable_help_flag(true),
                ))
            },
        )
}

fn admission_usage() -> String {
    "Manage protected Fleet-wide ingress admission\n\nUsage: canic admission <command> [OPTIONS]\n\nCommands:\n  apply   Apply one exact accepted admission plan\n  plan    Build a read-only exact admission plan\n  status  Inspect protected Fleet admission convergence\n  help    Print this message or the help of the given subcommand(s)\n\nExamples:\n  canic admission plan demo --add <principal> --fleet --out admission.json\n  canic admission apply demo admission.json\n  canic admission status demo".to_string()
}

fn plan_command() -> ClapCommand {
    ClapCommand::new(PLAN_COMMAND)
        .bin_name("canic admission plan")
        .about("Build a read-only exact Fleet-admission mutation plan")
        .disable_help_flag(true)
        .arg(value_arg(FLEET_ARG).value_name(FLEET_ARG).required(true))
        .arg(value_arg(ADD_ARG).long(ADD_ARG).value_name("PRINCIPAL"))
        .arg(value_arg(REMOVE_ARG).long(REMOVE_ARG).value_name("PRINCIPAL"))
        .group(ArgGroup::new("action").args([ADD_ARG, REMOVE_ARG]).required(true).multiple(false))
        .arg(flag_arg(FLEET_SELECTOR_ARG).long("fleet"))
        .arg(value_arg(COMPONENT_INSTANCE_ARG).long(COMPONENT_INSTANCE_ARG).value_name("ID"))
        .arg(value_arg(COMPONENT_SPEC_ARG).long(COMPONENT_SPEC_ARG).value_name("ID"))
        .arg(value_arg(FLEET_SUBNET_ROOT_ARG).long(FLEET_SUBNET_ROOT_ARG).value_name("SUBNET_ID"))
        .group(
            ArgGroup::new("selector")
                .args([
                    FLEET_SELECTOR_ARG,
                    COMPONENT_INSTANCE_ARG,
                    COMPONENT_SPEC_ARG,
                    FLEET_SUBNET_ROOT_ARG,
                ])
                .required(true)
                .multiple(false),
        )
        .arg(value_arg(OUT_ARG).long(OUT_ARG).value_name("FILE").required(true))
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
        .after_help("Examples:\n  canic admission plan demo --add <principal> --fleet --out admission.json\n  canic admission plan demo --remove <principal> --component-spec core --out admission.json")
}

fn apply_command() -> ClapCommand {
    ClapCommand::new(APPLY_COMMAND)
        .bin_name("canic admission apply")
        .about("Apply one exact accepted Fleet-admission plan")
        .disable_help_flag(true)
        .arg(value_arg(FLEET_ARG).value_name(FLEET_ARG).required(true))
        .arg(
            value_arg(PLAN_FILE_ARG)
                .value_name("PLAN_FILE")
                .required(true),
        )
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
        .after_help("Example:\n  canic admission apply demo admission.json")
}

fn status_command() -> ClapCommand {
    ClapCommand::new(STATUS_COMMAND)
        .bin_name("canic admission status")
        .about("Inspect protected Fleet-admission convergence")
        .disable_help_flag(true)
        .arg(value_arg(FLEET_ARG).value_name(FLEET_ARG).required(true))
        .arg(flag_arg(JSON_ARG).long(JSON_ARG))
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
        .after_help(
            "Examples:\n  canic admission status demo\n  canic admission status demo --json",
        )
}

fn plan_usage() -> String {
    render_usage(plan_command)
}

fn apply_usage() -> String {
    render_usage(apply_command)
}

fn status_usage() -> String {
    render_usage(status_command)
}

fn host_target(target: &IcpTargetOptions) -> AdmissionTarget {
    AdmissionTarget {
        environment: target.environment.clone(),
        icp: target.icp.clone(),
    }
}
