use crate::{cli::defaults::local_network, cli::help::print_help_or_version, version_text};
mod config;
mod live;
mod options;
mod parse;
mod render;

use canic_backup::discovery::DiscoveryError;
use canic_host::registry::RegistryParseError;
use config::{load_config_role_rows, missing_config_roles};
use live::{
    list_canic_versions, list_cycle_balances, list_module_hashes, list_ready_statuses,
    load_registry_entries, resolve_tree_anchor, resolve_wasm_sizes,
};
use options::{ListOptions, config_usage, usage};
use render::RegistryColumnData;
#[cfg(not(test))]
use render::render_config_output;
#[cfg(test)]
use render::{
    CANIC_HEADER, CANISTER_HEADER, CYCLES_HEADER, ConfigRoleRow, MODULE_HASH_HEADER, MODULE_HEADER,
    READY_HEADER, ROLE_HEADER, WASM_HEADER, render_config_output, render_registry_separator,
    render_registry_table_row, render_registry_tree,
};
use render::{ListTitle, render_list_output};
use std::{
    ffi::OsString,
    io::{self, IsTerminal},
};
use thiserror::Error as ThisError;

///
/// ListCommandError
///

#[derive(Debug, ThisError)]
pub enum ListCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    RegistryTree(#[from] crate::support::registry_tree::RegistryTreeError),

    #[error("icp command failed: {command}\n{stderr}")]
    IcpFailed { command: String, stderr: String },

    #[error("local replica query failed: {0}")]
    ReplicaQuery(String),

    #[error(
        "fleet {fleet} points to root {root}, but that canister is not present on network {network}. Local replica state was probably restarted or reset. Run `canic install {fleet}` to recreate it."
    )]
    StaleLocalFleet {
        fleet: String,
        network: String,
        root: String,
    },

    #[error("failed to read canic fleet state: {0}")]
    InstallState(String),

    #[error(
        "fleet {fleet} is not installed on network {network}; run `canic install {fleet}` to deploy it or `canic config {fleet}` to inspect its config"
    )]
    NoInstalledFleet { network: String, fleet: String },

    #[error("fleet {0} is not declared by any config under fleets; run `canic fleet list`")]
    UnknownFleet(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Discovery(#[from] DiscoveryError),

    #[error(transparent)]
    Registry(#[from] RegistryParseError),
}

/// Run the deployed canister listing command.
pub fn run<I>(args: I) -> Result<(), ListCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = ListOptions::parse_list(args)?;
    let registry = load_registry_entries(&options)?;
    let anchor = resolve_tree_anchor(&options);
    let readiness = list_ready_statuses(&options, &registry, anchor.as_deref())?;
    let canic_versions = list_canic_versions(&options, &registry, anchor.as_deref())?;
    let module_hashes = list_module_hashes(&registry, anchor.as_deref())?;
    let wasm_sizes = resolve_wasm_sizes(&options, &registry);
    let cycles = list_cycle_balances(&options, &registry, anchor.as_deref())?;
    let missing_roles = missing_config_roles(&options, &registry);
    let title = list_title(&options);
    let columns = RegistryColumnData {
        readiness: &readiness,
        canic_versions: &canic_versions,
        module_hashes: &module_hashes,
        wasm_sizes: &wasm_sizes,
        cycles: &cycles,
        full_module_hashes: options.verbose,
        color_module_variants: should_color_list_output(),
    };
    println!(
        "{}",
        render_list_output(
            &title,
            &registry,
            anchor.as_deref(),
            &columns,
            &missing_roles
        )?
    );
    Ok(())
}

/// Run the selected fleet config listing command.
pub fn run_config<I>(args: I) -> Result<(), ListCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, config_usage, version_text()) {
        return Ok(());
    }

    let options = ListOptions::parse_config(args)?;
    let title = list_title(&options);
    let rows = load_config_role_rows(&options)?;
    println!("{}", render_config_output(&title, &rows, options.verbose));
    Ok(())
}

fn list_title(options: &ListOptions) -> ListTitle {
    ListTitle {
        fleet: options.fleet.clone(),
        network: state_network(options),
    }
}

fn state_network(options: &ListOptions) -> String {
    options.network.clone().unwrap_or_else(local_network)
}

fn should_color_list_output() -> bool {
    io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

#[cfg(test)]
mod tests;
