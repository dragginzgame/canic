//! Module: canic_cli::state
//!
//! Responsibility: expose state manifest and audit reports as diagnostic CLI
//! commands.
//! Does not own: stable-memory reads, migration execution, generated files, or
//! runtime introspection.
//! Boundary: parses `canic state` command forms and delegates report
//! construction to `canic-host`.

#[cfg(test)]
mod tests;

use crate::{
    cli::{
        clap::{flag_arg, parse_matches, parse_required_subcommand, render_usage, string_option},
        help::print_help_or_version,
    },
    version_text,
};
use canic_core::state_contract::StateManifest;
use canic_host::{
    icp_config::resolve_current_canic_icp_root,
    install_root::discover_project_canic_config_choices,
    role_contract::finding_detail,
    state_manifest::{
        STATE_AUDIT_COMMAND, STATE_MANIFEST_COMMAND, StateAuditReport, StateAuditStatus,
        StateManifestResolution, build_state_audit_report, resolve_project_state_manifest,
    },
};
use clap::Command as ClapCommand;
use std::ffi::OsString;
use thiserror::Error as ThisError;

const AUDIT_COMMAND: &str = "audit";
const MANIFEST_COMMAND: &str = "manifest";
const JSON_ARG: &str = "json";
const ROLE_ARG: &str = "role";

const STATE_HELP_AFTER: &str = "\
Examples:
  canic state audit
  canic state audit --role root
  canic state audit --json
  canic state manifest
  canic state manifest --role root
  canic state manifest --json

State commands are diagnostic-only metadata reports. They do not read stable
memory values, run migrations, repair memory IDs, write generated files, modify
config, create deployment truth, or mutate canisters.";

const AUDIT_HELP_AFTER: &str = "\
Examples:
  canic state audit
  canic state audit --role root
  canic state audit --json

Audits Rust-authored state metadata declarations only. Warnings do not exit
nonzero; failing checks exit with code 1.";

const MANIFEST_HELP_AFTER: &str = "\
Examples:
  canic state manifest
  canic state manifest --role root
  canic state manifest --json

Renders the derived state manifest to stdout. This command does not write
manifest files.";

///
/// StateCommandError
///

#[derive(Debug, ThisError)]
pub enum StateCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("failed to render state JSON output: {0}")]
    Json(#[from] serde_json::Error),

    #[error("state audit failed")]
    AuditFailed,

    #[error("state contract resolution failed: {0}")]
    ContractRejected(String),
}

impl StateCommandError {
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) | Self::Json(_) => 2,
            Self::AuditFailed | Self::ContractRejected(_) => 1,
        }
    }

    pub const fn suppress_stderr(&self) -> bool {
        matches!(self, Self::AuditFailed)
    }
}

///
/// StateOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct StateOptions {
    role: Option<String>,
    json: bool,
}

impl StateOptions {
    fn parse_audit<I>(args: I) -> Result<Self, StateCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args, audit_command, audit_usage)
    }

    fn parse_manifest<I>(args: I) -> Result<Self, StateCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args, manifest_command, manifest_usage)
    }

    fn parse<I>(
        args: I,
        command: fn() -> ClapCommand,
        usage: fn() -> String,
    ) -> Result<Self, StateCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| StateCommandError::Usage(usage()))?;
        Ok(Self {
            role: string_option(&matches, ROLE_ARG),
            json: matches.get_flag(JSON_ARG),
        })
    }
}

pub fn run<I>(args: I) -> Result<(), StateCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    match parse_required_subcommand(state_command(), args)
        .map_err(|_| StateCommandError::Usage(usage()))?
    {
        (command, args) if command == AUDIT_COMMAND => run_audit(args),
        (command, args) if command == MANIFEST_COMMAND => run_manifest(args),
        _ => unreachable!("state dispatch command only defines known commands"),
    }
}

fn run_audit(args: Vec<OsString>) -> Result<(), StateCommandError> {
    if print_help_or_version(&args, audit_usage, version_text()) {
        return Ok(());
    }

    let options = StateOptions::parse_audit(args)?;
    let resolution = project_state_resolution(options.role.as_deref())?;
    let report = build_state_audit_report(&resolution, options.role.as_deref());
    if options.json {
        println!("{}", render_json(&report)?);
    } else {
        println!("{}", render_audit_text(&report));
    }
    if report.status == StateAuditStatus::Fail {
        return Err(StateCommandError::AuditFailed);
    }
    Ok(())
}

