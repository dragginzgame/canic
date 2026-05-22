use crate::{
    cli::{
        clap::{
            parse_matches, parse_subcommand, passthrough_subcommand, path_option, string_option,
            value_arg,
        },
        defaults::local_network,
        globals::internal_network_arg,
        help::print_help_or_version,
    },
    version_text,
};
use canic_host::{
    canister_build::CanisterBuildProfile,
    deployment_truth::{
        DeploymentCheckV1, DeploymentReceiptV1, SafetyReportV1, SafetyStatusV1,
        compare_plan_inventory_and_receipt,
    },
    icp_config::resolve_current_canic_icp_root,
    install_root::{
        InstallRootOptions, check_install_deployment_truth,
        latest_deployment_truth_receipt_path_from_root,
    },
};
use clap::Command as ClapCommand;
use std::{
    ffi::OsString,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

const DEFAULT_ROOT_TARGET: &str = "root";
const DEFAULT_READY_TIMEOUT_SECONDS: u64 = 120;
const DEPLOY_HELP_AFTER: &str = "\
Examples:
  canic deploy plan demo
  canic deploy inventory demo
  canic deploy diff demo
  canic deploy report demo
  canic deploy check demo
  canic deploy resume-report demo
  canic deploy resume-report --receipt receipt.json demo
  canic deploy check --profile fast demo

0.41 deploy commands are read-only deployment truth checks. Mutation still flows
through `canic install`.";
const DEPLOY_PLAN_HELP_AFTER: &str = "\
Examples:
  canic deploy plan demo
  canic --network local deploy plan --profile fast demo

Prints the local DeploymentPlanV1 JSON without installing or mutating state.";
const DEPLOY_INVENTORY_HELP_AFTER: &str = "\
Examples:
  canic deploy inventory demo
  canic --network local deploy inventory --profile fast demo

Prints the local DeploymentInventoryV1 JSON without installing or mutating state.";
const DEPLOY_DIFF_HELP_AFTER: &str = "\
Examples:
  canic deploy diff demo
  canic --network local deploy diff --profile fast demo

Prints the local DeploymentDiffV1 JSON without installing or mutating state.";
const DEPLOY_REPORT_HELP_AFTER: &str = "\
Examples:
  canic deploy report demo
  canic --network local deploy report --profile fast demo

Prints the local SafetyReportV1 JSON without installing or mutating state.";
const DEPLOY_CHECK_HELP_AFTER: &str = "\
Examples:
  canic deploy check demo
  canic --network local deploy check --profile fast demo

Prints the local DeploymentCheckV1 JSON without installing or mutating state.";
const DEPLOY_RESUME_REPORT_HELP_AFTER: &str = "\
Examples:
  canic deploy resume-report demo
  canic deploy resume-report --receipt receipt.json demo
  canic --network local deploy resume-report --receipt receipt.json --profile fast demo

Prints the passive ResumeSafetyV1 JSON for the current deployment truth check
and a prior DeploymentReceiptV1. When --receipt is omitted, Canic uses the
latest local receipt under .canic/<network>/deployment-receipts/<fleet>. It
does not resume, install, or mutate state.";

///
/// DeployCommandError
///
#[derive(Debug, ThisError)]
pub enum DeployCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Check(#[from] Box<dyn std::error::Error>),

    #[error("deployment truth check blocked: {0}")]
    Blocked(String),
}

///
/// DeployTruthOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeployTruthOptions {
    fleet: String,
    network: String,
    profile: Option<CanisterBuildProfile>,
}

///
/// DeployResumeReportOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeployResumeReportOptions {
    truth: DeployTruthOptions,
    receipt: Option<PathBuf>,
}

pub fn run<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(deploy_command(), args)
        .map_err(|_| DeployCommandError::Usage(usage()))?
    {
        None => {
            println!("{}", usage());
            Ok(())
        }
        Some((command, args)) => match command.as_str() {
            "plan" => run_plan(args),
            "inventory" => run_inventory(args),
            "diff" => run_diff(args),
            "report" => run_report(args),
            "resume-report" => run_resume_report(args),
            "check" => run_check(args),
            _ => unreachable!("deploy dispatch command only defines known commands"),
        },
    }
}

fn run_plan<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, plan_usage, version_text()) {
        return Ok(());
    }

    let check = load_deployment_check(DeployTruthOptions::parse(
        args,
        deploy_plan_command,
        plan_usage,
    )?)?;
    print_json(&check.plan)?;
    Ok(())
}

fn run_inventory<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, inventory_usage, version_text()) {
        return Ok(());
    }

    let check = load_deployment_check(DeployTruthOptions::parse(
        args,
        deploy_inventory_command,
        inventory_usage,
    )?)?;
    print_json(&check.inventory)?;
    Ok(())
}

