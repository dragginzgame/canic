use super::{
    DeployCommandError, current_observed_at, output_format::JsonTextOutputFormat, value_arg,
};
use crate::{
    cli::{
        clap::{
            flag_arg, parse_matches, parse_subcommand, passthrough_subcommand, path_option,
            render_usage, required_string, string_option_or_else,
        },
        defaults::local_environment,
        globals::internal_environment_arg,
        help::print_help_or_version,
    },
    output, version_text,
};
use canic_host::{
    fleet_catalog::{
        FleetCatalogReportV1, FleetCatalogRequest, build_fleet_catalog_report,
        fleet_catalog_report_text, inspect_fleet_catalog_report,
    },
    icp_config::resolve_current_canic_icp_root,
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

#[derive(Clone, Copy)]
struct CatalogCommand {
    name: &'static str,
    about: &'static str,
    bin_name: &'static str,
    help_after: &'static str,
}

const CATALOG_COMMANDS: &[CatalogCommand] = &[LIST_COMMAND, INSPECT_COMMAND];

const DEPLOY_CATALOG_HELP_AFTER: &str = "\
Examples:
  canic deploy inspect catalog list
  canic deploy inspect catalog inspect demo-local
  canic --environment local deploy inspect catalog list --json --output catalog.json

Catalog commands are read-only local-state reports. Environment profiles resolve
one canonical network catalog under .canic/networks/<network-id>/fleets and do
not query live Fleets, create deployment truth, mutate topology, change
controllers, install Wasm, or infer Fleets from App names.";
const DEPLOY_CATALOG_LIST_HELP_AFTER: &str = "\
Examples:
  canic deploy inspect catalog list
  canic deploy inspect catalog list --json
  canic --environment local deploy inspect catalog list --json --output catalog.json

Lists Fleets from the resolved canonical network catalog only. This does not
refresh live state or infer Fleets from App names.";
const DEPLOY_CATALOG_INSPECT_HELP_AFTER: &str = "\
Examples:
  canic deploy inspect catalog inspect demo-local
  canic deploy inspect catalog inspect demo-local --json
  canic --environment local deploy inspect catalog inspect demo-local --json --output demo-local.json

Inspects one Fleet from the resolved canonical network catalog only. The Fleet
argument is an operator-facing label, not an App identity.";
const JSON_ARG: &str = "json";

const LIST_COMMAND: CatalogCommand = CatalogCommand {
    name: "list",
    about: "List known Fleets from canonical network state",
    bin_name: "canic deploy inspect catalog list",
    help_after: DEPLOY_CATALOG_LIST_HELP_AFTER,
};
const INSPECT_COMMAND: CatalogCommand = CatalogCommand {
    name: "inspect",
    about: "Inspect one known Fleet from canonical network state",
    bin_name: "canic deploy inspect catalog inspect",
    help_after: DEPLOY_CATALOG_INSPECT_HELP_AFTER,
};

///
/// DeployCatalogOptions
///
#[derive(Debug)]
pub(super) struct DeployCatalogOptions {
    pub(super) fleet: Option<String>,
    pub(super) environment: String,
    pub(super) format: JsonTextOutputFormat,
    pub(super) output: Option<PathBuf>,
}

pub(super) fn run<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(command(), args).map_err(|_| DeployCommandError::Usage(usage()))? {
        Some((command, args)) if command == "list" => run_list(args),
        Some((command, args)) if command == "inspect" => run_inspect(args),
        _ => {
            println!("{}", usage());
            Ok(())
        }
    }
}

fn run_list<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, list_usage, version_text()) {
        return Ok(());
    }

    let options = DeployCatalogOptions::parse_list(args)?;
    let report = build_fleet_catalog_report(&request(&options)?)
        .map_err(Box::<dyn std::error::Error>::from)
        .map_err(DeployCommandError::from)?;
    write_report(&options, &report)
}

fn run_inspect<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, inspect_usage, version_text()) {
        return Ok(());
    }

    let options = DeployCatalogOptions::parse_inspect(args)?;
    let report = inspect_fleet_catalog_report(
        &request(&options)?,
        options
            .fleet
            .as_deref()
            .expect("catalog inspect parser requires Fleet"),
    )
    .map_err(Box::<dyn std::error::Error>::from)
    .map_err(DeployCommandError::from)?;
    write_report(&options, &report)
}

