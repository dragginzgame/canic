//! Module: canic_cli::diagnostic
//!
//! Responsibility: parse and render host lookup for one compact Canic diagnostic code.
//! Does not own: allocation history, runtime construction, or public response decoding.
//! Boundary: accepts only a raw decimal or exact uppercase `E` form and delegates to canic-host.

#[cfg(test)]
mod tests;

use crate::{
    cli::{
        clap::{parse_matches, render_usage, required_string, value_arg},
        help::print_help_or_version,
    },
    version_text,
};
use canic_core::diagnostics::DiagnosticCode;
use canic_host::diagnostics::{DiagnosticLookup, lookup_diagnostic};
use clap::Command;
use std::ffi::OsString;
use thiserror::Error as ThisError;

const CODE_ARGUMENT: &str = "code";
const DIAGNOSTIC_HELP_AFTER: &str = "\
Examples:
  canic diagnostic E123
  canic diagnostic 123";

///
/// DiagnosticCommandError
///
/// CLI boundary error for compact-code parsing and embedded catalogue access.
///

#[derive(Debug, ThisError)]
pub enum DiagnosticCommandError {
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
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let matches = parse_matches(diagnostic_command(), args)
        .map_err(|_| DiagnosticCommandError::Usage(usage()))?;
    let input = required_string(&matches, CODE_ARGUMENT);
    let code = parse_code(&input)?;
    let lookup = lookup_diagnostic(code);
    println!("{}", render_lookup(lookup));
    Ok(())
}

fn diagnostic_command() -> Command {
    Command::new("diagnostic")
        .about("Look up one compact Canic diagnostic code")
        .after_help(DIAGNOSTIC_HELP_AFTER)
        .arg(
            value_arg(CODE_ARGUMENT)
                .required(true)
                .value_name("code")
                .help("Raw decimal code or exact uppercase E-prefixed form"),
        )
}

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
