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
            parse_matches, render_usage, required_string, required_typed, string_option,
            string_option_or_else, value_arg,
        },
        defaults::default_icp,
        globals::{internal_environment_arg, internal_icp_arg},
        help::print_help_or_version,
    },
    output, version_text,
};
use canic_core::cdk::utils::hash::sha256_hex;
use canic_core::ids::ReleaseBuildId;
use canic_host::{
    fleet_ensure::{
        DesiredFleetLoadError, EnsureWorkflowError, FleetEnsureReport, FleetGenerateError,
        FleetGenerateRequest, IcpEnsurePlatform, IcpEnsurePlatformError, apply,
        generate_desired_fleet, load_desired_fleet, plan,
    },
    icp_config::{IcpConfigError, resolve_current_canic_icp_root},
};
use clap::{ArgAction, Command};
use std::{
    ffi::OsString,
    fs, io,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

const FLEET_HELP_AFTER: &str = "\
Examples:
  canic fleet ensure staging --desired fleets/staging.toml
  canic fleet generate staging --app-config apps/demo/canic.toml --release-build <sha256>

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

    #[error(transparent)]
    Generate(#[from] FleetGenerateError),

    #[error("generated Fleet output already exists with different contents: {0}")]
    OutputConflict(PathBuf),

    #[error(
        "generated Fleet output digest changed at {}: expected {expected}, actual {actual}",
        path.display()
    )]
    OutputDigestMismatch {
        actual: String,
        expected: String,
        path: PathBuf,
    },

    #[error("generated Fleet output replacement requires an existing file: {0}")]
    OutputMissingForReplacement(PathBuf),

    #[error(transparent)]
    TomlSerialize(#[from] toml::ser::Error),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GenerateOptions {
    app_config: PathBuf,
    environment: Option<String>,
    fleet: String,
    icp: String,
    output: PathBuf,
    release_build: ReleaseBuildId,
    replace: Option<String>,
    seed: PathBuf,
    source: PathBuf,
}

impl GenerateOptions {
    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_command(), args)
            .map_err(|error| FleetCommandError::Usage(format!("{error}\n{}", usage())))?;
        let Some(("generate", generate)) = matches.subcommand() else {
            unreachable!("generate options require the generate subcommand")
        };
        let fleet = required_string(generate, "fleet");
        Ok(Self {
            app_config: PathBuf::from(required_string(generate, "app-config")),
            environment: string_option(generate, "environment"),
            fleet: fleet.clone(),
            icp: string_option_or_else(generate, "icp", default_icp),
            output: string_option(generate, "output").map_or_else(
                || PathBuf::from("fleets").join(format!("{fleet}.toml")),
                PathBuf::from,
            ),
            release_build: required_typed(generate, "release-build"),
            replace: string_option(generate, "replace")
                .map(|value| parse_digest(&value))
                .transpose()
                .map_err(FleetCommandError::Usage)?,
            seed: string_option(generate, "seed").map_or_else(
                || PathBuf::from("deployments").join(format!("{fleet}.estate.toml")),
                PathBuf::from,
            ),
            source: string_option(generate, "source").map_or_else(
                || PathBuf::from("deployments").join(format!("{fleet}.toml")),
                PathBuf::from,
            ),
        })
    }
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
        let Some(("ensure", ensure)) = matches.subcommand() else {
            unreachable!("ensure options require the ensure subcommand")
        };
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
        .subcommand(generate_command())
        .after_help(FLEET_HELP_AFTER)
}