pub(super) fn write_report(
    options: &DeployCatalogOptions,
    report: &FleetCatalogReportV1,
) -> Result<(), DeployCommandError> {
    match options.format {
        JsonTextOutputFormat::Text => output::write_text::<Box<dyn std::error::Error>>(
            options.output.as_deref(),
            &fleet_catalog_report_text(report),
        )
        .map_err(DeployCommandError::from),
        JsonTextOutputFormat::Json => output::write_pretty_json::<_, Box<dyn std::error::Error>>(
            options.output.as_deref(),
            report,
        )
        .map_err(DeployCommandError::from),
    }
}

fn request(options: &DeployCatalogOptions) -> Result<FleetCatalogRequest, DeployCommandError> {
    let project_root = resolve_current_canic_icp_root()
        .map_err(Box::<dyn std::error::Error>::from)
        .map_err(DeployCommandError::from)?;
    Ok(FleetCatalogRequest {
        project_root,
        environment: options.environment.clone(),
        generated_at: current_observed_at()?,
    })
}

impl DeployCatalogOptions {
    #[cfg(test)]
    pub(super) fn parse_list_test<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse_list(args)
    }

    #[cfg(test)]
    pub(super) fn parse_inspect_test<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse_inspect(args)
    }

    fn parse_list<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(list_command(), args)
            .map_err(|_| DeployCommandError::Usage(list_usage()))?;
        Ok(Self {
            fleet: None,
            environment: string_option_or_else(&matches, "environment", local_environment),
            format: JsonTextOutputFormat::from_json_flag(matches.get_flag(JSON_ARG)),
            output: path_option(&matches, "output"),
        })
    }

    fn parse_inspect<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(inspect_command(), args)
            .map_err(|_| DeployCommandError::Usage(inspect_usage()))?;
        Ok(Self {
            fleet: Some(required_string(&matches, "fleet")),
            environment: string_option_or_else(&matches, "environment", local_environment),
            format: JsonTextOutputFormat::from_json_flag(matches.get_flag(JSON_ARG)),
            output: path_option(&matches, "output"),
        })
    }
}

pub(super) fn command() -> ClapCommand {
    CATALOG_COMMANDS
        .iter()
        .fold(
            ClapCommand::new("catalog")
                .bin_name("canic deploy inspect catalog")
                .about("List or inspect known Fleets")
                .disable_help_flag(true),
            |command, subcommand| command.subcommand(catalog_passthrough_command(*subcommand)),
        )
        .after_help(DEPLOY_CATALOG_HELP_AFTER)
}

fn list_command() -> ClapCommand {
    catalog_leaf_command(LIST_COMMAND)
}

fn inspect_command() -> ClapCommand {
    catalog_leaf_command(INSPECT_COMMAND).arg(
        value_arg("fleet")
            .value_name("fleet")
            .required(true)
            .help("Fleet name to inspect"),
    )
}

fn json_arg() -> clap::Arg {
    flag_arg(JSON_ARG).long(JSON_ARG).help("Print JSON output")
}

fn output_arg() -> clap::Arg {
    value_arg("output")
        .long("output")
        .value_name("path")
        .num_args(1)
        .help("Write the selected catalog report to this path")
}

fn catalog_passthrough_command(spec: CatalogCommand) -> ClapCommand {
    passthrough_subcommand(
        ClapCommand::new(spec.name)
            .about(spec.about)
            .disable_help_flag(true),
    )
}

fn catalog_leaf_command(spec: CatalogCommand) -> ClapCommand {
    ClapCommand::new(spec.name)
        .bin_name(spec.bin_name)
        .about(spec.about)
        .disable_help_flag(true)
        .arg(json_arg())
        .arg(output_arg())
        .arg(internal_environment_arg())
        .after_help(spec.help_after)
}

pub(super) fn usage() -> String {
    render_usage(command)
}

pub(super) fn list_usage() -> String {
    render_usage(list_command)
}

pub(super) fn inspect_usage() -> String {
    render_usage(inspect_command)
}
