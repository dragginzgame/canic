use canic_backup::{
    journal::JournalResumeReport,
    persistence::{BackupIntegrityReport, BackupLayout, PersistenceError},
};
use std::{
    ffi::OsString,
    fs,
    io::{self, Write},
    path::PathBuf,
};
use thiserror::Error as ThisError;

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

    #[error("option {0} requires a value")]
    MissingValue(&'static str),

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
/// BackupVerifyOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupVerifyOptions {
    pub dir: PathBuf,
    pub out: Option<PathBuf>,
}

impl BackupVerifyOptions {
    /// Parse backup verification options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut dir = None;
        let mut out = None;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| BackupCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--dir" => dir = Some(PathBuf::from(next_value(&mut args, "--dir")?)),
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--help" | "-h" => return Err(BackupCommandError::Usage(usage())),
                _ => return Err(BackupCommandError::UnknownOption(arg)),
            }
        }

        Ok(Self {
            dir: dir.ok_or(BackupCommandError::MissingOption("--dir"))?,
            out,
        })
    }
}

///
/// BackupStatusOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupStatusOptions {
    pub dir: PathBuf,
    pub out: Option<PathBuf>,
    pub require_complete: bool,
}

impl BackupStatusOptions {
    /// Parse backup status options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut dir = None;
        let mut out = None;
        let mut require_complete = false;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| BackupCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--dir" => dir = Some(PathBuf::from(next_value(&mut args, "--dir")?)),
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--require-complete" => require_complete = true,
                "--help" | "-h" => return Err(BackupCommandError::Usage(usage())),
                _ => return Err(BackupCommandError::UnknownOption(arg)),
            }
        }

        Ok(Self {
            dir: dir.ok_or(BackupCommandError::MissingOption("--dir"))?,
            out,
            require_complete,
        })
    }
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
        "help" | "--help" | "-h" => Err(BackupCommandError::Usage(usage())),
        _ => Err(BackupCommandError::UnknownOption(command)),
    }
}

/// Summarize a backup journal's resumable state.
pub fn backup_status(
    options: &BackupStatusOptions,
) -> Result<JournalResumeReport, BackupCommandError> {
    let layout = BackupLayout::new(options.dir.clone());
    let journal = layout.read_journal()?;
    Ok(journal.resume_report())
}

// Enforce caller-requested status requirements after the JSON report is written.
fn enforce_status_requirements(
    options: &BackupStatusOptions,
    report: &JournalResumeReport,
) -> Result<(), BackupCommandError> {
    if !options.require_complete || report.is_complete {
        return Ok(());
    }

    Err(BackupCommandError::IncompleteJournal {
        backup_id: report.backup_id.clone(),
        total_artifacts: report.total_artifacts,
        pending_artifacts: report.pending_artifacts,
    })
}

/// Verify a backup directory's manifest, journal, and durable artifacts.
pub fn verify_backup(
    options: &BackupVerifyOptions,
) -> Result<BackupIntegrityReport, BackupCommandError> {
    let layout = BackupLayout::new(options.dir.clone());
    layout.verify_integrity().map_err(BackupCommandError::from)
}

// Write the journal status report to stdout or a requested output file.
fn write_status_report(
    options: &BackupStatusOptions,
    report: &JournalResumeReport,
) -> Result<(), BackupCommandError> {
    if let Some(path) = &options.out {
        let data = serde_json::to_vec_pretty(report)?;
        fs::write(path, data)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, report)?;
    writeln!(handle)?;
    Ok(())
}

// Write the integrity report to stdout or a requested output file.
fn write_report(
    options: &BackupVerifyOptions,
    report: &BackupIntegrityReport,
) -> Result<(), BackupCommandError> {
    if let Some(path) = &options.out {
        let data = serde_json::to_vec_pretty(report)?;
        fs::write(path, data)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, report)?;
    writeln!(handle)?;
    Ok(())
}

