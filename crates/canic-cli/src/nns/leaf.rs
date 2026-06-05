use super::{NnsCommandError, OutputFormat};
use crate::{
    cli::{
        clap::{
            flag_arg, parse_matches, parse_required_subcommand, passthrough_subcommand,
            render_help, required_string, required_typed, typed_option, value_arg,
        },
        globals::internal_network_arg,
        help::print_help_or_version,
    },
    version_text,
};
use canic_host::duration::parse_duration_seconds;
use canic_subnet_catalog::MAINNET_NETWORK;
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

const FORMAT_ARG: &str = "format";
const INPUT_ARG: &str = "input";
const NETWORK_ARG: &str = "network";
const SOURCE_ENDPOINT_ARG: &str = "source-endpoint";
const LOCK_STALE_AFTER_ARG: &str = "lock-stale-after";
const DRY_RUN_ARG: &str = "dry-run";
const OUTPUT_ARG: &str = "output";
const VERBOSE_ARG: &str = "verbose";
const DEFAULT_FORMAT: &str = "text";
const DEFAULT_LOCK_STALE_AFTER: &str = "30m";

///
/// NnsLeafCommandSpec
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct NnsLeafCommandSpec {
    pub(super) command_name: &'static str,
    pub(super) bin_name: &'static str,
    pub(super) about: &'static str,
    pub(super) list_about: &'static str,
    pub(super) info_about: &'static str,
    pub(super) refresh_about: &'static str,
    pub(super) list_help_after: &'static str,
    pub(super) info_help_after: &'static str,
    pub(super) refresh_help_after: &'static str,
    pub(super) input_value_name: &'static str,
    pub(super) input_help: &'static str,
    pub(super) list_source_help: &'static str,
    pub(super) info_source_help: &'static str,
    pub(super) refresh_source_help: &'static str,
    pub(super) verbose_help: &'static str,
    pub(super) dry_run_help: &'static str,
    pub(super) output_help: &'static str,
}

///
/// NnsLeafListOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct NnsLeafListOptions {
    pub(super) network: String,
    pub(super) format: OutputFormat,
    pub(super) source_endpoint: String,
    pub(super) verbose: bool,
}

impl NnsLeafListOptions {
    pub(super) fn parse<I>(
        args: I,
        spec: &NnsLeafCommandSpec,
        default_source_endpoint: &'static str,
    ) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(list_command(spec, default_source_endpoint), args)
            .map_err(|_| NnsCommandError::Usage(list_usage(spec, default_source_endpoint)))?;
        Ok(Self {
            network: required_string(&matches, NETWORK_ARG),
            format: required_typed(&matches, FORMAT_ARG),
            source_endpoint: required_string(&matches, SOURCE_ENDPOINT_ARG),
            verbose: matches.get_flag(VERBOSE_ARG),
        })
    }
}

///
/// NnsLeafInfoOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct NnsLeafInfoOptions {
    pub(super) input: String,
    pub(super) network: String,
    pub(super) format: OutputFormat,
    pub(super) source_endpoint: String,
}

impl NnsLeafInfoOptions {
    pub(super) fn parse<I>(
        args: I,
        spec: &NnsLeafCommandSpec,
        default_source_endpoint: &'static str,
    ) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(info_command(spec, default_source_endpoint), args)
            .map_err(|_| NnsCommandError::Usage(info_usage(spec, default_source_endpoint)))?;
        Ok(Self {
            input: required_string(&matches, INPUT_ARG),
            network: required_string(&matches, NETWORK_ARG),
            format: required_typed(&matches, FORMAT_ARG),
            source_endpoint: required_string(&matches, SOURCE_ENDPOINT_ARG),
        })
    }
}

///
/// NnsLeafRefreshOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct NnsLeafRefreshOptions {
    pub(super) network: String,
    pub(super) format: OutputFormat,
    pub(super) source_endpoint: String,
    pub(super) lock_stale_after_seconds: u64,
    pub(super) dry_run: bool,
    pub(super) output_path: Option<PathBuf>,
}

impl NnsLeafRefreshOptions {
    pub(super) fn parse<I>(
        args: I,
        spec: &NnsLeafCommandSpec,
        default_source_endpoint: &'static str,
    ) -> Result<Self, NnsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(refresh_command(spec, default_source_endpoint), args)
            .map_err(|_| NnsCommandError::Usage(refresh_usage(spec, default_source_endpoint)))?;
        Ok(Self {
            network: required_string(&matches, NETWORK_ARG),
            format: required_typed(&matches, FORMAT_ARG),
            source_endpoint: required_string(&matches, SOURCE_ENDPOINT_ARG),
            lock_stale_after_seconds: required_typed(&matches, LOCK_STALE_AFTER_ARG),
            dry_run: matches.get_flag(DRY_RUN_ARG),
            output_path: typed_option(&matches, OUTPUT_ARG),
        })
    }
}