fn generate_command() -> Command {
    Command::new("generate")
        .bin_name("canic fleet generate")
        .about("Generate exact current desired state from policy, release, and live identity authority")
        .disable_help_flag(true)
        .arg(value_arg("fleet").value_name("fleet").required(true).help("Fleet identity"))
        .arg(
            value_arg("app-config")
                .long("app-config")
                .value_name("PATH")
                .required(true)
                .help("Canonical App canic.toml used to compile topology and init authority"),
        )
        .arg(
            value_arg("release-build")
                .long("release-build")
                .value_name("RELEASE_BUILD_ID")
                .required(true)
                .value_parser(clap::value_parser!(ReleaseBuildId))
                .help("Finalized current release build emitted by a complete canic build"),
        )
        .arg(value_arg("output").long("output").value_name("PATH").help("Generated desired TOML; defaults to fleets/<fleet>.toml"))
        .arg(value_arg("replace").long("replace").value_name("EXPECTED_SHA256").help("Replace existing output only when its exact SHA-256 matches"))
        .arg(value_arg("seed").long("seed").value_name("PATH").help("Explicit retained identity seed; defaults to deployments/<fleet>.estate.toml"))
        .arg(value_arg("source").long("source").value_name("PATH").help("Protected Fleet policy input; defaults to deployments/<fleet>.toml"))
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
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
    if args.first().and_then(|arg| arg.to_str()) == Some("generate")
        && print_help_or_version(&args[1..], generate_usage, version_text())
    {
        return Ok(());
    }
    if args.first().and_then(|arg| arg.to_str()) == Some("generate") {
        return run_generate(GenerateOptions::parse(args)?);
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

fn run_generate(options: GenerateOptions) -> Result<(), FleetCommandError> {
    let root = resolve_current_canic_icp_root()?;
    let environment = options.environment.as_deref().unwrap_or("local");
    let generated = generate_desired_fleet(&FleetGenerateRequest {
        app_config: &resolve_from_root(&root, &options.app_config),
        environment,
        fleet: &options.fleet,
        icp_executable: &options.icp,
        release_build_id: options.release_build,
        root: &root,
        seed: &resolve_from_root(&root, &options.seed),
        source: &resolve_from_root(&root, &options.source),
    })?;
    let output = resolve_from_root(&root, &options.output);
    let bytes = toml::to_string_pretty(&generated.desired)?.into_bytes();
    publish_generated(&output, &bytes, options.replace.as_deref())?;
    println!("fleet: {}", options.fleet);
    println!("release_build: {}", generated.release_build_id);
    println!("observed_canisters: {}", generated.observed_canisters);
    println!(
        "observed_controlled_cycles: {}",
        generated.observed_controlled_cycles
    );
    println!("desired: {}", output.display());
    Ok(())
}

fn resolve_from_root(root: &std::path::Path, path: &std::path::Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn publish_generated(
    path: &std::path::Path,
    bytes: &[u8],
    expected_sha256: Option<&str>,
) -> Result<(), FleetCommandError> {
    match fs::read(path) {
        Ok(existing) if existing == bytes && expected_sha256.is_none() => return Ok(()),
        Ok(existing) => {
            let actual = sha256_hex(&existing);
            let Some(expected) = expected_sha256 else {
                return Err(FleetCommandError::OutputConflict(path.to_path_buf()));
            };
            if actual != expected {
                return Err(FleetCommandError::OutputDigestMismatch {
                    actual,
                    expected: expected.to_string(),
                    path: path.to_path_buf(),
                });
            }
            if existing == bytes {
                return Ok(());
            }
            canic_host::durable_io::write_bytes(path, bytes)?;
            return Ok(());
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound && expected_sha256.is_some() => {
            return Err(FleetCommandError::OutputMissingForReplacement(
                path.to_path_buf(),
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    canic_host::durable_io::create_new_bytes_with_parents(path, bytes)?;
    Ok(())
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
    lines.extend(report.plan.canisters.iter().flat_map(|canister| {
        canister.actions.iter().filter_map(|action| {
            let canic_host::fleet_ensure::model::EnsureAction::Fund {
                amount,
                expected_post_cycles,
                funding_deficit_cycles,
                funding_margin_cycles,
                ledger,
                principal,
                ..
            } = action
            else {
                return None;
            };
            Some(format!(
                "  native_topup {}: cycles_ledger_withdraw={} ledger={} target={} deficit={} margin={} expected_native_post={}",
                canister.name,
                amount,
                ledger,
                principal,
                funding_deficit_cycles,
                funding_margin_cycles,
                expected_post_cycles
            ))
        })
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

fn generate_usage() -> String {
    render_usage(generate_command)
}
