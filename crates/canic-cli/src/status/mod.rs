use crate::{
    args::{default_network, parse_matches, print_help_or_version},
    version_text,
};
use canic_host::install_root::{InstallState, read_current_fleet_name, read_current_install_state};
use clap::Command as ClapCommand;
use std::ffi::OsString;
use thiserror::Error as ThisError;

const STATUS_HELP_AFTER: &str = "\
Examples:
  canic status

Use canic network for just the current network name.";

///
/// StatusCommandError
///

#[derive(Debug, ThisError)]
pub enum StatusCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Host(#[from] Box<dyn std::error::Error>),
}

///
/// StatusSummary
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct StatusSummary {
    network: String,
    fleet: Option<String>,
    state: Option<InstallState>,
}

/// Run the current default context summary command.
pub fn run<I>(args: I) -> Result<(), StatusCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    parse_matches(status_command(), args).map_err(|_| StatusCommandError::Usage(usage()))?;
    println!("{}", render_status(&load_status()?));
    Ok(())
}

// Load the current network, selected fleet name, and selected fleet install state.
fn load_status() -> Result<StatusSummary, StatusCommandError> {
    let network = default_network();
    let selected_fleet = read_current_fleet_name(&network)?;
    let state = read_current_install_state(&network)?;
    let fleet = state
        .as_ref()
        .map(|state| state.fleet.clone())
        .or(selected_fleet);

    Ok(StatusSummary {
        network,
        fleet,
        state,
    })
}

// Render the current default context for human inspection.
fn render_status(summary: &StatusSummary) -> String {
    let fleet = summary.fleet.as_deref().unwrap_or("-");
    let root = summary
        .state
        .as_ref()
        .map_or("-", |state| state.root_canister_id.as_str());
    let config = summary
        .state
        .as_ref()
        .map_or("-", |state| state.config_path.as_str());

    format!(
        "Network: {}\nFleet: {fleet}\nRoot: {root}\nConfig: {config}",
        summary.network
    )
}

// Build the status parser for help rendering and argument validation.
fn status_command() -> ClapCommand {
    ClapCommand::new("status")
        .bin_name("canic status")
        .about("Show current Canic defaults")
        .disable_help_flag(true)
        .after_help(STATUS_HELP_AFTER)
}

// Return status usage text.
fn usage() -> String {
    let mut command = status_command();
    command.render_help().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = "uxrrr-q7777-77774-qaaaq-cai";

    // Ensure status output shows the installed selected fleet details.
    #[test]
    fn renders_installed_status() {
        let text = render_status(&StatusSummary {
            network: "local".to_string(),
            fleet: Some("demo".to_string()),
            state: Some(state("demo")),
        });

        assert_eq!(
            text,
            format!("Network: local\nFleet: demo\nRoot: {ROOT}\nConfig: fleets/demo/canic.toml")
        );
    }

    // Ensure status output still explains a selected but not-yet-installed fleet.
    #[test]
    fn renders_selected_uninstalled_status() {
        let text = render_status(&StatusSummary {
            network: "ic".to_string(),
            fleet: Some("app".to_string()),
            state: None,
        });

        assert_eq!(text, "Network: ic\nFleet: app\nRoot: -\nConfig: -");
    }

    // Ensure missing fleet state renders without inventing defaults.
    #[test]
    fn renders_empty_status() {
        let text = render_status(&StatusSummary {
            network: "local".to_string(),
            fleet: None,
            state: None,
        });

        assert_eq!(text, "Network: local\nFleet: -\nRoot: -\nConfig: -");
    }

    // Ensure status help documents the command purpose.
    #[test]
    fn status_usage_lists_examples() {
        let text = usage();

        assert!(text.contains("Show current Canic defaults"));
        assert!(text.contains("canic status"));
        assert!(text.contains("canic network"));
    }

    // Build a representative installed fleet state.
    fn state(fleet: &str) -> InstallState {
        InstallState {
            schema_version: 1,
            fleet: fleet.to_string(),
            installed_at_unix_secs: 42,
            network: "local".to_string(),
            root_target: "root".to_string(),
            root_canister_id: ROOT.to_string(),
            root_build_target: "root".to_string(),
            workspace_root: "/tmp/canic".to_string(),
            dfx_root: "/tmp/canic".to_string(),
            config_path: format!("fleets/{fleet}/canic.toml"),
            release_set_manifest_path: ".dfx/local/root.release-set.json".to_string(),
        }
    }
}
