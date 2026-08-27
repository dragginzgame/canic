//! Module: canic_cli::fleet
//!
//! Responsibility: parse and render the sole `canic fleet ensure` operator workflow.
//! Does not own: desired-state policy, IC effects, durable intent, or historical compatibility.
//! Boundary: delegates immediately to the host reconciler after resolving local paths.

#[cfg(test)]
mod tests;

use crate::{
    cli::{
        clap::{
            parse_matches, render_usage, required_string, string_option, string_option_or_else,
            value_arg,
        },
        defaults::default_icp,
        globals::{internal_environment_arg, internal_icp_arg},
        help::print_help_or_version,
    },
    output, version_text,
};
use canic_host::{
    fleet_ensure::{
        DesiredFleetLoadError, EnsureWorkflowError, FleetEnsureReport, IcpEnsurePlatform,
        IcpEnsurePlatformError, apply, load_desired_fleet, plan,
    },
    icp_config::{IcpConfigError, resolve_current_canic_icp_root},
};
use clap::{ArgAction, Command};
use std::{
    ffi::OsString,
    io,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

const FLEET_HELP_AFTER: &str = "\
Example:
  canic fleet ensure staging --desired fleets/staging.toml

Planning is read-only. Review `plan_sha256`, then repeat the command with
`--apply <plan_sha256>`. Historical install and recovery state is not read.";

/// CLI failure for current Fleet convergence.

#[derive(Debug, ThisError)]
pub enum FleetCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("desired Fleet environment {desired} does not match selected environment {selected}")]
    EnvironmentMismatch { desired: String, selected: String },

    #[error("system time is before the Unix epoch")]
    InvalidClock,

    #[error(transparent)]
    Desired(#[from] DesiredFleetLoadError),

    #[error(transparent)]
    IcpRoot(#[from] IcpConfigError),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Workflow(#[from] EnsureWorkflowError<IcpEnsurePlatformError>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EnsureOptions {
    apply: Option<String>,
    desired: PathBuf,
    environment: Option<String>,
    fleet: String,
    icp: String,
    json: bool,
}

impl EnsureOptions {
    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_command(), args)
            .map_err(|error| FleetCommandError::Usage(format!("{error}\n{}", usage())))?;
        let (_, ensure) = matches
            .subcommand()
            .unwrap_or_else(|| unreachable!("Fleet command requires ensure"));
        let fleet = required_string(ensure, "fleet");
        let desired = string_option(ensure, "desired").map_or_else(
            || PathBuf::from("fleets").join(format!("{fleet}.toml")),
            PathBuf::from,
        );
        Ok(Self {
            apply: string_option(ensure, "apply"),
            desired,
            environment: string_option(ensure, "environment"),
            fleet,
            icp: string_option_or_else(ensure, "icp", default_icp),
            json: ensure.get_flag("json"),
        })
    }
}

fn fleet_command() -> Command {
    Command::new("fleet")
        .bin_name("canic fleet")
        .about("Converge one Fleet from current desired state")
        .disable_help_flag(true)
        .subcommand_required(true)
        .subcommand(ensure_command())
        .after_help(FLEET_HELP_AFTER)
}

fn ensure_command() -> Command {
    Command::new("ensure")
        .bin_name("canic fleet ensure")
        .about("Plan or apply one idempotent Fleet convergence")
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Fleet identity"),
        )
        .arg(
            value_arg("apply")
                .long("apply")
                .value_name("PLAN_SHA256")
                .value_parser(parse_digest)
                .help("Apply exactly the retained reviewed plan"),
        )
        .arg(
            value_arg("desired")
                .long("desired")
                .value_name("PATH")
                .help("Current desired Fleet TOML; defaults to fleets/<fleet>.toml"),
        )
        .arg(
            value_arg("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .num_args(0)
                .help("Print the complete machine-readable report"),
        )
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
}

