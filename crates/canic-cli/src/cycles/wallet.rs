use crate::{
    cli::clap::{
        flag_arg, parse_matches, passthrough_subcommand, render_usage, required_string,
        required_typed, string_option, value_arg,
    },
    cli::globals::{internal_environment_arg, internal_icp_arg},
    cli::help::print_help_or_version,
    cycles::{CyclesCommandError, convert, funding},
    support::{fleet_recipient, icp_target::IcpTargetOptions},
    version_text,
};
use canic_core::cdk::types::Principal;
use canic_host::{
    fleet_ensure::resolve_current_fleet,
    format::cycles_tc,
    icp::{command_display, run_output_with_stderr},
    icp_config::resolve_current_canic_icp_root,
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::Path};

///
/// WalletCommandKind
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WalletCommandKind {
    Balance,
    Convert,
    Funding,
    Mint,
    Topup,
    Transfer,
}

impl WalletCommandKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Balance => "balance",
            Self::Convert => "convert",
            Self::Funding => "funding",
            Self::Mint => "mint",
            Self::Topup => "topup",
            Self::Transfer => "transfer",
        }
    }

    const fn parse(command: &str) -> Option<Self> {
        match command.as_bytes() {
            b"balance" => Some(Self::Balance),
            b"convert" => Some(Self::Convert),
            b"funding" => Some(Self::Funding),
            b"mint" => Some(Self::Mint),
            b"topup" => Some(Self::Topup),
            b"transfer" => Some(Self::Transfer),
            _ => None,
        }
    }
}

const WALLET_COMMANDS: &[WalletCommandKind] = &[
    WalletCommandKind::Balance,
    WalletCommandKind::Convert,
    WalletCommandKind::Funding,
    WalletCommandKind::Mint,
    WalletCommandKind::Topup,
    WalletCommandKind::Transfer,
];

const AMOUNT_ARG: &str = "amount";
const INFRASTRUCTURE_TARGET_ARG: &str = "infrastructure-target";
const CYCLES_AMOUNT_ARG: &str = "cycles-amount";
const FLEET_ARG: &str = "fleet";
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
Wrap ICP cycles commands with Canic Fleet resolution

Usage: canic cycles <command> [OPTIONS]

Commands:
  balance   Display the selected identity cycles balance
  convert   Convert ICP held by a current Fleet Subnet Root to cycles for that root
  funding   Inspect protected current Coordinator and Root funding headroom
  mint      Convert ICP to cycles
  topup     Top up a current Fleet Coordinator or Root
  transfer  Transfer cycles to a principal or Canic Fleet target
  help      Print this message or the help of the given subcommand(s)

Examples:
  canic cycles balance
  canic cycles topup demo coordinator 4T
  canic cycles transfer 4T demo/app";

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
    fleet: String,
    infrastructure_target: String,
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
        WalletCommandKind::Funding => {
            if print_help_or_version(&args, funding::usage, version_text()) {
                return Ok(());
            }
            funding::run(args)
        }
        WalletCommandKind::Mint => {
            if print_help_or_version(&args, mint_usage, version_text()) {
                return Ok(());
            }
            let options = MintOptions::parse(args)?;
            run_mint(&options)
        }
        WalletCommandKind::Topup => {
            if print_help_or_version(&args, topup_usage, version_text()) {
                return Ok(());
            }
            let options = TopupOptions::parse(args)?;
            run_topup(&options)
        }
        WalletCommandKind::Transfer => {
            if print_help_or_version(&args, transfer_usage, version_text()) {
                return Ok(());
            }
            let options = TransferOptions::parse(args)?;
            run_transfer(&options)
        }
    }
}

pub(super) fn cycles_command() -> ClapCommand {
    WALLET_COMMANDS.iter().fold(
        ClapCommand::new("cycles").bin_name("canic cycles"),
        |command, kind| {
            command.subcommand(passthrough_subcommand(
                ClapCommand::new(kind.label()).disable_help_flag(true),
            ))
        },
    )
}

