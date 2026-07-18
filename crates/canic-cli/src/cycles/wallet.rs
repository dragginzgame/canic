use crate::{
    cli::clap::{
        flag_arg, parse_matches, passthrough_subcommand, render_usage, required_string,
        required_typed, string_option, string_option_or_else, value_arg,
    },
    cli::defaults::{default_icp, local_network},
    cli::globals::{internal_icp_arg, internal_network_arg},
    cli::help::print_help_or_version,
    cycles::{CyclesCommandError, convert},
    version_text,
};
use canic_host::{
    format::cycles_tc,
    icp::{IcpCli, command_display, run_output_with_stderr},
    icp_config::resolve_current_canic_icp_root,
    installed_deployment::{InstalledDeploymentRequest, resolve_installed_deployment_from_root},
    registry::RegistryEntry,
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::Path};

#[derive(Clone, Copy)]
struct WalletCommand {
    kind: WalletCommandKind,
}

///
/// WalletCommandKind
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WalletCommandKind {
    Balance,
    Convert,
    Mint,
    Transfer,
    Topup,
}

impl WalletCommandKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Balance => "balance",
            Self::Convert => "convert",
            Self::Mint => "mint",
            Self::Transfer => "transfer",
            Self::Topup => "topup",
        }
    }

    const fn parse(command: &str) -> Option<Self> {
        match command.as_bytes() {
            b"balance" => Some(Self::Balance),
            b"convert" => Some(Self::Convert),
            b"mint" => Some(Self::Mint),
            b"transfer" => Some(Self::Transfer),
            b"topup" => Some(Self::Topup),
            _ => None,
        }
    }
}

const WALLET_COMMANDS: &[WalletCommand] = &[
    WalletCommand {
        kind: WalletCommandKind::Balance,
    },
    WalletCommand {
        kind: WalletCommandKind::Convert,
    },
    WalletCommand {
        kind: WalletCommandKind::Mint,
    },
    WalletCommand {
        kind: WalletCommandKind::Transfer,
    },
    WalletCommand {
        kind: WalletCommandKind::Topup,
    },
];

const AMOUNT_ARG: &str = "amount";
const CANISTER_OR_ROLE_ARG: &str = "canister-or-role";
const CYCLES_AMOUNT_ARG: &str = "cycles-amount";
const DEPLOYMENT_ARG: &str = "deployment";
const DRY_RUN_ARG: &str = "dry-run";
const FROM_SUBACCOUNT_ARG: &str = "from-subaccount";
const ICP_AMOUNT_ARG: &str = "icp-amount";
const JSON_ARG: &str = "json";
const OF_PRINCIPAL_ARG: &str = "of-principal";
const QUIET_ARG: &str = "quiet";
const RECEIVER_ARG: &str = "receiver";
const SUBACCOUNT_ARG: &str = "subaccount";
const TO_SUBACCOUNT_ARG: &str = "to-subaccount";

const CYCLES_USAGE: &str = "\
Wrap ICP cycles commands with Canic deployment-target resolution

Usage: canic cycles <command> [OPTIONS]

Commands:
  balance   Display the selected identity cycles balance
  convert   Convert ICP held by a Canic canister to cycles
  mint      Convert ICP to cycles
  transfer  Transfer cycles to a principal or Canic deployment target
  topup     Top up an installed deployment canister
  help      Print this message or the help of the given subcommand(s)

Examples:
  canic cycles balance
  canic cycles transfer 4T aaaaa-aa
  canic cycles transfer 4T demo/root
  canic cycles transfer 4T demo/app
  canic cycles convert demo root --source root --icp-e8s 100000000 --dry-run
  canic cycles convert demo app --fabricate --cycles 4T --dry-run
  canic cycles topup demo app 4T";

///
/// IcpTargetOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct IcpTargetOptions {
    pub(super) network: String,
    pub(super) icp: String,
}

///
/// BalanceOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct BalanceOptions {
    target: IcpTargetOptions,
    json: bool,
    quiet: bool,
    subaccount: Option<String>,
    of_principal: Option<String>,
}

///
/// MintOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct MintOptions {
    target: IcpTargetOptions,
    icp_amount: Option<String>,
    cycles_amount: Option<String>,
    from_subaccount: Option<String>,
    to_subaccount: Option<String>,
    json: bool,
}

///
/// TransferOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct TransferOptions {
    target: IcpTargetOptions,
    amount: String,
    receiver: String,
    to_subaccount: Option<String>,
    from_subaccount: Option<String>,
    json: bool,
    quiet: bool,
    dry_run: bool,
}

