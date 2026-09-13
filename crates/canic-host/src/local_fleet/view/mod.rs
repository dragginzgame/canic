//! Developer discovery and resource observations; these are explicitly local simulation facts.

use candid::Principal;
use serde::Serialize;

/// One local role target and its exact simulator subnet.
#[derive(Clone, Debug, Serialize)]
pub struct LocalCanisterView {
    pub name: String,
    /// Role requested at allocation; a Ready pool member can later become an application role.
    pub allocation_role: String,
    pub canister_id: Option<Principal>,
    pub subnet_id: Principal,
    pub exists: bool,
    pub native_cycles: String,
}

/// Passive public-harness status, including the selected browser gateway and local trust.
#[derive(Clone, Debug, Serialize)]
pub struct LocalFleetView {
    pub schema_version: u16,
    pub session_id: String,
    pub name: String,
    pub environment: String,
    pub gateway: String,
    pub root_key_der_hex: String,
    pub application_subnets: Vec<Principal>,
    pub maximum_canisters: u16,
    pub canister_memory_bytes: u64,
    pub simulated_time_ns: String,
    pub canisters: Vec<LocalCanisterView>,
}

/// Exact terminal Fleet roles joined to placement observed in the owned simulator.
#[derive(Clone, Debug, Serialize)]
pub struct LocalFleetDiscoveryView {
    pub local: LocalFleetView,
    pub fleet: String,
    pub roles: Vec<LocalRoleView>,
}

/// Maintained current-inventory identity, distinct from the allocation's original role label.
#[derive(Clone, Debug, Serialize)]
pub struct LocalRoleView {
    pub canister_id: Principal,
    pub role: Option<String>,
    pub parent_canister_id: Option<String>,
    pub subnet_id: Principal,
    pub module_sha256: Option<String>,
}

/// Exact maintained initializer and authority for one newly allocated local Root.
pub struct LocalRootInstallationView {
    pub name: String,
    pub canister_id: Principal,
    pub operator: Principal,
    pub wasm: Vec<u8>,
    pub wasm_sha256: String,
    pub arguments: Vec<u8>,
    pub arguments_sha256: String,
    pub authority: canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
}

/// Resolved final controller set for one preallocated local canister.
pub struct LocalControllersView {
    pub canister_id: Principal,
    pub operator: Principal,
    pub controllers: Vec<Principal>,
}
