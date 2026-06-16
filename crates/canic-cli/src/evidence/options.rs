//! Module: evidence::options
//!
//! Responsibility: parse typed `canic evidence` command options from Clap matches.
//! Does not own: command dispatch, policy evaluation, report comparison, or output rendering.
//! Boundary: typed CLI request extraction for evidence commands.

use crate::cli::clap::{parse_matches, path_option, required_path, required_typed};
use clap::ValueEnum;
use std::{ffi::OsString, path::PathBuf};

use super::{
    EvidenceCommandError,
    command::{compare_usage, evidence_compare_command, evidence_gate_command, gate_usage},
};

///
/// EvidenceCompareOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EvidenceCompareOptions {
    pub(super) left: PathBuf,
    pub(super) right: PathBuf,
    pub(super) format: EvidenceCompareFormat,
}

impl EvidenceCompareOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, EvidenceCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(evidence_compare_command(), args)
            .map_err(|_| EvidenceCommandError::Usage(compare_usage()))?;
        Ok(Self {
            left: required_path(&matches, "left"),
            right: required_path(&matches, "right"),
            format: required_typed(&matches, "format"),
        })
    }
}

///
/// EvidenceCompareFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(super) enum EvidenceCompareFormat {
    Text,
    Json,
}

///
/// EvidenceGateOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EvidenceGateOptions {
    pub(super) policy: PathBuf,
    pub(super) input: EvidenceGateInput,
    pub(super) format: EvidenceGateFormat,
    pub(super) output: Option<PathBuf>,
}

///
/// EvidenceGateInput
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum EvidenceGateInput {
    Envelope(PathBuf),
    Manifest(PathBuf),
}

impl EvidenceGateOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, EvidenceCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(evidence_gate_command(), args)
            .map_err(|_| EvidenceCommandError::Usage(gate_usage()))?;
        Ok(Self {
            policy: required_path(&matches, "policy"),
            input: if let Some(envelope) = path_option(&matches, "envelope") {
                EvidenceGateInput::Envelope(envelope)
            } else {
                EvidenceGateInput::Manifest(
                    path_option(&matches, "manifest").expect(
                        "clap requires one of envelope or manifest through gate-input group",
                    ),
                )
            },
            format: required_typed(&matches, "format"),
            output: path_option(&matches, "output"),
        })
    }
}

///
/// EvidenceGateFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(super) enum EvidenceGateFormat {
    Text,
    Json,
    EnvelopeJson,
}