///
/// TopupOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct TopupOptions {
    target: IcpTargetOptions,
    deployment: String,
    canister_or_role: String,
    amount_cycles: u128,
    json: bool,
    dry_run: bool,
}

///
/// ResolvedCanisterTarget
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ResolvedCanisterTarget {
    pub(super) canister_id: String,
    pub(super) role: Option<String>,
}

pub(super) fn run_cycles_command(
    command: &str,
    args: Vec<OsString>,
) -> Result<(), CyclesCommandError> {
    let Some(command) = WalletCommandKind::parse(command) else {
        return Err(CyclesCommandError::Usage(cycles_usage()));
    };
    match command {
        WalletCommandKind::Balance => {
            if print_help_or_version(&args, balance_usage, version_text()) {
                return Ok(());
            }
            let options = BalanceOptions::parse(args)?;
            run_balance(&options)
        }
        WalletCommandKind::Convert => {
            if print_help_or_version(&args, convert::usage, version_text()) {
                return Ok(());
            }
            convert::run(args)
        }
        WalletCommandKind::Mint => {
            if print_help_or_version(&args, mint_usage, version_text()) {
                return Ok(());
            }
            let options = MintOptions::parse(args)?;
            run_mint(&options)
        }
        WalletCommandKind::Transfer => {
            if print_help_or_version(&args, transfer_usage, version_text()) {
                return Ok(());
            }
            let options = TransferOptions::parse(args)?;
            run_transfer(&options)
        }
        WalletCommandKind::Topup => {
            if print_help_or_version(&args, topup_usage, version_text()) {
                return Ok(());
            }
            let options = TopupOptions::parse(args)?;
            run_topup(&options)
        }
    }
}

pub(super) fn cycles_command() -> ClapCommand {
    WALLET_COMMANDS.iter().fold(
        ClapCommand::new("cycles").bin_name("canic cycles"),
        |command, spec| command.subcommand(wallet_passthrough_command(*spec)),
    )
}

pub(super) fn cycles_usage() -> String {
    CYCLES_USAGE.to_string()
}

fn wallet_passthrough_command(spec: WalletCommand) -> ClapCommand {
    passthrough_subcommand(ClapCommand::new(spec.kind.label()).disable_help_flag(true))
}

impl IcpTargetOptions {
    pub(super) fn parse(matches: &clap::ArgMatches) -> Self {
        Self {
            network: string_option_or_else(matches, "network", local_network),
            icp: string_option_or_else(matches, "icp", default_icp),
        }
    }
}

impl BalanceOptions {
    fn parse<I>(args: I) -> Result<Self, CyclesCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(balance_command(), args)
            .map_err(|_| CyclesCommandError::Usage(balance_usage()))?;
        Ok(Self {
            target: IcpTargetOptions::parse(&matches),
            json: matches.get_flag(JSON_ARG),
            quiet: matches.get_flag(QUIET_ARG),
            subaccount: string_option(&matches, SUBACCOUNT_ARG),
            of_principal: string_option(&matches, OF_PRINCIPAL_ARG),
        })
    }
}

impl MintOptions {
    fn parse<I>(args: I) -> Result<Self, CyclesCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(mint_command(), args)
            .map_err(|_| CyclesCommandError::Usage(mint_usage()))?;
        Ok(Self {
            target: IcpTargetOptions::parse(&matches),
            icp_amount: string_option(&matches, ICP_AMOUNT_ARG),
            cycles_amount: string_option(&matches, CYCLES_AMOUNT_ARG),
            from_subaccount: string_option(&matches, FROM_SUBACCOUNT_ARG),
            to_subaccount: string_option(&matches, TO_SUBACCOUNT_ARG),
            json: matches.get_flag(JSON_ARG),
        })
    }
}

impl TransferOptions {
    fn parse<I>(args: I) -> Result<Self, CyclesCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(transfer_command(), args)
            .map_err(|_| CyclesCommandError::Usage(transfer_usage()))?;
        let options = Self {
            target: IcpTargetOptions::parse(&matches),
            amount: required_string(&matches, AMOUNT_ARG),
            receiver: required_string(&matches, RECEIVER_ARG),
            to_subaccount: string_option(&matches, TO_SUBACCOUNT_ARG),
            from_subaccount: string_option(&matches, FROM_SUBACCOUNT_ARG),
            json: matches.get_flag(JSON_ARG),
            quiet: matches.get_flag(QUIET_ARG),
            dry_run: matches.get_flag(DRY_RUN_ARG),
        };
        Ok(options)
    }
}

