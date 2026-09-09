//! Module: canic_cli::fleet
//!
//! Responsibility: parse and render current Fleet generation and convergence workflows.
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
use canic_core::{
    cdk::{
        types::{BC, Cycles, QC, TC},
        utils::hash::sha256_hex,
    },
    ids::ReleaseBuildId,
};
use canic_host::{
    fleet_ensure::{
        DesiredFleetLoadError, EnsureWorkflowError, FleetEnsureReport, FleetGenerateError,
        FleetGenerateRequest, FreshEstateSeedRequest, IcpEnsurePlatform, IcpEnsurePlatformError,
        LoadedDesiredFleet, apply,
        dto::{FleetEnsurePhase, FleetEnsureProgress, FleetEnsureProgressState},
        generate_desired_fleet, initialize_fresh_estate_seed, load_desired_fleet,
        model::{EnsureAction, InstallMode},
        plan, plan_reinstall, report_json_value, retained_in_progress_plan,
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
`--apply <plan_sha256>`. A funding pause adds a separate funding review digest
for the exact additional transfer.";
const DEFAULT_CYCLES_LEDGER: &str = "um5iw-rqaaa-aaaaq-qaaba-cai";

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
    Workflow(Box<EnsureWorkflowError<IcpEnsurePlatformError>>),

    #[error(transparent)]
    Generate(Box<FleetGenerateError>),

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

impl From<EnsureWorkflowError<IcpEnsurePlatformError>> for FleetCommandError {
    fn from(error: EnsureWorkflowError<IcpEnsurePlatformError>) -> Self {
        Self::Workflow(Box::new(error))
    }
}

impl From<FleetGenerateError> for FleetCommandError {
    fn from(error: FleetGenerateError) -> Self {
        Self::Generate(Box::new(error))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GenerateOptions {
    app_config: PathBuf,
    cycles_ledger: String,
    environment: Option<String>,
    fleet: String,
    fresh: bool,
    icp: String,
    management_creation_fee_cycles: Option<u128>,
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
        let fresh = generate.get_flag("fresh");
        let management_creation_fee_cycles =
            string_option(generate, "management-creation-fee-cycles")
                .map(|value| {
                    Cycles::from_human_config_str(&value)
                        .map(|cycles| cycles.to_u128())
                        .map_err(|_| {
                            FleetCommandError::Usage(
                            "management creation fee must be an exact cycle amount such as 500B"
                                .to_string(),
                        )
                        })
                })
                .transpose()?;
        let cycles_ledger = string_option(generate, "cycles-ledger");
        if fresh && management_creation_fee_cycles.is_none() {
            return Err(FleetCommandError::Usage(
                "--fresh requires --management-creation-fee-cycles".to_string(),
            ));
        }
        if !fresh && (management_creation_fee_cycles.is_some() || cycles_ledger.is_some()) {
            return Err(FleetCommandError::Usage(
                "--cycles-ledger and --management-creation-fee-cycles require --fresh".to_string(),
            ));
        }
        Ok(Self {
            app_config: PathBuf::from(required_string(generate, "app-config")),
            cycles_ledger: cycles_ledger.unwrap_or_else(|| DEFAULT_CYCLES_LEDGER.to_string()),
            environment: string_option(generate, "environment"),
            fleet: fleet.clone(),
            fresh,
            icp: string_option_or_else(generate, "icp", default_icp),
            management_creation_fee_cycles,
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
    reinstall: bool,
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
            reinstall: ensure.get_flag("reinstall"),
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
        .about("Generate exact current desired state from policy, release, and estate authority")
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Fleet identity"),
        )
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
        .arg(
            value_arg("cycles-ledger")
                .long("cycles-ledger")
                .value_name("PRINCIPAL")
                .help("Exact Cycles Ledger used only when creating a fresh seed"),
        )
        .arg(
            value_arg("fresh")
                .long("fresh")
                .action(ArgAction::SetTrue)
                .num_args(0)
                .help("Create or replay a durable literally empty-estate seed"),
        )
        .arg(
            value_arg("management-creation-fee-cycles")
                .long("management-creation-fee-cycles")
                .value_name("CYCLES")
                .help("Exact creation fee such as 500B; required with --fresh, otherwise read from the seed"),
        )
        .arg(
            value_arg("output")
                .long("output")
                .value_name("PATH")
                .help("Generated desired TOML; defaults to fleets/<fleet>.toml"),
        )
        .arg(
            value_arg("replace")
                .long("replace")
                .value_name("EXPECTED_SHA256")
                .help("Replace existing output only when its exact SHA-256 matches"),
        )
        .arg(
            value_arg("seed").long("seed").value_name("PATH").help(
                "Explicit retained identity seed; defaults to deployments/<fleet>.estate.toml",
            ),
        )
        .arg(
            value_arg("source")
                .long("source")
                .value_name("PATH")
                .help("Protected Fleet policy input; defaults to deployments/<fleet>.toml"),
        )
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
                .help("Apply the exact retained plan or funding review digest"),
        )
        .arg(
            value_arg("desired")
                .long("desired")
                .value_name("PATH")
                .help("Current desired TOML; an in-progress plan uses its retained reviewed input"),
        )
        .arg(
            value_arg("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .num_args(0)
                .help("Print the JSON report to stdout and JSON phase events to stderr"),
        )
        .arg(
            value_arg("reinstall")
                .long("reinstall")
                .action(ArgAction::SetTrue)
                .num_args(0)
                .conflicts_with("apply")
                .help(
                    "Review a new same-release Fleet database wipe; preparation seals allocation",
                ),
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
    let loaded = load_ensure_authority(&root, &desired_path, &options)?;
    let json_progress = options.json;
    let next_review = format!(
        "canic fleet ensure {} --environment {} --desired {} --icp {}",
        quote_review_argument(&options.fleet),
        quote_review_argument(&loaded.desired.environment),
        quote_review_argument(&desired_path.to_string_lossy()),
        quote_review_argument(&options.icp)
    );
    let progress_review = next_review.clone();
    let platform = IcpEnsurePlatform::new(loaded.desired.clone(), &options.icp, &root)
        .with_progress_handler(move |mut progress| {
            if let FleetEnsureProgressState::ReviewRequired {
                review: Some(review),
                ..
            } = &mut progress.state
            {
                review.next_review_command.clone_from(&progress_review);
            }
            eprintln!("{}", render_progress(&progress, json_progress));
        });
    let mut platform = platform.with_observation_handler(move |timing| {
        eprintln!("{}", render_observation_timing(&timing, json_progress));
    });
    let report = if let Some(digest) = &options.apply {
        apply(
            &root,
            &loaded.desired,
            &loaded.sha256,
            &options.fleet,
            digest,
            &mut platform,
        ).map_err(|mut error| {
            if let canic_host::fleet_ensure::workflow::EnsureWorkflowError::SuccessorReviewRequired { review: Some(review), .. } = &mut error { review.next_review_command = next_review; }
            error
        })?
    } else if options.reinstall {
        plan_reinstall(
            &root,
            &loaded.desired,
            &loaded.sha256,
            &options.fleet,
            now_nanoseconds()?,
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

fn quote_review_argument(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn load_ensure_authority(
    root: &std::path::Path,
    desired_path: &std::path::Path,
    options: &EnsureOptions,
) -> Result<LoadedDesiredFleet, FleetCommandError> {
    if !options.reinstall
        && let Some(environment) = options.environment.as_deref()
        && let Some(plan) =
            retained_in_progress_plan::<IcpEnsurePlatformError>(root, environment, &options.fleet)?
        && let Some(desired) = plan.reviewed_desired
    {
        return Ok(LoadedDesiredFleet {
            desired: desired.into_desired(),
            sha256: plan.desired_sha256,
        });
    }
    let current = load_desired_fleet(desired_path)?;
    if let Some(selected) = &options.environment
        && selected != &current.desired.environment
    {
        return Err(FleetCommandError::EnvironmentMismatch {
            desired: current.desired.environment,
            selected: selected.clone(),
        });
    }
    if !options.reinstall
        && let Some(plan) = retained_in_progress_plan::<IcpEnsurePlatformError>(
            root,
            &current.desired.environment,
            &options.fleet,
        )?
        && let Some(desired) = plan.reviewed_desired
    {
        return Ok(LoadedDesiredFleet {
            desired: desired.into_desired(),
            sha256: plan.desired_sha256,
        });
    }
    Ok(current)
}

fn run_generate(options: GenerateOptions) -> Result<(), FleetCommandError> {
    let root = resolve_current_canic_icp_root()?;
    let environment = options.environment.as_deref().unwrap_or("local");
    let seed = resolve_from_root(&root, &options.seed);
    let source = resolve_from_root(&root, &options.source);
    if options.fresh {
        initialize_fresh_estate_seed(&FreshEstateSeedRequest {
            cycles_ledger: &options.cycles_ledger,
            management_creation_fee_cycles: options
                .management_creation_fee_cycles
                .expect("fresh option validation requires creation fee"),
            seed: &seed,
            source: &source,
        })?;
    }
    let generated = generate_desired_fleet(&FleetGenerateRequest {
        app_config: &resolve_from_root(&root, &options.app_config),
        environment,
        fleet: &options.fleet,
        icp_executable: &options.icp,
        release_build_id: options.release_build,
        root: &root,
        seed: &seed,
        source: &source,
    })?;
    let output = resolve_from_root(&root, &options.output);
    let bytes = toml::to_string_pretty(&generated.desired)?.into_bytes();
    publish_generated(&output, &bytes, options.replace.as_deref())?;
    println!("fleet: {}", options.fleet);
    println!("release_build: {}", generated.release_build_id);
    println!("observed_canisters: {}", generated.observed_canisters);
    println!(
        "observed_controlled_cycles: {}",
        format_cycles(generated.observed_controlled_cycles)
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

fn render_observation_timing(
    timing: &canic_host::fleet_ensure::dto::FleetObservationTiming,
    json: bool,
) -> String {
    if json {
        serde_json::json!({"event": "fleet_ensure_observation", "schema_version": 1, "observation": timing}).to_string()
    } else {
        format!(
            "Fleet observation {:?}: {} ms, {} remote call attempts{}",
            timing.stage,
            timing.elapsed_millis,
            timing.remote_call_attempts,
            if timing.succeeded { "" } else { ", failed" }
        )
    }
}

fn render_progress(progress: &FleetEnsureProgress, json: bool) -> String {
    if json {
        return serde_json::json!({
            "event": "fleet_ensure_progress",
            "schema_version": 1,
            "progress": progress,
        })
        .to_string();
    }
    let phase = match progress.phase {
        FleetEnsurePhase::Infrastructure => "infrastructure",
        FleetEnsurePhase::ImportReconciliation => "import reconciliation",
        FleetEnsurePhase::ControlPlane => "control plane",
        FleetEnsurePhase::WorkloadProvisioning => "Workload provisioning",
        FleetEnsurePhase::PoolReadiness => "pool readiness",
        FleetEnsurePhase::TerminalVerification => "terminal verification",
        FleetEnsurePhase::Complete => "complete",
    };
    let state = match &progress.state {
        FleetEnsureProgressState::Advancing => "advancing",
        FleetEnsureProgressState::AwaitingProgress => "awaiting progress",
        FleetEnsureProgressState::PrerequisiteComplete => "prerequisite complete",
        FleetEnsureProgressState::FundingRequired => "funding required",
        FleetEnsureProgressState::ReviewRequired { .. } => "new review required",
        FleetEnsureProgressState::Complete => "complete",
    };
    let details = match &progress.state {
        FleetEnsureProgressState::ReviewRequired {
            review: Some(review),
            ..
        } => format!("; {review}"),
        _ => String::new(),
    };
    format!(
        "Fleet ensure: {phase}: {state} ({}/{} reviewed effects applied){details}",
        progress.applied_effects, progress.reviewed_effects
    )
}

fn render_report(report: &FleetEnsureReport, json: bool) -> Result<(), FleetCommandError> {
    if json {
        return output::write_pretty_json(None, &report_json_value(report)?);
    }
    output::write_text(None, &render_text_report(report))
}

fn render_text_report(report: &FleetEnsureReport) -> String {
    let conservation = &report.plan.conservation;
    let (maximum_estate_funding_cycles, maximum_estate_creation_fee_cycles) =
        estate_funding_totals(conservation);
    let mut lines = vec![
        format!("fleet: {}", report.plan.fleet),
        format!("operation_id: {}", report.plan.operation_id),
        format!("plan_sha256: {}", report.plan.plan_sha256),
        format!("plan_scope: {}", report.plan.scope.as_str()),
        format!("terminal: {}", report.terminal),
        "cycle_budget: planning allowances; maximums are not measured expenditure".to_string(),
        format!(
            "observed_controlled_cycles: {}",
            format_cycles(conservation.observed_controlled_cycles)
        ),
        format!(
            "retained_in_reused_canisters_cycles: {}",
            format_cycles(conservation.retained_in_reused_canisters_cycles)
        ),
        format!(
            "scheduled_transfer_cycles: {}",
            format_cycles(conservation.scheduled_transfer_cycles)
        ),
        format!(
            "maximum_unavoidable_fee_cycles: {}",
            format_cycles(conservation.maximum_unavoidable_fee_cycles)
        ),
        format!(
            "maximum_execution_burn_cycles: {}",
            format_cycles(conservation.maximum_execution_burn_cycles)
        ),
        format!(
            "maximum_new_funding_cycles: {}",
            format_cycles(conservation.maximum_new_funding_cycles)
        ),
        format!(
            "maximum_operator_debit_cycles: {}",
            format_cycles(conservation.maximum_operator_debit_cycles)
        ),
        format!(
            "maximum_estate_funding_cycles: {}",
            format_cycles(maximum_estate_funding_cycles)
        ),
        format!(
            "maximum_estate_creation_fee_cycles: {}",
            format_cycles(maximum_estate_creation_fee_cycles)
        ),
        format!(
            "expected_post_operation_cycles: {}",
            format_cycles(conservation.expected_post_operation_cycles)
        ),
        "estate_funding_domains:".to_string(),
    ];
    append_recovery_review(&mut lines, report);
    append_reinstall_guidance(&mut lines, report);
    append_estate_funding_domains(&mut lines, conservation);
    append_canister_summaries(&mut lines, report);
    if let Some(review) = &report.funding_review {
        lines.extend([
            format!("funding_review_sha256: {}", review.review_sha256),
            format!("funding_root: {}", review.pause.root_principal),
            format!("funding_ledger: {}", review.pause.cycles_ledger),
            format!(
                "additional_funding_cycles: {}",
                format_cycles(review.pause.shortfall_cycles)
            ),
            format!(
                "additional_ledger_fee_cycles: {}",
                format_cycles(review.pause.ledger_fee_cycles)
            ),
            format!("funding_apply: --apply {}", review.review_sha256),
        ]);
    }
    lines.push(format!(
        "conservation_equation: {} observed controlled + {} maximum operator debit - {} maximum unavoidable fees - {} maximum Root-funded creation fees - {} maximum execution burn = {} expected remaining",
        format_cycles(conservation.observed_controlled_cycles),
        format_cycles(conservation.maximum_operator_debit_cycles),
        format_cycles(conservation.maximum_unavoidable_fee_cycles),
        format_cycles(maximum_estate_creation_fee_cycles),
        format_cycles(conservation.maximum_execution_burn_cycles),
        format_cycles(conservation.expected_post_operation_cycles)
    ));
    if let Some(actual) = &report.actual_conservation {
        lines.push(format!(
            "measured_estate_funding_cycles: {}",
            format_cycles(actual.estate_funding_cycles)
        ));
        lines.push(format!(
            "measured_conservation: {} observed starting + {} received funding - {} exact Root-funded creation fees - {} measured execution burn = {} final controlled",
            format_cycles(actual.observed_starting_cycles),
            format_cycles(actual.received_new_funding_cycles),
            format_cycles(actual.exact_estate_creation_fee_cycles),
            format_cycles(actual.measured_execution_burn_cycles),
            format_cycles(actual.final_controlled_cycles)
        ));
    }
    lines.join("\n")
}

fn append_recovery_review(lines: &mut Vec<String>, report: &FleetEnsureReport) {
    if let Some(review) = &report.plan.recovery_review {
        lines.extend([
            format!("base_execution_burn_cycles: {}", format_cycles(review.base_execution_burn_cycles)),
            format!("continuation_reserve_cycles: {}", format_cycles(review.continuation_reserve_cycles)),
            format!("whole_continuation_ceiling_cycles: {}", format_cycles(review.whole_continuation_ceiling_cycles)),
            "recovery_discovery: pending current protocol; this phase is not a complete deployment funding quote".into(),
        ]);
        for funding in &review.known_pool_funding {
            lines.push(format!("dependent_native_topup: {} via Root {}; {} cycles plus {} ledger fee; requires fresh review", funding.principal, funding.root, funding.amount_cycles, funding.ledger_fee_cycles));
        }
    }
}

fn append_reinstall_guidance(lines: &mut Vec<String>, report: &FleetEnsureReport) {
    if let Some(intent) = &report.plan.reinstall {
        lines.push(
            "reinstall: application data will be discarded; logical pool roles may be reassigned"
                .to_string(),
        );
        if let Some(activation) = &intent.activation_reset {
            lines.push(format!(
                "activation recovery: source_operation={} source_plan_document_sha256={}",
                activation.source.operation_id, activation.source.plan_document_sha256
            ));
            if report.plan.scope
                == canic_host::fleet_ensure::model::FleetEnsurePlanScope::ReinstallPreparation
            {
                lines.push(if report.terminal {
                    "next: run fleet ensure again to review the Root reset; the Coordinator remains stopped"
                } else {
                    "next: apply this stop/restart preparation digest, then review the Root reset"
                }.to_string());
                return;
            }
        }
        if report.plan.scope
            == canic_host::fleet_ensure::model::FleetEnsurePlanScope::ReinstallPreparation
        {
            lines.push(if report.terminal { "next: run fleet ensure again to review the sealed inventory reset; the Fleet remains sealed" } else { "next: apply this preparation digest, then run fleet ensure again to review the reset" }.to_string());
        }
    }
}

fn append_canister_summaries(lines: &mut Vec<String>, report: &FleetEnsureReport) {
    let initial_creations = report
        .plan
        .canisters
        .iter()
        .flat_map(|canister| &canister.actions)
        .filter(|action| matches!(action, EnsureAction::Create { .. }))
        .count();
    lines.push(format!("host_create_actions: {initial_creations} (initial or replacement canisters; separate from Root-funded pool creations)"));
    lines.push("canisters:".to_string());
    lines.extend(report.plan.canisters.iter().map(|canister| {
        format!(
            "  {}: disposition={:?} principal={} observed_cycles={} effects={} actions=[{}]",
            canister.name,
            canister.disposition,
            canister.principal.as_deref().unwrap_or("unallocated"),
            format_cycles(canister.observed_cycles),
            canister.actions.len(),
            canister
                .actions
                .iter()
                .map(action_label)
                .collect::<Vec<_>>()
                .join(", ")
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
                format_cycles(*amount),
                ledger,
                principal,
                format_cycles(*funding_deficit_cycles),
                format_cycles(*funding_margin_cycles),
                format_cycles(*expected_post_cycles)
            ))
        })
    }));
    lines.extend(report.plan.canisters.iter().flat_map(|canister| {
        canister.actions.iter().filter_map(|action| {
            let canic_host::fleet_ensure::model::EnsureAction::FundEstate {
                amount,
                expected_post_cycles,
                ledger,
                ledger_fee_cycles,
                principal,
                ..
            } = action
            else {
                return None;
            };
            Some(format!(
                "  estate_funding {}: cycles_ledger_transfer={} ledger={} account={} fee={} expected_ledger_post={}",
                canister.name,
                format_cycles(*amount),
                ledger,
                principal,
                format_cycles(*ledger_fee_cycles),
                format_cycles(*expected_post_cycles)
            ))
        })
    }));
}

const fn action_label(action: &EnsureAction) -> &'static str {
    match action {
        EnsureAction::SealAuthority { .. } => "seal authority",
        EnsureAction::Create { .. } => "create",
        EnsureAction::Delete { .. } => "delete",
        EnsureAction::FleetProtocol { .. } => "fleet_protocol",
        EnsureAction::Fund { .. } => "native_topup",
        EnsureAction::FundEstate { .. } => "estate_funding",
        EnsureAction::Install {
            mode: InstallMode::Install,
            ..
        } => "install",
        EnsureAction::Install {
            mode: InstallMode::Reinstall,
            ..
        } => "reinstall",
        EnsureAction::Protocol { .. } => "protocol",
        EnsureAction::SetControllers { .. } => "set_controllers",
        EnsureAction::Start { .. } => "start",
        EnsureAction::Stop { .. } => "stop",
        EnsureAction::Transfer { .. } => "transfer",
    }
}

fn estate_funding_totals(
    conservation: &canic_host::fleet_ensure::model::CycleConservation,
) -> (u128, u128) {
    conservation
        .estate_funding_domains
        .iter()
        .fold((0_u128, 0_u128), |(funding, fees), domain| {
            (
                funding
                    .checked_add(domain.maximum_funding_cycles)
                    .expect("verified plan bounds estate funding"),
                fees.checked_add(domain.maximum_creation_fee_cycles)
                    .expect("verified plan bounds estate creation fees"),
            )
        })
}

fn append_estate_funding_domains(
    lines: &mut Vec<String>,
    conservation: &canic_host::fleet_ensure::model::CycleConservation,
) {
    lines.extend(conservation.estate_funding_domains.iter().map(|domain| {
        format!(
            "  {}: root_principal={} ledger={} balance={} workloads={}/{} pool={}/{} ready={} pending={} pending_detail={} available_slots={} root_funded_creations={} creation_amount={} readiness_floor={} management_creation_fee={} execution_margin={} ledger_fee={} maximum_debit={} funding={} shortfall={}",
            domain.root,
            domain.root_principal.as_deref().unwrap_or("unallocated"),
            domain.cycles_ledger,
            format_optional_cycles(domain.available_cycles),
            domain.allocated_workloads,
            domain.planned_initial_workloads,
            domain.occupied_pool_assets,
            domain.pool_maximum_size,
            domain.eligible_ready_pool_assets,
            domain.pending_creation_count,
            domain.pending_creation.as_ref().map_or_else(
                || "none".to_string(),
                |pending| format!(
                    "operation:{} diagnostic:{:?} attempts:{} available:{} required:{} shortfall:{} last_attempt:{:?} retry_at:{:?}",
                    pending.operation_id,
                    pending.diagnostic,
                    pending.attempt_count,
                    format_optional_cycles(pending.available_cycles),
                    format_optional_cycles(pending.required_cycles),
                    format_optional_cycles(pending.shortfall_cycles),
                    pending.last_attempt_at_ns,
                    pending.retry_at_ns,
                ),
            ),
            domain.available_pool_slots,
            domain.required_creation_count,
            format_cycles(domain.creation_amount_cycles),
            format_cycles(domain.readiness_floor_cycles),
            format_cycles(domain.management_creation_fee_cycles),
            format_cycles(domain.creation_execution_margin_cycles),
            format_cycles(domain.ledger_fee_cycles),
            format_cycles(domain.maximum_creation_debit_cycles),
            format_cycles(domain.maximum_funding_cycles),
            format_cycles(domain.shortfall_cycles),
        )
    }));
}

fn format_optional_cycles(cycles: Option<u128>) -> String {
    cycles.map_or_else(|| "unobserved".to_string(), format_cycles)
}

fn format_cycles(cycles: u128) -> String {
    let (unit, suffix) = if cycles >= QC {
        (QC, "Q")
    } else if cycles >= TC {
        (TC, "T")
    } else {
        (BC, "B")
    };
    let unit_thousandth = unit / 1_000;
    let mut whole = cycles / unit;
    let remainder = cycles % unit;
    let mut thousandths = (remainder + unit_thousandth / 2) / unit_thousandth;
    if thousandths == 1_000 {
        whole += 1;
        thousandths = 0;
    }
    format!("{whole}.{thousandths:03}{suffix}")
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