// Read the next required option value.
fn next_value<I>(args: &mut I, option: &'static str) -> Result<String, BackupCommandError>
where
    I: Iterator<Item = OsString>,
{
    args.next()
        .and_then(|value| value.into_string().ok())
        .ok_or(BackupCommandError::MissingValue(option))
}

// Return backup command usage text.
const fn usage() -> &'static str {
    "usage: canic backup status --dir <backup-dir> [--out <file>] [--require-complete]\n       canic backup verify --dir <backup-dir> [--out <file>]"
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_backup::{
        artifacts::ArtifactChecksum,
        journal::{ArtifactJournalEntry, ArtifactState, DownloadJournal},
        manifest::{
            BackupUnit, BackupUnitKind, ConsistencyMode, ConsistencySection, FleetBackupManifest,
            FleetMember, FleetSection, IdentityMode, SourceMetadata, SourceSnapshot, ToolMetadata,
            VerificationCheck, VerificationPlan,
        },
    };
    use std::{
        fs,
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    const ROOT: &str = "aaaaa-aa";
    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    // Ensure backup verification options parse the intended command shape.
    #[test]
    fn parses_backup_verify_options() {
        let options = BackupVerifyOptions::parse([
            OsString::from("--dir"),
            OsString::from("backups/run"),
            OsString::from("--out"),
            OsString::from("report.json"),
        ])
        .expect("parse options");

        assert_eq!(options.dir, PathBuf::from("backups/run"));
        assert_eq!(options.out, Some(PathBuf::from("report.json")));
    }

    // Ensure backup status options parse the intended command shape.
    #[test]
    fn parses_backup_status_options() {
        let options = BackupStatusOptions::parse([
            OsString::from("--dir"),
            OsString::from("backups/run"),
            OsString::from("--out"),
            OsString::from("status.json"),
            OsString::from("--require-complete"),
        ])
        .expect("parse options");

        assert_eq!(options.dir, PathBuf::from("backups/run"));
        assert_eq!(options.out, Some(PathBuf::from("status.json")));
        assert!(options.require_complete);
    }

    // Ensure backup status reads the journal and reports resume actions.
    #[test]
    fn backup_status_reads_journal_resume_report() {
        let root = temp_dir("canic-cli-backup-status");
        let layout = BackupLayout::new(root.clone());
        layout
            .write_journal(&journal_with_checksum(HASH.to_string()))
            .expect("write journal");

        let options = BackupStatusOptions {
            dir: root.clone(),
            out: None,
            require_complete: false,
        };
        let report = backup_status(&options).expect("read backup status");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(report.backup_id, "backup-test");
        assert_eq!(report.total_artifacts, 1);
        assert!(report.is_complete);
        assert_eq!(report.pending_artifacts, 0);
        assert_eq!(report.counts.skip, 1);
    }

    // Ensure require-complete accepts already durable backup journals.
    #[test]
    fn require_complete_accepts_complete_status() {
        let options = BackupStatusOptions {
            dir: PathBuf::from("unused"),
            out: None,
            require_complete: true,
        };
        let report = journal_with_checksum(HASH.to_string()).resume_report();

        enforce_status_requirements(&options, &report).expect("complete status should pass");
    }

    // Ensure require-complete rejects journals that still need resume work.
    #[test]
    fn require_complete_rejects_incomplete_status() {
        let options = BackupStatusOptions {
            dir: PathBuf::from("unused"),
            out: None,
            require_complete: true,
        };
        let report = created_journal().resume_report();

        let err = enforce_status_requirements(&options, &report)
            .expect_err("incomplete status should fail");

        assert!(matches!(
            err,
            BackupCommandError::IncompleteJournal {
                pending_artifacts: 1,
                total_artifacts: 1,
                ..
            }
        ));
    }

    // Ensure the CLI verification path reads a layout and returns an integrity report.
    #[test]
    fn verify_backup_reads_layout_and_artifacts() {
        let root = temp_dir("canic-cli-backup-verify");
        let layout = BackupLayout::new(root.clone());
        let checksum = write_artifact(&root, b"root artifact");

        layout
            .write_manifest(&valid_manifest())
            .expect("write manifest");
        layout
            .write_journal(&journal_with_checksum(checksum.hash.clone()))
            .expect("write journal");

        let options = BackupVerifyOptions {
            dir: root.clone(),
            out: None,
        };
        let report = verify_backup(&options).expect("verify backup");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(report.backup_id, "backup-test");
        assert!(report.verified);
        assert_eq!(report.durable_artifacts, 1);
        assert_eq!(report.artifacts[0].checksum, checksum.hash);
    }

    // Build one valid manifest for CLI verification tests.
    fn valid_manifest() -> FleetBackupManifest {
        FleetBackupManifest {
            manifest_version: 1,
            backup_id: "backup-test".to_string(),
            created_at: "2026-05-03T00:00:00Z".to_string(),
            tool: ToolMetadata {
                name: "canic".to_string(),
                version: "0.30.3".to_string(),
            },
            source: SourceMetadata {
                environment: "local".to_string(),
                root_canister: ROOT.to_string(),
            },
            consistency: ConsistencySection {
                mode: ConsistencyMode::CrashConsistent,
                backup_units: vec![BackupUnit {
                    unit_id: "fleet".to_string(),
                    kind: BackupUnitKind::SubtreeRooted,
                    roles: vec!["root".to_string()],
                    consistency_reason: None,
                    dependency_closure: Vec::new(),
                    topology_validation: "subtree-closed".to_string(),
                    quiescence_strategy: None,
                }],
            },
            fleet: FleetSection {
                topology_hash_algorithm: "sha256".to_string(),
                topology_hash_input: "sorted(pid,parent_pid,role,module_hash)".to_string(),
                discovery_topology_hash: HASH.to_string(),
                pre_snapshot_topology_hash: HASH.to_string(),
                topology_hash: HASH.to_string(),
                members: vec![fleet_member()],
            },
            verification: VerificationPlan::default(),
        }
    }

    // Build one valid manifest member.
    fn fleet_member() -> FleetMember {
        FleetMember {
            role: "root".to_string(),
            canister_id: ROOT.to_string(),
            parent_canister_id: None,
            subnet_canister_id: Some(ROOT.to_string()),
            controller_hint: None,
            identity_mode: IdentityMode::Fixed,
            restore_group: 1,
            verification_class: "basic".to_string(),
            verification_checks: vec![VerificationCheck {
                kind: "status".to_string(),
                method: None,
                roles: vec!["root".to_string()],
            }],
            source_snapshot: SourceSnapshot {
                snapshot_id: "root-snapshot".to_string(),
                module_hash: None,
                wasm_hash: None,
                code_version: Some("v0.30.3".to_string()),
                artifact_path: "artifacts/root".to_string(),
                checksum_algorithm: "sha256".to_string(),
            },
        }
    }

    // Build one durable journal with a caller-provided checksum.
    fn journal_with_checksum(checksum: String) -> DownloadJournal {
        DownloadJournal {
            journal_version: 1,
            backup_id: "backup-test".to_string(),
            artifacts: vec![ArtifactJournalEntry {
                canister_id: ROOT.to_string(),
                snapshot_id: "root-snapshot".to_string(),
                state: ArtifactState::Durable,
                temp_path: None,
                artifact_path: "artifacts/root".to_string(),
                checksum_algorithm: "sha256".to_string(),
                checksum: Some(checksum),
                updated_at: "2026-05-03T00:00:00Z".to_string(),
            }],
        }
    }

    // Build one incomplete journal that still needs artifact download work.
    fn created_journal() -> DownloadJournal {
        DownloadJournal {
            journal_version: 1,
            backup_id: "backup-test".to_string(),
            artifacts: vec![ArtifactJournalEntry {
                canister_id: ROOT.to_string(),
                snapshot_id: "root-snapshot".to_string(),
                state: ArtifactState::Created,
                temp_path: None,
                artifact_path: "artifacts/root".to_string(),
                checksum_algorithm: "sha256".to_string(),
                checksum: None,
                updated_at: "2026-05-03T00:00:00Z".to_string(),
            }],
        }
    }

    // Write one artifact at the layout-relative path used by test journals.
    fn write_artifact(root: &Path, bytes: &[u8]) -> ArtifactChecksum {
        let path = root.join("artifacts/root");
        fs::create_dir_all(path.parent().expect("artifact has parent")).expect("create artifacts");
        fs::write(&path, bytes).expect("write artifact");
        ArtifactChecksum::from_bytes(bytes)
    }

    // Build a unique temporary directory.
    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
    }
}
