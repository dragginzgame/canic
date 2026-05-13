//! Host-side build, install, fleet, and release-set helpers for Canic workspaces.

use std::process::Command;

mod artifact_io;
mod bootstrap_store;
mod build_profile;
pub mod canister_build;
mod cargo_metadata;
pub mod format;
pub mod icp;
pub mod icp_config;
pub mod install_root;
pub mod installed_fleet;
pub mod registry;
pub mod release_set;
pub mod replica_query;
pub mod response_parse;
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

pub(crate) fn icp_environment_from_env() -> String {
    std::env::var("ICP_ENVIRONMENT").unwrap_or_else(|_| "local".to_string())
}

pub(crate) fn should_export_candid_artifacts(environment: &str) -> bool {
    environment == "local"
}

pub(crate) fn remove_optional_file(path: &std::path::Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::should_export_candid_artifacts;

    // Keep public Candid export restricted to local/development environments.
    #[test]
    fn candid_artifact_export_is_dev_only() {
        assert!(should_export_candid_artifacts("local"));
        assert!(!should_export_candid_artifacts("ic"));
    }
}