impl TopupOptions {
    fn parse<I>(args: I) -> Result<Self, CyclesCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(topup_command(), args)
            .map_err(|_| CyclesCommandError::Usage(topup_usage()))?;
        Ok(Self {
            target: IcpTargetOptions::parse(&matches),
            deployment: required_string(&matches, DEPLOYMENT_ARG),
            canister_or_role: required_string(&matches, CANISTER_OR_ROLE_ARG),
            amount_cycles: required_typed(&matches, AMOUNT_ARG),
            json: matches.get_flag(JSON_ARG),
            dry_run: matches.get_flag(DRY_RUN_ARG),
        })
    }
}

fn run_balance(options: &BalanceOptions) -> Result<(), CyclesCommandError> {
    let root = resolve_current_canic_icp_root().map_err(CyclesCommandError::IcpRoot)?;
    let mut command = icp_command(&options.target, &root);
    command.args(["cycles", WalletCommandKind::Balance.label()]);
    append_optional_long_arg(&mut command, SUBACCOUNT_ARG, options.subaccount.as_deref());
    append_optional_long_arg(
        &mut command,
        OF_PRINCIPAL_ARG,
        options.of_principal.as_deref(),
    );
    append_long_flag(&mut command, JSON_ARG, options.json);
    append_long_flag(&mut command, QUIET_ARG, options.quiet);
    append_target_args(&mut command, &options.target);
    run_or_print_command(&mut command, false)
}

fn run_mint(options: &MintOptions) -> Result<(), CyclesCommandError> {
    let root = resolve_current_canic_icp_root().map_err(CyclesCommandError::IcpRoot)?;
    let mut command = icp_command(&options.target, &root);
    command.args(["cycles", WalletCommandKind::Mint.label()]);
    append_optional_long_arg(&mut command, "icp", options.icp_amount.as_deref());
    append_optional_long_arg(&mut command, "cycles", options.cycles_amount.as_deref());
    append_optional_long_arg(
        &mut command,
        FROM_SUBACCOUNT_ARG,
        options.from_subaccount.as_deref(),
    );
    append_optional_long_arg(
        &mut command,
        TO_SUBACCOUNT_ARG,
        options.to_subaccount.as_deref(),
    );
    append_long_flag(&mut command, JSON_ARG, options.json);
    append_target_args(&mut command, &options.target);
    run_or_print_command(&mut command, false)
}

fn run_transfer(options: &TransferOptions) -> Result<(), CyclesCommandError> {
    let root = resolve_current_canic_icp_root().map_err(CyclesCommandError::IcpRoot)?;
    let receiver = transfer_receiver(&options.target, &root, &options.receiver)?;
    let mut command = icp_command(&options.target, &root);
    command.args(["cycles", WalletCommandKind::Transfer.label()]);
    command.arg(&options.amount);
    command.arg(receiver);
    append_optional_long_arg(
        &mut command,
        TO_SUBACCOUNT_ARG,
        options.to_subaccount.as_deref(),
    );
    append_optional_long_arg(
        &mut command,
        FROM_SUBACCOUNT_ARG,
        options.from_subaccount.as_deref(),
    );
    append_long_flag(&mut command, JSON_ARG, options.json);
    append_long_flag(&mut command, QUIET_ARG, options.quiet);
    append_target_args(&mut command, &options.target);
    run_or_print_command(&mut command, options.dry_run)
}

fn run_topup(options: &TopupOptions) -> Result<(), CyclesCommandError> {
    let root = resolve_current_canic_icp_root().map_err(CyclesCommandError::IcpRoot)?;
    let installed = resolve_deployment(&options.target, &root, &options.deployment)?;
    let target = resolve_canister_target(
        &options.deployment,
        &options.canister_or_role,
        &installed.state.root_canister_id,
        &installed.registry.entries,
    )?;
    let icp =
        IcpCli::new(&options.target.icp, Some(options.target.network.clone())).with_cwd(&root);
    if options.dry_run {
        println!(
            "{}",
            icp.canister_top_up_display(&target.canister_id, options.amount_cycles)
        );
        return Ok(());
    }

    let output = icp
        .canister_top_up_output(&target.canister_id, options.amount_cycles)
        .map_err(CyclesCommandError::from)?;
    if options.json {
        println!(
            "{}",
            serde_json::json!({
                "deployment": options.deployment,
                "role": target.role,
                "canister_id": target.canister_id,
                "amount_cycles": options.amount_cycles.to_string(),
                "amount_display": cycles_tc(options.amount_cycles),
                "icp_output": output,
            })
        );
    } else {
        println!(
            "Topped up {} with {}.",
            target_label(target.role.as_deref(), &target.canister_id),
            cycles_tc(options.amount_cycles)
        );
    }
    Ok(())
}

