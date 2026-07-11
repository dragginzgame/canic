//! Module: canic_cli::token
//!
//! Responsibility: wrap ICP token commands with Canic deployment-target recipient resolution.
//! Does not own: ledger semantics, ICP CLI execution, registry persistence, or token accounting.
//! Boundary: parses token command options and delegates resolved commands to the configured ICP CLI.

use crate::{
    cli::clap::{
        flag_arg, parse_matches, render_usage, required_string, string_option,
        string_option_or_else, value_arg,
    },
    cli::defaults::{default_icp, local_network},
    cli::globals::{internal_icp_arg, internal_network_arg},
    cli::help::print_help_or_version,
    version_text,
};
use canic_host::{
    icp::{IcpCli, IcpCommandError, command_display, run_output_with_stderr},
    icp_config::resolve_current_canic_icp_root,
    installed_deployment::{
        InstalledDeploymentError, InstalledDeploymentRequest,
        resolve_installed_deployment_from_root,
    },
    registry::{RegistryEntry, RegistryParseError},
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::Path};
use thiserror::Error as ThisError;

const TOKEN_USAGE: &str = "\
Wrap ICP token commands with Canic deployment-target resolution

Usage: canic token [token-or-ledger-id] <command> [OPTIONS]

Commands:
  balance   Display the selected identity token balance
  transfer  Transfer tokens to an account, principal, or Canic deployment target
  help      Print this message or the help of the given subcommand(s)

Examples:
  canic token balance
  canic token icp balance
  canic token transfer 1.25 aaaaa-aa
  canic token transfer 1.25 demo/root
  canic token icp transfer 1.25 demo/app";

///
/// TokenCommandError
///
/// CLI boundary error for token command parsing, deployment target lookup, and
/// delegated ICP CLI execution.
///

