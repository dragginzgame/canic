use crate::{
    cli::clap::{flag_arg, parse_matches, passthrough_subcommand, string_option, value_arg},
    cli::defaults::{default_icp, local_network},
    cli::globals::{internal_icp_arg, internal_network_arg},
    cli::help::print_help_or_version,
    cycles::CyclesCommandError,
    version_text,
};
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
use std::{ffi::OsString, path::Path};

const CYCLES_USAGE: &str = "\
Wrap ICP cycles commands with Canic deployment-target resolution

Usage: canic cycles <command> [OPTIONS]

Commands:
  balance   Display the selected identity cycles balance
  mint      Convert ICP to cycles
  transfer  Transfer cycles to a principal or Canic deployment target
  topup     Top up an installed deployment canister
  help      Print this message or the help of the given subcommand(s)

Examples:
  canic cycles balance
  canic cycles transfer 4T --to-deployment demo
  canic cycles transfer 4T --to-deployment demo --to-role app
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
/// TransferOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct TransferOptions {
    target: IcpTargetOptions,
    amount: String,
    receiver: Option<String>,
    to_deployment: Option<String>,
    to_role: Option<String>,
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
/// TopupTarget
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct TopupTarget {
    canister_id: String,
    role: Option<String>,
}

pub(super) fn run_cycles_command(
    command: &str,
    args: Vec<OsString>,
) -> Result<(), CyclesCommandError> {
    match command {
        "balance" => {
            if print_help_or_version(&args, balance_usage, version_text()) {
                return Ok(());
            }
            let options = BalanceOptions::parse(args)?;
            run_balance(&options)
        }
        "mint" => {
            if print_help_or_version(&args, mint_usage, version_text()) {
                return Ok(());
            }
            let options = MintOptions::parse(args)?;
            run_mint(&options)
        }
        "transfer" => {
            if print_help_or_version(&args, transfer_usage, version_text()) {
                return Ok(());
            }
            let options = TransferOptions::parse(args)?;
            run_transfer(&options)
        }
        "topup" => {
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
    ClapCommand::new("cycles")
        .bin_name("canic cycles")
        .subcommand(passthrough_subcommand(
            ClapCommand::new("balance").disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("mint").disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("transfer").disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("topup").disable_help_flag(true),
        ))
}

pub(super) fn cycles_usage() -> String {
    CYCLES_USAGE.to_string()
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
            json: matches.get_flag("json"),
            quiet: matches.get_flag("quiet"),
            subaccount: string_option(&matches, "subaccount"),
            of_principal: string_option(&matches, "of-principal"),
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
            icp_amount: string_option(&matches, "icp-amount"),
            cycles_amount: string_option(&matches, "cycles-amount"),
            from_subaccount: string_option(&matches, "from-subaccount"),
            to_subaccount: string_option(&matches, "to-subaccount"),
            json: matches.get_flag("json"),
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
            amount: string_option(&matches, "amount").expect("clap requires amount"),
            receiver: string_option(&matches, "receiver"),
            to_deployment: string_option(&matches, "to-deployment"),
            to_role: string_option(&matches, "to-role"),
            to_subaccount: string_option(&matches, "to-subaccount"),
            from_subaccount: string_option(&matches, "from-subaccount"),
            json: matches.get_flag("json"),
            quiet: matches.get_flag("quiet"),
            dry_run: matches.get_flag("dry-run"),
        };
        validate_recipient(
            options.receiver.as_ref(),
            options.to_deployment.as_ref(),
            options.to_role.as_ref(),
        )?;
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
        let amount = string_option(&matches, "amount").expect("clap requires amount");
        Ok(Self {
            target: IcpTargetOptions::parse(&matches),
            deployment: string_option(&matches, "deployment").expect("clap requires deployment"),
            canister_or_role: string_option(&matches, "canister-or-role")
                .expect("clap requires canister-or-role"),
            amount_cycles: parse_cycle_amount(&amount)?,
            json: matches.get_flag("json"),
            dry_run: matches.get_flag("dry-run"),
        })
    }
}

