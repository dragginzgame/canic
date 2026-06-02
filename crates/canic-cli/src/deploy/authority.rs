use super::{
    DeployCommandError, DeployTruthOptions, current_observed_at, deploy_truth_leaf_command,
    load_deployment_check,
    output_format::{AuthorityOutputFormat, parse_authority_output_format},
    print_json, value_arg,
};
use crate::{
    cli::{
        clap::{parse_matches, parse_subcommand, passthrough_subcommand, string_option},
        help::print_help_or_version,
    },
    version_text,
};
use canic_host::deployment_truth::{
    AuthorityDryRunEvidenceV1, DeploymentCheckV1,
    authority_dry_run_evidence_from_check_with_local_ids,
    authority_dry_run_receipt_from_check_with_local_id, authority_evidence_text,
    authority_plan_text, authority_receipt_text, authority_report_from_check_with_local_id,
    authority_report_text, build_authority_reconciliation_plan,
};
use clap::Command as ClapCommand;
use std::ffi::OsString;

const DEPLOY_AUTHORITY_HELP_AFTER: &str = "\
Examples:
  canic deploy authority check demo
  canic deploy authority evidence demo
  canic deploy authority report demo
  canic deploy authority receipt demo
  canic --network local deploy authority check --profile fast demo

0.42 authority commands are dry-run reports. They do not apply controller
changes. A successful command means the local authority artifact was produced,
not that the deployment is globally safe or that controller state was changed.";
const DEPLOY_AUTHORITY_EVIDENCE_HELP_AFTER: &str = "\
Examples:
  canic deploy authority evidence demo
  canic deploy authority evidence --format text demo
  canic --network local deploy authority evidence --profile fast demo

Prints AuthorityDryRunEvidenceV1 JSON by default, or a human-readable
read-only summary with --format text. No controller changes are attempted.
Success means evidence generation succeeded, not that every deployment safety
check is clean.";
const DEPLOY_AUTHORITY_CHECK_HELP_AFTER: &str = "\
Examples:
  canic deploy authority check demo
  canic deploy authority check --format text demo
  canic --network local deploy authority check --profile fast demo

Prints the local AuthorityReconciliationPlanV1 JSON by default, or a
human-readable read-only summary with --format text. No controller changes are
attempted. Success means the local plan was produced.";
const DEPLOY_AUTHORITY_REPORT_HELP_AFTER: &str = "\
Examples:
  canic deploy authority report demo
  canic deploy authority report --format text demo
  canic --network local deploy authority report --profile fast demo

Prints the local AuthorityReportV1 JSON by default, or a human-readable
read-only summary with --format text. No controller changes are attempted.
Authority status is authority-scoped; it is not a whole-deployment safety
verdict.";
const DEPLOY_AUTHORITY_RECEIPT_HELP_AFTER: &str = "\
Examples:
  canic deploy authority receipt demo
  canic deploy authority receipt --format text demo
  canic --network local deploy authority receipt --profile fast demo

Prints an evidence-only AuthorityReceiptV1 JSON by default, or a human-readable
read-only summary with --format text. No controller changes are attempted.
Success means the dry-run receipt was produced with zero attempted controller
actions.";

///
/// DeployAuthorityOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeployAuthorityOptions {
    pub(super) truth: DeployTruthOptions,
    pub(super) format: AuthorityOutputFormat,
}

pub(super) fn run<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(command(), args).map_err(|_| DeployCommandError::Usage(usage()))? {
        Some((command, args)) if command == "check" => run_check(args),
        Some((command, args)) if command == "evidence" => run_evidence(args),
        Some((command, args)) if command == "report" => run_report(args),
        Some((command, args)) if command == "receipt" => run_receipt(args),
        _ => {
            println!("{}", usage());
            Ok(())
        }
    }
}

fn run_evidence<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        evidence_command,
        evidence_usage,
        build_dry_run_evidence,
        authority_evidence_text,
    )
}

fn run_receipt<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        receipt_command,
        receipt_usage,
        build_dry_run_receipt,
        authority_receipt_text,
    )
}

fn run_report<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        report_command,
        report_usage,
        |check| Ok(authority_report_from_check_with_local_id(check)),
        authority_report_text,
    )
}

fn run_check<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        check_command,
        check_usage,
        |check| Ok(build_authority_reconciliation_plan(check)),
        authority_plan_text,
    )
}

