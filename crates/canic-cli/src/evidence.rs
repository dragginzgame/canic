//! Module: evidence
//!
//! Responsibility: dispatch `canic evidence` subcommands and surface command errors.
//! Does not own: option parsing, report rendering, or policy evaluation.
//! Boundary: facade between top-level CLI dispatch and evidence submodules.

mod command;
mod compare;
mod gate;
mod options;
#[cfg(test)]
mod tests;

use crate::{cli::clap::parse_required_subcommand, cli::help::print_help_or_version, version_text};
use canic_host::{evidence_envelope::ExitClassV1, policy_gate::PolicyGateError};
use command::{compare_usage, evidence_command, gate_usage, usage};
use compare::{
    EvidenceCompareStatus, compare_envelope_files, render_compare_differences, write_compare_report,
};
use gate::{evaluate_gate_files, is_success_exit_class, render_gate_findings, write_gate_report};
use options::{EvidenceCompareOptions, EvidenceGateOptions};
use std::ffi::OsString;
use thiserror::Error as ThisError;

///
/// EvidenceCommandError
///
#[derive(Debug, ThisError)]
pub enum EvidenceCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("envelopes differ:\n{0}")]
    EnvelopesDiffer(String),

    #[error(transparent)]
    PolicyGate(#[from] PolicyGateError),

    #[error("policy gate failed ({exit_class:?})\n{findings}")]
    PolicyGateFailed {
        exit_class: ExitClassV1,
        findings: String,
    },
}

/// Run an evidence subcommand.
pub fn run<I>(args: I) -> Result<(), EvidenceCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let (command, args) = parse_required_subcommand(evidence_command(), args)
        .map_err(|_| EvidenceCommandError::Usage(usage()))?;

    match command.as_str() {
        "compare" => run_compare(args),
        "gate" => run_gate(args),
        _ => unreachable!("evidence dispatch command only defines known commands"),
    }
}

fn run_compare(args: Vec<OsString>) -> Result<(), EvidenceCommandError> {
    if print_help_or_version(&args, compare_usage, version_text()) {
        return Ok(());
    }

    let options = EvidenceCompareOptions::parse(args)?;
    let report = compare_envelope_files(&options)?;
    write_compare_report(&options, &report)?;
    if report.status == EvidenceCompareStatus::Different {
        return Err(EvidenceCommandError::EnvelopesDiffer(
            render_compare_differences(&report),
        ));
    }
    Ok(())
}

fn run_gate(args: Vec<OsString>) -> Result<(), EvidenceCommandError> {
    if print_help_or_version(&args, gate_usage, version_text()) {
        return Ok(());
    }

    let options = EvidenceGateOptions::parse(args)?;
    let report = evaluate_gate_files(&options)?;
    write_gate_report(&options, &report)?;
    if !is_success_exit_class(report.gate_exit_class()) {
        return Err(EvidenceCommandError::PolicyGateFailed {
            exit_class: report.gate_exit_class(),
            findings: render_gate_findings(&report),
        });
    }
    Ok(())
}