fn run_balance(options: &BalanceOptions) -> Result<(), CyclesCommandError> {
    let root = resolve_current_canic_icp_root()
        .map_err(|err| CyclesCommandError::InstallState(err.to_string()))?;
    let mut command = icp_command(&options.target, &root);
    command.args(["cycles", "balance"]);
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

fn run_mint(options: &MintOptions) -> Result<(), CyclesCommandError> {
    let root = resolve_current_canic_icp_root()
        .map_err(|err| CyclesCommandError::InstallState(err.to_string()))?;
    let mut command = icp_command(&options.target, &root);
    command.args(["cycles", "mint"]);
    append_optional_arg(&mut command, "--icp", options.icp_amount.as_deref());
    append_optional_arg(&mut command, "--cycles", options.cycles_amount.as_deref());
    append_optional_arg(
        &mut command,
        "--from-subaccount",
        options.from_subaccount.as_deref(),
    );
    append_optional_arg(
        &mut command,
        "--to-subaccount",
        options.to_subaccount.as_deref(),
    );
    append_flag(&mut command, "--json", options.json);
    append_target_args(&mut command, &options.target);
    run_or_print_command(&mut command, false)
}

fn run_transfer(options: &TransferOptions) -> Result<(), CyclesCommandError> {
    let root = resolve_current_canic_icp_root()
        .map_err(|err| CyclesCommandError::InstallState(err.to_string()))?;
    let receiver = transfer_receiver(
        &options.target,
        &root,
        options.receiver.as_deref(),
        options.to_deployment.as_deref(),
        options.to_role.as_deref(),
    )?;
    let mut command = icp_command(&options.target, &root);
    command.args(["cycles", "transfer"]);
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

fn run_topup(options: &TopupOptions) -> Result<(), CyclesCommandError> {
    let root = resolve_current_canic_icp_root()
        .map_err(|err| CyclesCommandError::InstallState(err.to_string()))?;
    let installed = resolve_deployment(&options.target, &root, &options.deployment)?;
    let target = resolve_topup_target(
        &options.deployment,
        &options.canister_or_role,
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

fn transfer_receiver(
    target: &IcpTargetOptions,
    root: &Path,
    receiver: Option<&str>,
    deployment: Option<&str>,
    role: Option<&str>,
) -> Result<String, CyclesCommandError> {
    if let Some(receiver) = receiver {
        return Ok(receiver.to_string());
    }
    let deployment = deployment.ok_or(CyclesCommandError::InvalidRecipient)?;
    let installed = resolve_deployment(target, root, deployment)?;
    if let Some(role) = role {
        return resolve_role_principal(deployment, role, &installed.registry.entries);
    }
    Ok(installed.state.root_canister_id)
}

const fn validate_recipient(
    receiver: Option<&String>,
    deployment: Option<&String>,
    role: Option<&String>,
) -> Result<(), CyclesCommandError> {
    if role.is_some() && deployment.is_none() {
        return Err(CyclesCommandError::RoleWithoutDeployment);
    }
    if receiver.is_some() == deployment.is_some() {
        return Err(CyclesCommandError::InvalidRecipient);
    }
    Ok(())
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

fn resolve_topup_target(
    deployment: &str,
    target: &str,
    registry: &[RegistryEntry],
) -> Result<TopupTarget, CyclesCommandError> {
    if let Some(entry) = registry.iter().find(|entry| entry.pid == target) {
        return Ok(topup_target_from_entry(entry));
    }
    let pid = resolve_role_principal(deployment, target, registry)?;
    let entry = registry
        .iter()
        .find(|entry| entry.pid == pid)
        .expect("role principal came from registry");
    Ok(topup_target_from_entry(entry))
}

fn topup_target_from_entry(entry: &RegistryEntry) -> TopupTarget {
    TopupTarget {
        canister_id: entry.pid.clone(),
        role: entry.role.clone(),
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

fn parse_cycle_amount(value: &str) -> Result<u128, CyclesCommandError> {
    let value = value.trim();
    let compact = value.replace('_', "");
    let digits_len = compact
        .chars()
        .take_while(char::is_ascii_digit)
        .map(char::len_utf8)
        .sum::<usize>();
    if digits_len == 0 {
        return Err(CyclesCommandError::Usage(topup_usage()));
    }
    let amount = compact
        .get(..digits_len)
        .and_then(|digits| digits.parse::<u128>().ok())
        .ok_or_else(|| CyclesCommandError::Usage(topup_usage()))?;
    let suffix = compact[digits_len..].trim().to_ascii_lowercase();
    let multiplier = match suffix.as_str() {
        "" | "cycle" | "cycles" => 1,
        "k" => 1_000,
        "m" => 1_000_000,
        "b" => 1_000_000_000,
        "t" | "tc" => 1_000_000_000_000,
        _ => return Err(CyclesCommandError::Usage(topup_usage())),
    };
    amount
        .checked_mul(multiplier)
        .filter(|cycles| *cycles > 0)
        .ok_or_else(|| CyclesCommandError::Usage(topup_usage()))
}

fn target_label(role: Option<&str>, canister_id: &str) -> String {
    role.map_or_else(
        || format!("canister {canister_id}"),
        |role| format!("role {role} ({canister_id})"),
    )
}

fn balance_command() -> ClapCommand {
    ClapCommand::new("balance")
        .bin_name("canic cycles balance")
        .about("Display the selected identity cycles balance")
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

fn mint_command() -> ClapCommand {
    ClapCommand::new("mint")
        .bin_name("canic cycles mint")
        .about("Convert ICP to cycles")
        .disable_help_flag(true)
        .arg(value_arg("icp-amount").long("icp").value_name("amount"))
        .arg(
            value_arg("cycles-amount")
                .long("cycles")
                .value_name("amount"),
        )
        .arg(
            value_arg("from-subaccount")
                .long("from-subaccount")
                .value_name("subaccount"),
        )
        .arg(
            value_arg("to-subaccount")
                .long("to-subaccount")
                .value_name("subaccount"),
        )
        .arg(flag_arg("json").long("json"))
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}

fn transfer_command() -> ClapCommand {
    ClapCommand::new("transfer")
        .bin_name("canic cycles transfer")
        .about("Transfer cycles to a principal or Canic deployment target")
        .disable_help_flag(true)
        .arg(value_arg("amount").value_name("amount").required(true))
        .arg(value_arg("receiver").value_name("receiver"))
        .arg(
            value_arg("to-deployment")
                .long("to-deployment")
                .value_name("deployment"),
        )
        .arg(value_arg("to-role").long("to-role").value_name("role"))
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

fn topup_command() -> ClapCommand {
    ClapCommand::new("topup")
        .bin_name("canic cycles topup")
        .about("Top up cycles for one installed deployment canister")
        .disable_help_flag(true)
        .arg(
            value_arg("deployment")
                .value_name("deployment")
                .required(true),
        )
        .arg(
            value_arg("canister-or-role")
                .value_name("canister-or-role")
                .required(true),
        )
        .arg(value_arg("amount").value_name("amount").required(true))
        .arg(flag_arg("json").long("json"))
        .arg(flag_arg("dry-run").long("dry-run"))
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}

fn balance_usage() -> String {
    let mut command = balance_command();
    command.render_help().to_string()
}

fn mint_usage() -> String {
    let mut command = mint_command();
    command.render_help().to_string()
}

fn transfer_usage() -> String {
    let mut command = transfer_command();
    command.render_help().to_string()
}

fn topup_usage() -> String {
    let mut command = topup_command();
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
    fn parses_cycles_transfer_to_deployment_role() {
        let options = TransferOptions::parse([
            OsString::from("4T"),
            OsString::from("--to-deployment"),
            OsString::from("demo"),
            OsString::from("--to-role"),
            OsString::from("app"),
            OsString::from("--dry-run"),
        ])
        .expect("parse transfer");

        assert_eq!(options.amount, "4T");
        assert_eq!(options.to_deployment.as_deref(), Some("demo"));
        assert_eq!(options.to_role.as_deref(), Some("app"));
        assert!(options.dry_run);
    }

    // Avoid guessing between raw principals/accounts and Canic deployment names.
    #[test]
    fn transfer_requires_one_recipient_source() {
        std::assert_matches!(
            TransferOptions::parse([OsString::from("4T")]),
            Err(CyclesCommandError::InvalidRecipient)
        );
        std::assert_matches!(
            TransferOptions::parse([
                OsString::from("4T"),
                OsString::from("aaaaa-aa"),
                OsString::from("--to-deployment"),
                OsString::from("demo")
            ]),
            Err(CyclesCommandError::InvalidRecipient)
        );
        std::assert_matches!(
            TransferOptions::parse([
                OsString::from("4T"),
                OsString::from("--to-role"),
                OsString::from("app")
            ]),
            Err(CyclesCommandError::RoleWithoutDeployment)
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
