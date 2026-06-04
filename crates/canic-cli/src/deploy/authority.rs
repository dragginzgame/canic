use super::{
    DeployCommandError, DeployTruthOptions, current_observed_at, deploy_truth_leaf_command,
    load_deployment_check,
    output_format::{AuthorityOutputFormat, parse_authority_output_format},
    print_json, value_arg,
};
use crate::{
    cli::{
        clap::{parse_matches, parse_subcommand, passthrough_subcommand, typed_option},
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

#[derive(Clone, Copy)]
struct AuthorityCommand {
    name: &'static str,
    about: &'static str,
    bin_name: &'static str,
    help_after: &'static str,
}

const AUTHORITY_COMMANDS: &[AuthorityCommand] = &[
    CHECK_COMMAND,
    EVIDENCE_COMMAND,
    REPORT_COMMAND,
    RECEIPT_COMMAND,
];

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

const CHECK_COMMAND: AuthorityCommand = AuthorityCommand {
    name: "check",
    about: "Print the local authority reconciliation plan",
    bin_name: "canic deploy authority check",
    help_after: DEPLOY_AUTHORITY_CHECK_HELP_AFTER,
};
const EVIDENCE_COMMAND: AuthorityCommand = AuthorityCommand {
    name: "evidence",
    about: "Print the local authority dry-run evidence",
    bin_name: "canic deploy authority evidence",
    help_after: DEPLOY_AUTHORITY_EVIDENCE_HELP_AFTER,
};
const REPORT_COMMAND: AuthorityCommand = AuthorityCommand {
    name: "report",
    about: "Print the local authority reconciliation report",
    bin_name: "canic deploy authority report",
    help_after: DEPLOY_AUTHORITY_REPORT_HELP_AFTER,
};
const RECEIPT_COMMAND: AuthorityCommand = AuthorityCommand {
    name: "receipt",
    about: "Print the local authority dry-run receipt",
    bin_name: "canic deploy authority receipt",
    help_after: DEPLOY_AUTHORITY_RECEIPT_HELP_AFTER,
};

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
            truth: DeployTruthOptions::from_matches(&matches),
            format: typed_option(&matches, "format").unwrap_or(AuthorityOutputFormat::Json),
        })
    }
}

pub(super) fn command() -> ClapCommand {
    AUTHORITY_COMMANDS
        .iter()
        .fold(
            ClapCommand::new("authority")
                .bin_name("canic deploy authority")
                .about("Dry-run controller authority reconciliation")
                .disable_help_flag(true),
            |command, subcommand| command.subcommand(authority_passthrough_command(*subcommand)),
        )
        .after_help(DEPLOY_AUTHORITY_HELP_AFTER)
}

pub(super) fn check_command() -> ClapCommand {
    authority_leaf_command(CHECK_COMMAND)
}

pub(super) fn evidence_command() -> ClapCommand {
    authority_leaf_command(EVIDENCE_COMMAND)
}

pub(super) fn report_command() -> ClapCommand {
    authority_leaf_command(REPORT_COMMAND)
}

pub(super) fn receipt_command() -> ClapCommand {
    authority_leaf_command(RECEIPT_COMMAND)
}

fn format_arg() -> clap::Arg {
    value_arg("format")
        .long("format")
        .value_name("json|text")
        .num_args(1)
        .value_parser(clap::builder::ValueParser::new(
            parse_authority_output_format,
        ))
        .help("Output format; defaults to json")
}

fn authority_passthrough_command(spec: AuthorityCommand) -> ClapCommand {
    passthrough_subcommand(
        ClapCommand::new(spec.name)
            .about(spec.about)
            .disable_help_flag(true),
    )
}

fn authority_leaf_command(spec: AuthorityCommand) -> ClapCommand {
    deploy_truth_leaf_command(spec.name, spec.about)
        .arg(format_arg())
        .bin_name(spec.bin_name)
        .after_help(spec.help_after)
}

pub(super) fn usage() -> String {
    render_usage(command)
}

pub(super) fn check_usage() -> String {
    render_usage(check_command)
}

pub(super) fn evidence_usage() -> String {
    render_usage(evidence_command)
}

pub(super) fn report_usage() -> String {
    render_usage(report_command)
}

pub(super) fn receipt_usage() -> String {
    render_usage(receipt_command)
}

fn render_usage(command: fn() -> ClapCommand) -> String {
    let mut command = command();
    command.render_help().to_string()
}
