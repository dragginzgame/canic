use crate::{
    args::{
        default_network, first_arg_is_help, first_arg_is_version, parse_matches, string_option,
        string_values, value_arg,
    },
    version_text,
};
use canic_host::install_root::{
    FleetSummary, InstallState, list_current_fleets, select_current_fleet,
};
use canic_host::table::WhitespaceTable;
use clap::{Arg, Command as ClapCommand};
use std::ffi::OsString;
use thiserror::Error as ThisError;

const CURRENT_HEADER: &str = "CURRENT";
const FLEET_HEADER: &str = "FLEET";
const NETWORK_HEADER: &str = "NETWORK";
const ROOT_HEADER: &str = "ROOT";
const CONFIG_HEADER: &str = "CONFIG";
const FLEETS_HELP_AFTER: &str = "\
Examples:
  canic fleets
  canic fleets --network local";
const USE_HELP_AFTER: &str = "\
Examples:
  canic use demo
  canic use staging --network local";

///
/// FleetCommandError
///

#[derive(Debug, ThisError)]
pub enum FleetCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("missing fleet name")]
    MissingFleetName,

    #[error("multiple fleet names provided")]
    ConflictingFleetName,

    #[error("no Canic fleets are installed for network {0}; run canic install --config <path>")]
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
    if first_arg_is_help(&args) {
        println!("{}", usage());
        return Ok(());
    }
    if first_arg_is_version(&args) {
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
    if first_arg_is_help(&args) {
        println!("{}", use_usage());
        return Ok(());
    }
    if first_arg_is_version(&args) {
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
        let matches =
            parse_matches(fleets_command(), args).map_err(|_| FleetCommandError::Usage(usage()))?;

        Ok(Self {
            network: string_option(&matches, "network").unwrap_or_else(default_network),
        })
    }
}

impl UseFleetOptions {
    // Parse current fleet selection options.
    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(use_fleet_command(), args)
            .map_err(|_| FleetCommandError::Usage(use_usage()))?;
        let fleet_names = string_values(&matches, "fleet");
        let fleet = match fleet_names.as_slice() {
            [] => return Err(FleetCommandError::MissingFleetName),
            [fleet] => fleet.clone(),
            _ => return Err(FleetCommandError::ConflictingFleetName),
        };

        Ok(Self {
            fleet,
            network: string_option(&matches, "network").unwrap_or_else(default_network),
        })
    }
}

// Build the fleet list parser.
fn fleets_command() -> ClapCommand {
    ClapCommand::new("fleets")
        .bin_name("canic fleets")
        .about("List installed Canic fleets")
        .disable_help_flag(true)
        .arg(
            value_arg("network")
                .long("network")
                .value_name("name")
                .help("DFX network to inspect"),
        )
        .after_help(FLEETS_HELP_AFTER)
}

// Build the current-fleet selection parser.
fn use_fleet_command() -> ClapCommand {
    ClapCommand::new("use")
        .bin_name("canic use")
        .about("Select the current Canic fleet")
        .disable_help_flag(true)
        .arg(
            Arg::new("fleet")
                .num_args(0..=1)
                .value_name("name")
                .help("Installed fleet name to make current"),
        )
        .arg(
            value_arg("network")
                .long("network")
                .value_name("name")
                .help("DFX network to update"),
        )
        .after_help(USE_HELP_AFTER)
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
    let mut table = WhitespaceTable::new([
        CURRENT_HEADER,
        FLEET_HEADER,
        NETWORK_HEADER,
        ROOT_HEADER,
        CONFIG_HEADER,
    ]);
    for row in rows {
        table.push_row([row.0, row.1, row.2, row.3, row.4]);
    }
    table.render()
}

// Render the newly selected fleet.
fn render_selected_fleet(state: &InstallState) -> String {
    let mut table = WhitespaceTable::new([FLEET_HEADER, NETWORK_HEADER, ROOT_HEADER]);
    table.push_row([
        state.fleet.as_str(),
        state.network.as_str(),
        state.root_canister_id.as_str(),
    ]);
    ["Current fleet:".to_string(), table.render()].join("\n")
}

// Return fleet list usage text.
fn usage() -> String {
    let mut command = fleets_command();
    command.render_help().to_string()
}

// Return fleet selection usage text.
fn use_usage() -> String {
    let mut command = use_fleet_command();
    command.render_help().to_string()
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

    // Ensure fleet command help is no longer a terse one-line usage string.
    #[test]
    fn fleet_usage_lists_options_and_examples() {
        let text = usage();

        assert!(text.contains("List installed Canic fleets"));
        assert!(text.contains("Usage: canic fleets"));
        assert!(text.contains("--network <name>"));
        assert!(text.contains("Examples:"));
    }

    // Ensure current-fleet help renders the singular fleet argument.
    #[test]
    fn use_usage_lists_singular_fleet_argument() {
        let text = use_usage();

        assert!(text.contains("Select the current Canic fleet"));
        assert!(text.contains("Usage: canic use"));
        assert!(text.contains("[name]"));
        assert!(!text.contains("[name]..."));
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
