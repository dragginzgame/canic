use super::{DeployCommandError, catalog, compare, resume_report, root, truth};
use crate::{
    cli::{
        clap::{parse_subcommand, passthrough_subcommand, render_usage},
        help::print_help_or_version,
    },
    version_text,
};
use clap::Command as ClapCommand;
use std::ffi::OsString;

const DEPLOY_INSPECT_HELP_AFTER: &str = "\
Examples:
  canic deploy inspect plan demo
  canic deploy inspect inventory demo
  canic deploy inspect diff demo
  canic deploy inspect report demo
  canic deploy inspect compare --left staging-check.json --right prod-check.json
  canic deploy inspect catalog list
  canic deploy inspect root --request root-verification.json
  canic deploy inspect resume-report --receipt receipt.json demo

These commands print raw deployment-truth JSON artifacts without installing,
resuming, or mutating state. Use `canic deploy check <deployment>` for the
compact operator summary. Use `canic inspect` for live runtime-observed
canister status from `canic_runtime_status`.";

pub(super) fn run<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(command(), args).map_err(|_| DeployCommandError::Usage(usage()))? {
        None => {
            println!("{}", usage());
            Ok(())
        }
        Some((command, args)) => match command.as_str() {
            "plan" => truth::run_plan(args),
            "inventory" => truth::run_inventory(args),
            "diff" => truth::run_diff(args),
            "report" => truth::run_report(args),
            "compare" => compare::run(args),
            "catalog" => catalog::run(args),
            "root" => root::run_inspect(args),
            "resume-report" => resume_report::run(args),
            _ => unreachable!("deploy inspect dispatch only defines known commands"),
        },
    }
}

pub(super) fn command() -> ClapCommand {
    [
        "plan",
        "inventory",
        "diff",
        "report",
        "compare",
        "catalog",
        "root",
        "resume-report",
    ]
    .into_iter()
    .fold(
        ClapCommand::new("inspect")
            .bin_name("canic deploy inspect")
            .about("Inspect raw deployment truth artifacts")
            .disable_help_flag(true),
        |command, name| command.subcommand(passthrough_subcommand(ClapCommand::new(name))),
    )
    .after_help(DEPLOY_INSPECT_HELP_AFTER)
}

pub(super) fn usage() -> String {
    render_usage(command)
}
