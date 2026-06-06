mod pending;

use super::wallet::{
    IcpTargetOptions, ResolvedCanisterTarget, cycles_icp_error, parse_cycle_amount,
    resolve_canister_target, resolve_deployment, target_label,
};
use crate::{
    cli::{
        clap::{
            flag_arg, parse_matches, render_usage, required_string, string_option, typed_option,
            value_arg,
        },
        globals::{internal_icp_arg, internal_network_arg},
    },
    cycles::CyclesCommandError,
};
use canic_core::cdk::utils::hash::{decode_hex, hex_bytes, sha256_bytes};
use canic_host::{format::cycles_tc, icp::IcpCli, icp_config::resolve_current_canic_icp_root};
use clap::{ArgGroup, Command as ClapCommand};
use pending::{
    PendingIcpRefillOperationInput, PendingOperationLogError,
    complete_pending_icp_refill_operation, reserve_pending_icp_refill_operation,
};
use std::{
    ffi::OsString,
    fmt::Write as _,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

const AMOUNT_ARG: &str = "amount";
const CANISTER_OR_ROLE_ARG: &str = "canister-or-role";
const CYCLES_AMOUNT_ARG: &str = "cycles-amount";
const DEPLOYMENT_ARG: &str = "deployment";
const DRY_RUN_ARG: &str = "dry-run";
const FABRICATE_ARG: &str = "fabricate";
const FROM_SUBACCOUNT_ARG: &str = "from-subaccount";
const ICP_E8S_ARG: &str = "icp-e8s";
const ICP_REFILL_METHOD: &str = "canic_icp_refill";
const JSON_ARG: &str = "json";
const MANAGEMENT_CANISTER_ID: &str = "aaaaa-aa";
const OPERATION_ID_ARG: &str = "operation-id";
const PROVISIONAL_TOP_UP_METHOD: &str = "provisional_top_up_canister";
const SOURCE_ARG: &str = "source";
const FABRICATE_MODE_MESSAGE: &str = "mode=fabricate (does not call canister refill endpoint)";
const CANISTER_MODE_ARGS: [&str; 4] = [
    SOURCE_ARG,
    ICP_E8S_ARG,
    FROM_SUBACCOUNT_ARG,
    OPERATION_ID_ARG,
];

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
/// OperationIdSource
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OperationIdSource {
    Provided,
    Generated,
    PendingLog,
}

impl OperationIdSource {
    const fn label(self) -> &'static str {
        match self {
            Self::Provided => "provided",
            Self::Generated => "generated",
            Self::PendingLog => "pending_log",
        }
    }
}

pub(super) fn run(args: Vec<OsString>) -> Result<(), CyclesCommandError> {
    let options = ConvertOptions::parse(args)?;
    run_options(&options)
}

pub(super) fn usage() -> String {
    render_usage(command)
}

impl ConvertOptions {
    fn parse<I>(args: I) -> Result<Self, CyclesCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| CyclesCommandError::Usage(usage()))?;
        Ok(Self {
            target: IcpTargetOptions::parse(&matches),
            deployment: required_string(&matches, DEPLOYMENT_ARG),
            canister_or_role: required_string(&matches, CANISTER_OR_ROLE_ARG),
            source_canister_or_role: string_option(&matches, SOURCE_ARG),
            amount_e8s: typed_option(&matches, ICP_E8S_ARG),
            cycles_amount: typed_option(&matches, CYCLES_AMOUNT_ARG),
            source_subaccount: typed_option(&matches, FROM_SUBACCOUNT_ARG),
            operation_id: typed_option(&matches, OPERATION_ID_ARG),
            json: matches.get_flag(JSON_ARG),
            dry_run: matches.get_flag(DRY_RUN_ARG),
            fabricate: matches.get_flag(FABRICATE_ARG),
        })
    }
}

