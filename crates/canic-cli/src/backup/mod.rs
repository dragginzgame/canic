use crate::{
    args::{parse_subcommand, passthrough_subcommand, print_help_or_version},
    output, version_text,
};
use canic_backup::{
    journal::JournalResumeReport,
    persistence::{BackupIntegrityReport, BackupLayout, PersistenceError},
};
use canic_host::table::WhitespaceTable;
use clap::Command as ClapCommand;
use std::{
    ffi::OsString,
    fs,
    io::{self, Write},
    path::PathBuf,
};
use thiserror::Error as ThisError;

mod options;

pub use options::{BackupListOptions, BackupStatusOptions, BackupVerifyOptions};

///
/// BackupCommandError
///

#[derive(Debug, ThisError)]
pub enum BackupCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(
        "backup journal {backup_id} is incomplete: {pending_artifacts}/{total_artifacts} artifacts still require resume work"
    )]
    IncompleteJournal {
        backup_id: String,
        total_artifacts: usize,
        pending_artifacts: usize,
    },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Persistence(#[from] PersistenceError),
}

///
/// BackupListEntry
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupListEntry {
    pub dir: PathBuf,
    pub backup_id: String,
    pub created_at: String,
    pub members: usize,
    pub status: String,
}

pub fn run<I>(args: I) -> Result<(), BackupCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let Some((command, args)) =
        parse_subcommand(backup_command(), args).map_err(|_| BackupCommandError::Usage(usage()))?
    else {
        return Err(BackupCommandError::Usage(usage()));
    };

    match command.as_str() {
        "list" => {
            if print_help_or_version(&args, list_usage, version_text()) {
                return Ok(());
            }
            let options = BackupListOptions::parse(args)?;
            let entries = backup_list(&options)?;
            write_list_report(&options, &entries)?;
            Ok(())
        }
        "status" => {
            if print_help_or_version(&args, status_usage, version_text()) {
                return Ok(());
            }
            let options = BackupStatusOptions::parse(args)?;
            let report = backup_status(&options)?;
            write_status_report(&options, &report)?;
            enforce_status_requirements(&options, &report)?;
            Ok(())
        }
        "verify" => {
            if print_help_or_version(&args, verify_usage, version_text()) {
                return Ok(());
            }
            let options = BackupVerifyOptions::parse(args)?;
            let report = verify_backup(&options)?;
            write_report(&options, &report)?;
            Ok(())
        }
        _ => unreachable!("backup dispatch command only defines known commands"),
    }
}

pub fn backup_list(
    options: &BackupListOptions,
) -> Result<Vec<BackupListEntry>, BackupCommandError> {
    if !options.dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(&options.dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|path| path.is_dir())
        .filter_map(backup_list_entry)
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.dir.cmp(&left.dir))
    });
    Ok(entries)
}

pub fn backup_status(
    options: &BackupStatusOptions,
) -> Result<JournalResumeReport, BackupCommandError> {
    let layout = BackupLayout::new(options.dir.clone());
    let journal = layout.read_journal()?;
    Ok(journal.resume_report())
}

pub fn verify_backup(
    options: &BackupVerifyOptions,
) -> Result<BackupIntegrityReport, BackupCommandError> {
    let layout = BackupLayout::new(options.dir.clone());
    layout.verify_integrity().map_err(BackupCommandError::from)
}

fn backup_list_entry(dir: PathBuf) -> Option<BackupListEntry> {
    let layout = BackupLayout::new(dir.clone());
    if !layout.manifest_path().is_file() {
        return None;
    }

    let Ok(manifest) = layout.read_manifest() else {
        return Some(BackupListEntry {
            dir,
            backup_id: "-".to_string(),
            created_at: "-".to_string(),
            members: 0,
            status: "invalid-manifest".to_string(),
        });
    };

    Some(BackupListEntry {
        dir,
        backup_id: manifest.backup_id,
        created_at: manifest.created_at,
        members: manifest.fleet.members.len(),
        status: "ok".to_string(),
    })
}

fn ensure_complete_status(report: &JournalResumeReport) -> Result<(), BackupCommandError> {
    if report.is_complete {
        return Ok(());
    }

    Err(BackupCommandError::IncompleteJournal {
        backup_id: report.backup_id.clone(),
        total_artifacts: report.total_artifacts,
        pending_artifacts: report.pending_artifacts,
    })
}

fn enforce_status_requirements(
    options: &BackupStatusOptions,
    report: &JournalResumeReport,
) -> Result<(), BackupCommandError> {
    if !options.require_complete {
        return Ok(());
    }

    ensure_complete_status(report)
}

fn write_status_report(
    options: &BackupStatusOptions,
    report: &JournalResumeReport,
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

// Write the backup directory list as a compact whitespace table.
fn write_list_report(
    options: &BackupListOptions,
    entries: &[BackupListEntry],
) -> Result<(), BackupCommandError> {
    let text = render_backup_list(entries);
    if let Some(path) = &options.out {
        fs::write(path, text)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "{text}")?;
    Ok(())
}

fn render_backup_list(entries: &[BackupListEntry]) -> String {
    let mut table = WhitespaceTable::new(["DIR", "BACKUP_ID", "CREATED_AT", "MEMBERS", "STATUS"]);
    for entry in entries {
        table.push_row([
            entry.dir.display().to_string(),
            entry.backup_id.clone(),
            display_created_at(&entry.created_at),
            entry.members.to_string(),
            entry.status.clone(),
        ]);
    }
    table.render()
}

fn display_created_at(created_at: &str) -> String {
    created_at
        .strip_prefix("unix:")
        .and_then(|seconds| seconds.parse::<u64>().ok())
        .map_or_else(|| created_at.to_string(), backup_list_timestamp)
}

fn backup_list_timestamp(seconds: u64) -> String {
    let days = i64::try_from(seconds / 86_400).unwrap_or(i64::MAX);
    let seconds_of_day = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;

    format!("{day:02}/{month:02}/{year:04} {hour:02}:{minute:02}")
}

// Convert days since 1970-01-01 into a proleptic Gregorian UTC date.
const fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = year + (month <= 2) as i64;

    (year, month, day)
}

fn usage() -> String {
    let mut command = backup_command();
    command.render_help().to_string()
}

fn status_usage() -> String {
    let mut command = options::backup_status_command();
    command.render_help().to_string()
}

fn list_usage() -> String {
    let mut command = options::backup_list_command();
    command.render_help().to_string()
}

fn verify_usage() -> String {
    let mut command = options::backup_verify_command();
    command.render_help().to_string()
}

fn backup_command() -> ClapCommand {
    ClapCommand::new("backup")
        .bin_name("canic backup")
        .about("Inspect and verify backup artifacts")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("list")
                .about("List backup directories under a backup root")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("verify")
                .about("Verify layout, journal agreement, and durable artifact checksums")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("status")
                .about("Summarize resumable download journal state")
                .disable_help_flag(true),
        ))
}

#[cfg(test)]
mod tests;
