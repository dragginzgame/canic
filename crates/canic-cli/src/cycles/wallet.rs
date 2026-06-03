use crate::{
    cli::clap::{flag_arg, parse_matches, passthrough_subcommand, string_option, value_arg},
    cli::defaults::{default_icp, local_network},
    cli::globals::{internal_icp_arg, internal_network_arg},
    cli::help::print_help_or_version,
    cycles::CyclesCommandError,
    version_text,
};
use canic_core::cdk::utils::hash::{decode_hex, hex_bytes, sha256_bytes};
use canic_host::{
    format::cycles_tc,
    icp::{IcpCli, IcpCommandError, command_display, run_output_with_stderr},
    icp_config::resolve_current_canic_icp_root,
    installed_deployment::{
        InstalledDeploymentError, InstalledDeploymentRequest,
        resolve_installed_deployment_from_root,
    },
    registry::RegistryEntry,
};
use clap::Command as ClapCommand;
use std::{
    ffi::OsString,
    fmt::Write as _,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy)]
struct WalletCommand {
    name: &'static str,
}

const BALANCE_COMMAND: &str = "balance";
const CONVERT_COMMAND: &str = "convert";
const MINT_COMMAND: &str = "mint";
const TRANSFER_COMMAND: &str = "transfer";
const TOPUP_COMMAND: &str = "topup";

const WALLET_COMMANDS: &[WalletCommand] = &[
    WalletCommand {
        name: BALANCE_COMMAND,
    },
    WalletCommand {
        name: CONVERT_COMMAND,
    },
    WalletCommand { name: MINT_COMMAND },
    WalletCommand {
        name: TRANSFER_COMMAND,
    },
    WalletCommand {
        name: TOPUP_COMMAND,
    },
];

