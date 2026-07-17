use super::{
    DeployCommandError, output_format::JsonTextOutputFormat, print_json_or_text, read_json_file,
    value_arg,
};
use crate::{
    cli::{
        clap::{
            flag_arg, parse_matches, parse_subcommand, passthrough_subcommand, render_usage,
            required_path, required_string, string_option_or_else,
        },
        defaults::local_network,
        globals::internal_network_arg,
        help::print_help_or_version,
    },
    version_text,
};
use canic_host::{
    deployment_truth::{
        DeploymentCheckV1, DeploymentRootVerificationReportV1, DeploymentRootVerificationRequestV1,
        deployment_root_verification_receipt_text, deployment_root_verification_report_from_check,
        deployment_root_verification_report_text, validate_deployment_root_verification_report,
    },
    icp_config::resolve_current_canic_icp_root,
    install_root::{VerifyDeploymentRootOptions, verify_registered_deployment_root},
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

#[derive(Clone, Copy)]
struct RootCommand {
    name: &'static str,
    about: &'static str,
    bin_name: &'static str,
    usage: &'static str,
    help_after: &'static str,
}

const ROOT_COMMANDS: &[RootCommand] = &[VERIFY_COMMAND];

const DEPLOY_ROOT_HELP_AFTER: &str = "\
Examples:
  canic deploy root verify demo-local --from-check deployment-check.json

Deployment-root verification records verified root state only when a registered
deployment target and DeploymentCheckV1 source evidence match. Use
`canic deploy inspect root` to build passive root-verification reports without
writing state.";
const DEPLOY_ROOT_INSPECT_HELP_AFTER: &str = "\
Examples:
  canic deploy inspect root --request root-verification.json
  canic deploy inspect root --request root-verification.json --text

Reads a DeploymentRootVerificationRequestV1-shaped JSON file and prints a
DeploymentRootVerificationReportV1 JSON artifact by default, or host-owned
passive text with --text. EvidenceSatisfied means the supplied
deployment-truth evidence is sufficient for a later explicit state transition;
this command does not persist verified root state.";
const DEPLOY_ROOT_VERIFY_HELP_AFTER: &str = "\
Examples:
  canic deploy root verify demo-local --from-check deployment-check.json
  canic deploy root verify demo-local --from-check deployment-check.json --text

Verifies a registered deployment root from a deployment-truth check artifact
and records verified root state only when deployment target identity and source
evidence match. This is not full deployment verification and does not install
code or mutate canisters.";

const INSPECT_COMMAND: RootCommand = RootCommand {
    name: "root",
    about: "Inspect deployment-root verification evidence",
    bin_name: "canic deploy inspect root",
    usage: "canic deploy inspect root --request <file>",
    help_after: DEPLOY_ROOT_INSPECT_HELP_AFTER,
};
const VERIFY_COMMAND: RootCommand = RootCommand {
    name: "verify",
    about: "Verify a registered deployment root from check evidence",
    bin_name: "canic deploy root verify",
    usage: "canic deploy root verify <deployment> --from-check <file>",
    help_after: DEPLOY_ROOT_VERIFY_HELP_AFTER,
};
const TEXT_ARG: &str = "text";

///
/// DeployRootInspectOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeployRootInspectOptions {
    pub(super) request: PathBuf,
    pub(super) format: JsonTextOutputFormat,
}

///
/// DeployRootVerifyOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeployRootVerifyOptions {
    pub(super) deployment: String,
    pub(super) from_check: PathBuf,
    pub(super) network: String,
    pub(super) format: JsonTextOutputFormat,
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
        Some((command, args)) if command == "verify" => run_verify(args),
        _ => {
            println!("{}", usage());
            Ok(())
        }
    }
}

pub(super) fn run_inspect<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, inspect_usage, version_text()) {
        return Ok(());
    }

    let options = DeployRootInspectOptions::parse(args)?;
    let request = read_json_file::<DeploymentRootVerificationRequestV1>(&options.request)?;
    let report = build_verification_report(request)?;
    print_json_or_text(
        options.format,
        &report,
        deployment_root_verification_report_text,
    )
}

