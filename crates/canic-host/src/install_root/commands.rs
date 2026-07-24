use crate::icp::{self, IcpCommandError, LocalReplicaTarget};
use candid::IDLValue;
use canic_core::{cdk::types::Principal, dto::fleet_activation::CurrentRootInstallIdentity};
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

pub(super) fn root_init_args(
    identity: &CurrentRootInstallIdentity,
) -> Result<String, candid::Error> {
    let value = IDLValue::try_from_candid_type(identity)?;
    Ok(format!("({value})"))
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

pub(super) fn add_icp_environment_target(
    command: &mut Command,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
) {
    icp::add_target_args(command, Some(environment), local_replica);
}
