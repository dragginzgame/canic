//! Module: canic_cli::diagnostic
//!
//! Responsibility: render compact diagnostic codes and read-only build-lock inspection.
//! Does not own: allocation history, runtime construction, or public response decoding.
//! Boundary: delegates observations to canic-host without taking recovery actions.

#[cfg(test)]
mod tests;

#[cfg(test)]
use crate::cli::clap::render_usage;
use crate::{
    cli::clap::{flag_arg, parse_matches, required_string, value_arg},
    support::build_lock::render_inspection,
};
use canic_core::diagnostics::DiagnosticCode;
use canic_host::diagnostics::{DiagnosticLookup, lookup_diagnostic};
use clap::Command;
use std::{ffi::OsString, path::PathBuf};
use thiserror::Error as ThisError;

const CODE_ARGUMENT: &str = "code";
const DIAGNOSTIC_HELP_AFTER: &str = "\
Examples:
  canic diagnostic E123
  canic diagnostic 123
  canic diagnostic build-lock --lock .canic/locks/complete-build-reuse.lock";

///
/// DiagnosticCommandError
///
/// CLI boundary error for compact-code parsing and embedded catalogue access.
///

#[derive(Debug, ThisError)]
pub enum DiagnosticCommandError {
    #[error("build lock inspection failed: {0}")]
    Inspection(#[from] std::io::Error),

    #[error("build lock report serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid diagnostic code '{0}'; expected an unsigned decimal or uppercase E prefix")]
    InvalidCode(String),

    #[error("{0}")]
    Usage(String),
}

/// Run `canic diagnostic` lookup.
pub fn run<I>(args: I) -> Result<(), DiagnosticCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let matches = match parse_matches(diagnostic_command(), args) {
        Ok(matches) => matches,
        Err(error)
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            print!("{error}");
            return Ok(());
        }
        Err(error) => return Err(DiagnosticCommandError::Usage(error.to_string())),
    };
    if let Some(options) = matches.subcommand_matches("build-lock") {
        let path = options
            .get_one::<PathBuf>("lock")
            .expect("required by parser");
        let report = canic_host::canister_build::inspect_build_lock(path)?;
        if options.get_flag("json") {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            println!("{}", render_inspection(&report));
        }
        return Ok(());
    }
    let input = required_string(&matches, CODE_ARGUMENT);
    let code = parse_code(&input)?;
    let lookup = lookup_diagnostic(code);
    println!("{}", render_lookup(lookup));
    Ok(())
}

fn diagnostic_command() -> Command {
    Command::new("diagnostic")
        .bin_name("canic diagnostic")
        .about("Look up a diagnostic code or inspect a build lock")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_negates_reqs(true)
        .args_conflicts_with_subcommands(true)
        .after_help(DIAGNOSTIC_HELP_AFTER)
        .subcommand(Command::new("build-lock")
            .about("Inspect an existing build lock without changing it")
            .after_help("Example:\n  canic diagnostic build-lock --lock .canic/locks/complete-build-reuse.lock --json")
            .arg(flag_arg("json").long("json").help("Print the structured observation"))
            .arg(value_arg("lock").long("lock").required(true).value_name("path")
                .value_parser(clap::value_parser!(PathBuf)).help("Exact existing complete-build lock path")))
        .arg(
            value_arg(CODE_ARGUMENT)
                .required(true)
                .value_name("code")
                .help("Raw decimal code or exact uppercase E-prefixed form"),
        )
}

#[cfg(test)]
fn usage() -> String {
    render_usage(diagnostic_command)
}

fn parse_code(input: &str) -> Result<DiagnosticCode, DiagnosticCommandError> {
    let numeric = input.strip_prefix('E').unwrap_or(input);
    if numeric.is_empty() || !numeric.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(DiagnosticCommandError::InvalidCode(input.to_string()));
    }
    numeric
        .parse::<u16>()
        .map(DiagnosticCode::from_raw)
        .map_err(|_| DiagnosticCommandError::InvalidCode(input.to_string()))
}

fn render_lookup(lookup: DiagnosticLookup<'_>) -> String {
    match lookup {
        DiagnosticLookup::Current(entry) => {
            let mut rendered = format!(
                "code: {}\nknown: true\nstatus: current\nname: {}\norigin: {}\nsummary: {}",
                entry.code, entry.name, entry.origin, entry.summary,
            );
            if let Some(guidance) = entry.guidance {
                rendered.push_str("\nguidance: ");
                rendered.push_str(guidance);
            }
            rendered
        }
        DiagnosticLookup::Retired(entry) => format!(
            "code: {}\nknown: true\nstatus: retired\nname: {}",
            entry.code, entry.name,
        ),
        DiagnosticLookup::Unknown(code) => {
            format!("code: {code}\nknown: false\nstatus: unknown")
        }
    }
}
