//! Host-side build, install, fleet, and release-set helpers for Canic workspaces.

use std::process::Command;

mod artifact_io;
mod bootstrap_store;
pub mod canister_build;
mod cargo_metadata;
pub mod dfx;
pub mod install_root;
pub mod release_set;
pub mod replica_query;
pub mod table;
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

pub(crate) fn dfx_network_from_env() -> String {
    std::env::var("DFX_NETWORK").unwrap_or_else(|_| "local".to_string())
}