pub(super) fn run_leaf<I>(
    args: I,
    spec: &NnsLeafCommandSpec,
    run_list: fn(Vec<OsString>) -> Result<(), NnsCommandError>,
    run_info: fn(Vec<OsString>) -> Result<(), NnsCommandError>,
    run_refresh: fn(Vec<OsString>) -> Result<(), NnsCommandError>,
) -> Result<(), NnsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, || usage(spec), version_text()) {
        return Ok(());
    }
    let (command_name, args) = parse_required_subcommand(command(spec), args)
        .map_err(|_| NnsCommandError::Usage(usage(spec)))?;

    match command_name.as_str() {
        "list" => run_list(args),
        "info" => run_info(args),
        "refresh" => run_refresh(args),
        _ => unreachable!("nns leaf dispatch command only defines known commands"),
    }
}

pub(super) fn command(spec: &NnsLeafCommandSpec) -> ClapCommand {
    ClapCommand::new(spec.command_name)
        .bin_name(spec.bin_name)
        .about(spec.about)
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("list").about(spec.list_about),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("info").about(spec.info_about),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("refresh").about(spec.refresh_about),
        ))
}

pub(super) fn list_command(
    spec: &NnsLeafCommandSpec,
    default_source_endpoint: &'static str,
) -> ClapCommand {
    ClapCommand::new("list")
        .bin_name(format!("{} list", spec.bin_name))
        .about(spec.list_about)
        .disable_help_flag(true)
        .arg(format_arg())
        .arg(source_endpoint_arg(default_source_endpoint).help(spec.list_source_help))
        .arg(
            flag_arg(VERBOSE_ARG)
                .long(VERBOSE_ARG)
                .help(spec.verbose_help),
        )
        .arg(network_arg())
        .after_help(spec.list_help_after)
}

pub(super) fn info_command(
    spec: &NnsLeafCommandSpec,
    default_source_endpoint: &'static str,
) -> ClapCommand {
    ClapCommand::new("info")
        .bin_name(format!("{} info", spec.bin_name))
        .about(spec.info_about)
        .disable_help_flag(true)
        .arg(
            value_arg(INPUT_ARG)
                .value_name(spec.input_value_name)
                .required(true)
                .help(spec.input_help),
        )
        .arg(format_arg())
        .arg(source_endpoint_arg(default_source_endpoint).help(spec.info_source_help))
        .arg(network_arg())
        .after_help(spec.info_help_after)
}

pub(super) fn refresh_command(
    spec: &NnsLeafCommandSpec,
    default_source_endpoint: &'static str,
) -> ClapCommand {
    ClapCommand::new("refresh")
        .bin_name(format!("{} refresh", spec.bin_name))
        .about(spec.refresh_about)
        .disable_help_flag(true)
        .arg(format_arg())
        .arg(source_endpoint_arg(default_source_endpoint).help(spec.refresh_source_help))
        .arg(refresh_lock_stale_after_arg())
        .arg(
            flag_arg(DRY_RUN_ARG)
                .long(DRY_RUN_ARG)
                .help(spec.dry_run_help),
        )
        .arg(output_path_arg().help(spec.output_help))
        .arg(network_arg())
        .after_help(spec.refresh_help_after)
}

pub(super) fn usage(spec: &NnsLeafCommandSpec) -> String {
    render_help(command(spec))
}

pub(super) fn list_usage(
    spec: &NnsLeafCommandSpec,
    default_source_endpoint: &'static str,
) -> String {
    render_help(list_command(spec, default_source_endpoint))
}

pub(super) fn info_usage(
    spec: &NnsLeafCommandSpec,
    default_source_endpoint: &'static str,
) -> String {
    render_help(info_command(spec, default_source_endpoint))
}

pub(super) fn refresh_usage(
    spec: &NnsLeafCommandSpec,
    default_source_endpoint: &'static str,
) -> String {
    render_help(refresh_command(spec, default_source_endpoint))
}

pub(super) fn format_arg() -> clap::Arg {
    value_arg(FORMAT_ARG)
        .long(FORMAT_ARG)
        .value_name("text|json")
        .default_value(DEFAULT_FORMAT)
        .value_parser(clap::value_parser!(OutputFormat))
        .help("Output format; defaults to text")
}

pub(super) fn source_endpoint_arg(default_source_endpoint: &'static str) -> clap::Arg {
    value_arg(SOURCE_ENDPOINT_ARG)
        .long(SOURCE_ENDPOINT_ARG)
        .value_name("url")
        .default_value(default_source_endpoint)
}

pub(super) fn network_arg() -> clap::Arg {
    internal_network_arg().default_value(MAINNET_NETWORK)
}

pub(super) fn refresh_lock_stale_after_arg() -> clap::Arg {
    value_arg(LOCK_STALE_AFTER_ARG)
        .long(LOCK_STALE_AFTER_ARG)
        .value_name("duration")
        .default_value(DEFAULT_LOCK_STALE_AFTER)
        .value_parser(clap::builder::ValueParser::new(
            parse_refresh_lock_stale_after,
        ))
        .help("Treat an existing refresh lock as stale after this duration; defaults to 30m")
}

pub(super) fn output_path_arg() -> clap::Arg {
    value_arg(OUTPUT_ARG)
        .long(OUTPUT_ARG)
        .value_name("path")
        .value_parser(clap::value_parser!(PathBuf))
}

fn parse_refresh_lock_stale_after(value: &str) -> Result<u64, String> {
    parse_duration_seconds(value).map_err(|err| err.to_string())
}
