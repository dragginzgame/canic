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
        AuthorityDryRunEvidenceV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION, DeploymentCheckV1,
        DeploymentReceiptV1, SafetyReportV1, SafetyStatusV1, authority_dry_run_receipt_from_plan,
        authority_report_from_plan_with_check_id, build_authority_reconciliation_plan,
        compare_plan_inventory_and_receipt, validate_authority_dry_run_evidence,
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
  canic deploy authority check demo
  canic deploy authority evidence demo
  canic deploy authority report demo
  canic deploy authority receipt demo
  canic deploy resume-report demo
  canic deploy resume-report --receipt receipt.json demo
  canic deploy check --profile fast demo

Deployment truth commands are read-only checks. Mutation still flows through
`canic install`. Authority commands are dry-run reconciliation reports and do
not mutate controller state.";
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
const DEPLOY_AUTHORITY_HELP_AFTER: &str = "\
Examples:
  canic deploy authority check demo
  canic deploy authority evidence demo
  canic deploy authority report demo
  canic deploy authority receipt demo
  canic --network local deploy authority check --profile fast demo

0.42 authority commands are dry-run reports. They do not apply controller
changes.";
const DEPLOY_AUTHORITY_EVIDENCE_HELP_AFTER: &str = "\
Examples:
  canic deploy authority evidence demo
  canic --network local deploy authority evidence --profile fast demo

Prints AuthorityDryRunEvidenceV1 JSON containing the local authority plan,
report, and dry-run receipt. No controller changes are attempted.";
const DEPLOY_AUTHORITY_CHECK_HELP_AFTER: &str = "\
Examples:
  canic deploy authority check demo
  canic --network local deploy authority check --profile fast demo

Prints the local AuthorityReconciliationPlanV1 JSON without applying controller
changes.";
const DEPLOY_AUTHORITY_REPORT_HELP_AFTER: &str = "\
Examples:
  canic deploy authority report demo
  canic --network local deploy authority report --profile fast demo

Prints the local AuthorityReportV1 JSON without applying controller changes.";
const DEPLOY_AUTHORITY_RECEIPT_HELP_AFTER: &str = "\
Examples:
  canic deploy authority receipt demo
  canic --network local deploy authority receipt --profile fast demo

Prints an evidence-only AuthorityReceiptV1 JSON for the local authority dry run.
No controller changes are attempted.";
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
            "authority" => run_authority(args),
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

fn run_authority<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, authority_usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(deploy_authority_command(), args)
        .map_err(|_| DeployCommandError::Usage(authority_usage()))?
    {
        Some((command, args)) if command == "check" => run_authority_check(args),
        Some((command, args)) if command == "evidence" => run_authority_evidence(args),
        Some((command, args)) if command == "report" => run_authority_report(args),
        Some((command, args)) if command == "receipt" => run_authority_receipt(args),
        _ => {
            println!("{}", authority_usage());
            Ok(())
        }
    }
}

fn run_authority_evidence<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, authority_evidence_usage, version_text()) {
        return Ok(());
    }

    let check = load_deployment_check(DeployTruthOptions::parse(
        args,
        deploy_authority_evidence_command,
        authority_evidence_usage,
    )?)?;
    let evidence = build_authority_dry_run_evidence(&check)?;
    print_json(&evidence)?;
    Ok(())
}

fn run_authority_receipt<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, authority_receipt_usage, version_text()) {
        return Ok(());
    }

    let check = load_deployment_check(DeployTruthOptions::parse(
        args,
        deploy_authority_receipt_command,
        authority_receipt_usage,
    )?)?;
    let evidence = build_authority_dry_run_evidence(&check)?;
    print_json(&evidence.authority_receipt)?;
    Ok(())
}

fn run_authority_report<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, authority_report_usage, version_text()) {
        return Ok(());
    }

    let check = load_deployment_check(DeployTruthOptions::parse(
        args,
        deploy_authority_report_command,
        authority_report_usage,
    )?)?;
    let authority = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        format!(
            "local:{}:{}:authority-report",
            check.plan.runtime_variant, check.plan.deployment_identity.deployment_name
        ),
        Some(check.check_id),
        &authority,
    );
    print_json(&report)?;
    Ok(())
}

fn run_authority_check<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, authority_check_usage, version_text()) {
        return Ok(());
    }

    let check = load_deployment_check(DeployTruthOptions::parse(
        args,
        deploy_authority_check_command,
        authority_check_usage,
    )?)?;
    let authority = build_authority_reconciliation_plan(&check);
    print_json(&authority)?;
    Ok(())
}

