//! Host-side App build and desired-state Fleet reconciliation for Canic workspaces.

use std::{
    io,
    process::{Command, Output},
    thread,
    time::Duration,
};

const EXECUTABLE_BUSY_RETRY_ATTEMPTS: usize = 8;
const EXECUTABLE_BUSY_RETRY_DELAY: Duration = Duration::from_millis(10);

mod artifact_io;
pub mod binaryen;
mod bootstrap_candid;
mod bootstrap_coordinator;
mod bootstrap_pool_ledger_recovery;
mod bootstrap_store;
mod build_profile;
pub mod build_provenance;
pub mod candid_endpoints;
pub mod canic_metadata;
pub mod canister_build;
mod canister_protocol;
pub use canister_protocol::{
    CanisterProtocolError, call_with_arg as call_canister_with_arg,
    query_with_arg as query_canister_with_arg,
};
pub mod canister_ready;
mod cargo_metadata;
pub mod component_topology;
pub mod config_discovery;
pub mod cycle_balance;
pub mod diagnostics;
pub mod durable_io;
mod entropy;
pub mod evidence_envelope;
pub mod fleet_catalog;
pub mod fleet_ensure;
pub mod format;
pub mod icp;
pub mod icp_config;
pub mod network;
pub mod policy_gate;
pub mod protocol_binding;
pub mod registry;
pub mod release_build;
pub mod release_set;
pub mod replica_query;
pub mod role_contract;
pub mod state_manifest;
pub mod subnet_catalog;
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

pub(crate) fn output_with_executable_busy_retry(command: &mut Command) -> io::Result<Output> {
    for attempt in 0..EXECUTABLE_BUSY_RETRY_ATTEMPTS {
        match command.output() {
            Err(error)
                if error.kind() == io::ErrorKind::ExecutableFileBusy
                    && attempt + 1 < EXECUTABLE_BUSY_RETRY_ATTEMPTS =>
            {
                thread::sleep(EXECUTABLE_BUSY_RETRY_DELAY);
            }
            result => return result,
        }
    }
    unreachable!("bounded executable-busy retry always returns on its final attempt")
}

pub(crate) fn should_embed_candid_metadata(build_network: canic_core::ids::BuildNetwork) -> bool {
    build_network == canic_core::ids::BuildNetwork::Local
}

#[cfg(test)]
mod tests;