fn run_verify<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, verify_usage, version_text()) {
        return Ok(());
    }

    let options = DeployRootVerifyOptions::parse(args)?;
    let check = read_json_file::<DeploymentCheckV1>(&options.from_check)?;
    let receipt = verify_registered_deployment_root(VerifyDeploymentRootOptions {
        deployment_name: options.deployment,
        network: options.network,
        deployment_check: check,
        verified_at_unix_secs: None,
        icp_root: Some(resolve_current_canic_icp_root()?),
    })
    .map_err(DeployCommandError::from)?;
    print_json_or_text(
        options.format,
        &receipt,
        deployment_root_verification_receipt_text,
    )
}

pub(super) fn build_verification_report(
    request: DeploymentRootVerificationRequestV1,
) -> Result<DeploymentRootVerificationReportV1, DeployCommandError> {
    let report = deployment_root_verification_report_from_check(request);
    validate_deployment_root_verification_report(&report)
        .map_err(|err| DeployCommandError::Check(Box::new(err)))?;
    Ok(report)
}

impl DeployRootInspectOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(inspect_command(), args)
            .map_err(|_| DeployCommandError::Usage(inspect_usage()))?;
        Ok(Self {
            request: required_path(&matches, "request"),
            format: JsonTextOutputFormat::from_text_flag(matches.get_flag(TEXT_ARG)),
        })
    }
}

impl DeployRootVerifyOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(verify_command(), args)
            .map_err(|_| DeployCommandError::Usage(verify_usage()))?;
        Ok(Self {
            deployment: required_string(&matches, "deployment"),
            from_check: required_path(&matches, "from-check"),
            network: string_option_or_else(&matches, "network", local_network),
            format: JsonTextOutputFormat::from_text_flag(matches.get_flag(TEXT_ARG)),
        })
    }
}

pub(super) fn command() -> ClapCommand {
    ROOT_COMMANDS
        .iter()
        .fold(
            ClapCommand::new("root")
                .bin_name("canic deploy root")
                .about("Verify deployment-root state")
                .disable_help_flag(true),
            |command, subcommand| command.subcommand(root_passthrough_command(*subcommand)),
        )
        .after_help(DEPLOY_ROOT_HELP_AFTER)
}

pub(super) fn inspect_command() -> ClapCommand {
    root_leaf_command(INSPECT_COMMAND).arg(
        value_arg("request")
            .long("request")
            .value_name("file")
            .required(true)
            .help("DeploymentRootVerificationRequestV1 JSON file to inspect"),
    )
}

fn verify_command() -> ClapCommand {
    root_leaf_command(VERIFY_COMMAND)
        .arg(
            value_arg("deployment")
                .value_name("deployment")
                .required(true)
                .help("Registered deployment target whose root should be verified"),
        )
        .arg(
            value_arg("from-check")
                .long("from-check")
                .value_name("file")
                .required(true)
                .help("DeploymentCheckV1 JSON artifact carrying explicit root evidence"),
        )
        .arg(internal_network_arg())
}

fn text_arg() -> clap::Arg {
    flag_arg(TEXT_ARG)
        .long(TEXT_ARG)
        .help("Print human-readable text output")
}

fn root_passthrough_command(spec: RootCommand) -> ClapCommand {
    passthrough_subcommand(
        ClapCommand::new(spec.name)
            .about(spec.about)
            .disable_help_flag(true),
    )
}

fn root_leaf_command(spec: RootCommand) -> ClapCommand {
    ClapCommand::new(spec.name)
        .bin_name(spec.bin_name)
        .about(spec.about)
        .disable_help_flag(true)
        .override_usage(spec.usage)
        .arg(text_arg())
        .after_help(spec.help_after)
}

pub(super) fn usage() -> String {
    render_usage(command)
}

pub(super) fn inspect_usage() -> String {
    render_usage(inspect_command)
}

pub(super) fn verify_usage() -> String {
    render_usage(verify_command)
}
