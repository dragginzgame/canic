use crate::{
    durable_io::write_bytes,
    icp::{self, LocalReplicaTarget},
};
use candid::CandidType;
use canic_core::cdk::types::Principal;
use serde_json::Value as JsonValue;
use std::{path::Path, process::Command};

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

fn parse_canister_id_json(value: &JsonValue) -> Option<String> {
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

pub(super) fn write_candid_args<T: CandidType>(
    path: &Path,
    args: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    write_bytes(path, &candid::encode_one(args)?)?;
    Ok(())
}

pub(super) fn run_command(command: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    icp::run_status(command).map_err(Into::into)
}

pub(super) fn icp_canister_command(icp_root: &Path) -> Command {
    let mut command = icp::default_command_in(icp_root);
    command.arg("canister");
    command
}

pub(super) fn add_icp_environment_target(
    command: &mut Command,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
) {
    icp::add_target_args(command, Some(environment), local_replica);
}

pub(super) fn icp_e8s_text(e8s: u64) -> String {
    const E8S_PER_ICP: u64 = 100_000_000;
    let whole = e8s / E8S_PER_ICP;
    let remainder = e8s % E8S_PER_ICP;
    if remainder == 0 {
        return whole.to_string();
    }
    let fractional = format!("{remainder:08}");
    format!("{whole}.{}", fractional.trim_end_matches('0'))
}