fn run_diff<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, diff_usage, version_text()) {
        return Ok(());
    }

    let check = load_deployment_check(DeployTruthOptions::parse(
        args,
        deploy_diff_command,
        diff_usage,
    )?)?;
    print_json(&check.diff)?;
    Ok(())
}

fn run_report<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, report_usage, version_text()) {
        return Ok(());
    }

    let check = load_deployment_check(DeployTruthOptions::parse(
        args,
        deploy_report_command,
        report_usage,
    )?)?;
    print_json(&check.report)?;
    Ok(())
}

fn run_check<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, check_usage, version_text()) {
        return Ok(());
    }

    let check = load_deployment_check(DeployTruthOptions::parse(
        args,
        deploy_check_command,
        check_usage,
    )?)?;
    print_json(&check)?;
    enforce_deployment_check_status(&check.report)
}

fn run_resume_report<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, resume_report_usage, version_text()) {
        return Ok(());
    }

    let options = DeployResumeReportOptions::parse(args)?;
    let receipt_path = options.receipt_path()?;
    let receipt = read_deployment_receipt(&receipt_path)?;
    let check = load_deployment_check(options.truth)?;
    let diff = compare_plan_inventory_and_receipt(&check.plan, &check.inventory, &receipt);
    print_json(&diff.resume_safety)?;
    Ok(())
}

fn load_deployment_check(
    options: DeployTruthOptions,
) -> Result<DeploymentCheckV1, DeployCommandError> {
    let icp_root = resolve_current_canic_icp_root().ok();
    check_install_deployment_truth(
        &options.into_install_root_options_with_icp_root(icp_root),
        current_observed_at()?,
    )
    .map_err(DeployCommandError::from)
}

fn print_json<T>(value: &T) -> Result<(), DeployCommandError>
where
    T: serde::Serialize,
{
    let json = serde_json::to_string_pretty(value).map_err(Box::<dyn std::error::Error>::from)?;
    println!("{json}");
    Ok(())
}

fn read_deployment_receipt(path: &PathBuf) -> Result<DeploymentReceiptV1, DeployCommandError> {
    let bytes = fs::read(path).map_err(Box::<dyn std::error::Error>::from)?;
    serde_json::from_slice(&bytes)
        .map_err(Box::<dyn std::error::Error>::from)
        .map_err(DeployCommandError::from)
}

fn enforce_deployment_check_status(report: &SafetyReportV1) -> Result<(), DeployCommandError> {
    if report.status == SafetyStatusV1::Blocked {
        return Err(DeployCommandError::Blocked(report.summary.clone()));
    }
    Ok(())
}

impl DeployResumeReportOptions {
    fn parse<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(deploy_resume_report_command(), args)
            .map_err(|_| DeployCommandError::Usage(resume_report_usage()))?;
        Ok(Self {
            truth: DeployTruthOptions::from_matches(&matches, resume_report_usage)?,
            receipt: path_option(&matches, "receipt"),
        })
    }

    fn receipt_path(&self) -> Result<PathBuf, DeployCommandError> {
        if let Some(path) = &self.receipt {
            return Ok(path.clone());
        }

        let icp_root = resolve_current_canic_icp_root().map_err(|err| {
            DeployCommandError::Usage(format!(
                "could not discover current Canic project root for latest deployment receipt: {err}; pass --receipt <file>"
            ))
        })?;

        latest_deployment_truth_receipt_path_from_root(
            &icp_root,
            &self.truth.network,
            &self.truth.fleet,
        )
        .map_err(DeployCommandError::from)?
        .ok_or_else(|| {
            DeployCommandError::Usage(format!(
                "no deployment receipt found under {} for fleet {}; pass --receipt <file>",
                icp_root
                    .join(".canic")
                    .join(&self.truth.network)
                    .join("deployment-receipts")
                    .join(&self.truth.fleet)
                    .display(),
                self.truth.fleet
            ))
        })
    }
}

impl DeployTruthOptions {
    fn parse<I>(
        args: I,
        command: impl FnOnce() -> ClapCommand,
        usage: fn() -> String,
    ) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| DeployCommandError::Usage(usage()))?;
        Self::from_matches(&matches, usage)
    }

    fn from_matches(
        matches: &clap::ArgMatches,
        usage: fn() -> String,
    ) -> Result<Self, DeployCommandError> {
        Ok(Self {
            fleet: string_option(matches, "fleet").expect("clap requires fleet"),
            network: string_option(matches, "network").unwrap_or_else(local_network),
            profile: string_option(matches, "profile")
                .as_deref()
                .map(|profile| parse_profile(profile, usage))
                .transpose()?,
        })
    }

    fn into_install_root_options_with_icp_root(
        self,
        icp_root: Option<std::path::PathBuf>,
    ) -> InstallRootOptions {
        InstallRootOptions {
            root_canister: DEFAULT_ROOT_TARGET.to_string(),
            root_build_target: DEFAULT_ROOT_TARGET.to_string(),
            network: self.network,
            icp_root,
            build_profile: self.profile,
            ready_timeout_seconds: DEFAULT_READY_TIMEOUT_SECONDS,
            config_path: Some(default_fleet_config_path(&self.fleet)),
            expected_fleet: Some(self.fleet),
            interactive_config_selection: false,
        }
    }
}

