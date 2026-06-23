//! Module: canic_cli::blob_storage::options
//!
//! Responsibility: parse blob-storage CLI options and validate operator input.
//! Does not own: transport execution, JSON rendering, or canister policy.
//! Boundary: turns argv into typed command requests.

use crate::{
    blob_storage::BlobStorageCommandError,
    cli::{
        clap::{
            flag_arg, parse_matches, render_usage, required_string, string_option_or_else,
            value_arg,
        },
        defaults::{default_icp, local_network},
        globals::{internal_icp_arg, internal_network_arg},
    },
};
use clap::Command as ClapCommand;
use std::ffi::OsString;

const COMMAND_NAME: &str = "blob-storage";
const STATUS_COMMAND: &str = "status";
const SYNC_GATEWAYS_COMMAND: &str = "sync-gateways";
const FUND_COMMAND: &str = "fund";
const DEPLOYMENT_ARG: &str = "deployment";
const CANISTER_ARG: &str = "canister";
const CYCLES_ARG: &str = "cycles";
const DRY_RUN_ARG: &str = "dry-run";
const JSON_ARG: &str = "json";

const HELP_AFTER: &str = "\
Examples:
  canic blob-storage status local backend
  canic blob-storage status local backend --json
  canic blob-storage sync-gateways local backend --dry-run
  canic blob-storage fund local backend --cycles 1000000000000 --dry-run";

///
/// BlobStorageCommand
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum BlobStorageCommand {
    Status(StatusOptions),
    SyncGateways(SyncGatewaysOptions),
    Fund(FundOptions),
}

///
/// CommonOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CommonOptions {
    pub(super) network: String,
    pub(super) icp: String,
}

///
/// StatusOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StatusOptions {
    pub(super) deployment: String,
    pub(super) canister: String,
    pub(super) json: bool,
    pub(super) common: CommonOptions,
}

///
/// SyncGatewaysOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SyncGatewaysOptions {
    pub(super) deployment: String,
    pub(super) canister: String,
    pub(super) json: bool,
    pub(super) dry_run: bool,
    pub(super) common: CommonOptions,
}

///
/// FundOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct FundOptions {
    pub(super) deployment: String,
    pub(super) canister: String,
    pub(super) cycles: u128,
    pub(super) json: bool,
    pub(super) dry_run: bool,
    pub(super) common: CommonOptions,
}

///
/// BlobStorageOptions
///

pub(super) struct BlobStorageOptions;

impl BlobStorageOptions {
    pub(super) fn parse<I>(args: I) -> Result<BlobStorageCommand, BlobStorageCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(blob_storage_command(), args)
            .map_err(|_| BlobStorageCommandError::Usage(usage()))?;
        match matches.subcommand() {
            Some((STATUS_COMMAND, matches)) => Ok(BlobStorageCommand::Status(StatusOptions {
                deployment: required_string(matches, DEPLOYMENT_ARG),
                canister: required_string(matches, CANISTER_ARG),
                json: matches.get_flag(JSON_ARG),
                common: common_options(matches),
            })),
            Some((SYNC_GATEWAYS_COMMAND, matches)) => {
                Ok(BlobStorageCommand::SyncGateways(SyncGatewaysOptions {
                    deployment: required_string(matches, DEPLOYMENT_ARG),
                    canister: required_string(matches, CANISTER_ARG),
                    json: matches.get_flag(JSON_ARG),
                    dry_run: matches.get_flag(DRY_RUN_ARG),
                    common: common_options(matches),
                }))
            }
            Some((FUND_COMMAND, matches)) => Ok(BlobStorageCommand::Fund(FundOptions {
                deployment: required_string(matches, DEPLOYMENT_ARG),
                canister: required_string(matches, CANISTER_ARG),
                cycles: parse_cycles(&required_string(matches, CYCLES_ARG))
                    .map_err(BlobStorageCommandError::Usage)?,
                json: matches.get_flag(JSON_ARG),
                dry_run: matches.get_flag(DRY_RUN_ARG),
                common: common_options(matches),
            })),
            _ => Err(BlobStorageCommandError::Usage(usage())),
        }
    }
}

pub(super) fn usage() -> String {
    render_usage(blob_storage_command)
}

pub(super) fn sync_gateways_usage_with_bin_name() -> String {
    render_usage(|| sync_gateways_command().bin_name("canic blob-storage sync-gateways"))
}

pub(super) fn fund_usage_with_bin_name() -> String {
    render_usage(|| fund_command().bin_name("canic blob-storage fund"))
}

fn common_options(matches: &clap::ArgMatches) -> CommonOptions {
    CommonOptions {
        network: string_option_or_else(matches, "network", local_network),
        icp: string_option_or_else(matches, "icp", default_icp),
    }
}

fn blob_storage_command() -> ClapCommand {
    ClapCommand::new(COMMAND_NAME)
        .bin_name("canic blob-storage")
        .disable_help_flag(true)
        .about("Inspect and provision blob-storage billing")
        .subcommand_required(true)
        .subcommand(status_command())
        .subcommand(sync_gateways_command())
        .subcommand(fund_command())
        .after_help(HELP_AFTER)
}

fn status_command() -> ClapCommand {
    command_with_target(STATUS_COMMAND, "Inspect blob-storage billing readiness")
        .arg(flag_arg(JSON_ARG).long(JSON_ARG).help("Print JSON output"))
}

fn sync_gateways_command() -> ClapCommand {
    command_with_target(
        SYNC_GATEWAYS_COMMAND,
        "Render or run gateway-principal sync for blob-storage billing",
    )
    .arg(flag_arg(JSON_ARG).long(JSON_ARG).help("Print JSON output"))
    .arg(
        flag_arg(DRY_RUN_ARG)
            .long(DRY_RUN_ARG)
            .help("Render the canister call without executing it"),
    )
}

fn fund_command() -> ClapCommand {
    command_with_target(FUND_COMMAND, "Render or run explicit blob-storage funding")
        .arg(
            value_arg(CYCLES_ARG)
                .long(CYCLES_ARG)
                .value_name("cycles")
                .required(true)
                .help("Unsigned base-10 cycle amount"),
        )
        .arg(flag_arg(JSON_ARG).long(JSON_ARG).help("Print JSON output"))
        .arg(
            flag_arg(DRY_RUN_ARG)
                .long(DRY_RUN_ARG)
                .help("Render the canister call without executing it"),
        )
}

fn command_with_target(name: &'static str, about: &'static str) -> ClapCommand {
    ClapCommand::new(name)
        .disable_help_flag(true)
        .about(about)
        .arg(
            value_arg(DEPLOYMENT_ARG)
                .value_name(DEPLOYMENT_ARG)
                .required(true)
                .help("Installed deployment target name"),
        )
        .arg(
            value_arg(CANISTER_ARG)
                .value_name("canister-or-role")
                .required(true)
                .help("Canister principal or role name to target"),
        )
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}

fn parse_cycles(value: &str) -> Result<u128, String> {
    if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(format!(
            "invalid --cycles value {value}; use an unsigned base-10 integer"
        ));
    }
    let cycles = value
        .parse::<u128>()
        .map_err(|_| format!("invalid --cycles value {value}; exceeds u128"))?;
    if cycles == 0 {
        return Err("--cycles must be greater than zero".to_string());
    }
    Ok(cycles)
}
