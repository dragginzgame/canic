use super::{NnsCommandError, OutputFormat, leaf, now_unix_secs, write_text_or_json};
use crate::{
    cli::{
        clap::{
            parse_matches, parse_required_subcommand, passthrough_subcommand, render_help,
            required_string, required_typed,
        },
        help::{first_arg_is_help, print_help_or_version},
    },
    version_text,
};
use canic_host::nns_registry::{
    DEFAULT_NNS_REGISTRY_SOURCE_ENDPOINT, NnsRegistryVersionRequest,
    build_nns_registry_version_report, nns_registry_version_report_text,
};
use clap::Command as ClapCommand;
use std::ffi::OsString;

const REGISTRY_VERSION_HELP_AFTER: &str = "\
Examples:
  canic nns registry version
  canic --network ic nns registry version --format json
  canic nns registry version --source-endpoint https://icp-api.io";

///
/// RegistryVersionOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RegistryVersionOptions {
    pub(super) network: String,
    pub(super) format: OutputFormat,
    pub(super) source_endpoint: String,
}

pub(super) fn run<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_registry_help_or_version(&args) {
        return Ok(());
    }
    let (command, args) = parse_required_subcommand(registry_command(), args)
        .map_err(|_| NnsCommandError::Usage(registry_usage()))?;

    match command.as_str() {
        "version" => run_registry_version(args),
        _ => unreachable!("nns registry dispatch command only defines known commands"),
    }
}

fn print_registry_help_or_version(args: &[OsString]) -> bool {
    if first_arg_is_help(args) {
        println!("{}", registry_usage());
        return true;
    }
    if args.first().is_some_and(is_version_flag) {
        println!("{}", version_text());
        return true;
    }
    false
}

fn is_version_flag(arg: &OsString) -> bool {
    arg.to_str()
        .is_some_and(|arg| matches!(arg, "--version" | "-V"))
}

fn run_registry_version<I>(args: I) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, registry_version_usage, version_text()) {
        return Ok(());
    }
    let options = RegistryVersionOptions::parse(args)?;
    let request = NnsRegistryVersionRequest {
        network: options.network,
        source_endpoint: options.source_endpoint,
        now_unix_secs: now_unix_secs()?,
    };
    let report = build_nns_registry_version_report(&request)?;
    write_text_or_json(options.format, &report, nns_registry_version_report_text)
}

impl RegistryVersionOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(registry_version_command(), args)
            .map_err(|_| NnsCommandError::Usage(registry_version_usage()))?;
        Ok(Self {
            network: required_string(&matches, "network"),
            format: required_typed(&matches, "format"),
            source_endpoint: required_string(&matches, "source-endpoint"),
        })
    }
}

pub(super) fn registry_command() -> ClapCommand {
    ClapCommand::new("registry")
        .bin_name("canic nns registry")
        .about("Inspect NNS registry metadata")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("version").about("Show the latest mainnet NNS registry version"),
        ))
}

fn registry_version_command() -> ClapCommand {
    ClapCommand::new("version")
        .bin_name("canic nns registry version")
        .about("Show the latest mainnet NNS registry version")
        .disable_help_flag(true)
        .arg(leaf::format_arg())
        .arg(
            leaf::source_endpoint_arg(DEFAULT_NNS_REGISTRY_SOURCE_ENDPOINT)
                .help("IC API endpoint used for the native NNS registry query"),
        )
        .arg(leaf::network_arg())
        .after_help(REGISTRY_VERSION_HELP_AFTER)
}

pub(super) fn registry_usage() -> String {
    render_help(registry_command())
}

pub(super) fn registry_version_usage() -> String {
    render_help(registry_version_command())
}