fn deploy_command() -> ClapCommand {
    ClapCommand::new("deploy")
        .bin_name("canic deploy")
        .about("Check deployment truth before mutation")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("check")
                .about("Print the local deployment truth check JSON")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("diff")
                .about("Print the local deployment diff JSON")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("inventory")
                .about("Print the local deployment inventory JSON")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("plan")
                .about("Print the local deployment plan JSON")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("report")
                .about("Print the local deployment safety report JSON")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("resume-report")
                .about("Print passive resume safety JSON from a receipt")
                .disable_help_flag(true),
        ))
        .after_help(DEPLOY_HELP_AFTER)
}

fn deploy_plan_command() -> ClapCommand {
    deploy_truth_leaf_command("plan", "Print the local deployment plan JSON")
        .after_help(DEPLOY_PLAN_HELP_AFTER)
}

fn deploy_inventory_command() -> ClapCommand {
    deploy_truth_leaf_command("inventory", "Print the local deployment inventory JSON")
        .after_help(DEPLOY_INVENTORY_HELP_AFTER)
}

fn deploy_diff_command() -> ClapCommand {
    deploy_truth_leaf_command("diff", "Print the local deployment diff JSON")
        .after_help(DEPLOY_DIFF_HELP_AFTER)
}

fn deploy_report_command() -> ClapCommand {
    deploy_truth_leaf_command("report", "Print the local deployment safety report JSON")
        .after_help(DEPLOY_REPORT_HELP_AFTER)
}

fn deploy_check_command() -> ClapCommand {
    deploy_truth_leaf_command("check", "Print the local deployment truth check JSON")
        .after_help(DEPLOY_CHECK_HELP_AFTER)
}

fn deploy_resume_report_command() -> ClapCommand {
    deploy_truth_leaf_command(
        "resume-report",
        "Print passive resume safety JSON from a prior deployment receipt",
    )
    .arg(
        value_arg("receipt")
            .long("receipt")
            .value_name("file")
            .help("DeploymentReceiptV1 JSON file to compare with current deployment truth"),
    )
    .after_help(DEPLOY_RESUME_REPORT_HELP_AFTER)
}

fn deploy_truth_leaf_command(name: &'static str, about: &'static str) -> ClapCommand {
    ClapCommand::new(name)
        .bin_name(format!("canic deploy {name}"))
        .about(about)
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Config-defined fleet name to check"),
        )
        .arg(
            value_arg("profile")
                .long("profile")
                .value_name("debug|fast|release")
                .num_args(1)
                .help("Expected canister wasm build profile"),
        )
        .arg(internal_network_arg())
}

fn usage() -> String {
    let mut command = deploy_command();
    command.render_help().to_string()
}

fn plan_usage() -> String {
    let mut command = deploy_plan_command();
    command.render_help().to_string()
}

fn inventory_usage() -> String {
    let mut command = deploy_inventory_command();
    command.render_help().to_string()
}

fn diff_usage() -> String {
    let mut command = deploy_diff_command();
    command.render_help().to_string()
}

fn report_usage() -> String {
    let mut command = deploy_report_command();
    command.render_help().to_string()
}

fn check_usage() -> String {
    let mut command = deploy_check_command();
    command.render_help().to_string()
}

fn resume_report_usage() -> String {
    let mut command = deploy_resume_report_command();
    command.render_help().to_string()
}

fn parse_profile(
    value: &str,
    usage: fn() -> String,
) -> Result<CanisterBuildProfile, DeployCommandError> {
    match value {
        "debug" => Ok(CanisterBuildProfile::Debug),
        "fast" => Ok(CanisterBuildProfile::Fast),
        "release" => Ok(CanisterBuildProfile::Release),
        _ => Err(DeployCommandError::Usage(format!(
            "invalid build profile: {value}\n\n{}",
            usage()
        ))),
    }
}

fn default_fleet_config_path(fleet: &str) -> String {
    format!("fleets/{fleet}/canic.toml")
}