/// Run the current Fleet command group.
pub fn run<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }
    if args.first().and_then(|arg| arg.to_str()) == Some("ensure")
        && print_help_or_version(&args[1..], ensure_usage, version_text())
    {
        return Ok(());
    }
    let options = EnsureOptions::parse(args)?;
    let root = resolve_current_canic_icp_root()?;
    let desired_path = if options.desired.is_absolute() {
        options.desired.clone()
    } else {
        root.join(&options.desired)
    };
    let loaded = load_desired_fleet(&desired_path)?;
    if let Some(selected) = &options.environment
        && selected != &loaded.desired.environment
    {
        return Err(FleetCommandError::EnvironmentMismatch {
            desired: loaded.desired.environment,
            selected: selected.clone(),
        });
    }
    let mut platform = IcpEnsurePlatform::new(loaded.desired.clone(), &options.icp, &root);
    let report = if let Some(digest) = &options.apply {
        apply(
            &root,
            &loaded.desired,
            &loaded.sha256,
            &options.fleet,
            digest,
            &mut platform,
        )?
    } else {
        plan(
            &root,
            &loaded.desired,
            &loaded.sha256,
            &options.fleet,
            now_nanoseconds()?,
            &mut platform,
        )?
    };
    render_report(&report, options.json)
}

fn render_report(report: &FleetEnsureReport, json: bool) -> Result<(), FleetCommandError> {
    if json {
        return output::write_pretty_json(None, report);
    }
    let conservation = &report.plan.conservation;
    let mut lines = vec![
        format!("fleet: {}", report.plan.fleet),
        format!("operation_id: {}", report.plan.operation_id),
        format!("plan_sha256: {}", report.plan.plan_sha256),
        format!("terminal: {}", report.terminal),
        format!(
            "observed_controlled_cycles: {}",
            conservation.observed_controlled_cycles
        ),
        format!(
            "retained_in_reused_canisters_cycles: {}",
            conservation.retained_in_reused_canisters_cycles
        ),
        format!(
            "scheduled_transfer_cycles: {}",
            conservation.scheduled_transfer_cycles
        ),
        format!(
            "maximum_unavoidable_fee_cycles: {}",
            conservation.maximum_unavoidable_fee_cycles
        ),
        format!(
            "maximum_execution_burn_cycles: {}",
            conservation.maximum_execution_burn_cycles
        ),
        format!(
            "maximum_new_funding_cycles: {}",
            conservation.maximum_new_funding_cycles
        ),
        format!(
            "maximum_operator_debit_cycles: {}",
            conservation.maximum_operator_debit_cycles
        ),
        format!(
            "expected_post_operation_cycles: {}",
            conservation.expected_post_operation_cycles
        ),
        "canisters:".to_string(),
    ];
    lines.extend(report.plan.canisters.iter().map(|canister| {
        format!(
            "  {}: disposition={:?} principal={} observed_cycles={} effects={}",
            canister.name,
            canister.disposition,
            canister.principal.as_deref().unwrap_or("unallocated"),
            canister.observed_cycles,
            canister.actions.len()
        )
    }));
    lines.push(format!(
        "conservation_equation: {} + {} - {} - {} = {}",
        conservation.observed_controlled_cycles,
        conservation.maximum_operator_debit_cycles,
        conservation.maximum_unavoidable_fee_cycles,
        conservation.maximum_execution_burn_cycles,
        conservation.expected_post_operation_cycles
    ));
    if let Some(actual) = &report.actual_conservation {
        lines.push(format!(
            "measured_conservation: {} + {} - {} = {}",
            actual.observed_starting_cycles,
            actual.received_new_funding_cycles,
            actual.measured_execution_burn_cycles,
            actual.final_controlled_cycles
        ));
    }
    output::write_text(None, &lines.join("\n"))
}

fn parse_digest(value: &str) -> Result<String, String> {
    (value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')))
    .then(|| value.to_string())
    .ok_or_else(|| "plan digest must be exactly 64 lowercase hexadecimal characters".to_string())
}

fn now_nanoseconds() -> Result<u64, FleetCommandError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| FleetCommandError::InvalidClock)?
        .as_nanos();
    u64::try_from(nanos).map_err(|_| FleetCommandError::InvalidClock)
}

fn usage() -> String {
    render_usage(fleet_command)
}

fn ensure_usage() -> String {
    render_usage(ensure_command)
}
