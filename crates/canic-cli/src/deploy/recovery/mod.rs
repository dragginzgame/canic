//! Module: deploy::recovery
//!
//! Responsibility: expose local-only verification and import of a retained install bundle.
//! Does not own: installation, repair decisions, ICP calls, or bundle construction.
//! Boundary: verify is read-only; import writes only missing exact files below one ICP root.

use super::{DeployCommandError, print_json};
use crate::{
    cli::{
        clap::{parse_matches, parse_subcommand, passthrough_subcommand, render_usage, value_arg},
        help::print_help_or_version,
    },
    version_text,
};
use canic_host::{
    icp_config::resolve_current_canic_icp_root,
    install_root::{import_fleet_install_recovery_bundle, verify_fleet_install_recovery_bundle},
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

const BUNDLE_ARG: &str = "bundle";
const INTO_ARG: &str = "into";

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
            "import" => import(args),
            "verify" => verify(args),
            _ => unreachable!("recovery dispatch defines only known commands"),
        },
    }
}

fn import<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let matches = parse_matches(import_command(), args)
        .map_err(|_| DeployCommandError::Usage(render_usage(import_command)))?;
    let bundle = required_path(&matches, BUNDLE_ARG);
    let icp_root = matches
        .get_one::<String>(INTO_ARG)
        .map(PathBuf::from)
        .map_or_else(resolve_current_canic_icp_root, Ok)?;
    let report = import_fleet_install_recovery_bundle(&bundle, &icp_root)
        .map_err(|error| DeployCommandError::Check(Box::new(error)))?;
    print_json(&report)
}

fn verify<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let matches = parse_matches(verify_command(), args)
        .map_err(|_| DeployCommandError::Usage(render_usage(verify_command)))?;
    let report = verify_fleet_install_recovery_bundle(&required_path(&matches, BUNDLE_ARG))
        .map_err(|error| DeployCommandError::Check(Box::new(error)))?;
    print_json(&report)
}

fn command() -> ClapCommand {
    ["import", "verify"].into_iter().fold(
        ClapCommand::new("recovery")
            .bin_name("canic deploy recovery")
            .about("Verify or import local retained-install evidence")
            .disable_help_flag(true),
        |command, name| command.subcommand(passthrough_subcommand(ClapCommand::new(name))),
    )
    .after_help(
        "Examples:\n  canic deploy recovery verify /secure/canic-bundle\n  canic deploy recovery import /secure/canic-bundle --into /srv/app",
    )
}

fn import_command() -> ClapCommand {
    bundle_leaf(
        "import",
        "Import missing exact files after complete verification",
    )
    .arg(
        value_arg(INTO_ARG)
            .long(INTO_ARG)
            .value_name("ICP_ROOT")
            .num_args(1)
            .help("Destination ICP root; defaults to the current ICP project root"),
    )
}

fn verify_command() -> ClapCommand {
    bundle_leaf(
        "verify",
        "Verify a bundle without changing local or remote state",
    )
}

fn bundle_leaf(name: &'static str, about: &'static str) -> ClapCommand {
    ClapCommand::new(name)
        .bin_name(format!("canic deploy recovery {name}"))
        .about(about)
        .disable_help_flag(true)
        .arg(
            value_arg(BUNDLE_ARG)
                .value_name("BUNDLE_PATH")
                .required(true)
                .help("Path to one content-addressed recovery bundle"),
        )
}

fn required_path(matches: &clap::ArgMatches, name: &str) -> PathBuf {
    PathBuf::from(
        matches
            .get_one::<String>(name)
            .expect("required recovery path"),
    )
}

fn usage() -> String {
    render_usage(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_help_exposes_only_local_verify_and_import() {
        let help = usage();
        assert!(help.contains("import"));
        assert!(help.contains("verify"));
        assert!(!help.contains("canic install"));
        assert!(!help.contains("resume"));
    }

    #[test]
    fn recovery_import_parses_explicit_bundle_and_destination() {
        let matches = parse_matches(
            import_command(),
            ["bundle", "--into", "/srv/operator-state"].map(OsString::from),
        )
        .expect("parse recovery import");
        assert_eq!(required_path(&matches, BUNDLE_ARG), PathBuf::from("bundle"));
        assert_eq!(
            matches.get_one::<String>(INTO_ARG).map(String::as_str),
            Some("/srv/operator-state")
        );
    }
}