fn current_observed_at() -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!(
        "unix:{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deploy_check_parses_required_fleet() {
        let options =
            DeployTruthOptions::parse([OsString::from("demo")], deploy_check_command, check_usage)
                .expect("parse deploy check");

        assert_eq!(options.fleet, "demo");
        assert_eq!(options.network, "local");
        assert_eq!(options.profile, None);
    }

    #[test]
    fn deploy_check_accepts_internal_network_and_profile() {
        let options = DeployTruthOptions::parse(
            [
                OsString::from("--profile"),
                OsString::from("fast"),
                OsString::from("demo"),
                OsString::from("--__canic-network"),
                OsString::from("ic"),
            ],
            deploy_check_command,
            check_usage,
        )
        .expect("parse deploy check");

        assert_eq!(options.network, "ic");
        assert_eq!(options.profile, Some(CanisterBuildProfile::Fast));
    }

    #[test]
    fn deploy_check_rejects_invalid_profile() {
        assert!(matches!(
            DeployTruthOptions::parse(
                [
                    OsString::from("--profile"),
                    OsString::from("turbo"),
                    OsString::from("demo"),
                ],
                deploy_check_command,
                check_usage,
            ),
            Err(DeployCommandError::Usage(_))
        ));
    }

    #[test]
    fn deploy_check_status_rejects_blocked_report() {
        let report = SafetyReportV1 {
            schema_version: 1,
            report_id: "report-1".to_string(),
            diff_id: None,
            status: SafetyStatusV1::Blocked,
            summary: "deployment inventory has 1 blocking issue(s) and 0 warning(s)".to_string(),
            hard_failures: Vec::new(),
            warnings: Vec::new(),
            next_actions: Vec::new(),
        };

        assert!(matches!(
            enforce_deployment_check_status(&report),
            Err(DeployCommandError::Blocked(message))
                if message == "deployment inventory has 1 blocking issue(s) and 0 warning(s)"
        ));
    }

    #[test]
    fn deploy_check_status_allows_warning_report() {
        let report = SafetyReportV1 {
            schema_version: 1,
            report_id: "report-1".to_string(),
            diff_id: None,
            status: SafetyStatusV1::Warning,
            summary: "deployment inventory has 1 warning(s)".to_string(),
            hard_failures: Vec::new(),
            warnings: Vec::new(),
            next_actions: Vec::new(),
        };

        enforce_deployment_check_status(&report).expect("warning report should not fail check");
    }

    #[test]
    fn deploy_leaf_commands_parse_like_check() {
        let plan =
            DeployTruthOptions::parse([OsString::from("demo")], deploy_plan_command, plan_usage)
                .expect("parse deploy plan");
        let inventory = DeployTruthOptions::parse(
            [OsString::from("demo")],
            deploy_inventory_command,
            inventory_usage,
        )
        .expect("parse deploy inventory");
        let diff =
            DeployTruthOptions::parse([OsString::from("demo")], deploy_diff_command, diff_usage)
                .expect("parse deploy diff");
        let report = DeployTruthOptions::parse(
            [OsString::from("demo")],
            deploy_report_command,
            report_usage,
        )
        .expect("parse deploy report");
        let resume_report = DeployResumeReportOptions::parse([
            OsString::from("--receipt"),
            OsString::from("receipt.json"),
            OsString::from("demo"),
        ])
        .expect("parse deploy resume-report");

        assert_eq!(plan.fleet, "demo");
        assert_eq!(inventory.fleet, "demo");
        assert_eq!(diff.fleet, "demo");
        assert_eq!(report.fleet, "demo");
        assert_eq!(resume_report.truth.fleet, "demo");
        assert_eq!(resume_report.receipt, Some(PathBuf::from("receipt.json")));
    }

    #[test]
    fn deploy_resume_report_allows_latest_local_receipt_lookup() {
        let resume_report = DeployResumeReportOptions::parse([OsString::from("demo")])
            .expect("parse deploy resume-report");

        assert_eq!(resume_report.truth.fleet, "demo");
        assert_eq!(resume_report.receipt, None);
    }

    #[test]
    fn deploy_check_builds_current_install_options() {
        let options = DeployTruthOptions {
            fleet: "demo".to_string(),
            network: "local".to_string(),
            profile: Some(CanisterBuildProfile::Fast),
        }
        .into_install_root_options_with_icp_root(Some(std::path::PathBuf::from("/tmp/icp")));

        assert_eq!(options.root_canister, "root");
        assert_eq!(options.root_build_target, "root");
        assert_eq!(options.network, "local");
        assert_eq!(options.build_profile, Some(CanisterBuildProfile::Fast));
        assert_eq!(
            options.config_path.as_deref(),
            Some("fleets/demo/canic.toml")
        );
        assert_eq!(options.expected_fleet.as_deref(), Some("demo"));
    }
}