const AMOUNT_ARG: &str = "amount";
const CANISTER_OR_ROLE_ARG: &str = "canister-or-role";
const CYCLES_AMOUNT_ARG: &str = "cycles-amount";
const DEPLOYMENT_ARG: &str = "deployment";
const DRY_RUN_ARG: &str = "dry-run";
const FABRICATE_ARG: &str = "fabricate";
const FROM_SUBACCOUNT_ARG: &str = "from-subaccount";
const ICP_E8S_ARG: &str = "icp-e8s";
const ICP_AMOUNT_ARG: &str = "icp-amount";
const JSON_ARG: &str = "json";
const OPERATION_ID_ARG: &str = "operation-id";
const OF_PRINCIPAL_ARG: &str = "of-principal";
const QUIET_ARG: &str = "quiet";
const RECEIVER_ARG: &str = "receiver";
const SOURCE_ARG: &str = "source";
const SUBACCOUNT_ARG: &str = "subaccount";
const TO_SUBACCOUNT_ARG: &str = "to-subaccount";
const ICP_REFILL_METHOD: &str = "canic_icp_refill";
const MANAGEMENT_CANISTER_ID: &str = "aaaaa-aa";
const PROVISIONAL_TOP_UP_METHOD: &str = "provisional_top_up_canister";
const FABRICATE_MODE_MESSAGE: &str = "mode=fabricate (does not call canister refill endpoint)";

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
struct IcpTargetOptions {
    network: String,
    icp: String,
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
/// ConvertOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct ConvertOptions {
    target: IcpTargetOptions,
    deployment: String,
    canister_or_role: String,
    source_canister_or_role: Option<String>,
    amount_e8s: Option<u64>,
    cycles_amount: Option<u128>,
    source_subaccount: Option<[u8; 32]>,
    operation_id: Option<[u8; 32]>,
    json: bool,
    dry_run: bool,
    fabricate: bool,
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
struct ResolvedCanisterTarget {
    canister_id: String,
    role: Option<String>,
}

pub(super) fn run_cycles_command(
    command: &str,
    args: Vec<OsString>,
) -> Result<(), CyclesCommandError> {
    match command {
        BALANCE_COMMAND => {
            if print_help_or_version(&args, balance_usage, version_text()) {
                return Ok(());
            }
            let options = BalanceOptions::parse(args)?;
            run_balance(&options)
        }
        CONVERT_COMMAND => {
            if print_help_or_version(&args, convert_usage, version_text()) {
                return Ok(());
            }
            let options = ConvertOptions::parse(args)?;
            run_convert(&options)
        }
        MINT_COMMAND => {
            if print_help_or_version(&args, mint_usage, version_text()) {
                return Ok(());
            }
            let options = MintOptions::parse(args)?;
            run_mint(&options)
        }
        TRANSFER_COMMAND => {
            if print_help_or_version(&args, transfer_usage, version_text()) {
                return Ok(());
            }
            let options = TransferOptions::parse(args)?;
            run_transfer(&options)
        }
        TOPUP_COMMAND => {
            if print_help_or_version(&args, topup_usage, version_text()) {
                return Ok(());
            }
            let options = TopupOptions::parse(args)?;
            run_topup(&options)
        }
        _ => Err(CyclesCommandError::Usage(cycles_usage())),
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
    passthrough_subcommand(ClapCommand::new(spec.name).disable_help_flag(true))
}

impl IcpTargetOptions {
    fn parse(matches: &clap::ArgMatches) -> Self {
        Self {
            network: string_option(matches, "network").unwrap_or_else(local_network),
            icp: string_option(matches, "icp").unwrap_or_else(default_icp),
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

impl ConvertOptions {
    fn parse<I>(args: I) -> Result<Self, CyclesCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(convert_command(), args)
            .map_err(|_| CyclesCommandError::Usage(convert_usage()))?;
        let cycles_amount = string_option(&matches, CYCLES_AMOUNT_ARG)
            .map(|amount| parse_cycle_amount_for_usage(&amount, convert_usage))
            .transpose()?;
        let amount_e8s = string_option(&matches, ICP_E8S_ARG)
            .map(|amount| parse_icp_e8s_amount(&amount))
            .transpose()?;
        let source_subaccount = string_option(&matches, FROM_SUBACCOUNT_ARG)
            .map(|value| parse_fixed_32_hex(FROM_SUBACCOUNT_ARG, &value))
            .transpose()?;
        let operation_id = string_option(&matches, OPERATION_ID_ARG)
            .map(|value| parse_fixed_32_hex(OPERATION_ID_ARG, &value))
            .transpose()?;
        let options = Self {
            target: IcpTargetOptions::parse(&matches),
            deployment: string_option(&matches, DEPLOYMENT_ARG).expect("clap requires deployment"),
            canister_or_role: string_option(&matches, CANISTER_OR_ROLE_ARG)
                .expect("clap requires canister-or-role"),
            source_canister_or_role: string_option(&matches, SOURCE_ARG),
            amount_e8s,
            cycles_amount,
            source_subaccount,
            operation_id,
            json: matches.get_flag(JSON_ARG),
            dry_run: matches.get_flag(DRY_RUN_ARG),
            fabricate: matches.get_flag(FABRICATE_ARG),
        };
        validate_convert_options(&options)?;
        Ok(options)
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
            amount: string_option(&matches, AMOUNT_ARG).expect("clap requires amount"),
            receiver: string_option(&matches, RECEIVER_ARG).expect("clap requires receiver"),
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
        let amount = string_option(&matches, AMOUNT_ARG).expect("clap requires amount");
        Ok(Self {
            target: IcpTargetOptions::parse(&matches),
            deployment: string_option(&matches, DEPLOYMENT_ARG).expect("clap requires deployment"),
            canister_or_role: string_option(&matches, CANISTER_OR_ROLE_ARG)
                .expect("clap requires canister-or-role"),
            amount_cycles: parse_cycle_amount(&amount)?,
            json: matches.get_flag(JSON_ARG),
            dry_run: matches.get_flag(DRY_RUN_ARG),
        })
    }
}

fn run_balance(options: &BalanceOptions) -> Result<(), CyclesCommandError> {
    let root = resolve_current_canic_icp_root()
        .map_err(|err| CyclesCommandError::InstallState(err.to_string()))?;
    let mut command = icp_command(&options.target, &root);
    command.args(["cycles", BALANCE_COMMAND]);
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
    let root = resolve_current_canic_icp_root()
        .map_err(|err| CyclesCommandError::InstallState(err.to_string()))?;
    let mut command = icp_command(&options.target, &root);
    command.args(["cycles", MINT_COMMAND]);
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

fn run_convert(options: &ConvertOptions) -> Result<(), CyclesCommandError> {
    let root = resolve_current_canic_icp_root()
        .map_err(|err| CyclesCommandError::InstallState(err.to_string()))?;
    let installed = resolve_deployment(&options.target, &root, &options.deployment)?;
    let target = resolve_canister_target(
        &options.deployment,
        &options.canister_or_role,
        &installed.state.root_canister_id,
        &installed.registry.entries,
    )?;
    let icp = IcpCli::new(
        &options.target.icp,
        None,
        Some(options.target.network.clone()),
    )
    .with_cwd(&root);

    if options.fabricate {
        return run_convert_fabricate(options, &icp, &target);
    }

    let source_selector = options
        .source_canister_or_role
        .as_deref()
        .expect("convert validation requires source");
    let source = resolve_canister_target(
        &options.deployment,
        source_selector,
        &installed.state.root_canister_id,
        &installed.registry.entries,
    )?;
    let amount_e8s = options
        .amount_e8s
        .expect("convert validation requires ICP e8s amount");
    let operation_id = options.operation_id.unwrap_or_else(|| {
        generated_convert_operation_id(
            &options.deployment,
            &source.canister_id,
            &target.canister_id,
            amount_e8s,
            current_unix_nanos(),
        )
    });
    let request_arg = icp_refill_request_arg(
        operation_id,
        &source.canister_id,
        options.source_subaccount,
        &target.canister_id,
        amount_e8s,
        options.dry_run,
    );
    let command = icp.canister_call_arg_output_display(
        &source.canister_id,
        ICP_REFILL_METHOD,
        &request_arg,
        json_output_arg(options.json),
    );

    if options.dry_run {
        write_convert_canister_dry_run(options, &source, &target, operation_id, &command);
        return Ok(());
    }

    let output = icp
        .canister_call_arg_output(
            &source.canister_id,
            ICP_REFILL_METHOD,
            &request_arg,
            json_output_arg(options.json),
        )
        .map_err(cycles_icp_error)?;
    if options.json {
        println!(
            "{}",
            serde_json::json!({
                "mode": "canister",
                "deployment": options.deployment,
                "source": source.role.as_deref(),
                "source_canister_id": source.canister_id,
                "source_subaccount": options.source_subaccount.map(hex_bytes),
                "target": target.role.as_deref(),
                "target_canister_id": target.canister_id,
                "amount_e8s": amount_e8s,
                "operation_id": hex_bytes(operation_id),
                "dry_run": false,
                "command": command,
                "icp_output": output,
            })
        );
    } else if !output.is_empty() {
        println!("{output}");
    }
    Ok(())
}

fn run_transfer(options: &TransferOptions) -> Result<(), CyclesCommandError> {
    let root = resolve_current_canic_icp_root()
        .map_err(|err| CyclesCommandError::InstallState(err.to_string()))?;
    let receiver = transfer_receiver(&options.target, &root, &options.receiver)?;
    let mut command = icp_command(&options.target, &root);
    command.args(["cycles", TRANSFER_COMMAND]);
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
    let root = resolve_current_canic_icp_root()
        .map_err(|err| CyclesCommandError::InstallState(err.to_string()))?;
    let installed = resolve_deployment(&options.target, &root, &options.deployment)?;
    let target = resolve_canister_target(
        &options.deployment,
        &options.canister_or_role,
        &installed.state.root_canister_id,
        &installed.registry.entries,
    )?;
    let icp = IcpCli::new(
        &options.target.icp,
        None,
        Some(options.target.network.clone()),
    )
    .with_cwd(&root);
    if options.dry_run {
        println!(
            "{}",
            icp.canister_top_up_display(&target.canister_id, options.amount_cycles)
        );
        return Ok(());
    }

    let output = icp
        .canister_top_up_output(&target.canister_id, options.amount_cycles)
        .map_err(cycles_icp_error)?;
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

fn run_convert_fabricate(
    options: &ConvertOptions,
    icp: &IcpCli,
    target: &ResolvedCanisterTarget,
) -> Result<(), CyclesCommandError> {
    ensure_fabricate_local_network(&options.target.network)?;
    let amount_cycles = options
        .cycles_amount
        .expect("convert validation requires cycles amount for fabrication");
    let request_arg = provisional_top_up_arg(&target.canister_id, amount_cycles);
    let command = icp.canister_call_arg_output_display(
        MANAGEMENT_CANISTER_ID,
        PROVISIONAL_TOP_UP_METHOD,
        &request_arg,
        json_output_arg(options.json),
    );

    if options.dry_run {
        write_convert_fabricate_dry_run(options, target, amount_cycles, &command);
        return Ok(());
    }

    let output = icp
        .canister_call_arg_output(
            MANAGEMENT_CANISTER_ID,
            PROVISIONAL_TOP_UP_METHOD,
            &request_arg,
            json_output_arg(options.json),
        )
        .map_err(cycles_icp_error)?;
    if options.json {
        println!(
            "{}",
            serde_json::json!({
                "mode": "fabricate",
                "message": FABRICATE_MODE_MESSAGE,
                "deployment": options.deployment,
                "target": target.role.as_deref(),
                "target_canister_id": target.canister_id,
                "amount_cycles": amount_cycles.to_string(),
                "amount_display": cycles_tc(amount_cycles),
                "dry_run": false,
                "command": command,
                "icp_output": output,
            })
        );
    } else {
        println!(
            "Fabricated {} for {}.",
            cycles_tc(amount_cycles),
            target_label(target.role.as_deref(), &target.canister_id)
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

fn validate_convert_options(options: &ConvertOptions) -> Result<(), CyclesCommandError> {
    if options.fabricate {
        if options.cycles_amount.is_some()
            && options.source_canister_or_role.is_none()
            && options.amount_e8s.is_none()
            && options.source_subaccount.is_none()
            && options.operation_id.is_none()
        {
            return Ok(());
        }
        return Err(CyclesCommandError::Usage(convert_usage()));
    }

    if options.source_canister_or_role.is_some()
        && options.amount_e8s.is_some()
        && options.cycles_amount.is_none()
    {
        return Ok(());
    }

    Err(CyclesCommandError::Usage(convert_usage()))
}

fn ensure_fabricate_local_network(network: &str) -> Result<(), CyclesCommandError> {
    if network == "local" {
        Ok(())
    } else {
        Err(CyclesCommandError::FabricationRequiresLocal {
            network: network.to_string(),
        })
    }
}

fn parse_icp_e8s_amount(value: &str) -> Result<u64, CyclesCommandError> {
    let compact = value.trim().replace('_', "");
    compact
        .parse::<u64>()
        .ok()
        .filter(|amount| *amount > 0)
        .ok_or_else(|| CyclesCommandError::InvalidIcpE8sAmount {
            value: value.to_string(),
        })
}

fn parse_fixed_32_hex(field: &'static str, value: &str) -> Result<[u8; 32], CyclesCommandError> {
    let trimmed = value.trim();
    let bytes = decode_hex(trimmed).map_err(|err| CyclesCommandError::InvalidHexField {
        field,
        reason: err.to_string(),
    })?;
    <[u8; 32]>::try_from(bytes.as_slice()).map_err(|_| CyclesCommandError::InvalidHexField {
        field,
        reason: format!(
            "expected 32 bytes (64 hex chars), got {} bytes",
            bytes.len()
        ),
    })
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

fn resolve_deployment(
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
    .map_err(cycles_installed_deployment_error)
}

fn resolve_role_principal(
    deployment: &str,
    role: &str,
    registry: &[RegistryEntry],
) -> Result<String, CyclesCommandError> {
    let matches = registry
        .iter()
        .filter(|entry| entry.role.as_deref() == Some(role))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [entry] => Ok(entry.pid.clone()),
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

fn resolve_canister_target(
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
    let pid = resolve_role_principal(deployment, target, registry)?;
    let entry = registry
        .iter()
        .find(|entry| entry.pid == pid)
        .expect("role principal came from registry");
    Ok(resolved_target_from_entry(entry))
}

fn resolved_target_from_entry(entry: &RegistryEntry) -> ResolvedCanisterTarget {
    ResolvedCanisterTarget {
        canister_id: entry.pid.clone(),
        role: entry.role.clone(),
    }
}

fn write_convert_canister_dry_run(
    options: &ConvertOptions,
    source: &ResolvedCanisterTarget,
    target: &ResolvedCanisterTarget,
    operation_id: [u8; 32],
    command: &str,
) {
    if options.json {
        println!(
            "{}",
            serde_json::json!({
                "mode": "canister",
                "deployment": options.deployment,
                "source": source.role.as_deref(),
                "source_canister_id": source.canister_id,
                "source_subaccount": options.source_subaccount.map(hex_bytes),
                "target": target.role.as_deref(),
                "target_canister_id": target.canister_id,
                "amount_e8s": options.amount_e8s.expect("convert validation requires ICP e8s amount"),
                "operation_id": hex_bytes(operation_id),
                "dry_run": true,
                "command": command,
            })
        );
    } else {
        println!("mode=canister");
        println!("{command}");
    }
}

fn write_convert_fabricate_dry_run(
    options: &ConvertOptions,
    target: &ResolvedCanisterTarget,
    amount_cycles: u128,
    command: &str,
) {
    if options.json {
        println!(
            "{}",
            serde_json::json!({
                "mode": "fabricate",
                "message": FABRICATE_MODE_MESSAGE,
                "deployment": options.deployment,
                "target": target.role.as_deref(),
                "target_canister_id": target.canister_id,
                "amount_cycles": amount_cycles.to_string(),
                "amount_display": cycles_tc(amount_cycles),
                "dry_run": true,
                "command": command,
            })
        );
    } else {
        println!("{FABRICATE_MODE_MESSAGE}");
        println!("{command}");
    }
}

fn icp_command(target: &IcpTargetOptions, root: &Path) -> std::process::Command {
    let icp = IcpCli::new(&target.icp, None, Some(target.network.clone())).with_cwd(root);
    icp.command()
}

fn append_target_args(command: &mut std::process::Command, target: &IcpTargetOptions) {
    canic_host::icp::add_target_args(command, None, Some(&target.network));
}

fn run_or_print_command(
    command: &mut std::process::Command,
    dry_run: bool,
) -> Result<(), CyclesCommandError> {
    if dry_run {
        println!("{}", command_display(command));
        return Ok(());
    }
    let output = run_output_with_stderr(command).map_err(cycles_icp_error)?;
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

fn parse_cycle_amount(value: &str) -> Result<u128, CyclesCommandError> {
    parse_cycle_amount_for_usage(value, topup_usage)
}

fn parse_cycle_amount_for_usage(
    value: &str,
    usage: fn() -> String,
) -> Result<u128, CyclesCommandError> {
    let value = value.trim();
    let compact = value.replace('_', "");
    let digits_len = compact
        .chars()
        .take_while(char::is_ascii_digit)
        .map(char::len_utf8)
        .sum::<usize>();
    if digits_len == 0 {
        return Err(CyclesCommandError::Usage(usage()));
    }
    let amount = compact
        .get(..digits_len)
        .and_then(|digits| digits.parse::<u128>().ok())
        .ok_or_else(|| CyclesCommandError::Usage(usage()))?;
    let suffix = compact[digits_len..].trim().to_ascii_lowercase();
    let multiplier = match suffix.as_str() {
        "" | "cycle" | "cycles" => 1,
        "k" => 1_000,
        "m" => 1_000_000,
        "b" => 1_000_000_000,
        "t" | "tc" => 1_000_000_000_000,
        _ => return Err(CyclesCommandError::Usage(usage())),
    };
    amount
        .checked_mul(multiplier)
        .filter(|cycles| *cycles > 0)
        .ok_or_else(|| CyclesCommandError::Usage(usage()))
}

fn target_label(role: Option<&str>, canister_id: &str) -> String {
    role.map_or_else(
        || format!("canister {canister_id}"),
        |role| format!("role {role} ({canister_id})"),
    )
}

const fn json_output_arg(json: bool) -> Option<&'static str> {
    if json { Some("json") } else { None }
}

fn icp_refill_request_arg(
    operation_id: [u8; 32],
    source_canister: &str,
    source_subaccount: Option<[u8; 32]>,
    target_canister: &str,
    amount_e8s: u64,
    dry_run: bool,
) -> String {
    format!(
        "(record {{ operation_id = {}; source_canister = principal \"{}\"; source_subaccount = {}; \
         target_canister = principal \"{}\"; amount_e8s = {} : nat64; dry_run = {}; \
         mode = variant {{ Canister }} }})",
        idl_blob(&operation_id),
        source_canister,
        optional_idl_blob(source_subaccount),
        target_canister,
        amount_e8s,
        dry_run,
    )
}

fn provisional_top_up_arg(canister_id: &str, amount_cycles: u128) -> String {
    format!(
        "(record {{ canister_id = principal \"{canister_id}\"; amount = {amount_cycles} : nat }})"
    )
}

fn optional_idl_blob(bytes: Option<[u8; 32]>) -> String {
    bytes.map_or_else(
        || "null".to_string(),
        |bytes| format!("opt {}", idl_blob(&bytes)),
    )
}

fn idl_blob(bytes: &[u8]) -> String {
    let mut encoded = String::from("blob \"");
    for byte in bytes {
        let _ = write!(encoded, "\\{byte:02X}");
    }
    encoded.push('"');
    encoded
}

fn generated_convert_operation_id(
    deployment: &str,
    source_canister: &str,
    target_canister: &str,
    amount_e8s: u64,
    now_nanos: u128,
) -> [u8; 32] {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"canic:cycles-convert:icp-refill:v1");
    extend_operation_id_part(&mut bytes, deployment.as_bytes());
    extend_operation_id_part(&mut bytes, source_canister.as_bytes());
    extend_operation_id_part(&mut bytes, target_canister.as_bytes());
    extend_operation_id_part(&mut bytes, &amount_e8s.to_be_bytes());
    extend_operation_id_part(&mut bytes, &now_nanos.to_be_bytes());
    let digest = sha256_bytes(&bytes);
    let mut operation_id = [0; 32];
    operation_id.copy_from_slice(&digest);
    operation_id
}

fn extend_operation_id_part(bytes: &mut Vec<u8>, part: &[u8]) {
    bytes.extend_from_slice(&(part.len() as u64).to_be_bytes());
    bytes.extend_from_slice(part);
}

fn current_unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos())
}

fn balance_command() -> ClapCommand {
    ClapCommand::new(BALANCE_COMMAND)
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

fn convert_command() -> ClapCommand {
    ClapCommand::new(CONVERT_COMMAND)
        .bin_name("canic cycles convert")
        .about("Convert ICP held by a Canic canister to cycles")
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
            value_arg(SOURCE_ARG)
                .long(SOURCE_ARG)
                .value_name(CANISTER_OR_ROLE_ARG),
        )
        .arg(value_arg(ICP_E8S_ARG).long(ICP_E8S_ARG).value_name("e8s"))
        .arg(
            value_arg(CYCLES_AMOUNT_ARG)
                .long("cycles")
                .value_name(AMOUNT_ARG),
        )
        .arg(
            value_arg(FROM_SUBACCOUNT_ARG)
                .long(FROM_SUBACCOUNT_ARG)
                .value_name("hex64"),
        )
        .arg(
            value_arg(OPERATION_ID_ARG)
                .long(OPERATION_ID_ARG)
                .value_name("hex64"),
        )
        .arg(flag_arg(FABRICATE_ARG).long(FABRICATE_ARG))
        .arg(flag_arg(JSON_ARG).long(JSON_ARG))
        .arg(flag_arg(DRY_RUN_ARG).long(DRY_RUN_ARG))
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}

fn mint_command() -> ClapCommand {
    ClapCommand::new(MINT_COMMAND)
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
    ClapCommand::new(TRANSFER_COMMAND)
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
    ClapCommand::new(TOPUP_COMMAND)
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
        .arg(value_arg(AMOUNT_ARG).value_name(AMOUNT_ARG).required(true))
        .arg(flag_arg(JSON_ARG).long(JSON_ARG))
        .arg(flag_arg(DRY_RUN_ARG).long(DRY_RUN_ARG))
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}

fn balance_usage() -> String {
    render_usage(balance_command)
}

fn convert_usage() -> String {
    render_usage(convert_command)
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

fn render_usage(command: fn() -> ClapCommand) -> String {
    let mut command = command();
    command.render_help().to_string()
}

fn cycles_installed_deployment_error(error: InstalledDeploymentError) -> CyclesCommandError {
    match error {
        InstalledDeploymentError::NoInstalledDeployment {
            network,
            deployment,
        } => CyclesCommandError::NoInstalledDeployment {
            network,
            deployment,
        },
        InstalledDeploymentError::InstallState(error) => CyclesCommandError::InstallState(error),
        InstalledDeploymentError::ReplicaQuery(error) => CyclesCommandError::ReplicaQuery(error),
        InstalledDeploymentError::IcpFailed { command, stderr } => {
            CyclesCommandError::IcpFailed { command, stderr }
        }
        InstalledDeploymentError::LostLocalDeployment { root, .. } => {
            CyclesCommandError::ReplicaQuery(format!("root canister {root} is not present"))
        }
        InstalledDeploymentError::Registry(error) => CyclesCommandError::Registry(error),
        InstalledDeploymentError::Io(error) => CyclesCommandError::Io(error),
    }
}

fn cycles_icp_error(error: IcpCommandError) -> CyclesCommandError {
    match error {
        IcpCommandError::Io(error) => CyclesCommandError::Io(error),
        IcpCommandError::Failed { command, stderr } => {
            CyclesCommandError::IcpFailed { command, stderr }
        }
        IcpCommandError::Json {
            command, output, ..
        } => CyclesCommandError::IcpFailed {
            command,
            stderr: output,
        },
        IcpCommandError::SnapshotIdUnavailable { output } => CyclesCommandError::IcpFailed {
            command: "icp canister snapshot create".to_string(),
            stderr: output,
        },
    }
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

    // Keep canister-side ICP conversion as a thin endpoint caller.
    #[test]
    fn parses_cycles_convert_canister_options() {
        let operation_id = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        let subaccount = "202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f";
        let options = ConvertOptions::parse([
            OsString::from("demo"),
            OsString::from("root"),
            OsString::from("--source"),
            OsString::from("funding_hub"),
            OsString::from("--icp-e8s"),
            OsString::from("100_000_000"),
            OsString::from("--from-subaccount"),
            OsString::from(subaccount),
            OsString::from("--operation-id"),
            OsString::from(operation_id),
            OsString::from("--dry-run"),
            OsString::from("--json"),
        ])
        .expect("parse convert");

        assert_eq!(options.deployment, "demo");
        assert_eq!(options.canister_or_role, "root");
        assert_eq!(
            options.source_canister_or_role.as_deref(),
            Some("funding_hub")
        );
        assert_eq!(options.amount_e8s, Some(100_000_000));
        assert_eq!(
            options.operation_id.map(hex_bytes).as_deref(),
            Some(operation_id)
        );
        assert_eq!(
            options.source_subaccount.map(hex_bytes).as_deref(),
            Some(subaccount)
        );
        assert!(options.dry_run);
        assert!(options.json);
        assert!(!options.fabricate);
    }

    // Keep local fabrication separate from the canister-side refill endpoint.
    #[test]
    fn parses_cycles_convert_fabricate_options() {
        let options = ConvertOptions::parse([
            OsString::from("demo"),
            OsString::from("app"),
            OsString::from("--fabricate"),
            OsString::from("--cycles"),
            OsString::from("4T"),
            OsString::from("--dry-run"),
        ])
        .expect("parse fabricate");

        assert_eq!(options.deployment, "demo");
        assert_eq!(options.canister_or_role, "app");
        assert_eq!(options.cycles_amount, Some(4_000_000_000_000));
        assert!(options.fabricate);
        assert!(options.dry_run);
    }

    #[test]
    fn convert_rejects_mixed_fabricate_and_endpoint_args() {
        std::assert_matches!(
            ConvertOptions::parse([
                OsString::from("demo"),
                OsString::from("app"),
                OsString::from("--fabricate"),
                OsString::from("--cycles"),
                OsString::from("4T"),
                OsString::from("--source"),
                OsString::from("root"),
            ]),
            Err(CyclesCommandError::Usage(_))
        );
    }

    #[test]
    fn convert_rejects_non_32_byte_hex() {
        std::assert_matches!(
            ConvertOptions::parse([
                OsString::from("demo"),
                OsString::from("app"),
                OsString::from("--source"),
                OsString::from("root"),
                OsString::from("--icp-e8s"),
                OsString::from("1"),
                OsString::from("--operation-id"),
                OsString::from("abcd"),
            ]),
            Err(CyclesCommandError::InvalidHexField {
                field: OPERATION_ID_ARG,
                ..
            })
        );
    }

    #[test]
    fn fabricate_requires_local_network() {
        std::assert_matches!(
            ensure_fabricate_local_network("ic"),
            Err(CyclesCommandError::FabricationRequiresLocal { .. })
        );
        assert!(ensure_fabricate_local_network("local").is_ok());
    }

    #[test]
    fn renders_icp_refill_request_arg() {
        let arg = icp_refill_request_arg(
            [1; 32],
            "source-principal",
            Some([2; 32]),
            "target-principal",
            100_000_000,
            true,
        );

        assert!(arg.contains(r#"operation_id = blob "\01\01\01"#));
        assert!(arg.contains(r#"source_canister = principal "source-principal""#));
        assert!(arg.contains(r#"source_subaccount = opt blob "\02\02\02"#));
        assert!(arg.contains(r#"target_canister = principal "target-principal""#));
        assert!(arg.contains("amount_e8s = 100000000 : nat64"));
        assert!(arg.contains("dry_run = true"));
        assert!(arg.contains("mode = variant { Canister }"));
    }

    #[test]
    fn renders_fabrication_arg_and_message() {
        assert_eq!(
            provisional_top_up_arg("target-principal", 4_000_000_000_000),
            r#"(record { canister_id = principal "target-principal"; amount = 4000000000000 : nat })"#
        );
        assert_eq!(
            FABRICATE_MODE_MESSAGE,
            "mode=fabricate (does not call canister refill endpoint)"
        );
    }

    #[test]
    fn generated_convert_operation_id_binds_input() {
        let left = generated_convert_operation_id("demo", "source", "target", 1, 10);
        let right = generated_convert_operation_id("demo", "source", "target", 2, 10);
        let next_time = generated_convert_operation_id("demo", "source", "target", 1, 11);

        assert_ne!(left, right);
        assert_ne!(left, next_time);
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
            kind: None,
            parent_pid: None,
            module_hash: None,
        }
    }
}
