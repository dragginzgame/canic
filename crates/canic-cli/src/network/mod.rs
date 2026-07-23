//! Module: canic_cli::network
//!
//! Responsibility: expose explicit canonical network trust enrollment.
//! Does not own: trust derivation, durable authority, or environment resolution.
//! Boundary: parses operator confirmation and delegates immediately to canic-host.

#[cfg(test)]
mod tests;

use crate::{
    cli::{
        clap::{
            parse_matches, parse_subcommand, passthrough_subcommand, render_usage, required_path,
            required_string,
        },
        help::print_help_or_version,
    },
    version_text,
};
use canic_host::{
    install_root::{ConfigDiscoveryError, current_canic_project_root},
    network::{
        NetworkEnrollmentOptions, NetworkEnrollmentReport, NetworkIdentityError, enroll_network,
    },
};
use clap::{Arg, Command as ClapCommand};
use std::{ffi::OsString, path::PathBuf};
use thiserror::Error as ThisError;

const NETWORK_HELP_AFTER: &str = "\
Examples:
  canic network enroll local --root-key ./root-key.der --fingerprint <sha256>";
const NETWORK_ENROLL_HELP_AFTER: &str = "\
The root key must be a regular, non-symlink DER file. Canic verifies the exact
SHA-256 fingerprint before publishing any durable network authority.

Example:
  canic network enroll local --root-key ./root-key.der --fingerprint <64-lowercase-hex>";

///
/// NetworkCommandError
///
/// CLI boundary error for canonical network trust enrollment.
///

#[derive(Debug, ThisError)]
pub enum NetworkCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Project(#[from] ConfigDiscoveryError),

    #[error(transparent)]
    Network(#[from] NetworkIdentityError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EnrollOptions {
    environment: String,
    root_key: PathBuf,
    fingerprint: String,
}

impl EnrollOptions {
    fn parse<I>(args: I) -> Result<Self, NetworkCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(network_enroll_command(), args)
            .map_err(|_| NetworkCommandError::Usage(network_enroll_usage()))?;
        Ok(Self {
            environment: required_string(&matches, "environment"),
            root_key: required_path(&matches, "root-key"),
            fingerprint: required_string(&matches, "fingerprint"),
        })
    }
}

pub fn run<I>(args: I) -> Result<(), NetworkCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(network_command(), args)
        .map_err(|_| NetworkCommandError::Usage(usage()))?
    {
        None => {
            println!("{}", usage());
            Ok(())
        }
        Some((command, args)) => match command.as_str() {
            "enroll" => run_enroll(args),
            _ => unreachable!("network dispatch command only defines known commands"),
        },
    }
}

fn run_enroll<I>(args: I) -> Result<(), NetworkCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, network_enroll_usage, version_text()) {
        return Ok(());
    }

    let options = EnrollOptions::parse(args)?;
    let project_root = current_canic_project_root()?;
    let report = enroll_network(NetworkEnrollmentOptions {
        project_root: &project_root,
        environment: &options.environment,
        root_key: &options.root_key,
        fingerprint: &options.fingerprint,
    })?;
    println!("{}", render_enrollment(&report));
    Ok(())
}

fn network_command() -> ClapCommand {
    ClapCommand::new("network")
        .bin_name("canic network")
        .about("Enroll canonical network trust identities")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("enroll")
                .about("Enroll an exact DER root trust anchor")
                .disable_help_flag(true),
        ))
        .after_help(NETWORK_HELP_AFTER)
}

fn network_enroll_command() -> ClapCommand {
    ClapCommand::new("enroll")
        .bin_name("canic network enroll")
        .about("Enroll an exact DER root trust anchor")
        .disable_help_flag(true)
        .arg(
            Arg::new("environment")
                .value_name("environment")
                .required(true)
                .help("ICP environment profile to bind"),
        )
        .arg(
            Arg::new("root-key")
                .long("root-key")
                .value_name("der-file")
                .required(true)
                .help("DER-encoded IC root public key"),
        )
        .arg(
            Arg::new("fingerprint")
                .long("fingerprint")
                .value_name("sha256")
                .required(true)
                .help("Expected 64-character lowercase SHA-256 fingerprint"),
        )
        .after_help(NETWORK_ENROLL_HELP_AFTER)
}

fn usage() -> String {
    render_usage(network_command)
}

fn network_enroll_usage() -> String {
    render_usage(network_enroll_command)
}

fn render_enrollment(report: &NetworkEnrollmentReport) -> String {
    [
        "Canonical network enrollment:".to_string(),
        format!("  environment: {}", report.environment),
        format!("  canonical_network_id: {}", report.canonical_network_id),
        format!("  root_key_fingerprint: {}", report.root_key_fingerprint),
        format!("  authority: {}", report.authority_directory.display()),
        format!("  profile: {}", report.profile_path.display()),
        format!(
            "  status: {}",
            if report.created_profile {
                "enrolled"
            } else {
                "already_enrolled"
            }
        ),
    ]
    .join("\n")
}
