//! Module: evidence::command
//!
//! Responsibility: build `canic evidence` Clap command definitions and usage text.
//! Does not own: command dispatch, option parsing, policy evaluation, or report rendering.
//! Boundary: passive CLI surface construction for evidence commands.

use crate::cli::clap::{flag_arg, passthrough_subcommand, render_usage, value_arg};
use clap::{ArgGroup, Command as ClapCommand};

pub(super) fn usage() -> String {
    render_usage(evidence_command)
}

pub(super) fn compare_usage() -> String {
    render_usage(evidence_compare_command)
}

pub(super) fn gate_usage() -> String {
    render_usage(evidence_gate_command)
}

pub(super) const JSON_ARG: &str = "json";
pub(super) const EVIDENCE_ENVELOPE_ARG: &str = "evidence-envelope";

pub(super) fn evidence_command() -> ClapCommand {
    ClapCommand::new("evidence")
        .bin_name("canic evidence")
        .about("Evaluate and compare stable Canic evidence envelopes")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("gate")
                .about("Evaluate one EvidenceEnvelopeV1 against a CI policy")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("compare")
                .about("Compare two EvidenceEnvelopeV1 JSON files")
                .disable_help_flag(true),
        ))
}

pub(super) fn evidence_gate_command() -> ClapCommand {
    ClapCommand::new("gate")
        .bin_name("canic evidence gate")
        .about("Evaluate one EvidenceEnvelopeV1 against a CI policy")
        .disable_help_flag(true)
        .arg(
            value_arg("policy")
                .long("policy")
                .value_name("path")
                .required(true),
        )
        .arg(
            value_arg("envelope")
                .long("envelope")
                .value_name("path")
                .required(false),
        )
        .arg(
            value_arg("manifest")
                .long("manifest")
                .value_name("path")
                .required(false),
        )
        .arg(
            flag_arg(JSON_ARG)
                .long(JSON_ARG)
                .conflicts_with(EVIDENCE_ENVELOPE_ARG)
                .help("Print raw policy-gate report JSON output"),
        )
        .arg(
            flag_arg(EVIDENCE_ENVELOPE_ARG)
                .long(EVIDENCE_ENVELOPE_ARG)
                .help("Print the stable CI/GitOps evidence envelope"),
        )
        .arg(value_arg("output").long("output").value_name("path"))
        .group(
            ArgGroup::new("gate-input")
                .args(["envelope", "manifest"])
                .required(true)
                .multiple(false),
        )
        .after_help(
            "Examples:\n  canic evidence gate --policy ci/canic-policy.toml --envelope artifacts/canic/build-provenance.json\n  canic evidence gate --policy ci/canic-policy.toml --manifest ci/canic-evidence.toml --json --output artifacts/canic/policy-gate-report.json\n\nReads exactly one policy file and either one existing EvidenceEnvelopeV1 or one project evidence manifest. The gate is passive: it does not run builds, deploy, discover live state, mutate inputs, or turn policy success into deployment truth.",
        )
}

pub(super) fn evidence_compare_command() -> ClapCommand {
    ClapCommand::new("compare")
        .bin_name("canic evidence compare")
        .about("Compare two EvidenceEnvelopeV1 JSON files")
        .disable_help_flag(true)
        .arg(
            value_arg("left")
                .long("left")
                .value_name("file")
                .required(true),
        )
        .arg(
            value_arg("right")
                .long("right")
                .value_name("file")
                .required(true),
        )
        .arg(flag_arg(JSON_ARG).long(JSON_ARG).help("Print JSON output"))
        .after_help(
            "Compares stable envelope fields and ignores generated_at, canic_version, and the nested payload body. The payload_sha256 field is compared.",
        )
}