fn build_authority_dry_run_evidence(
    check: &DeploymentCheckV1,
) -> Result<AuthorityDryRunEvidenceV1, DeployCommandError> {
    let authority = build_authority_reconciliation_plan(check);
    let report_id = format!(
        "local:{}:{}:authority-report",
        check.plan.runtime_variant, check.plan.deployment_identity.deployment_name
    );
    let receipt_id = format!(
        "local:{}:{}:authority-dry-run-receipt",
        check.plan.runtime_variant, check.plan.deployment_identity.deployment_name
    );
    let evidence_id = format!(
        "local:{}:{}:authority-evidence",
        check.plan.runtime_variant, check.plan.deployment_identity.deployment_name
    );
    let generated_at = current_observed_at()?;
    let report = authority_report_from_plan_with_check_id(
        report_id,
        Some(check.check_id.clone()),
        &authority,
    );
    let receipt = authority_dry_run_receipt_from_plan(
        &authority,
        &report,
        Some(check.check_id.clone()),
        receipt_id,
        generated_at.clone(),
        Some(generated_at.clone()),
    )
    .map_err(|err| DeployCommandError::Check(Box::new(err)))?;

    let evidence = AuthorityDryRunEvidenceV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        evidence_id,
        check_id: check.check_id.clone(),
        generated_at,
        reconciliation_plan: authority,
        authority_report: report,
        authority_receipt: receipt,
    };
    validate_authority_dry_run_evidence(&evidence)
        .map_err(|err| DeployCommandError::Check(Box::new(err)))?;
    Ok(evidence)
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
            ClapCommand::new("authority")
                .about("Dry-run controller authority reconciliation")
                .disable_help_flag(true),
        ))
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

fn deploy_authority_command() -> ClapCommand {
    ClapCommand::new("authority")
        .bin_name("canic deploy authority")
        .about("Dry-run controller authority reconciliation")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("check")
                .about("Print the local authority reconciliation plan JSON")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("evidence")
                .about("Print the local authority dry-run evidence JSON")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("report")
                .about("Print the local authority reconciliation report JSON")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("receipt")
                .about("Print the local authority dry-run receipt JSON")
                .disable_help_flag(true),
        ))
        .after_help(DEPLOY_AUTHORITY_HELP_AFTER)
}

fn deploy_authority_check_command() -> ClapCommand {
    deploy_truth_leaf_command(
        "check",
        "Print the local authority reconciliation plan JSON",
    )
    .bin_name("canic deploy authority check")
    .after_help(DEPLOY_AUTHORITY_CHECK_HELP_AFTER)
}

fn deploy_authority_evidence_command() -> ClapCommand {
    deploy_truth_leaf_command(
        "evidence",
        "Print the local authority dry-run evidence JSON",
    )
    .bin_name("canic deploy authority evidence")
    .after_help(DEPLOY_AUTHORITY_EVIDENCE_HELP_AFTER)
}

fn deploy_authority_report_command() -> ClapCommand {
    deploy_truth_leaf_command(
        "report",
        "Print the local authority reconciliation report JSON",
    )
    .bin_name("canic deploy authority report")
    .after_help(DEPLOY_AUTHORITY_REPORT_HELP_AFTER)
}

