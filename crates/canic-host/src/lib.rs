//! Host-side App build, Fleet install, deployment, and release-set helpers for Canic workspaces.

use std::process::Command;

pub mod adoption;
mod artifact_io;
mod bootstrap_candid;
mod bootstrap_coordinator;
mod bootstrap_store;
mod build_profile;
pub mod build_provenance;
pub mod candid_endpoints;
pub mod canic_metadata;
pub mod canister_build;
mod canister_protocol;
pub use canister_protocol::{CanisterProtocolError, query_with_arg as query_canister_with_arg};
pub mod canister_ready;
mod cargo_metadata;
pub mod component_topology;
pub mod cycle_balance;
pub mod deployment_truth;
pub mod diagnostics;
pub mod durable_io;
mod entropy;
pub mod evidence_envelope;
pub mod fleet_catalog;
pub mod fleet_install_input;
pub mod fleet_install_plan;
pub mod fleet_subnet_root_deletion;
pub mod format;
pub mod icp;
pub mod icp_config;
pub mod install_root;
pub mod installed_fleet;
pub mod network;
pub mod policy_gate;
pub mod protocol_binding;
pub mod registry;
pub mod release_build;
pub mod release_set;
pub mod replica_query;
pub mod role_contract;
pub mod state_manifest;
mod subnet_catalog;
pub mod table;
pub mod terminal;
#[cfg(test)]
mod test_support;
mod workspace_discovery;

pub(crate) fn cargo_command() -> Command {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);

    if let Some(toolchain) = std::env::var_os("RUSTUP_TOOLCHAIN") {
        command.env("RUSTUP_TOOLCHAIN", toolchain);
    }
    command
}

pub(crate) fn should_embed_candid_metadata(build_network: canic_core::ids::BuildNetwork) -> bool {
    build_network == canic_core::ids::BuildNetwork::Local
}

#[cfg(test)]
mod tests;
