//! Module: canic_cli::token
//!
//! Responsibility: wrap ICP token commands with Canic Fleet-target recipient resolution.
//! Does not own: ledger semantics, ICP CLI execution, registry persistence, or token accounting.
//! Boundary: parses token command options and delegates resolved commands to the configured ICP CLI.

use crate::{
    cli::clap::{flag_arg, parse_matches, render_usage, required_string, string_option, value_arg},
    cli::globals::{internal_environment_arg, internal_icp_arg},
    cli::help::print_help_or_version,
    support::fleet_recipient::FleetRecipientError,
    support::icp_command::{append_flag, append_optional_arg, run_or_print},
    support::{fleet_recipient, icp_target::IcpTargetOptions},
    version_text,
};
use canic_host::{
    fleet_ensure::CurrentFleetInventoryError,
    icp::IcpCommandError,
    icp_config::{IcpConfigError, resolve_current_canic_icp_root},
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::Path};
use thiserror::Error as ThisError;

const TOKEN_USAGE: &str = "\
Wrap ICP token commands with Canic Fleet-target resolution

Usage: canic token [token-or-ledger-id] <command> [OPTIONS]

Commands:
  balance   Display the selected identity token balance
  transfer  Transfer tokens to an account, principal, or Canic Fleet target
  help      Print this message or the help of the given subcommand(s)

Examples:
  canic token balance
  canic token transfer 1.25 aaaaa-aa
  canic token icp transfer 1.25 demo/app";

///
/// TokenCommandError
///
/// CLI boundary error for token command parsing, Fleet target lookup, and
/// delegated ICP CLI execution.
///

