use super::ReplicaQueryError;
use candid::{CandidType, Decode, Principal};
use canic_core::dto::{error::Error as CanicDtoError, state::BootstrapStatusResponse};
use serde::Deserialize;

pub(super) fn decode_bootstrap_status_response(
    bytes: &[u8],
) -> Result<BootstrapStatusResponse, ReplicaQueryError> {
    Decode!(bytes, BootstrapStatusResponse).map_err(|err| ReplicaQueryError::Query(err.to_string()))
}

pub(super) fn decode_cycle_balance_response(bytes: &[u8]) -> Result<u128, ReplicaQueryError> {
    let result = Decode!(bytes, Result<u128, CanicDtoError>)
        .map_err(|err| ReplicaQueryError::Query(err.to_string()))?;
    result.map_err(|err| ReplicaQueryError::Query(err.to_string()))
}

pub(super) fn decode_subnet_registry_response(
    bytes: &[u8],
) -> Result<SubnetRegistryResponseWire, ReplicaQueryError> {
    let result = Decode!(&bytes, Result<SubnetRegistryResponseWire, CanicDtoError>)
        .map_err(|err| ReplicaQueryError::Query(err.to_string()))?;
    result.map_err(|err| ReplicaQueryError::Query(err.to_string()))
}

///
/// SubnetRegistryResponseWire
///

#[derive(CandidType, Deserialize)]
pub(super) struct SubnetRegistryResponseWire(pub(super) Vec<SubnetRegistryEntryWire>);

impl SubnetRegistryResponseWire {
    // Return registry roles in the same order the root reported them.
    pub(super) fn roles(&self) -> Vec<String> {
        self.0.iter().map(|entry| entry.role.clone()).collect()
    }

    // Convert direct Candid query output into the command JSON shape the discovery parser accepts.
    pub(super) fn to_cli_json(&self) -> serde_json::Value {
        serde_json::json!({
            "Ok": self.0.iter().map(SubnetRegistryEntryWire::to_cli_json).collect::<Vec<_>>()
        })
    }
}

///
/// SubnetRegistryEntryWire
///

#[derive(CandidType, Deserialize)]
pub(super) struct SubnetRegistryEntryWire {
    pub(super) pid: Principal,
    pub(super) role: String,
    pub(super) record: CanisterInfoWire,
}

impl SubnetRegistryEntryWire {
    // Convert one registry entry into the command JSON shape used by existing list rendering.
    fn to_cli_json(&self) -> serde_json::Value {
        serde_json::json!({
            "pid": self.pid.to_text(),
            "role": self.role,
            "record": self.record.to_cli_json(),
        })
    }
}

///
/// CanisterInfoWire
///

#[derive(CandidType, Deserialize)]
pub(super) struct CanisterInfoWire {
    pub(super) pid: Principal,
    pub(super) role: String,
    pub(super) parent_pid: Option<Principal>,
    pub(super) module_hash: Option<Vec<u8>>,
    pub(super) created_at: u64,
}

impl CanisterInfoWire {
    // Convert one canister info record into a CLI-like JSON object.
    fn to_cli_json(&self) -> serde_json::Value {
        serde_json::json!({
            "pid": self.pid.to_text(),
            "role": self.role,
            "parent_pid": self.parent_pid.as_ref().map(Principal::to_text),
            "module_hash": self.module_hash,
            "created_at": self.created_at.to_string(),
        })
    }
}

#[cfg(test)]
mod tests;