pub(super) fn cycles_usage() -> String {
    CYCLES_USAGE.to_string()
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
            fleet: required_string(&matches, FLEET_ARG),
            infrastructure_target: required_string(&matches, INFRASTRUCTURE_TARGET_ARG),
            amount_cycles: required_typed(&matches, AMOUNT_ARG),
            json: matches.get_flag(JSON_ARG),
            dry_run: matches.get_flag(DRY_RUN_ARG),
        })
    }
}

fn run_balance(options: &BalanceOptions) -> Result<(), CyclesCommandError> {
    let root = resolve_current_canic_icp_root().map_err(CyclesCommandError::IcpRoot)?;
    let mut command = options.target.icp_cli(&root).command();
    command.args(["cycles", WalletCommandKind::Balance.label()]);
    append_optional_long_arg(&mut command, SUBACCOUNT_ARG, options.subaccount.as_deref());
    append_optional_long_arg(
        &mut command,
        OF_PRINCIPAL_ARG,
        options.of_principal.as_deref(),
    );
    append_long_flag(&mut command, JSON_ARG, options.json);
    append_long_flag(&mut command, QUIET_ARG, options.quiet);
    options.target.append_target_args(&mut command);
    run_or_print_command(&mut command, false)
}

fn run_mint(options: &MintOptions) -> Result<(), CyclesCommandError> {
    let root = resolve_current_canic_icp_root().map_err(CyclesCommandError::IcpRoot)?;
    let mut command = options.target.icp_cli(&root).command();
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
    options.target.append_target_args(&mut command);
    run_or_print_command(&mut command, false)
}

fn run_transfer(options: &TransferOptions) -> Result<(), CyclesCommandError> {
    let root = resolve_current_canic_icp_root().map_err(CyclesCommandError::IcpRoot)?;
    let mut command = transfer_command_for_receiver(options, &root)?;
    run_or_print_command(&mut command, options.dry_run)
}

fn transfer_command_for_receiver(
    options: &TransferOptions,
    root: &Path,
) -> Result<std::process::Command, CyclesCommandError> {
    let receiver = fleet_recipient::resolve(&options.target, root, &options.receiver)
        .map_err(CyclesCommandError::from)?;
    let mut command = options.target.icp_cli(root).command();
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
    options.target.append_target_args(&mut command);
    Ok(command)
}