fn transfer_receiver(
    target: &IcpTargetOptions,
    root: &Path,
    receiver: &str,
) -> Result<String, CyclesCommandError> {
    let Some((deployment, canister_or_role)) = split_deployment_target(receiver)? else {
        return Ok(receiver.to_string());
    };
    let installed = resolve_deployment(target, root, deployment)?;
    resolve_canister_or_role(
        deployment,
        canister_or_role,
        &installed.state.root_canister_id,
        &installed.registry.entries,
    )
}

fn split_deployment_target(receiver: &str) -> Result<Option<(&str, &str)>, CyclesCommandError> {
    let Some((deployment, canister_or_role)) = receiver.split_once('/') else {
        return Ok(None);
    };
    if deployment.is_empty() || canister_or_role.is_empty() || canister_or_role.contains('/') {
        return Err(CyclesCommandError::InvalidRecipient);
    }
    Ok(Some((deployment, canister_or_role)))
}

fn resolve_canister_or_role(
    deployment: &str,
    target: &str,
    root_canister_id: &str,
    registry: &[RegistryEntry],
) -> Result<String, CyclesCommandError> {
    if target == "root" || target == root_canister_id {
        return Ok(root_canister_id.to_string());
    }
    if registry.iter().any(|entry| entry.pid == target) {
        return Ok(target.to_string());
    }
    resolve_role_principal(deployment, target, registry)
}

pub(super) fn resolve_deployment(
    target: &IcpTargetOptions,
    root: &Path,
    deployment: &str,
) -> Result<canic_host::installed_deployment::InstalledDeploymentResolution, CyclesCommandError> {
    resolve_installed_deployment_from_root(
        &InstalledDeploymentRequest {
            deployment: deployment.to_string(),
            network: target.network.clone(),
            icp: target.icp.clone(),
            detect_lost_local_root: true,
        },
        root,
    )
    .map_err(CyclesCommandError::from)
}

fn resolve_role_principal(
    deployment: &str,
    role: &str,
    registry: &[RegistryEntry],
) -> Result<String, CyclesCommandError> {
    resolve_role_entry(deployment, role, registry).map(|entry| entry.pid.clone())
}

fn resolve_role_entry<'a>(
    deployment: &str,
    role: &str,
    registry: &'a [RegistryEntry],
) -> Result<&'a RegistryEntry, CyclesCommandError> {
    let matches = registry
        .iter()
        .filter(|entry| entry.role.as_deref() == Some(role))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [entry] => Ok(entry),
        [] => Err(CyclesCommandError::UnknownTarget {
            deployment: deployment.to_string(),
            target: role.to_string(),
        }),
        _ => Err(CyclesCommandError::AmbiguousRole {
            deployment: deployment.to_string(),
            role: role.to_string(),
        }),
    }
}

pub(super) fn resolve_canister_target(
    deployment: &str,
    target: &str,
    root_canister_id: &str,
    registry: &[RegistryEntry],
) -> Result<ResolvedCanisterTarget, CyclesCommandError> {
    if target == "root" || target == root_canister_id {
        return Ok(ResolvedCanisterTarget {
            canister_id: root_canister_id.to_string(),
            role: Some("root".to_string()),
        });
    }
    if let Some(entry) = registry.iter().find(|entry| entry.pid == target) {
        return Ok(resolved_target_from_entry(entry));
    }
    let entry = resolve_role_entry(deployment, target, registry)?;
    Ok(resolved_target_from_entry(entry))
}

fn resolved_target_from_entry(entry: &RegistryEntry) -> ResolvedCanisterTarget {
    ResolvedCanisterTarget {
        canister_id: entry.pid.clone(),
        role: entry.role.clone(),
    }
}

fn icp_command(target: &IcpTargetOptions, root: &Path) -> std::process::Command {
    let icp = IcpCli::new(&target.icp, Some(target.network.clone())).with_cwd(root);
    icp.command()
}

fn append_target_args(command: &mut std::process::Command, target: &IcpTargetOptions) {
    canic_host::icp::add_target_args(command, Some(&target.network), None);
}

