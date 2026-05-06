use crate::{output, restore as cli_restore, version_text};
use canic_backup::{
    journal::JournalResumeReport,
    persistence::{
        BackupInspectionReport, BackupIntegrityReport, BackupLayout, BackupProvenanceReport,
        PersistenceError,
    },
    restore::RestorePlanError,
};
use serde::Serialize;
use std::{ffi::OsString, path::PathBuf};
use thiserror::Error as ThisError;

mod options;
mod preflight;
mod smoke;

pub use options::{
    BackupInspectOptions, BackupPreflightOptions, BackupProvenanceOptions, BackupSmokeOptions,
    BackupStatusOptions, BackupVerifyOptions,
};
pub use preflight::{BackupPreflightReport, backup_preflight};
pub use smoke::{BackupSmokeReport, backup_smoke};

///
/// BackupCommandError
///

#[derive(Debug, ThisError)]
pub enum BackupCommandError {
    #[error("{0}")]
    Usage(&'static str),

    #[error("missing required option {0}")]
    MissingOption(&'static str),

    #[error("unknown option {0}")]
    UnknownOption(String),

    #[error(
        "backup journal {backup_id} is incomplete: {pending_artifacts}/{total_artifacts} artifacts still require resume work"
    )]
    IncompleteJournal {
        backup_id: String,
        total_artifacts: usize,
        pending_artifacts: usize,
    },