fn run_topup(options: &TopupOptions) -> Result<(), CyclesCommandError> {
    let root = resolve_current_canic_icp_root().map_err(CyclesCommandError::IcpRoot)?;
    let current = resolve_current_fleet(&root, &options.target.environment, &options.fleet)?;
    let target = if options.infrastructure_target == "coordinator" {
        ResolvedCanisterTarget {
            canister_id: current.topology.coordinator_canister_id,
            role: Some("coordinator".to_string()),
        }
    } else {
        let selected_root = options
            .infrastructure_target
            .parse::<Principal>()
            .map_err(|_| CyclesCommandError::Usage(topup_usage()))?;
        let selected_root = selected_root.to_text();
        if !current
            .topology
            .fleet_subnet_root_canister_ids
            .contains(&selected_root)
        {
            return Err(CyclesCommandError::UnknownTarget {
                fleet: options.fleet.clone(),
                target: selected_root,
            });
        }
        ResolvedCanisterTarget {
            canister_id: selected_root,
            role: Some("root".to_string()),
        }
    };
    let icp = options.target.icp_cli(&root);
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
                "fleet": options.fleet,
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

fn parse_cycle_amount(value: &str) -> Result<u128, String> {
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

fn target_label(role: Option<&str>, canister_id: &str) -> String {
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
        .arg(internal_environment_arg())
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
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
}

fn transfer_command() -> ClapCommand {
    ClapCommand::new(WalletCommandKind::Transfer.label())
        .bin_name("canic cycles transfer")
        .about("Transfer cycles to a principal or Canic Fleet target")
        .disable_help_flag(true)
        .arg(
            value_arg(AMOUNT_ARG)
                .value_name(AMOUNT_ARG)
                .required(true)
                .help("Cycles amount to transfer"),
        )
        .arg(
            value_arg(RECEIVER_ARG)
                .value_name("receiver-or-fleet-target")
                .required(true)
                .help("Raw principal, or Canic selector like <fleet>/<role-or-canister>"),
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
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
}

fn topup_command() -> ClapCommand {
    ClapCommand::new(WalletCommandKind::Topup.label())
        .bin_name("canic cycles topup")
        .about("Top up one current Fleet Coordinator or explicit Root")
        .disable_help_flag(true)
        .arg(value_arg(FLEET_ARG).value_name(FLEET_ARG).required(true))
        .arg(
            value_arg(INFRASTRUCTURE_TARGET_ARG)
                .value_name("coordinator-or-root-principal")
                .value_parser(clap::builder::ValueParser::new(parse_infrastructure_target))
                .required(true)
                .help("Use coordinator or one explicit current Fleet Subnet Root Principal"),
        )
        .arg(
            value_arg(AMOUNT_ARG)
                .value_name(AMOUNT_ARG)
                .required(true)
                .value_parser(clap::builder::ValueParser::new(parse_cycle_amount)),
        )
        .arg(flag_arg(JSON_ARG).long(JSON_ARG))
        .arg(flag_arg(DRY_RUN_ARG).long(DRY_RUN_ARG))
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
}

fn parse_infrastructure_target(value: &str) -> Result<String, String> {
    if value == "coordinator" || value.parse::<Principal>().is_ok() {
        Ok(value.to_string())
    } else {
        Err("expected coordinator or one explicit Root Principal".to_string())
    }
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
    fn parses_cycles_transfer_to_fleet_target() {
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

    // Avoid guessing between raw principals and Canic Fleet names.
    #[test]
    fn transfer_requires_receiver() {
        std::assert_matches!(
            TransferOptions::parse([OsString::from("4T")]),
            Err(CyclesCommandError::Usage(_))
        );
    }

    // Keep canister top-up available under the cycles family instead of a custom top-level command.
    #[test]
    fn parses_cycles_topup_options() {
        let options = TopupOptions::parse([
            OsString::from("demo"),
            OsString::from("coordinator"),
            OsString::from("4T"),
            OsString::from("--dry-run"),
            OsString::from("--json"),
        ])
        .expect("parse topup");

        assert_eq!(options.fleet, "demo");
        assert_eq!(options.infrastructure_target, "coordinator");
        assert_eq!(options.amount_cycles, 4_000_000_000_000);
        assert!(options.dry_run);
        assert!(options.json);
    }

    #[test]
    fn topup_rejects_generic_roles_and_accepts_explicit_root_principals() {
        assert!(
            TopupOptions::parse([
                OsString::from("demo"),
                OsString::from("rrkah-fqaaa-aaaaa-aaaaq-cai"),
                OsString::from("4T"),
            ])
            .is_ok()
        );
        assert!(
            TopupOptions::parse([
                OsString::from("demo"),
                OsString::from("app"),
                OsString::from("4T"),
            ])
            .is_err()
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
        let transfer_args = ["cycles", "transfer", "1", "aaaaa-aa"];
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
            Err(CyclesCommandError::InvalidRecipient)
        );
        let mut options = transfer_options("demo/app");
        options.target.environment = "fixture".to_string();
        std::assert_matches!(
            transfer_command_for_receiver(&options, root),
            Err(CyclesCommandError::CurrentFleet(canic_host::fleet_ensure::CurrentFleetInventoryError::NotConverged { environment, fleet }))
                if environment == "fixture" && fleet == "demo"
        );
    }

    fn transfer_options(receiver: &str) -> TransferOptions {
        TransferOptions::parse([OsString::from("1"), OsString::from(receiver)])
            .expect("parse transfer options")
    }
}