fn run_options(options: &ConvertOptions) -> Result<(), CyclesCommandError> {
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
        return run_fabricate(options, &icp, &target);
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
    let now_nanos = current_unix_nanos();
    let pending_input =
        pending_operation_input(&root, options, &source, &target, amount_e8s, now_nanos);
    let (operation_id, operation_id_source, pending_operation_key) = resolve_operation_id(
        options.operation_id,
        &pending_input,
        options.dry_run,
        now_nanos,
    )?;
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
        write_canister_dry_run(
            options,
            &source,
            &target,
            operation_id,
            operation_id_source,
            &command,
        );
        return Ok(());
    }

    write_generated_operation_id_notice(options.json, operation_id, operation_id_source);

    let output = icp
        .canister_call_arg_output(
            &source.canister_id,
            ICP_REFILL_METHOD,
            &request_arg,
            json_output_arg(options.json),
        )
        .map_err(cycles_icp_error)?;
    mark_pending_operation_completed(&root, pending_operation_key.as_deref(), operation_id);
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

fn pending_operation_input<'a>(
    root: &'a Path,
    options: &'a ConvertOptions,
    source: &'a ResolvedCanisterTarget,
    target: &'a ResolvedCanisterTarget,
    amount_e8s: u64,
    now_nanos: u128,
) -> PendingIcpRefillOperationInput<'a> {
    PendingIcpRefillOperationInput {
        icp_root: root,
        network: &options.target.network,
        deployment: &options.deployment,
        source: source.role.as_deref(),
        source_canister_id: &source.canister_id,
        source_subaccount: options.source_subaccount,
        target: target.role.as_deref(),
        target_canister_id: &target.canister_id,
        amount_e8s,
        created_at_unix_nanos: now_nanos,
    }
}

fn mark_pending_operation_completed(
    root: &Path,
    operation_key: Option<&str>,
    operation_id: [u8; 32],
) {
    if let Some(operation_key) = operation_key {
        let _ = complete_pending_icp_refill_operation(
            root,
            operation_key,
            operation_id,
            current_unix_nanos(),
        );
    }
}

fn run_fabricate(
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
        write_fabricate_dry_run(options, target, amount_cycles, &command);
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

fn ensure_fabricate_local_network(network: &str) -> Result<(), CyclesCommandError> {
    if network == "local" {
        Ok(())
    } else {
        Err(CyclesCommandError::FabricationRequiresLocal {
            network: network.to_string(),
        })
    }
}

fn parse_icp_e8s_amount(value: &str) -> Result<u64, String> {
    let compact = value.trim().replace('_', "");
    compact
        .parse::<u64>()
        .ok()
        .filter(|amount| *amount > 0)
        .ok_or_else(|| format!("invalid ICP e8s amount {value}; use a positive u64 e8s value"))
}

fn parse_fixed_32_hex(field: &'static str, value: &str) -> Result<[u8; 32], String> {
    let trimmed = value.trim();
    let bytes = decode_hex(trimmed).map_err(|err| format!("invalid {field}: {err}"))?;
    <[u8; 32]>::try_from(bytes.as_slice()).map_err(|_| {
        format!(
            "invalid {field}: expected 32 bytes (64 hex chars), got {} bytes",
            bytes.len()
        )
    })
}

fn write_canister_dry_run(
    options: &ConvertOptions,
    source: &ResolvedCanisterTarget,
    target: &ResolvedCanisterTarget,
    operation_id: [u8; 32],
    operation_id_source: OperationIdSource,
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
        write_generated_operation_id_notice(options.json, operation_id, operation_id_source);
        println!("{command}");
    }
}

