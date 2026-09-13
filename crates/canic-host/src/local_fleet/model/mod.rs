//! Explicit local resource budgets and current persistent ownership records.

use candid::Principal;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// One private developer instance, separate from ICP CLI's ordinary local launcher.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalFleetConfig {
    pub schema_version: u16,
    pub name: String,
    pub server_binary: PathBuf,
    pub server_binary_sha256: String,
    pub gateway_port: u16,
    pub application_subnets: u8,
    pub maximum_canisters: u16,
    pub canister_memory_bytes: u64,
    pub allocation_debit_cycles: u64,
    pub request_timeout_secs: u32,
    pub server_lifetime_secs: u32,
}

/// Named allocation selected before creating one new local canister.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalAllocationInput {
    pub name: String,
    pub role: String,
    pub application_subnet: u8,
    pub controller: Principal,
}

/// Persisted target identity makes a lost local creation reply reconcilable.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalAllocationRecord {
    pub input: LocalAllocationInput,
    pub canister_id: Principal,
    pub subnet_id: Principal,
}

/// Same-release local environment identity; no predecessor import or executable fallback.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalFleetRecord {
    pub schema_version: u16,
    pub canic_version: String,
    pub session_id: String,
    pub configuration: LocalFleetConfig,
    pub application_subnets: Vec<Principal>,
    pub root_key_der_hex: String,
    pub allocations: Vec<LocalAllocationIntentRecord>,
    pub checkpoint: LocalCheckpoint,
    #[serde(deserialize_with = "Option::deserialize")]
    pub release_build_id: Option<canic_core::ids::ReleaseBuildId>,
}

/// Only an acknowledged simulator save authorizes reopening the recorded state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalCheckpoint {
    Saved,
    Running,
}

/// Exact Ledger request persisted before submission, including an explicit nullable result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalAllocationIntentRecord {
    pub input: LocalAllocationInput,
    pub subnet_id: Principal,
    pub created_at_time: u64,
    #[serde(deserialize_with = "Option::deserialize")]
    pub canister_id: Option<Principal>,
}

/// Local discard intent and terminal replay receipt, bound to the superseded session.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalResetRecord {
    pub schema_version: u16,
    pub session_id: String,
    pub complete: bool,
}

/// Current desired input bound before local preallocation or Root initialization.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalPreparationRecord {
    pub schema_version: u16,
    pub source_sha256: String,
    pub workspace_root: PathBuf,
    #[serde(deserialize_with = "Option::deserialize")]
    pub prepared: Option<crate::fleet_ensure::model::DesiredFleet>,
    pub roots: Vec<LocalRootInstallRecord>,
    pub complete: bool,
}

/// A bounded initial Root install, bound to exact bytes before the platform effect.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalRootInstallRecord {
    pub name: String,
    pub wasm_sha256: String,
    pub arguments_sha256: String,
    pub attempts: u32,
    pub verified: bool,
}
