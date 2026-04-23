//! Published installer helpers for downstream Canic workspaces.

use std::process::Command;

pub mod bootstrap_store;
pub mod canister_build;
pub mod install_root;
pub mod release_set;
mod workspace_discovery;

pub(crate) fn cargo_command() -> Command {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);

    if let Some(toolchain) = std::env::var_os("RUSTUP_TOOLCHAIN") {
        command.env("RUSTUP_TOOLCHAIN", toolchain);
    }

    command
}
