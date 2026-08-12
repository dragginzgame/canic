use super::{DeployCommandError, catalog, compare, resume_report, truth};
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
  canic deploy inspect catalog list
  canic deploy inspect plan demo

Raw, read-only artifacts. Use `canic deploy check` for a compact summary or
`canic inspect` for live runtime status.";

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
            "catalog" => catalog::run(args),
            "compare" => compare::run(args),
            "diff" => truth::run_diff(args),
            "inventory" => truth::run_inventory(args),
            "plan" => truth::run_plan(args),
            "report" => truth::run_report(args),
            "resume-report" => resume_report::run(args),
            _ => unreachable!("deploy inspect dispatch only defines known commands"),
        },
    }
}

pub(super) fn command() -> ClapCommand {
    [
        "catalog",
        "compare",
        "diff",
        "inventory",
        "plan",
        "report",
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