#[derive(Debug, ThisError)]
pub enum TokenCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("failed to read Canic Fleet state: {0}")]
    IcpRoot(#[source] IcpConfigError),

    #[error(transparent)]
    CurrentFleet(#[from] CurrentFleetInventoryError),

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error("recipient must be a principal/account or <fleet>/<role-or-canister>")]
    InvalidRecipient,

    #[error("Fleet target {fleet} has no canister or role named {target}")]
    UnknownTarget { fleet: String, target: String },

    #[error("role {role} is ambiguous in Fleet target {fleet}; use one canister principal")]
    AmbiguousRole { fleet: String, role: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<FleetRecipientError> for TokenCommandError {
    fn from(error: FleetRecipientError) -> Self {
        match error {
            FleetRecipientError::AmbiguousRole { fleet, role } => {
                Self::AmbiguousRole { fleet, role }
            }
            FleetRecipientError::CurrentFleet(error) => Self::CurrentFleet(error),
            FleetRecipientError::InvalidRecipient => Self::InvalidRecipient,
            FleetRecipientError::UnknownTarget { fleet, target } => {
                Self::UnknownTarget { fleet, target }
            }
        }
    }
}

/// Split token command request with optional token symbol prefix.

#[derive(Clone, Debug, Eq, PartialEq)]
struct TokenCommandRequest {
    token: String,
    command: TokenCommandKind,
    args: Vec<OsString>,
}

///
/// TokenCommandKind
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TokenCommandKind {
    Balance,
    Transfer,
}

impl TokenCommandKind {
    const fn parse(command: &str) -> Option<Self> {
        match command.as_bytes() {
            b"balance" => Some(Self::Balance),
            b"transfer" => Some(Self::Transfer),
            _ => None,
        }
    }
}

/// Parsed `canic token balance` options.

#[derive(Clone, Debug, Eq, PartialEq)]
struct TokenBalanceOptions {
    target: IcpTargetOptions,
    token: String,
    json: bool,
    quiet: bool,
    subaccount: Option<String>,
    of_principal: Option<String>,
}

/// Parsed `canic token transfer` options.

#[derive(Clone, Debug, Eq, PartialEq)]
struct TokenTransferOptions {
    target: IcpTargetOptions,
    token: String,
    amount: String,
    receiver: String,
    to_subaccount: Option<String>,
    from_subaccount: Option<String>,
    json: bool,
    quiet: bool,
    dry_run: bool,
}

pub fn run<I>(args: I) -> Result<(), TokenCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let request = split_token_command(args)?;
    match request.command {
        TokenCommandKind::Balance => {
            if print_help_or_version(&request.args, balance_usage, version_text()) {
                return Ok(());
            }
            let options = TokenBalanceOptions::parse(request.token, request.args)?;
            run_balance(&options)
        }
        TokenCommandKind::Transfer => {
            if print_help_or_version(&request.args, transfer_usage, version_text()) {
                return Ok(());
            }
            let options = TokenTransferOptions::parse(request.token, request.args)?;
            run_transfer(&options)
        }
    }
}

fn split_token_command(args: Vec<OsString>) -> Result<TokenCommandRequest, TokenCommandError> {
    let Some((first, tail)) = args.split_first() else {
        return Err(TokenCommandError::Usage(usage()));
    };
    let first = first
        .to_str()
        .ok_or_else(|| TokenCommandError::Usage(usage()))?;
    if let Some(command) = TokenCommandKind::parse(first) {
        return Ok(TokenCommandRequest {
            token: "icp".to_string(),
            command,
            args: tail.to_vec(),
        });
    }

    let Some((command, tail)) = tail.split_first() else {
        return Err(TokenCommandError::Usage(usage()));
    };
    let command = command
        .to_str()
        .ok_or_else(|| TokenCommandError::Usage(usage()))?;
    let command =
        TokenCommandKind::parse(command).ok_or_else(|| TokenCommandError::Usage(usage()))?;
    Ok(TokenCommandRequest {
        token: first.to_string(),
        command,
        args: tail.to_vec(),
    })
}

impl TokenBalanceOptions {
    fn parse(token: String, args: Vec<OsString>) -> Result<Self, TokenCommandError> {
        let matches = parse_matches(balance_command(), args)
            .map_err(|_| TokenCommandError::Usage(balance_usage()))?;
        Ok(Self {
            target: IcpTargetOptions::parse(&matches),
            token,
            json: matches.get_flag("json"),
            quiet: matches.get_flag("quiet"),
            subaccount: string_option(&matches, "subaccount"),
            of_principal: string_option(&matches, "of-principal"),
        })
    }
}

impl TokenTransferOptions {
    fn parse(token: String, args: Vec<OsString>) -> Result<Self, TokenCommandError> {
        let matches = parse_matches(transfer_command(), args)
            .map_err(|_| TokenCommandError::Usage(transfer_usage()))?;
        let options = Self {
            target: IcpTargetOptions::parse(&matches),
            token,
            amount: required_string(&matches, "amount"),
            receiver: required_string(&matches, "receiver"),
            to_subaccount: string_option(&matches, "to-subaccount"),
            from_subaccount: string_option(&matches, "from-subaccount"),
            json: matches.get_flag("json"),
            quiet: matches.get_flag("quiet"),
            dry_run: matches.get_flag("dry-run"),
        };
        Ok(options)
    }
}

fn run_balance(options: &TokenBalanceOptions) -> Result<(), TokenCommandError> {
    let root = resolve_current_canic_icp_root().map_err(TokenCommandError::IcpRoot)?;
    let mut command = options.target.icp_cli(&root).command();
    command.args(["token", &options.token, "balance"]);
    append_optional_arg(&mut command, "--subaccount", options.subaccount.as_deref());
    append_optional_arg(
        &mut command,
        "--of-principal",
        options.of_principal.as_deref(),
    );
    append_flag(&mut command, "--json", options.json);
    append_flag(&mut command, "--quiet", options.quiet);
    options.target.append_target_args(&mut command);
    run_or_print(&mut command, false).map_err(TokenCommandError::from)
}

fn run_transfer(options: &TokenTransferOptions) -> Result<(), TokenCommandError> {
    let root = resolve_current_canic_icp_root().map_err(TokenCommandError::IcpRoot)?;
    let mut command = transfer_command_for_receiver(options, &root)?;
    run_or_print(&mut command, options.dry_run).map_err(TokenCommandError::from)
}

fn transfer_command_for_receiver(
    options: &TokenTransferOptions,
    root: &Path,
) -> Result<std::process::Command, TokenCommandError> {
    let receiver = fleet_recipient::resolve(&options.target, root, &options.receiver)
        .map_err(TokenCommandError::from)?;
    let mut command = options.target.icp_cli(root).command();
    command.args(["token", &options.token, "transfer"]);
    command.arg(&options.amount);
    command.arg(receiver);
    append_optional_arg(
        &mut command,
        "--to-subaccount",
        options.to_subaccount.as_deref(),
    );
    append_optional_arg(
        &mut command,
        "--from-subaccount",
        options.from_subaccount.as_deref(),
    );
    append_flag(&mut command, "--json", options.json);
    append_flag(&mut command, "--quiet", options.quiet);
    options.target.append_target_args(&mut command);
    Ok(command)
}

fn balance_command() -> ClapCommand {
    ClapCommand::new("balance")
        .bin_name("canic token balance")
        .about("Display the selected identity token balance")
        .disable_help_flag(true)
        .arg(flag_arg("json").long("json"))
        .arg(flag_arg("quiet").long("quiet").short('q'))
        .arg(
            value_arg("subaccount")
                .long("subaccount")
                .value_name("subaccount"),
        )
        .arg(
            value_arg("of-principal")
                .long("of-principal")
                .value_name("principal"),
        )
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
}

fn transfer_command() -> ClapCommand {
    ClapCommand::new("transfer")
        .bin_name("canic token transfer")
        .about("Transfer tokens to an account, principal, or Canic Fleet target")
        .disable_help_flag(true)
        .arg(
            value_arg("amount")
                .value_name("amount")
                .required(true)
                .help("Token amount to transfer"),
        )
        .arg(
            value_arg("receiver")
                .value_name("receiver-or-fleet-target")
                .required(true)
                .help("Raw receiver, or Canic selector like <fleet>/<role-or-canister>"),
        )
        .arg(
            value_arg("to-subaccount")
                .long("to-subaccount")
                .value_name("subaccount"),
        )
        .arg(
            value_arg("from-subaccount")
                .long("from-subaccount")
                .value_name("subaccount"),
        )
        .arg(flag_arg("json").long("json"))
        .arg(flag_arg("quiet").long("quiet").short('q'))
        .arg(flag_arg("dry-run").long("dry-run"))
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
}

fn usage() -> String {
    TOKEN_USAGE.to_string()
}

fn balance_usage() -> String {
    render_usage(balance_command)
}

fn transfer_usage() -> String {
    render_usage(transfer_command)
}

// -----------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    // Accept ICP's optional token prefix shape.
    #[test]
    fn splits_optional_token_prefix() {
        let default = split_token_command(vec![OsString::from("balance")]).expect("split default");
        assert_eq!(default.token, "icp");
        assert_eq!(default.command, TokenCommandKind::Balance);

        let explicit = split_token_command(vec![
            OsString::from("ckbtc"),
            OsString::from("transfer"),
            OsString::from("1"),
        ])
        .expect("split explicit");
        assert_eq!(explicit.token, "ckbtc");
        assert_eq!(explicit.command, TokenCommandKind::Transfer);
        assert_eq!(explicit.args, vec![OsString::from("1")]);
    }

    // Avoid guessing between raw accounts and Canic Fleet names.
    #[test]
    fn transfer_requires_receiver() {
        std::assert_matches!(
            TokenTransferOptions::parse("icp".to_string(), vec![OsString::from("1")]),
            Err(TokenCommandError::Usage(_))
        );
    }

    #[test]
    fn transfer_dispatch_preserves_receiver_and_environment() {
        let mut options = transfer_options("aaaaa-aa");
        options.target.environment = "fixture".to_string();
        options.target.icp = "custom-icp".to_string();
        let command =
            transfer_command_for_receiver(&options, Path::new("/missing-canic-workspace"))
                .expect("build transfer command");
        assert_eq!(command.get_program(), "custom-icp");
        let args = command.get_args().collect::<Vec<_>>();
        let transfer_args = ["token", "icp", "transfer", "1", "aaaaa-aa"];
        assert!(
            args.windows(transfer_args.len())
                .any(|window| window == transfer_args)
        );
        assert!(args.windows(2).any(|window| window == ["-e", "fixture"]));
    }

    #[test]
    fn transfer_rejects_malformed_or_unresolved_fleet_targets() {
        let root = Path::new("/missing-canic-workspace");
        std::assert_matches!(
            transfer_command_for_receiver(&transfer_options("demo/app/extra"), root),
            Err(TokenCommandError::InvalidRecipient)
        );
        let mut options = transfer_options("demo/app");
        options.target.environment = "fixture".to_string();
        std::assert_matches!(
            transfer_command_for_receiver(&options, root),
            Err(TokenCommandError::CurrentFleet(CurrentFleetInventoryError::NotConverged { environment, fleet }))
                if environment == "fixture" && fleet == "demo"
        );
    }

    fn transfer_options(receiver: &str) -> TokenTransferOptions {
        TokenTransferOptions::parse(
            "icp".to_string(),
            vec![OsString::from("1"), OsString::from(receiver)],
        )
        .expect("parse transfer options")
    }
}
