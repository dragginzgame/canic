use crate::icp::{self, IcpCommandError, LocalReplicaTarget};
use canic_core::cdk::{types::Principal, utils::hash::wasm_hash};
use serde_json::Value as JsonValue;
use std::{fs, path::Path, process::Command};

pub(super) fn parse_created_canister_id(output: &str) -> Option<String> {
    if let Ok(value) = serde_json::from_str::<JsonValue>(output) {
        return parse_canister_id_json(&value);
    }

    output
        .lines()
        .map(str::trim)
        .find(|line| Principal::from_text(*line).is_ok())
        .map(ToString::to_string)
}

pub(super) fn parse_canister_id_json(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::String(text) if Principal::from_text(text).is_ok() => Some(text.clone()),
        JsonValue::Array(values) => values.iter().find_map(parse_canister_id_json),
        JsonValue::Object(object) => ["canister_id", "id", "principal"]
            .iter()
            .filter_map(|key| object.get(*key))
            .find_map(parse_canister_id_json),
        _ => None,
    }
}

pub(super) fn add_create_root_target(
    command: &mut Command,
    root_canister: &str,
    local_replica: Option<&LocalReplicaTarget>,
) {
    if local_replica.is_some() {
        command.args(["create", "--detached", "--json"]);
    } else {
        command.args(["create", root_canister, "--json"]);
    }
}

pub(super) fn root_init_args(root_wasm: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let wasm = fs::read(root_wasm)?;
    Ok(format!(
        "(variant {{ PrimeWithModuleHash = {} }})",
        idl_blob(&wasm_hash(&wasm))
    ))
}

fn idl_blob(bytes: &[u8]) -> String {
    let mut encoded = String::from("blob \"");
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(encoded, "\\{byte:02X}");
    }
    encoded.push('"');
    encoded
}

pub(super) fn run_command(command: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    icp::run_status(command).map_err(Into::into)
}

pub(super) fn run_command_stdout(command: &mut Command) -> Result<String, IcpCommandError> {
    icp::run_output(command)
}

pub(super) fn icp_canister_command(icp_root: &Path) -> Command {
    let mut command = icp::default_command_in(icp_root);
    command.arg("canister");
    command
}

pub(super) fn add_icp_network_target(
    command: &mut Command,
    network: &str,
    local_replica: Option<&LocalReplicaTarget>,
) {
    icp::add_target_args(command, Some(network), local_replica);
}