fn run_or_print_command(
    command: &mut std::process::Command,
    dry_run: bool,
) -> Result<(), CyclesCommandError> {
    if dry_run {
        println!("{}", command_display(command));
        return Ok(());
    }
    let output = run_output_with_stderr(command).map_err(CyclesCommandError::from)?;
    if !output.is_empty() {
        println!("{output}");
    }
    Ok(())
}

fn append_optional_long_arg(command: &mut std::process::Command, name: &str, value: Option<&str>) {
    if let Some(value) = value {
        command.arg(format!("--{name}")).arg(value);
    }
}

fn append_long_flag(command: &mut std::process::Command, name: &str, enabled: bool) {
    if enabled {
        command.arg(format!("--{name}"));
    }
}

pub(super) fn parse_cycle_amount(value: &str) -> Result<u128, String> {
    let value = value.trim();
    let compact = value.replace('_', "");
    let digits_len = compact
        .chars()
        .take_while(char::is_ascii_digit)
        .map(char::len_utf8)
        .sum::<usize>();
    if digits_len == 0 {
        return Err(invalid_cycle_amount(value));
    }
    let amount = compact
        .get(..digits_len)
        .and_then(|digits| digits.parse::<u128>().ok())
        .ok_or_else(|| invalid_cycle_amount(value))?;
    let suffix = compact[digits_len..].trim().to_ascii_lowercase();
    let multiplier = match suffix.as_str() {
        "" | "cycle" | "cycles" => 1,
        "k" => 1_000,
        "m" => 1_000_000,
        "b" => 1_000_000_000,
        "t" | "tc" => 1_000_000_000_000,
        _ => return Err(invalid_cycle_amount(value)),
    };
    amount
        .checked_mul(multiplier)
        .filter(|cycles| *cycles > 0)
        .ok_or_else(|| invalid_cycle_amount(value))
}

fn invalid_cycle_amount(value: &str) -> String {
    format!("invalid cycles amount {value}; use a positive amount such as 4T, 500B, or 1000000")
}

pub(super) fn target_label(role: Option<&str>, canister_id: &str) -> String {
    role.map_or_else(
        || format!("canister {canister_id}"),
        |role| format!("role {role} ({canister_id})"),
    )
}

fn balance_command() -> ClapCommand {
    ClapCommand::new(WalletCommandKind::Balance.label())
        .bin_name("canic cycles balance")
        .about("Display the selected identity cycles balance")
        .disable_help_flag(true)
        .arg(flag_arg(JSON_ARG).long(JSON_ARG))
        .arg(flag_arg(QUIET_ARG).long(QUIET_ARG).short('q'))
        .arg(
            value_arg(SUBACCOUNT_ARG)
                .long(SUBACCOUNT_ARG)
                .value_name(SUBACCOUNT_ARG),
        )
        .arg(
            value_arg(OF_PRINCIPAL_ARG)
                .long(OF_PRINCIPAL_ARG)
                .value_name("principal"),
        )
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}

fn mint_command() -> ClapCommand {
    ClapCommand::new(WalletCommandKind::Mint.label())
        .bin_name("canic cycles mint")
        .about("Convert ICP to cycles")
        .disable_help_flag(true)
        .arg(value_arg(ICP_AMOUNT_ARG).long("icp").value_name("amount"))
        .arg(
            value_arg(CYCLES_AMOUNT_ARG)
                .long("cycles")
                .value_name("amount"),
        )
        .arg(
            value_arg(FROM_SUBACCOUNT_ARG)
                .long(FROM_SUBACCOUNT_ARG)
                .value_name(SUBACCOUNT_ARG),
        )
        .arg(
            value_arg(TO_SUBACCOUNT_ARG)
                .long(TO_SUBACCOUNT_ARG)
                .value_name(SUBACCOUNT_ARG),
        )
        .arg(flag_arg(JSON_ARG).long(JSON_ARG))
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}

