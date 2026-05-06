use crate::version_text;
use canic_installer::install_root::{
    FleetSummary, InstallState, list_current_fleets, select_current_fleet,
};
use std::{env, ffi::OsString};
use thiserror::Error as ThisError;

const CURRENT_HEADER: &str = "CURRENT";
const FLEET_HEADER: &str = "FLEET";
const NETWORK_HEADER: &str = "NETWORK";
const ROOT_HEADER: &str = "ROOT";
const CONFIG_HEADER: &str = "CONFIG";

///
/// FleetCommandError
///

#[derive(Debug, ThisError)]
pub enum FleetCommandError {
    #[error("{0}")]
    Usage(&'static str),

    #[error("unknown option {0}")]
    UnknownOption(String),

    #[error("option {0} requires a value")]
    MissingValue(&'static str),

    #[error("missing fleet name")]
    MissingFleetName,

    #[error("multiple fleet names provided")]
    ConflictingFleetName,

    #[error("no Canic fleets are installed for network {0}")]
    NoFleets(String),

    #[error(transparent)]
    Installer(#[from] Box<dyn std::error::Error>),
}

///
/// FleetOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct FleetOptions {
    network: String,
}

///
/// UseFleetOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct UseFleetOptions {
    fleet: String,
    network: String,
}

/// Run the fleet listing command.
pub fn run<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if args
        .first()
        .and_then(|arg| arg.to_str())
        .is_some_and(|arg| matches!(arg, "help" | "--help" | "-h"))
    {
        println!("{}", usage());
        return Ok(());
    }
    if args
        .first()
        .and_then(|arg| arg.to_str())
        .is_some_and(|arg| matches!(arg, "version" | "--version" | "-V"))
    {
        println!("{}", version_text());
        return Ok(());
    }

    let options = FleetOptions::parse(args)?;
    let fleets = list_current_fleets(&options.network)?;
    if fleets.is_empty() {
        return Err(FleetCommandError::NoFleets(options.network));
    }
    println!("{}", render_fleets(&fleets));
    Ok(())
}

/// Run the current fleet selection command.
pub fn run_use<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if args
        .first()
        .and_then(|arg| arg.to_str())
        .is_some_and(|arg| matches!(arg, "help" | "--help" | "-h"))
    {
        println!("{}", use_usage());
        return Ok(());
    }
    if args
        .first()
        .and_then(|arg| arg.to_str())
        .is_some_and(|arg| matches!(arg, "version" | "--version" | "-V"))
    {
        println!("{}", version_text());
        return Ok(());
    }

    let options = UseFleetOptions::parse(args)?;
    let state = select_current_fleet(&options.network, &options.fleet)?;
    println!("{}", render_selected_fleet(&state));
    Ok(())
}

impl FleetOptions {
    // Parse fleet listing options.
    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut network = default_network();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| FleetCommandError::Usage(usage()))?;
            if let Some(value) = arg.strip_prefix("--network=") {
                network = value.to_string();
                continue;
            }
            match arg.as_str() {
                "--network" => network = next_value(&mut args, "--network")?,
                "--help" | "-h" => return Err(FleetCommandError::Usage(usage())),
                _ => return Err(FleetCommandError::UnknownOption(arg)),
            }
        }

        Ok(Self { network })
    }
}

impl UseFleetOptions {
    // Parse current fleet selection options.
    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut fleet = None;
        let mut network = default_network();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| FleetCommandError::Usage(use_usage()))?;
            if let Some(value) = arg.strip_prefix("--network=") {
                network = value.to_string();
                continue;
            }
            match arg.as_str() {
                "--network" => network = next_value(&mut args, "--network")?,
                "--help" | "-h" => return Err(FleetCommandError::Usage(use_usage())),
                _ if arg.starts_with('-') => return Err(FleetCommandError::UnknownOption(arg)),
                _ => set_fleet_name(&mut fleet, arg)?,
            }
        }

        Ok(Self {
            fleet: fleet.ok_or(FleetCommandError::MissingFleetName)?,
            network,
        })
    }
}

// Render installed fleets as a compact whitespace table.
fn render_fleets(fleets: &[FleetSummary]) -> String {
    let rows = fleets
        .iter()
        .map(|fleet| {
            (
                if fleet.current { "*" } else { "" },
                fleet.name.as_str(),
                fleet.state.network.as_str(),
                fleet.state.root_canister_id.as_str(),
                fleet.state.config_path.as_str(),
            )
        })
        .collect::<Vec<_>>();
    let current_width = CURRENT_HEADER.len();
    let fleet_width = max_width(rows.iter().map(|row| row.1), FLEET_HEADER);
    let network_width = max_width(rows.iter().map(|row| row.2), NETWORK_HEADER);
    let root_width = max_width(rows.iter().map(|row| row.3), ROOT_HEADER);

    let mut lines = Vec::new();
    lines.push(format!(
        "{CURRENT_HEADER:<current_width$}  {FLEET_HEADER:<fleet_width$}  {NETWORK_HEADER:<network_width$}  {ROOT_HEADER:<root_width$}  {CONFIG_HEADER}"
    ));
    for row in rows {
        lines.push(format!(
            "{:<current_width$}  {:<fleet_width$}  {:<network_width$}  {:<root_width$}  {}",
            row.0, row.1, row.2, row.3, row.4
        ));
    }
    lines.join("\n")
}