    #[error(
        "backup inspection {backup_id} is not ready for verification: backup_id_matches={backup_id_matches}, topology_receipts_match={topology_receipts_match}, journal_complete={journal_complete}, topology_mismatches={topology_mismatches}, missing={missing_artifacts}, unexpected={unexpected_artifacts}, path_mismatches={path_mismatches}, checksum_mismatches={checksum_mismatches}"
    )]
    InspectionNotReady {
        backup_id: String,
        backup_id_matches: bool,
        topology_receipts_match: bool,
        journal_complete: bool,
        topology_mismatches: usize,
        missing_artifacts: usize,
        unexpected_artifacts: usize,
        path_mismatches: usize,
        checksum_mismatches: usize,
    },

    #[error(
        "backup provenance {backup_id} is not consistent: backup_id_matches={backup_id_matches}, topology_receipts_match={topology_receipts_match}, topology_mismatches={topology_mismatches}"
    )]
    ProvenanceNotConsistent {
        backup_id: String,
        backup_id_matches: bool,
        topology_receipts_match: bool,
        topology_mismatches: usize,
    },

    #[error("restore plan for backup {backup_id} is not restore-ready: reasons={reasons:?}")]
    RestoreNotReady {
        backup_id: String,
        reasons: Vec<String>,
    },

    #[error("backup manifest {backup_id} is not design ready")]
    DesignConformanceNotReady { backup_id: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Persistence(#[from] PersistenceError),

    #[error(transparent)]
    RestorePlan(#[from] RestorePlanError),

    #[error(transparent)]
    RestoreCli(#[from] cli_restore::RestoreCommandError),
}

/// Run a backup subcommand.
pub fn run<I>(args: I) -> Result<(), BackupCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let Some(command) = args.next().and_then(|arg| arg.into_string().ok()) else {
        return Err(BackupCommandError::Usage(usage()));
    };

    match command.as_str() {
        "preflight" => {
            let options = BackupPreflightOptions::parse(args)?;
            backup_preflight(&options)?;
            Ok(())
        }
        "smoke" => {
            let options = BackupSmokeOptions::parse(args)?;
            backup_smoke(&options)?;
            Ok(())
        }
        "inspect" => {
            let options = BackupInspectOptions::parse(args)?;
            let report = inspect_backup(&options)?;
            write_inspect_report(&options, &report)?;
            enforce_inspection_requirements(&options, &report)?;
            Ok(())
        }
        "provenance" => {
            let options = BackupProvenanceOptions::parse(args)?;
            let report = backup_provenance(&options)?;
            write_provenance_report(&options, &report)?;
            enforce_provenance_requirements(&options, &report)?;
            Ok(())
        }
        "status" => {
            let options = BackupStatusOptions::parse(args)?;
            let report = backup_status(&options)?;
            write_status_report(&options, &report)?;
            enforce_status_requirements(&options, &report)?;
            Ok(())
        }
        "verify" => {
            let options = BackupVerifyOptions::parse(args)?;
            let report = verify_backup(&options)?;
            write_report(&options, &report)?;
            Ok(())
        }
        "help" | "--help" | "-h" => {
            println!("{}", usage());
            Ok(())
        }
        "version" | "--version" | "-V" => {
            println!("{}", version_text());
            Ok(())
        }
        _ => Err(BackupCommandError::UnknownOption(command)),
    }
}

/// Inspect manifest and journal agreement without reading artifact bytes.
pub fn inspect_backup(
    options: &BackupInspectOptions,
) -> Result<BackupInspectionReport, BackupCommandError> {
    let layout = BackupLayout::new(options.dir.clone());
    layout.inspect().map_err(BackupCommandError::from)
}

/// Report manifest and journal provenance without reading artifact bytes.
pub fn backup_provenance(
    options: &BackupProvenanceOptions,
) -> Result<BackupProvenanceReport, BackupCommandError> {
    let layout = BackupLayout::new(options.dir.clone());
    layout.provenance().map_err(BackupCommandError::from)
}

/// Summarize a backup journal's resumable state.
pub fn backup_status(
    options: &BackupStatusOptions,
) -> Result<JournalResumeReport, BackupCommandError> {
    let layout = BackupLayout::new(options.dir.clone());
    let journal = layout.read_journal()?;
    Ok(journal.resume_report())
}

/// Verify a backup directory's manifest, journal, and durable artifacts.
pub fn verify_backup(
    options: &BackupVerifyOptions,
) -> Result<BackupIntegrityReport, BackupCommandError> {
    let layout = BackupLayout::new(options.dir.clone());
    layout.verify_integrity().map_err(BackupCommandError::from)
}

// Ensure provenance is internally consistent when requested by scripts.
fn enforce_provenance_requirements(
    options: &BackupProvenanceOptions,
    report: &BackupProvenanceReport,
) -> Result<(), BackupCommandError> {
    if !options.require_consistent || (report.backup_id_matches && report.topology_receipts_match) {
        return Ok(());
    }

    Err(BackupCommandError::ProvenanceNotConsistent {
        backup_id: report.backup_id.clone(),
        backup_id_matches: report.backup_id_matches,
        topology_receipts_match: report.topology_receipts_match,
        topology_mismatches: report.topology_receipt_mismatches.len(),
    })
}

// Ensure an inspection report is ready for full verification when requested.
fn enforce_inspection_requirements(
    options: &BackupInspectOptions,
    report: &BackupInspectionReport,
) -> Result<(), BackupCommandError> {
    if !options.require_ready || report.ready_for_verify {
        return Ok(());
    }

    Err(BackupCommandError::InspectionNotReady {
        backup_id: report.backup_id.clone(),
        backup_id_matches: report.backup_id_matches,
        topology_receipts_match: report.topology_receipt_mismatches.is_empty(),
        journal_complete: report.journal_complete,
        topology_mismatches: report.topology_receipt_mismatches.len(),
        missing_artifacts: report.missing_journal_artifacts.len(),
        unexpected_artifacts: report.unexpected_journal_artifacts.len(),
        path_mismatches: report.path_mismatches.len(),
        checksum_mismatches: report.checksum_mismatches.len(),
    })
}

// Ensure a journal status report has no remaining resume work.
pub(super) fn ensure_complete_status(
    report: &JournalResumeReport,
) -> Result<(), BackupCommandError> {
    if report.is_complete {
        return Ok(());
    }

    Err(BackupCommandError::IncompleteJournal {
        backup_id: report.backup_id.clone(),
        total_artifacts: report.total_artifacts,
        pending_artifacts: report.pending_artifacts,
    })
}

// Enforce caller-requested status requirements after the JSON report is written.
fn enforce_status_requirements(
    options: &BackupStatusOptions,
    report: &JournalResumeReport,
) -> Result<(), BackupCommandError> {
    if !options.require_complete {
        return Ok(());
    }

    ensure_complete_status(report)
}

// Write the journal status report to stdout or a requested output file.
fn write_status_report(
    options: &BackupStatusOptions,
    report: &JournalResumeReport,
) -> Result<(), BackupCommandError> {
    output::write_pretty_json(options.out.as_ref(), report)
}

// Write the inspection report to stdout or a requested output file.
fn write_inspect_report(
    options: &BackupInspectOptions,
    report: &BackupInspectionReport,
) -> Result<(), BackupCommandError> {
    output::write_pretty_json(options.out.as_ref(), report)
}

// Write the provenance report to stdout or a requested output file.
fn write_provenance_report(
    options: &BackupProvenanceOptions,
    report: &BackupProvenanceReport,
) -> Result<(), BackupCommandError> {
    output::write_pretty_json(options.out.as_ref(), report)
}

// Write the integrity report to stdout or a requested output file.
fn write_report(
    options: &BackupVerifyOptions,
    report: &BackupIntegrityReport,
) -> Result<(), BackupCommandError> {
    output::write_pretty_json(options.out.as_ref(), report)
}

// Write one pretty JSON value artifact, creating its parent directory when needed.
pub(super) fn write_json_file<T>(path: &PathBuf, value: &T) -> Result<(), BackupCommandError>
where
    T: Serialize,
{
    output::write_pretty_json_file(path, value)
}

// Return backup command usage text.
const fn usage() -> &'static str {
    "usage: canic backup <command> [<args>]\n\ncommands:\n  smoke       Run the post-capture no-mutation smoke path.\n  preflight   Write the standard validation, integrity, plan, and status bundle.\n  inspect     Check manifest and journal agreement without reading artifact bytes.\n  provenance  Summarize backup source, topology, and artifact provenance.\n  status      Summarize resumable download journal state.\n  verify      Verify layout and durable artifact checksums."
}

#[cfg(test)]
mod tests;