#[derive(Debug, ThisError)]
pub enum TokenCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(
        "deployment target {deployment} is not installed on network {network}; run `canic install <fleet-template>` or `canic deploy register {deployment} --fleet-template <fleet-template> --root <principal> --allow-unverified` before using token commands"
    )]
    NoInstalledDeployment { network: String, deployment: String },

    #[error("failed to read canic deployment state: {0}")]
    InstallState(String),

    #[error("local replica query failed: {0}")]
    ReplicaQuery(String),

    #[error("icp command failed: {command}\n{stderr}")]
    IcpFailed { command: String, stderr: String },

    #[error("recipient must be a principal/account or <deployment>/<role-or-canister>")]
    InvalidRecipient,

    #[error("deployment target {deployment} has no canister or role named {target}")]
    UnknownTarget { deployment: String, target: String },

    #[error(
        "role {role} is ambiguous in deployment target {deployment}; use one canister principal"
    )]
    AmbiguousRole { deployment: String, role: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Registry(#[from] RegistryParseError),
}

/// Parsed ICP CLI target context shared by token subcommands.

#[derive(Clone, Debug, Eq, PartialEq)]
struct IcpTargetOptions {
    network: String,
    icp: String,
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

impl IcpTargetOptions {
    fn parse(matches: &clap::ArgMatches) -> Self {
        Self {
            network: string_option_or_else(matches, "network", local_network),
            icp: string_option_or_else(matches, "icp", default_icp),
        }
    }
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
    let root = resolve_current_canic_icp_root()
        .map_err(|err| TokenCommandError::InstallState(err.to_string()))?;
    let mut command = icp_command(&options.target, &root);
    command.args(["token", &options.token, "balance"]);
    append_optional_arg(&mut command, "--subaccount", options.subaccount.as_deref());
    append_optional_arg(
        &mut command,
        "--of-principal",
        options.of_principal.as_deref(),
    );
    append_flag(&mut command, "--json", options.json);
    append_flag(&mut command, "--quiet", options.quiet);
    append_target_args(&mut command, &options.target);
    run_or_print_command(&mut command, false)
}

fn run_transfer(options: &TokenTransferOptions) -> Result<(), TokenCommandError> {
    let root = resolve_current_canic_icp_root()
        .map_err(|err| TokenCommandError::InstallState(err.to_string()))?;
    let receiver = transfer_receiver(&options.target, &root, &options.receiver)?;
    let mut command = icp_command(&options.target, &root);
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
    append_target_args(&mut command, &options.target);
    run_or_print_command(&mut command, options.dry_run)
}

fn transfer_receiver(
    target: &IcpTargetOptions,
    root: &Path,
    receiver: &str,
) -> Result<String, TokenCommandError> {
    let Some((deployment, canister_or_role)) = split_deployment_target(receiver)? else {
        return Ok(receiver.to_string());
    };
    let installed = resolve_installed_deployment_from_root(
        &InstalledDeploymentRequest {
            deployment: deployment.to_string(),
            network: target.network.clone(),
            icp: target.icp.clone(),
            detect_lost_local_root: true,
        },
        root,
    )
    .map_err(token_installed_deployment_error)?;
    resolve_canister_or_role(
        deployment,
        canister_or_role,
        &installed.state.root_canister_id,
        &installed.registry.entries,
    )
}

fn split_deployment_target(receiver: &str) -> Result<Option<(&str, &str)>, TokenCommandError> {
    let Some((deployment, canister_or_role)) = receiver.split_once('/') else {
        return Ok(None);
    };
    if deployment.is_empty() || canister_or_role.is_empty() || canister_or_role.contains('/') {
        return Err(TokenCommandError::InvalidRecipient);
    }
    Ok(Some((deployment, canister_or_role)))
}

fn resolve_canister_or_role(
    deployment: &str,
    target: &str,
    root_canister_id: &str,
    registry: &[RegistryEntry],
) -> Result<String, TokenCommandError> {
    if target == "root" || target == root_canister_id {
        return Ok(root_canister_id.to_string());
    }
    if registry.iter().any(|entry| entry.pid == target) {
        return Ok(target.to_string());
    }
    resolve_role_principal(deployment, target, registry)
}

fn resolve_role_principal(
    deployment: &str,
    role: &str,
    registry: &[RegistryEntry],
) -> Result<String, TokenCommandError> {
    let matches = registry
        .iter()
        .filter(|entry| entry.role.as_deref() == Some(role))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [entry] => Ok(entry.pid.clone()),
        [] => Err(TokenCommandError::UnknownTarget {
            deployment: deployment.to_string(),
            target: role.to_string(),
        }),
        _ => Err(TokenCommandError::AmbiguousRole {
            deployment: deployment.to_string(),
            role: role.to_string(),
        }),
    }
}

fn icp_command(target: &IcpTargetOptions, root: &Path) -> std::process::Command {
    let icp = IcpCli::new(&target.icp, None, Some(target.network.clone())).with_cwd(root);
    icp.command()
}

fn append_target_args(command: &mut std::process::Command, target: &IcpTargetOptions) {
    canic_host::icp::add_target_args(command, None, Some(&target.network), None);
}

fn run_or_print_command(
    command: &mut std::process::Command,
    dry_run: bool,
) -> Result<(), TokenCommandError> {
    if dry_run {
        println!("{}", command_display(command));
        return Ok(());
    }
    let output = run_output_with_stderr(command).map_err(token_icp_error)?;
    if !output.is_empty() {
        println!("{output}");
    }
    Ok(())
}

fn append_optional_arg(command: &mut std::process::Command, flag: &str, value: Option<&str>) {
    if let Some(value) = value {
        command.args([flag, value]);
    }
}

fn append_flag(command: &mut std::process::Command, flag: &str, enabled: bool) {
    if enabled {
        command.arg(flag);
    }
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
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}

fn transfer_command() -> ClapCommand {
    ClapCommand::new("transfer")
        .bin_name("canic token transfer")
        .about("Transfer tokens to an account, principal, or Canic deployment target")
        .disable_help_flag(true)
        .arg(
            value_arg("amount")
                .value_name("amount")
                .required(true)
                .help("Token amount to transfer"),
        )
        .arg(
            value_arg("receiver")
                .value_name("receiver-or-deployment-target")
                .required(true)
                .help("Raw receiver, or Canic selector like <deployment>/<role-or-canister>"),
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
        .arg(internal_network_arg())
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

fn token_installed_deployment_error(error: InstalledDeploymentError) -> TokenCommandError {
    match error {
        InstalledDeploymentError::NoInstalledDeployment {
            network,
            deployment,
        } => TokenCommandError::NoInstalledDeployment {
            network,
            deployment,
        },
        InstalledDeploymentError::InstallState(error) => TokenCommandError::InstallState(error),
        InstalledDeploymentError::ReplicaQuery(error) => TokenCommandError::ReplicaQuery(error),
        InstalledDeploymentError::IcpFailed { command, stderr } => {
            TokenCommandError::IcpFailed { command, stderr }
        }
        InstalledDeploymentError::LostLocalDeployment { root, .. } => {
            TokenCommandError::ReplicaQuery(format!("root canister {root} is not present"))
        }
        InstalledDeploymentError::Registry(error) => TokenCommandError::Registry(error),
        InstalledDeploymentError::Io(error) => TokenCommandError::Io(error),
    }
}

fn token_icp_error(error: IcpCommandError) -> TokenCommandError {
    match error {
        IcpCommandError::Io(error) => TokenCommandError::Io(error),
        IcpCommandError::Failed { command, stderr } => {
            TokenCommandError::IcpFailed { command, stderr }
        }
        IcpCommandError::Json {
            command, output, ..
        } => TokenCommandError::IcpFailed {
            command,
            stderr: output,
        },
        error @ (IcpCommandError::MissingCli { .. }
        | IcpCommandError::IncompatibleCliVersion { .. }) => TokenCommandError::IcpFailed {
            command: "icp --version".to_string(),
            stderr: error.to_string(),
        },
        IcpCommandError::SnapshotIdUnavailable { output } => TokenCommandError::IcpFailed {
            command: "icp canister snapshot create".to_string(),
            stderr: output,
        },
    }
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

    // Avoid guessing between raw accounts and Canic deployment names.
    #[test]
    fn transfer_requires_receiver() {
        std::assert_matches!(
            TokenTransferOptions::parse("icp".to_string(), vec![OsString::from("1")]),
            Err(TokenCommandError::Usage(_))
        );
    }

    #[test]
    fn parses_compact_deployment_target_receiver() {
        assert_eq!(
            split_deployment_target("demo/app").expect("split target"),
            Some(("demo", "app"))
        );
        assert_eq!(
            split_deployment_target("aaaaa-aa").expect("split raw receiver"),
            None
        );
        std::assert_matches!(
            split_deployment_target("demo/app/extra"),
            Err(TokenCommandError::InvalidRecipient)
        );
    }

    #[test]
    fn resolves_compact_deployment_target_receiver() {
        let registry = vec![registry_entry("child-principal", "app")];

        assert_eq!(
            resolve_canister_or_role("demo", "root", "root-principal", &registry)
                .expect("resolve root"),
            "root-principal"
        );
        assert_eq!(
            resolve_canister_or_role("demo", "child-principal", "root-principal", &registry)
                .expect("resolve child principal"),
            "child-principal"
        );
        assert_eq!(
            resolve_canister_or_role("demo", "app", "root-principal", &registry)
                .expect("resolve role"),
            "child-principal"
        );
    }

    fn registry_entry(pid: &str, role: &str) -> RegistryEntry {
        RegistryEntry {
            pid: pid.to_string(),
            role: Some(role.to_string()),
            kind: None,
            parent_pid: None,
            module_hash: None,
        }
    }
}