fn run_output<I, T>(
    args: I,
    command: impl FnOnce() -> ClapCommand,
    usage: fn() -> String,
    build: impl FnOnce(&DeploymentCheckV1) -> Result<T, DeployCommandError>,
    render_text: impl FnOnce(&T) -> String,
) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
    T: serde::Serialize,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = DeployAuthorityOptions::parse(args, command, usage)?;
    let check = load_deployment_check(options.truth)?;
    let output = build(&check)?;
    match options.format {
        AuthorityOutputFormat::Json => print_json(&output)?,
        AuthorityOutputFormat::Text => println!("{}", render_text(&output)),
    }
    Ok(())
}

pub(super) fn build_dry_run_evidence(
    check: &DeploymentCheckV1,
) -> Result<AuthorityDryRunEvidenceV1, DeployCommandError> {
    let generated_at = current_observed_at()?;
    authority_dry_run_evidence_from_check_with_local_ids(check, generated_at)
        .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

pub(super) fn build_dry_run_receipt(
    check: &DeploymentCheckV1,
) -> Result<canic_host::deployment_truth::AuthorityReceiptV1, DeployCommandError> {
    let generated_at = current_observed_at()?;
    authority_dry_run_receipt_from_check_with_local_id(check, generated_at)
        .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

impl DeployAuthorityOptions {
    pub(super) fn parse<I>(
        args: I,
        command: impl FnOnce() -> ClapCommand,
        usage: fn() -> String,
    ) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| DeployCommandError::Usage(usage()))?;
        Ok(Self {
            truth: DeployTruthOptions::from_matches(&matches, usage)?,
            format: parse_authority_output_format(
                string_option(&matches, "format").as_deref(),
                usage,
            )?,
        })
    }
}

pub(super) fn command() -> ClapCommand {
    ClapCommand::new("authority")
        .bin_name("canic deploy authority")
        .about("Dry-run controller authority reconciliation")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("check")
                .about("Print the local authority reconciliation plan")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("evidence")
                .about("Print the local authority dry-run evidence")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("report")
                .about("Print the local authority reconciliation report")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("receipt")
                .about("Print the local authority dry-run receipt")
                .disable_help_flag(true),
        ))
        .after_help(DEPLOY_AUTHORITY_HELP_AFTER)
}

pub(super) fn check_command() -> ClapCommand {
    deploy_truth_leaf_command("check", "Print the local authority reconciliation plan")
        .arg(format_arg())
        .bin_name("canic deploy authority check")
        .after_help(DEPLOY_AUTHORITY_CHECK_HELP_AFTER)
}

pub(super) fn evidence_command() -> ClapCommand {
    deploy_truth_leaf_command("evidence", "Print the local authority dry-run evidence")
        .arg(format_arg())
        .bin_name("canic deploy authority evidence")
        .after_help(DEPLOY_AUTHORITY_EVIDENCE_HELP_AFTER)
}

pub(super) fn report_command() -> ClapCommand {
    deploy_truth_leaf_command("report", "Print the local authority reconciliation report")
        .arg(format_arg())
        .bin_name("canic deploy authority report")
        .after_help(DEPLOY_AUTHORITY_REPORT_HELP_AFTER)
}

pub(super) fn receipt_command() -> ClapCommand {
    deploy_truth_leaf_command("receipt", "Print the local authority dry-run receipt")
        .arg(format_arg())
        .bin_name("canic deploy authority receipt")
        .after_help(DEPLOY_AUTHORITY_RECEIPT_HELP_AFTER)
}

fn format_arg() -> clap::Arg {
    value_arg("format")
        .long("format")
        .value_name("json|text")
        .num_args(1)
        .help("Output format; defaults to json")
}

pub(super) fn usage() -> String {
    let mut command = command();
    command.render_help().to_string()
}

pub(super) fn check_usage() -> String {
    let mut command = check_command();
    command.render_help().to_string()
}

pub(super) fn evidence_usage() -> String {
    let mut command = evidence_command();
    command.render_help().to_string()
}

pub(super) fn report_usage() -> String {
    let mut command = report_command();
    command.render_help().to_string()
}

pub(super) fn receipt_usage() -> String {
    let mut command = receipt_command();
    command.render_help().to_string()
}
