use super::ListCommandError;
use crate::args::{default_dfx, flag_arg, parse_matches, string_option, value_arg};
use clap::Command as ClapCommand;
use std::ffi::OsString;

const LIST_HELP_AFTER: &str = "\
Examples:
  canic list
  canic list --fleet demo
  canic list --from user_hub
  canic list --standalone";

///
/// ListOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ListOptions {
    pub(super) source: ListSource,
    pub(super) fleet: Option<String>,
    pub(super) root: Option<String>,
    pub(super) anchor: Option<String>,
    pub(super) network: Option<String>,
    pub(super) dfx: String,
}

impl ListOptions {
    /// Parse canister listing options from CLI arguments.
    pub(super) fn parse<I>(args: I) -> Result<Self, ListCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let args = args.into_iter().collect::<Vec<_>>();
        let matches =
            parse_matches(list_command(), args).map_err(|_| ListCommandError::Usage(usage()))?;
        let standalone = matches.get_flag("standalone");
        let root = string_option(&matches, "root");

        if standalone && root.is_some() {
            return Err(ListCommandError::ConflictingListSources);
        }

        let source = if root.is_some() {
            ListSource::RootRegistry
        } else if standalone {
            ListSource::Standalone
        } else {
            ListSource::Auto
        };

        Ok(Self {
            source,
            fleet: string_option(&matches, "fleet"),
            root,
            anchor: string_option(&matches, "from"),
            network: string_option(&matches, "network"),
            dfx: string_option(&matches, "dfx").unwrap_or_else(default_dfx),
        })
    }
}

///
/// ListSource
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ListSource {
    Auto,
    Standalone,
    RootRegistry,
}

// Build the list parser.
fn list_command() -> ClapCommand {
    ClapCommand::new("list")
        .bin_name("canic list")
        .about("Show registry canisters as a tree table")
        .disable_help_flag(true)
        .arg(
            flag_arg("standalone")
                .long("standalone")
                .help("List local dfx canister ids without reading fleet state"),
        )
        .arg(
            value_arg("fleet")
                .long("fleet")
                .value_name("name")
                .help("Read a named installed fleet instead of the current fleet"),
        )
        .arg(
            value_arg("root")
                .long("root")
                .value_name("name-or-principal")
                .help("Query a specific root canister registry"),
        )
        .arg(
            value_arg("from")
                .long("from")
                .value_name("name-or-principal")
                .help("Render a subtree anchored at one canister"),
        )
        .arg(
            value_arg("network")
                .long("network")
                .value_name("name")
                .help("DFX network to inspect"),
        )
        .arg(
            value_arg("dfx")
                .long("dfx")
                .value_name("path")
                .help("Path to the dfx executable"),
        )
        .after_help(LIST_HELP_AFTER)
}

// Return list command usage text.
pub(super) fn usage() -> String {
    let mut command = list_command();
    command.render_help().to_string()
}
