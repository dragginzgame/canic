use crate::{cli::defaults::local_environment, cli::help::print_help_or_version, version_text};
mod config;
mod live;
mod options;
mod render;

use canic_backup::discovery::DiscoveryError;
use canic_host::{
    icp::IcpCommandError, icp_config::IcpConfigError, install_root::ConfigDiscoveryError,
    installed_deployment::InstalledDeploymentError, registry::RegistryParseError,
    release_set::AppConfigError,
};
use config::{load_config_role_rows, missing_config_roles};
use live::{
    list_canic_versions, list_cycle_balances, list_module_hashes, list_ready_statuses,
    load_registry_entries, resolve_wasm_sizes,
};
use options::{ListOptions, config_usage, info_usage};
use render::RegistryColumnData;
#[cfg(not(test))]
use render::render_config_output;
#[cfg(test)]
use render::{ConfigRoleRow, render_config_output, render_registry_tree};
use render::{ListTitle, ListTitleSource, render_list_output};
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

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error("{source}\nHint: {hint}")]
    IcpHint {
        source: IcpCommandError,
        hint: &'static str,
    },

    #[error(transparent)]
    InstalledDeployment(#[from] InstalledDeploymentError),

    #[error("{source}\nHint: {hint}")]
    InstalledDeploymentHint {
        source: InstalledDeploymentError,
        hint: &'static str,
    },

    #[error("failed to resolve ICP project root: {0}")]
    IcpRoot(#[from] IcpConfigError),

    #[error(transparent)]
    AppConfig(#[from] AppConfigError),

    #[error("failed to discover Canic project configs: {0}")]
    ConfigDiscovery(#[from] ConfigDiscoveryError),

    #[error("app {0} is not declared by any config under apps; run `canic app list`")]
    UnknownApp(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Discovery(#[from] DiscoveryError),

    #[error(transparent)]
    Registry(#[from] RegistryParseError),
}

/// Run the deployed canister listing command under `canic info`.
pub fn run_info<I>(args: I) -> Result<(), ListCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, info_usage, version_text()) {
        return Ok(());
    }

    let options = ListOptions::parse_info_list(args)?;
    run_list_options(options)
}

fn run_list_options(options: ListOptions) -> Result<(), ListCommandError> {
    let registry = load_registry_entries(&options)?;
    let anchor = options.subtree.clone();
    let readiness = list_ready_statuses(&options, &registry, anchor.as_deref())?;
    let canic_versions = list_canic_versions(&options, &registry, anchor.as_deref())?;
    let module_hashes = list_module_hashes(&registry, anchor.as_deref())?;
    let wasm_sizes = resolve_wasm_sizes(&options, &registry)?;
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

/// Run the selected App config listing command.
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
    let source = match options.source {
        options::ListSource::Config => ListTitleSource::App,
        options::ListSource::RootRegistry => ListTitleSource::Deployment,
    };
    ListTitle {
        source,
        name: options.target.clone(),
        environment: state_environment(options),
    }
}

fn state_environment(options: &ListOptions) -> String {
    options
        .environment
        .clone()
        .unwrap_or_else(local_environment)
}

fn should_color_list_output() -> bool {
    io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

#[cfg(test)]
mod tests;