fn transfer_command() -> ClapCommand {
    ClapCommand::new(WalletCommandKind::Transfer.label())
        .bin_name("canic cycles transfer")
        .about("Transfer cycles to a principal or Canic deployment target")
        .disable_help_flag(true)
        .arg(
            value_arg(AMOUNT_ARG)
                .value_name(AMOUNT_ARG)
                .required(true)
                .help("Cycles amount to transfer"),
        )
        .arg(
            value_arg(RECEIVER_ARG)
                .value_name("receiver-or-deployment-target")
                .required(true)
                .help("Raw principal, or Canic selector like <deployment>/<role-or-canister>"),
        )
        .arg(
            value_arg(TO_SUBACCOUNT_ARG)
                .long(TO_SUBACCOUNT_ARG)
                .value_name(SUBACCOUNT_ARG),
        )
        .arg(
            value_arg(FROM_SUBACCOUNT_ARG)
                .long(FROM_SUBACCOUNT_ARG)
                .value_name(SUBACCOUNT_ARG),
        )
        .arg(flag_arg(JSON_ARG).long(JSON_ARG))
        .arg(flag_arg(QUIET_ARG).long(QUIET_ARG).short('q'))
        .arg(flag_arg(DRY_RUN_ARG).long(DRY_RUN_ARG))
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}

fn topup_command() -> ClapCommand {
    ClapCommand::new(WalletCommandKind::Topup.label())
        .bin_name("canic cycles topup")
        .about("Top up cycles for one installed deployment canister")
        .disable_help_flag(true)
        .arg(
            value_arg(DEPLOYMENT_ARG)
                .value_name(DEPLOYMENT_ARG)
                .required(true),
        )
        .arg(
            value_arg(CANISTER_OR_ROLE_ARG)
                .value_name(CANISTER_OR_ROLE_ARG)
                .required(true),
        )
        .arg(
            value_arg(AMOUNT_ARG)
                .value_name(AMOUNT_ARG)
                .required(true)
                .value_parser(clap::builder::ValueParser::new(parse_cycle_amount)),
        )
        .arg(flag_arg(JSON_ARG).long(JSON_ARG))
        .arg(flag_arg(DRY_RUN_ARG).long(DRY_RUN_ARG))
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}

fn balance_usage() -> String {
    render_usage(balance_command)
}

fn mint_usage() -> String {
    render_usage(mint_command)
}

fn transfer_usage() -> String {
    render_usage(transfer_command)
}

fn topup_usage() -> String {
    render_usage(topup_command)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Keep the public cycles namespace ICP-shaped while adding Canic target selectors.
    #[test]
    fn parses_cycles_transfer_to_deployment_target() {
        let options = TransferOptions::parse([
            OsString::from("4T"),
            OsString::from("demo/app"),
            OsString::from("--dry-run"),
        ])
        .expect("parse transfer");

        assert_eq!(options.amount, "4T");
        assert_eq!(options.receiver, "demo/app");
        assert!(options.dry_run);
    }

    // Avoid guessing between raw principals and Canic deployment names.
    #[test]
    fn transfer_requires_receiver() {
        std::assert_matches!(
            TransferOptions::parse([OsString::from("4T")]),
            Err(CyclesCommandError::Usage(_))
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
            Err(CyclesCommandError::InvalidRecipient)
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

    #[test]
    fn resolves_root_as_canister_target() {
        let registry = vec![registry_entry("child-principal", "app")];

        let root = resolve_canister_target("demo", "root", "root-principal", &registry)
            .expect("resolve root");

        assert_eq!(
            root,
            ResolvedCanisterTarget {
                canister_id: "root-principal".to_string(),
                role: Some("root".to_string()),
            }
        );
    }

    // Keep canister top-up available under the cycles family instead of a custom top-level command.
    #[test]
    fn parses_cycles_topup_options() {
        let options = TopupOptions::parse([
            OsString::from("demo"),
            OsString::from("app"),
            OsString::from("4T"),
            OsString::from("--dry-run"),
            OsString::from("--json"),
        ])
        .expect("parse topup");

        assert_eq!(options.deployment, "demo");
        assert_eq!(options.canister_or_role, "app");
        assert_eq!(options.amount_cycles, 4_000_000_000_000);
        assert!(options.dry_run);
        assert!(options.json);
    }

    // Role resolution must not silently choose between same-role canisters.
    #[test]
    fn duplicate_role_is_ambiguous() {
        let registry = vec![
            registry_entry("shard-a", "user_shard"),
            registry_entry("shard-b", "user_shard"),
        ];

        std::assert_matches!(
            resolve_role_principal("demo", "user_shard", &registry),
            Err(CyclesCommandError::AmbiguousRole { .. })
        );
    }

    fn registry_entry(pid: &str, role: &str) -> RegistryEntry {
        RegistryEntry {
            pid: pid.to_string(),
            role: Some(role.to_string()),
            parent_pid: None,
            module_hash: None,
        }
    }
}