fn write_fabricate_dry_run(
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

fn generated_operation_id(
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

fn resolve_operation_id(
    provided: Option<[u8; 32]>,
    pending_input: &PendingIcpRefillOperationInput<'_>,
    dry_run: bool,
    now_nanos: u128,
) -> Result<([u8; 32], OperationIdSource, Option<String>), CyclesCommandError> {
    if let Some(operation_id) = provided {
        return Ok((operation_id, OperationIdSource::Provided, None));
    }
    let generated = generated_operation_id(
        pending_input.deployment,
        pending_input.source_canister_id,
        pending_input.target_canister_id,
        pending_input.amount_e8s,
        now_nanos,
    );
    if dry_run {
        return Ok((generated, OperationIdSource::Generated, None));
    }
    let reserved = reserve_pending_icp_refill_operation(pending_input, generated)
        .map_err(pending_operation_log_error)?;
    let source = if reserved.reused {
        OperationIdSource::PendingLog
    } else {
        OperationIdSource::Generated
    };
    Ok((reserved.operation_id, source, Some(reserved.operation_key)))
}

fn pending_operation_log_error(err: PendingOperationLogError) -> CyclesCommandError {
    CyclesCommandError::PendingOperationLog(err.to_string())
}

fn generated_operation_id_notice(
    operation_id: [u8; 32],
    source: OperationIdSource,
) -> Option<String> {
    matches!(
        source,
        OperationIdSource::Generated | OperationIdSource::PendingLog
    )
    .then(|| {
        format!(
            "operation_id={}\noperation_id_source={}",
            hex_bytes(operation_id),
            source.label()
        )
    })
}

fn write_generated_operation_id_notice(
    json: bool,
    operation_id: [u8; 32],
    source: OperationIdSource,
) {
    if json {
        return;
    }
    if let Some(notice) = generated_operation_id_notice(operation_id, source) {
        println!("{notice}");
    }
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

fn command() -> ClapCommand {
    ClapCommand::new("convert")
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
                .value_name(CANISTER_OR_ROLE_ARG)
                .requires(ICP_E8S_ARG)
                .conflicts_with(FABRICATE_ARG),
        )
        .arg(
            value_arg(ICP_E8S_ARG)
                .long(ICP_E8S_ARG)
                .value_name("e8s")
                .value_parser(clap::builder::ValueParser::new(parse_icp_e8s_amount))
                .requires(SOURCE_ARG)
                .conflicts_with(FABRICATE_ARG),
        )
        .arg(
            value_arg(CYCLES_AMOUNT_ARG)
                .long("cycles")
                .value_name(AMOUNT_ARG)
                .value_parser(clap::builder::ValueParser::new(parse_cycle_amount))
                .requires(FABRICATE_ARG)
                .conflicts_with_all(CANISTER_MODE_ARGS),
        )
        .arg(
            value_arg(FROM_SUBACCOUNT_ARG)
                .long(FROM_SUBACCOUNT_ARG)
                .value_name("hex64")
                .value_parser(clap::builder::ValueParser::new(|value: &str| {
                    parse_fixed_32_hex(FROM_SUBACCOUNT_ARG, value)
                }))
                .requires_all([SOURCE_ARG, ICP_E8S_ARG])
                .conflicts_with(FABRICATE_ARG),
        )
        .arg(
            value_arg(OPERATION_ID_ARG)
                .long(OPERATION_ID_ARG)
                .value_name("hex64")
                .value_parser(clap::builder::ValueParser::new(|value: &str| {
                    parse_fixed_32_hex(OPERATION_ID_ARG, value)
                }))
                .requires_all([SOURCE_ARG, ICP_E8S_ARG])
                .conflicts_with(FABRICATE_ARG),
        )
        .arg(
            flag_arg(FABRICATE_ARG)
                .long(FABRICATE_ARG)
                .requires(CYCLES_AMOUNT_ARG)
                .conflicts_with_all(CANISTER_MODE_ARGS),
        )
        .arg(flag_arg(JSON_ARG).long(JSON_ARG))
        .arg(flag_arg(DRY_RUN_ARG).long(DRY_RUN_ARG))
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
        .group(
            ArgGroup::new("convert-mode")
                .args([SOURCE_ARG, FABRICATE_ARG])
                .required(true),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;
    use std::path::Path;

    // Keep canister-side ICP conversion as a thin endpoint caller.
    #[test]
    fn parses_canister_options() {
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
    fn parses_fabricate_options() {
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
    fn rejects_mixed_fabricate_and_endpoint_args() {
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
    fn rejects_missing_convert_mode() {
        std::assert_matches!(
            ConvertOptions::parse([OsString::from("demo"), OsString::from("app")]),
            Err(CyclesCommandError::Usage(_))
        );
    }

    #[test]
    fn rejects_incomplete_canister_mode() {
        std::assert_matches!(
            ConvertOptions::parse([
                OsString::from("demo"),
                OsString::from("app"),
                OsString::from("--source"),
                OsString::from("root"),
            ]),
            Err(CyclesCommandError::Usage(_))
        );
    }

    #[test]
    fn rejects_cycles_without_fabricate_mode() {
        std::assert_matches!(
            ConvertOptions::parse([
                OsString::from("demo"),
                OsString::from("app"),
                OsString::from("--cycles"),
                OsString::from("4T"),
            ]),
            Err(CyclesCommandError::Usage(_))
        );
    }

    #[test]
    fn rejects_non_32_byte_hex() {
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
            Err(CyclesCommandError::Usage(_))
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
    fn generated_operation_id_binds_input() {
        let left = generated_operation_id("demo", "source", "target", 1, 10);
        let right = generated_operation_id("demo", "source", "target", 2, 10);
        let next_time = generated_operation_id("demo", "source", "target", 1, 11);

        assert_ne!(left, right);
        assert_ne!(left, next_time);
    }

    #[test]
    fn resolves_provided_operation_id_without_generation_notice() {
        let root = temp_dir("canic-cli-convert-provided-operation-id");
        let pending_input = pending_input(&root);
        let operation_id = [7; 32];
        let (resolved, source, pending_key) =
            resolve_operation_id(Some(operation_id), &pending_input, false, 10)
                .expect("resolve operation id");

        assert_eq!(resolved, operation_id);
        assert_eq!(source, OperationIdSource::Provided);
        assert_eq!(pending_key, None);
        assert_eq!(generated_operation_id_notice(resolved, source), None);
    }

    #[test]
    fn generated_operation_id_notice_is_operator_visible() {
        let root = temp_dir("canic-cli-convert-generated-operation-id");
        let pending_input = pending_input(&root);
        let (operation_id, source, pending_key) =
            resolve_operation_id(None, &pending_input, true, 10).expect("resolve operation id");
        let notice = generated_operation_id_notice(operation_id, source)
            .expect("generated operation id should be printed");

        assert_eq!(source, OperationIdSource::Generated);
        assert_eq!(pending_key, None);
        assert!(notice.contains(&format!("operation_id={}", hex_bytes(operation_id))));
        assert!(notice.contains("operation_id_source=generated"));
    }

    #[test]
    fn pending_log_reuse_notice_is_operator_visible() {
        let root = temp_dir("canic-cli-convert-pending-operation-id");
        let pending_input = pending_input(&root);
        let (first_id, first_source, first_key) =
            resolve_operation_id(None, &pending_input, false, 10).expect("resolve first id");
        let (second_id, second_source, second_key) =
            resolve_operation_id(None, &pending_input, false, 11).expect("resolve second id");
        let notice = generated_operation_id_notice(second_id, second_source)
            .expect("pending operation id should be printed");

        assert_eq!(first_source, OperationIdSource::Generated);
        assert_eq!(second_source, OperationIdSource::PendingLog);
        assert_eq!(first_id, second_id);
        assert_eq!(first_key, second_key);
        assert!(notice.contains(&format!("operation_id={}", hex_bytes(second_id))));
        assert!(notice.contains("operation_id_source=pending_log"));
    }

    fn pending_input(root: &Path) -> PendingIcpRefillOperationInput<'_> {
        PendingIcpRefillOperationInput {
            icp_root: root,
            network: "ic",
            deployment: "demo",
            source: Some("funding_hub"),
            source_canister_id: "source",
            source_subaccount: None,
            target: Some("app"),
            target_canister_id: "target",
            amount_e8s: 1,
            created_at_unix_nanos: 10,
        }
    }
}
