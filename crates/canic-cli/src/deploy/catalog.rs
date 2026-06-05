use super::{
    DeployCommandError, current_observed_at, output_format::CatalogOutputFormat, value_arg,
};
use crate::{
    cli::{
        clap::{
            parse_matches, parse_subcommand, passthrough_subcommand, path_option, render_usage,
            required_string, required_typed, string_option_or_else,
        },
        defaults::local_network,
        globals::internal_network_arg,
        help::print_help_or_version,
    },
    output, version_text,
};
use canic_host::{
    deployment_catalog::{
        DeploymentCatalogReportV1, DeploymentCatalogRequest, build_deployment_catalog_report,
        deployment_catalog_report_text, inspect_deployment_catalog_report,
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
  canic deploy catalog list
  canic deploy catalog inspect demo-local
  canic --network local deploy catalog list --format json --output catalog.json

Catalog commands are read-only local-state reports. They list or inspect
deployment targets recorded under .canic/<network>/deployments and do not query
live deployments, create deployment truth, mutate topology, change
controllers, install Wasm, or infer deployments from fleet-template names.";
const DEPLOY_CATALOG_LIST_HELP_AFTER: &str = "\
Examples:
  canic deploy catalog list
  canic deploy catalog list --format json
  canic --network local deploy catalog list --format json --output catalog.json

Lists deployment targets from existing local deployment-target state only. This
does not refresh live state or infer deployments from fleet-template names.";
const DEPLOY_CATALOG_INSPECT_HELP_AFTER: &str = "\
Examples:
  canic deploy catalog inspect demo-local
  canic deploy catalog inspect demo-local --format json
  canic --network local deploy catalog inspect demo-local --format json --output demo-local.json

Inspects one deployment target from existing local deployment-target state
only. The deployment argument is a deployment target, not a fleet template.";

const LIST_COMMAND: CatalogCommand = CatalogCommand {
    name: "list",
    about: "List known deployment targets from local state",
    bin_name: "canic deploy catalog list",
    help_after: DEPLOY_CATALOG_LIST_HELP_AFTER,
};
const INSPECT_COMMAND: CatalogCommand = CatalogCommand {
    name: "inspect",
    about: "Inspect one known deployment target from local state",
    bin_name: "canic deploy catalog inspect",
    help_after: DEPLOY_CATALOG_INSPECT_HELP_AFTER,
};

///
/// DeployCatalogOptions
///
#[derive(Debug)]
pub(super) struct DeployCatalogOptions {
    pub(super) deployment: Option<String>,
    pub(super) network: String,
    pub(super) format: CatalogOutputFormat,
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
    let report = build_deployment_catalog_report(&request(&options)?)
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
    let report = inspect_deployment_catalog_report(
        &request(&options)?,
        options
            .deployment
            .as_deref()
            .expect("catalog inspect parser requires deployment"),
    )
    .map_err(Box::<dyn std::error::Error>::from)
    .map_err(DeployCommandError::from)?;
    write_report(&options, &report)
}

pub(super) fn write_report(
    options: &DeployCatalogOptions,
    report: &DeploymentCatalogReportV1,
) -> Result<(), DeployCommandError> {
    match options.format {
        CatalogOutputFormat::Text => output::write_text::<Box<dyn std::error::Error>>(
            options.output.as_ref(),
            &deployment_catalog_report_text(report),
        )
        .map_err(DeployCommandError::from),
        CatalogOutputFormat::Json => output::write_pretty_json::<_, Box<dyn std::error::Error>>(
            options.output.as_ref(),
            report,
        )
        .map_err(DeployCommandError::from),
    }
}

fn request(options: &DeployCatalogOptions) -> Result<DeploymentCatalogRequest, DeployCommandError> {
    let icp_root = resolve_current_canic_icp_root()
        .map_err(Box::<dyn std::error::Error>::from)
        .map_err(DeployCommandError::from)?;
    Ok(DeploymentCatalogRequest {
        icp_root,
        network: options.network.clone(),
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
            deployment: None,
            network: string_option_or_else(&matches, "network", local_network),
            format: required_typed(&matches, "format"),
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
            deployment: Some(required_string(&matches, "deployment")),
            network: string_option_or_else(&matches, "network", local_network),
            format: required_typed(&matches, "format"),
            output: path_option(&matches, "output"),
        })
    }
}

pub(super) fn command() -> ClapCommand {
    CATALOG_COMMANDS
        .iter()
        .fold(
            ClapCommand::new("catalog")
                .bin_name("canic deploy catalog")
                .about("List or inspect known deployment targets")
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
        value_arg("deployment")
            .value_name("deployment")
            .required(true)
            .help("Deployment target name to inspect"),
    )
}

fn format_arg() -> clap::Arg {
    value_arg("format")
        .long("format")
        .value_name("text|json")
        .num_args(1)
        .default_value("text")
        .value_parser(clap::value_parser!(CatalogOutputFormat))
        .help("Output format; defaults to text")
}

fn output_arg() -> clap::Arg {
    value_arg("output")
        .long("output")
        .value_name("path")
        .num_args(1)
        .help("Write the selected catalog output format to this path")
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
        .arg(format_arg())
        .arg(output_arg())
        .arg(internal_network_arg())
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