fn deploy_authority_receipt_command() -> ClapCommand {
    deploy_truth_leaf_command("receipt", "Print the local authority dry-run receipt JSON")
        .bin_name("canic deploy authority receipt")
        .after_help(DEPLOY_AUTHORITY_RECEIPT_HELP_AFTER)
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

fn authority_usage() -> String {
    let mut command = deploy_authority_command();
    command.render_help().to_string()
}

fn authority_check_usage() -> String {
    let mut command = deploy_authority_check_command();
    command.render_help().to_string()
}

fn authority_evidence_usage() -> String {
    let mut command = deploy_authority_evidence_command();
    command.render_help().to_string()
}

fn authority_report_usage() -> String {
    let mut command = deploy_authority_report_command();
    command.render_help().to_string()
}

fn authority_receipt_usage() -> String {
    let mut command = deploy_authority_receipt_command();
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
    use canic_host::deployment_truth::{
        AuthorityProfileV1, CanisterControlClassV1, DeploymentDiffV1, DeploymentIdentityV1,
        DeploymentInventoryV1, DeploymentPlanV1, ExpectedCanisterV1, LocalDeploymentConfigV1,
        ObservationStatusV1, ObservedCanisterV1, ResumeSafetyV1, TrustDomainV1,
        VerifierReadinessExpectationV1, VerifierReadinessObservationV1,
    };

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
        let authority_check = DeployTruthOptions::parse(
            [OsString::from("demo")],
            deploy_authority_check_command,
            authority_check_usage,
        )
        .expect("parse deploy authority check");
        let authority_evidence = DeployTruthOptions::parse(
            [OsString::from("demo")],
            deploy_authority_evidence_command,
            authority_evidence_usage,
        )
        .expect("parse deploy authority evidence");
        let authority_report = DeployTruthOptions::parse(
            [OsString::from("demo")],
            deploy_authority_report_command,
            authority_report_usage,
        )
        .expect("parse deploy authority report");
        let authority_receipt = DeployTruthOptions::parse(
            [OsString::from("demo")],
            deploy_authority_receipt_command,
            authority_receipt_usage,
        )
        .expect("parse deploy authority receipt");
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

        assert_eq!(authority_check.fleet, "demo");
        assert_eq!(authority_evidence.fleet, "demo");
        assert_eq!(authority_report.fleet, "demo");
        assert_eq!(authority_receipt.fleet, "demo");
        assert_eq!(plan.fleet, "demo");
        assert_eq!(inventory.fleet, "demo");
        assert_eq!(diff.fleet, "demo");
        assert_eq!(report.fleet, "demo");
        assert_eq!(resume_report.truth.fleet, "demo");
        assert_eq!(resume_report.receipt, Some(PathBuf::from("receipt.json")));
    }

    #[test]
    fn deploy_authority_command_dispatches_check() {
        let parsed = parse_subcommand(
            deploy_command(),
            [
                OsString::from("authority"),
                OsString::from("check"),
                OsString::from("demo"),
            ],
        )
        .expect("parse deploy authority")
        .expect("authority command");

        assert_eq!(parsed.0, "authority");
        assert_eq!(
            parsed.1,
            vec![OsString::from("check"), OsString::from("demo")]
        );

        let nested = parse_subcommand(deploy_authority_command(), parsed.1)
            .expect("parse nested authority")
            .expect("authority check command");
        assert_eq!(nested.0, "check");
        assert_eq!(nested.1, vec![OsString::from("demo")]);
    }

    #[test]
    fn deploy_authority_command_dispatches_evidence() {
        let parsed = parse_subcommand(
            deploy_command(),
            [
                OsString::from("authority"),
                OsString::from("evidence"),
                OsString::from("demo"),
            ],
        )
        .expect("parse deploy authority")
        .expect("authority command");

        assert_eq!(parsed.0, "authority");
        assert_eq!(
            parsed.1,
            vec![OsString::from("evidence"), OsString::from("demo")]
        );

        let nested = parse_subcommand(deploy_authority_command(), parsed.1)
            .expect("parse nested authority")
            .expect("authority evidence command");
        assert_eq!(nested.0, "evidence");
        assert_eq!(nested.1, vec![OsString::from("demo")]);
    }

    #[test]
    fn deploy_authority_command_dispatches_report() {
        let parsed = parse_subcommand(
            deploy_command(),
            [
                OsString::from("authority"),
                OsString::from("report"),
                OsString::from("demo"),
            ],
        )
        .expect("parse deploy authority")
        .expect("authority command");

        assert_eq!(parsed.0, "authority");
        assert_eq!(
            parsed.1,
            vec![OsString::from("report"), OsString::from("demo")]
        );

        let nested = parse_subcommand(deploy_authority_command(), parsed.1)
            .expect("parse nested authority")
            .expect("authority report command");
        assert_eq!(nested.0, "report");
        assert_eq!(nested.1, vec![OsString::from("demo")]);
    }

    #[test]
    fn deploy_authority_command_dispatches_receipt() {
        let parsed = parse_subcommand(
            deploy_command(),
            [
                OsString::from("authority"),
                OsString::from("receipt"),
                OsString::from("demo"),
            ],
        )
        .expect("parse deploy authority")
        .expect("authority command");

        assert_eq!(parsed.0, "authority");
        assert_eq!(
            parsed.1,
            vec![OsString::from("receipt"), OsString::from("demo")]
        );

        let nested = parse_subcommand(deploy_authority_command(), parsed.1)
            .expect("parse nested authority")
            .expect("authority receipt command");
        assert_eq!(nested.0, "receipt");
        assert_eq!(nested.1, vec![OsString::from("demo")]);
    }

    #[test]
    fn authority_evidence_builder_preserves_source_ids() {
        let check = sample_authority_check();

        let evidence =
            build_authority_dry_run_evidence(&check).expect("build authority dry-run evidence");

        assert_eq!(evidence.check_id, "check-1");
        assert_eq!(
            evidence.authority_report.check_id.as_deref(),
            Some("check-1")
        );
        assert_eq!(evidence.authority_report.inventory_id, "inventory-1");
        assert_eq!(
            evidence.authority_report.authority_profile_hash.as_deref(),
            Some("authority")
        );
        assert_eq!(
            evidence.authority_receipt.check_id.as_deref(),
            Some("check-1")
        );
        assert_eq!(evidence.authority_receipt.inventory_id, "inventory-1");
        assert_eq!(
            evidence.authority_receipt.authority_profile_hash.as_deref(),
            Some("authority")
        );
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

    fn sample_authority_check() -> DeploymentCheckV1 {
        let identity = sample_deployment_identity();
        let plan = sample_deployment_plan(identity.clone());
        let inventory = sample_deployment_inventory(identity);
        let diff = sample_deployment_diff(&plan, &inventory);
        let report = sample_safety_report();

        DeploymentCheckV1 {
            schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            check_id: "check-1".to_string(),
            plan,
            inventory,
            diff,
            report,
        }
    }

    fn sample_deployment_identity() -> DeploymentIdentityV1 {
        DeploymentIdentityV1 {
            deployment_name: "demo".to_string(),
            network: "local".to_string(),
            root_principal: Some("aaaaa-aa".to_string()),
            authority_profile_hash: Some("authority".to_string()),
            role_topology_hash: None,
            deployment_manifest_digest: None,
            canonical_runtime_config_digest: None,
            role_embedded_config_set_digest: None,
            artifact_set_digest: None,
            pool_identity_set_digest: None,
            canic_version: None,
            ic_memory_version: None,
        }
    }

    fn sample_deployment_plan(identity: DeploymentIdentityV1) -> DeploymentPlanV1 {
        DeploymentPlanV1 {
            schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            plan_id: "plan-1".to_string(),
            deployment_identity: identity,
            trust_domain: TrustDomainV1 {
                root_trust_anchor: Some("aaaaa-aa".to_string()),
                migration_from: None,
            },
            fleet_template: "demo".to_string(),
            runtime_variant: "local".to_string(),
            authority_profile: AuthorityProfileV1 {
                profile_id: "authority-profile-1".to_string(),
                expected_controllers: vec!["aaaaa-aa".to_string()],
                staging_controllers: Vec::new(),
                emergency_controllers: Vec::new(),
            },
            role_artifacts: Vec::new(),
            expected_canisters: vec![ExpectedCanisterV1 {
                role: "root".to_string(),
                canister_id: Some("aaaaa-aa".to_string()),
                control_class: CanisterControlClassV1::DeploymentControlled,
            }],
            expected_pool: Vec::new(),
            expected_verifier_readiness: VerifierReadinessExpectationV1 {
                required: false,
                expected_role_epochs: Vec::new(),
            },
            unresolved_assumptions: Vec::new(),
        }
    }

    fn sample_deployment_inventory(identity: DeploymentIdentityV1) -> DeploymentInventoryV1 {
        DeploymentInventoryV1 {
            schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            inventory_id: "inventory-1".to_string(),
            observed_at: "2026-05-23T00:00:00Z".to_string(),
            observed_identity: Some(identity),
            local_config: LocalDeploymentConfigV1 {
                config_path: None,
                raw_config_sha256: None,
                canonical_embedded_config_sha256: None,
            },
            observed_canisters: vec![ObservedCanisterV1 {
                canister_id: "aaaaa-aa".to_string(),
                role: Some("root".to_string()),
                control_class: CanisterControlClassV1::DeploymentControlled,
                controllers: vec!["aaaaa-aa".to_string()],
                module_hash: None,
                status: Some("running".to_string()),
                root_trust_anchor: Some("aaaaa-aa".to_string()),
                canonical_embedded_config_digest: None,
                role_assignment_source: Some("test".to_string()),
            }],
            observed_pool: Vec::new(),
            observed_artifacts: Vec::new(),
            observed_verifier_readiness: VerifierReadinessObservationV1 {
                status: ObservationStatusV1::NotObserved,
                role_epochs: Vec::new(),
            },
            unresolved_observations: Vec::new(),
        }
    }

    fn sample_deployment_diff(
        plan: &DeploymentPlanV1,
        inventory: &DeploymentInventoryV1,
    ) -> DeploymentDiffV1 {
        DeploymentDiffV1 {
            schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            plan_identity: plan.deployment_identity.clone(),
            observed_identity: inventory.observed_identity.clone(),
            artifact_diff: Vec::new(),
            controller_diff: Vec::new(),
            pool_diff: Vec::new(),
            embedded_config_diff: Vec::new(),
            module_hash_diff: Vec::new(),
            verifier_readiness_diff: Vec::new(),
            resume_safety: ResumeSafetyV1 {
                status: SafetyStatusV1::Safe,
                reasons: vec!["safe".to_string()],
            },
            hard_failures: Vec::new(),
            warnings: Vec::new(),
            resumable_phases: Vec::new(),
        }
    }

    fn sample_safety_report() -> SafetyReportV1 {
        SafetyReportV1 {
            schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            report_id: "safety-report-1".to_string(),
            diff_id: None,
            status: SafetyStatusV1::Safe,
            summary: "safe".to_string(),
            hard_failures: Vec::new(),
            warnings: Vec::new(),
            next_actions: Vec::new(),
        }
    }
}