// Render the newly selected fleet.
fn render_selected_fleet(state: &InstallState) -> String {
    let fleet_width = FLEET_HEADER.len().max(state.fleet.len());
    let network_width = NETWORK_HEADER.len().max(state.network.len());
    let root_width = ROOT_HEADER.len().max(state.root_canister_id.len());
    [
        "Current fleet:".to_string(),
        format!("{FLEET_HEADER:<fleet_width$}  {NETWORK_HEADER:<network_width$}  {ROOT_HEADER:<root_width$}"),
        format!(
            "{:<fleet_width$}  {:<network_width$}  {:<root_width$}",
            state.fleet, state.network, state.root_canister_id
        ),
    ]
    .join("\n")
}

// Return the maximum display width for one table column.
fn max_width<'a>(values: impl Iterator<Item = &'a str>, header: &str) -> usize {
    values
        .map(str::len)
        .chain([header.len()])
        .max()
        .unwrap_or(header.len())
}

// Set the selected fleet once.
fn set_fleet_name(target: &mut Option<String>, value: String) -> Result<(), FleetCommandError> {
    if target.replace(value).is_some() {
        return Err(FleetCommandError::ConflictingFleetName);
    }

    Ok(())
}

// Read the next required option value.
fn next_value<I>(args: &mut I, option: &'static str) -> Result<String, FleetCommandError>
where
    I: Iterator<Item = OsString>,
{
    args.next()
        .and_then(|value| value.into_string().ok())
        .ok_or(FleetCommandError::MissingValue(option))
}

// Resolve the network using the same local default as installer commands.
fn default_network() -> String {
    env::var("DFX_NETWORK").unwrap_or_else(|_| "local".to_string())
}

// Return fleet list usage text.
const fn usage() -> &'static str {
    "usage: canic fleets [--network <name>]"
}

// Return fleet selection usage text.
const fn use_usage() -> &'static str {
    "usage: canic use <fleet> [--network <name>]"
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = "uxrrr-q7777-77774-qaaaq-cai";

    // Ensure fleet listing options accept network selection.
    #[test]
    fn parses_fleet_options() {
        let options = FleetOptions::parse([OsString::from("--network"), OsString::from("ic")])
            .expect("parse fleet options");

        assert_eq!(options.network, "ic");
    }

    // Ensure fleet use options require exactly one fleet name.
    #[test]
    fn parses_use_fleet_options() {
        let options = UseFleetOptions::parse([
            OsString::from("demo"),
            OsString::from("--network"),
            OsString::from("local"),
        ])
        .expect("parse use options");

        assert_eq!(options.fleet, "demo");
        assert_eq!(options.network, "local");
    }

    // Ensure fleet listing renders deterministic whitespace columns.
    #[test]
    fn renders_fleets_table() {
        let fleets = vec![summary("demo", true), summary("staging", false)];

        assert_eq!(
            render_fleets(&fleets),
            format!(
                "{:<7}  {:<7}  {:<7}  {:<27}  {}\n{:<7}  {:<7}  {:<7}  {:<27}  {}\n{:<7}  {:<7}  {:<7}  {:<27}  {}",
                "CURRENT",
                "FLEET",
                "NETWORK",
                "ROOT",
                "CONFIG",
                "*",
                "demo",
                "local",
                ROOT,
                "canisters/demo/canic.toml",
                "",
                "staging",
                "local",
                ROOT,
                "canisters/staging/canic.toml",
            )
        );
    }

    // Build a representative fleet summary.
    fn summary(name: &str, current: bool) -> FleetSummary {
        FleetSummary {
            name: name.to_string(),
            current,
            state: InstallState {
                schema_version: 1,
                fleet: name.to_string(),
                installed_at_unix_secs: 42,
                network: "local".to_string(),
                root_target: "root".to_string(),
                root_canister_id: ROOT.to_string(),
                root_build_target: "root".to_string(),
                workspace_root: "/tmp/canic".to_string(),
                dfx_root: "/tmp/canic".to_string(),
                config_path: format!("canisters/{name}/canic.toml"),
                release_set_manifest_path: ".dfx/local/root.release-set.json".to_string(),
            },
        }
    }
}