fn run_manifest(args: Vec<OsString>) -> Result<(), StateCommandError> {
    if print_help_or_version(&args, manifest_usage, version_text()) {
        return Ok(());
    }

    let options = StateOptions::parse_manifest(args)?;
    let resolution = project_state_resolution(options.role.as_deref())?;
    let manifest = match resolution {
        StateManifestResolution::Resolved { manifest, .. } => manifest,
        StateManifestResolution::Rejected { errors } => {
            return Err(StateCommandError::ContractRejected(
                errors
                    .iter()
                    .map(|finding| format!("{}: {}", finding.code(), finding_detail(finding)))
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }
    };
    if options.json {
        println!("{}", render_json(&manifest)?);
    } else {
        println!("{}", render_manifest_text(&manifest));
    }
    Ok(())
}

fn project_state_resolution(
    role: Option<&str>,
) -> Result<StateManifestResolution, StateCommandError> {
    let project_root = resolve_current_canic_icp_root()
        .map_err(|error| StateCommandError::ContractRejected(error.to_string()))?;
    let configs = discover_project_canic_config_choices(&project_root)
        .map_err(|error| StateCommandError::ContractRejected(error.to_string()))?;
    Ok(resolve_project_state_manifest(
        &project_root,
        &configs,
        role,
    ))
}

fn render_json<T: serde::Serialize>(value: &T) -> Result<String, StateCommandError> {
    serde_json::to_string_pretty(value).map_err(StateCommandError::from)
}

fn state_command() -> ClapCommand {
    ClapCommand::new("state")
        .bin_name("canic state")
        .about("Audit declared Canic state metadata")
        .disable_help_flag(true)
        .subcommand(crate::cli::clap::passthrough_subcommand(
            ClapCommand::new(AUDIT_COMMAND)
                .about("Audit declared state metadata")
                .disable_help_flag(true),
        ))
        .subcommand(crate::cli::clap::passthrough_subcommand(
            ClapCommand::new(MANIFEST_COMMAND)
                .about("Render the derived state manifest")
                .disable_help_flag(true),
        ))
        .after_help(STATE_HELP_AFTER)
}

fn audit_command() -> ClapCommand {
    ClapCommand::new(AUDIT_COMMAND)
        .bin_name(STATE_AUDIT_COMMAND)
        .about("Audit declared state metadata")
        .disable_help_flag(true)
        .arg(
            crate::cli::clap::value_arg(ROLE_ARG)
                .long(ROLE_ARG)
                .value_name("role")
                .help("Limit the report to one canister role"),
        )
        .arg(flag_arg(JSON_ARG).long(JSON_ARG).help("Print JSON output"))
        .after_help(AUDIT_HELP_AFTER)
}

fn manifest_command() -> ClapCommand {
    ClapCommand::new(MANIFEST_COMMAND)
        .bin_name(STATE_MANIFEST_COMMAND)
        .about("Render the derived state manifest")
        .disable_help_flag(true)
        .arg(
            crate::cli::clap::value_arg(ROLE_ARG)
                .long(ROLE_ARG)
                .value_name("role")
                .help("Limit the manifest to one canister role"),
        )
        .arg(flag_arg(JSON_ARG).long(JSON_ARG).help("Print JSON output"))
        .after_help(MANIFEST_HELP_AFTER)
}

fn usage() -> String {
    render_usage(state_command)
}

fn audit_usage() -> String {
    render_usage(audit_command)
}

fn manifest_usage() -> String {
    render_usage(manifest_command)
}

fn render_audit_text(report: &StateAuditReport) -> String {
    let mut lines = vec![
        report.command.to_string(),
        format!("status: {}", report.status.label()),
        format!("schema_version: {}", report.schema_version),
        format!("scope: {}", report.scope.label()),
    ];
    if let Some(role) = &report.role {
        lines.push(format!("role: {role}"));
    }
    lines.push(String::new());
    lines.push("checks".to_string());
    for check in &report.checks {
        lines.push(format!(
            "{} [{}] {}",
            check.category.label(),
            check.status.label(),
            check.code
        ));
        lines.push(format!("  subject: {}", check.subject));
        lines.push(format!("  detail: {}", check.detail));
        if let Some(next) = &check.next {
            lines.push(format!("  next: {next}"));
        }
        lines.push(format!("  source: {}", check.source.label()));
    }
    if !report.next_actions.is_empty() {
        lines.push(String::new());
        lines.push("next actions".to_string());
        for action in &report.next_actions {
            lines.push(format!("  - {action}"));
        }
    }
    lines.join("\n")
}

fn render_manifest_text(manifest: &StateManifest) -> String {
    let mut lines = vec![
        "canic state manifest".to_string(),
        format!("schema_version: {}", manifest.schema_version),
    ];
    for role in &manifest.roles {
        lines.push(String::new());
        lines.push(format!("role: {}", role.canister_role));
        lines.push("state".to_string());
        for domain in &role.state {
            lines.push(format!("  {} [{}]", domain.domain, domain.storage.as_str()));
            lines.push(format!("    version: {}", domain.version));
            lines.push(format!(
                "    memory_id: {}",
                domain
                    .memory_id
                    .map_or_else(|| "none".to_string(), |id| id.to_string())
            ));
            lines.push(format!("    owner: {}", domain.owner));
            lines.push(format!("    record: {}", domain.record));
            lines.push(format!("    snapshot: {}", domain.snapshot));
            lines.push(format!(
                "    min_supported_version: {}",
                domain.min_supported_version
            ));
            lines.push(format!(
                "    migration_policy: {}",
                domain.migration_policy.as_str()
            ));
        }
        if !role.removed_state.is_empty() {
            lines.push("removed_state".to_string());
            for entry in &role.removed_state {
                lines.push(format!("  {}", entry.domain));
                lines.push(format!("    disposition: {}", entry.disposition));
                lines.push(format!("    reason: {}", entry.reason));
            }
        }
        if !role.reserved_memory.is_empty() {
            lines.push("reserved_memory".to_string());
            for entry in &role.reserved_memory {
                lines.push(format!("  {}", entry.label));
                lines.push(format!("    memory_id: {}", entry.memory_id));
                lines.push(format!("    owner: {}", entry.owner));
                lines.push(format!("    reason: {}", entry.reason));
            }
        }
    }
    lines.join("\n")
}
