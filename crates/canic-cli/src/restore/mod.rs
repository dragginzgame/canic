use canic_backup::{
    manifest::FleetBackupManifest,
    persistence::{BackupLayout, PersistenceError},
    restore::{
        RestoreApplyDryRun, RestoreApplyDryRunError, RestoreMapping, RestorePlan, RestorePlanError,
        RestorePlanner, RestoreStatus,
    },
};
use std::{
    ffi::OsString,
    fs,
    io::{self, Write},
    path::PathBuf,
};
use thiserror::Error as ThisError;

///
/// RestoreCommandError
///

#[derive(Debug, ThisError)]
pub enum RestoreCommandError {
    #[error("{0}")]
    Usage(&'static str),

    #[error("missing required option {0}")]
    MissingOption(&'static str),

    #[error("use either --manifest or --backup-dir, not both")]
    ConflictingManifestSources,

    #[error("--require-verified requires --backup-dir")]
    RequireVerifiedNeedsBackupDir,

    #[error("restore apply currently requires --dry-run")]
    ApplyRequiresDryRun,

    #[error("restore plan for backup {backup_id} is not restore-ready: reasons={reasons:?}")]
    RestoreNotReady {
        backup_id: String,
        reasons: Vec<String>,
    },

    #[error("unknown option {0}")]
    UnknownOption(String),

    #[error("option {0} requires a value")]
    MissingValue(&'static str),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Persistence(#[from] PersistenceError),

    #[error(transparent)]
    RestorePlan(#[from] RestorePlanError),

    #[error(transparent)]
    RestoreApplyDryRun(#[from] RestoreApplyDryRunError),
}

///
/// RestorePlanOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestorePlanOptions {
    pub manifest: Option<PathBuf>,
    pub backup_dir: Option<PathBuf>,
    pub mapping: Option<PathBuf>,
    pub out: Option<PathBuf>,
    pub require_verified: bool,
    pub require_restore_ready: bool,
}

impl RestorePlanOptions {
    /// Parse restore planning options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut manifest = None;
        let mut backup_dir = None;
        let mut mapping = None;
        let mut out = None;
        let mut require_verified = false;
        let mut require_restore_ready = false;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| RestoreCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--manifest" => {
                    manifest = Some(PathBuf::from(next_value(&mut args, "--manifest")?));
                }
                "--backup-dir" => {
                    backup_dir = Some(PathBuf::from(next_value(&mut args, "--backup-dir")?));
                }
                "--mapping" => mapping = Some(PathBuf::from(next_value(&mut args, "--mapping")?)),
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--require-verified" => require_verified = true,
                "--require-restore-ready" => require_restore_ready = true,
                "--help" | "-h" => return Err(RestoreCommandError::Usage(usage())),
                _ => return Err(RestoreCommandError::UnknownOption(arg)),
            }
        }

        if manifest.is_some() && backup_dir.is_some() {
            return Err(RestoreCommandError::ConflictingManifestSources);
        }

        if manifest.is_none() && backup_dir.is_none() {
            return Err(RestoreCommandError::MissingOption(
                "--manifest or --backup-dir",
            ));
        }

        if require_verified && backup_dir.is_none() {
            return Err(RestoreCommandError::RequireVerifiedNeedsBackupDir);
        }

        Ok(Self {
            manifest,
            backup_dir,
            mapping,
            out,
            require_verified,
            require_restore_ready,
        })
    }
}

///
/// RestoreStatusOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreStatusOptions {
    pub plan: PathBuf,
    pub out: Option<PathBuf>,
}

impl RestoreStatusOptions {
    /// Parse restore status options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut plan = None;
        let mut out = None;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| RestoreCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--plan" => plan = Some(PathBuf::from(next_value(&mut args, "--plan")?)),
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--help" | "-h" => return Err(RestoreCommandError::Usage(usage())),
                _ => return Err(RestoreCommandError::UnknownOption(arg)),
            }
        }

        Ok(Self {
            plan: plan.ok_or(RestoreCommandError::MissingOption("--plan"))?,
            out,
        })
    }
}

///
/// RestoreApplyOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreApplyOptions {
    pub plan: PathBuf,
    pub status: Option<PathBuf>,
    pub out: Option<PathBuf>,
    pub dry_run: bool,
}

impl RestoreApplyOptions {
    /// Parse restore apply options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut plan = None;
        let mut status = None;
        let mut out = None;
        let mut dry_run = false;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| RestoreCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--plan" => plan = Some(PathBuf::from(next_value(&mut args, "--plan")?)),
                "--status" => status = Some(PathBuf::from(next_value(&mut args, "--status")?)),
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--dry-run" => dry_run = true,
                "--help" | "-h" => return Err(RestoreCommandError::Usage(usage())),
                _ => return Err(RestoreCommandError::UnknownOption(arg)),
            }
        }

        if !dry_run {
            return Err(RestoreCommandError::ApplyRequiresDryRun);
        }

        Ok(Self {
            plan: plan.ok_or(RestoreCommandError::MissingOption("--plan"))?,
            status,
            out,
            dry_run,
        })
    }
}

/// Run a restore subcommand.
pub fn run<I>(args: I) -> Result<(), RestoreCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let Some(command) = args.next().and_then(|arg| arg.into_string().ok()) else {
        return Err(RestoreCommandError::Usage(usage()));
    };

    match command.as_str() {
        "plan" => {
            let options = RestorePlanOptions::parse(args)?;
            let plan = plan_restore(&options)?;
            write_plan(&options, &plan)?;
            enforce_restore_plan_requirements(&options, &plan)?;
            Ok(())
        }
        "status" => {
            let options = RestoreStatusOptions::parse(args)?;
            let status = restore_status(&options)?;
            write_status(&options, &status)?;
            Ok(())
        }
        "apply" => {
            let options = RestoreApplyOptions::parse(args)?;
            let dry_run = restore_apply_dry_run(&options)?;
            write_apply_dry_run(&options, &dry_run)?;
            Ok(())
        }
        "help" | "--help" | "-h" => Err(RestoreCommandError::Usage(usage())),
        _ => Err(RestoreCommandError::UnknownOption(command)),
    }
}

/// Build a no-mutation restore plan from a manifest and optional mapping.
pub fn plan_restore(options: &RestorePlanOptions) -> Result<RestorePlan, RestoreCommandError> {
    verify_backup_layout_if_required(options)?;

    let manifest = read_manifest_source(options)?;
    let mapping = options.mapping.as_ref().map(read_mapping).transpose()?;

    RestorePlanner::plan(&manifest, mapping.as_ref()).map_err(RestoreCommandError::from)
}

/// Build the initial no-mutation restore status from a restore plan.
pub fn restore_status(
    options: &RestoreStatusOptions,
) -> Result<RestoreStatus, RestoreCommandError> {
    let plan = read_plan(&options.plan)?;
    Ok(RestoreStatus::from_plan(&plan))
}

/// Build a no-mutation restore apply dry-run from a restore plan.
pub fn restore_apply_dry_run(
    options: &RestoreApplyOptions,
) -> Result<RestoreApplyDryRun, RestoreCommandError> {
    let plan = read_plan(&options.plan)?;
    let status = options.status.as_ref().map(read_status).transpose()?;
    RestoreApplyDryRun::try_from_plan(&plan, status.as_ref()).map_err(RestoreCommandError::from)
}

// Enforce caller-requested restore plan requirements after the plan is emitted.
fn enforce_restore_plan_requirements(
    options: &RestorePlanOptions,
    plan: &RestorePlan,
) -> Result<(), RestoreCommandError> {
    if !options.require_restore_ready || plan.readiness_summary.ready {
        return Ok(());
    }

    Err(RestoreCommandError::RestoreNotReady {
        backup_id: plan.backup_id.clone(),
        reasons: plan.readiness_summary.reasons.clone(),
    })
}

// Verify backup layout integrity before restore planning when requested.
fn verify_backup_layout_if_required(
    options: &RestorePlanOptions,
) -> Result<(), RestoreCommandError> {
    if !options.require_verified {
        return Ok(());
    }

    let Some(dir) = &options.backup_dir else {
        return Err(RestoreCommandError::RequireVerifiedNeedsBackupDir);
    };

    BackupLayout::new(dir.clone()).verify_integrity()?;
    Ok(())
}

// Read the manifest from a direct path or canonical backup layout.
fn read_manifest_source(
    options: &RestorePlanOptions,
) -> Result<FleetBackupManifest, RestoreCommandError> {
    if let Some(path) = &options.manifest {
        return read_manifest(path);
    }

    let Some(dir) = &options.backup_dir else {
        return Err(RestoreCommandError::MissingOption(
            "--manifest or --backup-dir",
        ));
    };

    BackupLayout::new(dir.clone())
        .read_manifest()
        .map_err(RestoreCommandError::from)
}

// Read and decode a fleet backup manifest from disk.
fn read_manifest(path: &PathBuf) -> Result<FleetBackupManifest, RestoreCommandError> {
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(RestoreCommandError::from)
}

// Read and decode an optional source-to-target restore mapping from disk.
fn read_mapping(path: &PathBuf) -> Result<RestoreMapping, RestoreCommandError> {
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(RestoreCommandError::from)
}

// Read and decode a restore plan from disk.
fn read_plan(path: &PathBuf) -> Result<RestorePlan, RestoreCommandError> {
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(RestoreCommandError::from)
}

// Read and decode a restore status from disk.
fn read_status(path: &PathBuf) -> Result<RestoreStatus, RestoreCommandError> {
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(RestoreCommandError::from)
}

// Write the computed plan to stdout or a requested output file.
fn write_plan(options: &RestorePlanOptions, plan: &RestorePlan) -> Result<(), RestoreCommandError> {
    if let Some(path) = &options.out {
        let data = serde_json::to_vec_pretty(plan)?;
        fs::write(path, data)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, plan)?;
    writeln!(handle)?;
    Ok(())
}

// Write the computed status to stdout or a requested output file.
fn write_status(
    options: &RestoreStatusOptions,
    status: &RestoreStatus,
) -> Result<(), RestoreCommandError> {
    if let Some(path) = &options.out {
        let data = serde_json::to_vec_pretty(status)?;
        fs::write(path, data)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, status)?;
    writeln!(handle)?;
    Ok(())
}

// Write the computed apply dry-run to stdout or a requested output file.
fn write_apply_dry_run(
    options: &RestoreApplyOptions,
    dry_run: &RestoreApplyDryRun,
) -> Result<(), RestoreCommandError> {
    if let Some(path) = &options.out {
        let data = serde_json::to_vec_pretty(dry_run)?;
        fs::write(path, data)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, dry_run)?;
    writeln!(handle)?;
    Ok(())
}

// Read the next required option value.
fn next_value<I>(args: &mut I, option: &'static str) -> Result<String, RestoreCommandError>
where
    I: Iterator<Item = OsString>,
{
    args.next()
        .and_then(|value| value.into_string().ok())
        .ok_or(RestoreCommandError::MissingValue(option))
}

// Return restore command usage text.
const fn usage() -> &'static str {
    "usage: canic restore plan (--manifest <file> | --backup-dir <dir>) [--mapping <file>] [--out <file>] [--require-verified] [--require-restore-ready]\n       canic restore status --plan <file> [--out <file>]\n       canic restore apply --plan <file> [--status <file>] --dry-run [--out <file>]"
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_backup::{
        artifacts::ArtifactChecksum,
        journal::{ArtifactJournalEntry, ArtifactState, DownloadJournal},
        manifest::{
            BackupUnit, BackupUnitKind, ConsistencyMode, ConsistencySection, FleetMember,
            FleetSection, IdentityMode, SourceMetadata, SourceSnapshot, ToolMetadata,
            VerificationCheck, VerificationPlan,
        },
    };
    use serde_json::json;
    use std::{
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    const ROOT: &str = "aaaaa-aa";
    const CHILD: &str = "renrk-eyaaa-aaaaa-aaada-cai";
    const MAPPED_CHILD: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    // Ensure restore plan options parse the intended no-mutation command.
    #[test]
    fn parses_restore_plan_options() {
        let options = RestorePlanOptions::parse([
            OsString::from("--manifest"),
            OsString::from("manifest.json"),
            OsString::from("--mapping"),
            OsString::from("mapping.json"),
            OsString::from("--out"),
            OsString::from("plan.json"),
            OsString::from("--require-restore-ready"),
        ])
        .expect("parse options");

        assert_eq!(options.manifest, Some(PathBuf::from("manifest.json")));
        assert_eq!(options.backup_dir, None);
        assert_eq!(options.mapping, Some(PathBuf::from("mapping.json")));
        assert_eq!(options.out, Some(PathBuf::from("plan.json")));
        assert!(!options.require_verified);
        assert!(options.require_restore_ready);
    }

    // Ensure verified restore plan options parse with the canonical backup source.
    #[test]
    fn parses_verified_restore_plan_options() {
        let options = RestorePlanOptions::parse([
            OsString::from("--backup-dir"),
            OsString::from("backups/run"),
            OsString::from("--require-verified"),
        ])
        .expect("parse verified options");

        assert_eq!(options.manifest, None);
        assert_eq!(options.backup_dir, Some(PathBuf::from("backups/run")));
        assert_eq!(options.mapping, None);
        assert_eq!(options.out, None);
        assert!(options.require_verified);
        assert!(!options.require_restore_ready);
    }

    // Ensure restore status options parse the intended no-mutation command.
    #[test]
    fn parses_restore_status_options() {
        let options = RestoreStatusOptions::parse([
            OsString::from("--plan"),
            OsString::from("restore-plan.json"),
            OsString::from("--out"),
            OsString::from("restore-status.json"),
        ])
        .expect("parse status options");

        assert_eq!(options.plan, PathBuf::from("restore-plan.json"));
        assert_eq!(options.out, Some(PathBuf::from("restore-status.json")));
    }

    // Ensure restore apply options require the explicit dry-run mode.
    #[test]
    fn parses_restore_apply_dry_run_options() {
        let options = RestoreApplyOptions::parse([
            OsString::from("--plan"),
            OsString::from("restore-plan.json"),
            OsString::from("--status"),
            OsString::from("restore-status.json"),
            OsString::from("--dry-run"),
            OsString::from("--out"),
            OsString::from("restore-apply-dry-run.json"),
        ])
        .expect("parse apply options");

        assert_eq!(options.plan, PathBuf::from("restore-plan.json"));
        assert_eq!(options.status, Some(PathBuf::from("restore-status.json")));
        assert_eq!(
            options.out,
            Some(PathBuf::from("restore-apply-dry-run.json"))
        );
        assert!(options.dry_run);
    }

    // Ensure restore apply refuses non-dry-run execution while apply is scaffolded.
    #[test]
    fn restore_apply_requires_dry_run() {
        let err = RestoreApplyOptions::parse([
            OsString::from("--plan"),
            OsString::from("restore-plan.json"),
        ])
        .expect_err("apply without dry-run should fail");

        assert!(matches!(err, RestoreCommandError::ApplyRequiresDryRun));
    }

    // Ensure backup-dir restore planning reads the canonical layout manifest.
    #[test]
    fn plan_restore_reads_manifest_from_backup_dir() {
        let root = temp_dir("canic-cli-restore-plan-layout");
        let layout = BackupLayout::new(root.clone());
        layout
            .write_manifest(&valid_manifest())
            .expect("write manifest");

        let options = RestorePlanOptions {
            manifest: None,
            backup_dir: Some(root.clone()),
            mapping: None,
            out: None,
            require_verified: false,
            require_restore_ready: false,
        };

        let plan = plan_restore(&options).expect("plan restore");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(plan.backup_id, "backup-test");
        assert_eq!(plan.member_count, 2);
    }

    // Ensure restore planning has exactly one manifest source.
    #[test]
    fn parse_rejects_conflicting_manifest_sources() {
        let err = RestorePlanOptions::parse([
            OsString::from("--manifest"),
            OsString::from("manifest.json"),
            OsString::from("--backup-dir"),
            OsString::from("backups/run"),
        ])
        .expect_err("conflicting sources should fail");

        assert!(matches!(
            err,
            RestoreCommandError::ConflictingManifestSources
        ));
    }

    // Ensure verified planning requires the canonical backup layout source.
    #[test]
    fn parse_rejects_require_verified_with_manifest_source() {
        let err = RestorePlanOptions::parse([
            OsString::from("--manifest"),
            OsString::from("manifest.json"),
            OsString::from("--require-verified"),
        ])
        .expect_err("verification should require a backup layout");

        assert!(matches!(
            err,
            RestoreCommandError::RequireVerifiedNeedsBackupDir
        ));
    }

    // Ensure restore planning can require manifest, journal, and artifact integrity.
    #[test]
    fn plan_restore_requires_verified_backup_layout() {
        let root = temp_dir("canic-cli-restore-plan-verified");
        let layout = BackupLayout::new(root.clone());
        let manifest = valid_manifest();
        write_verified_layout(&root, &layout, &manifest);

        let options = RestorePlanOptions {
            manifest: None,
            backup_dir: Some(root.clone()),
            mapping: None,
            out: None,
            require_verified: true,
            require_restore_ready: false,
        };

        let plan = plan_restore(&options).expect("plan verified restore");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(plan.backup_id, "backup-test");
        assert_eq!(plan.member_count, 2);
    }

    // Ensure required verification fails before planning when the layout is incomplete.
    #[test]
    fn plan_restore_rejects_unverified_backup_layout() {
        let root = temp_dir("canic-cli-restore-plan-unverified");
        let layout = BackupLayout::new(root.clone());
        layout
            .write_manifest(&valid_manifest())
            .expect("write manifest");

        let options = RestorePlanOptions {
            manifest: None,
            backup_dir: Some(root.clone()),
            mapping: None,
            out: None,
            require_verified: true,
            require_restore_ready: false,
        };

        let err = plan_restore(&options).expect_err("missing journal should fail");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(matches!(err, RestoreCommandError::Persistence(_)));
    }

    // Ensure the CLI planning path validates manifests and applies mappings.
    #[test]
    fn plan_restore_reads_manifest_and_mapping() {
        let root = temp_dir("canic-cli-restore-plan");
        fs::create_dir_all(&root).expect("create temp root");
        let manifest_path = root.join("manifest.json");
        let mapping_path = root.join("mapping.json");

        fs::write(
            &manifest_path,
            serde_json::to_vec(&valid_manifest()).expect("serialize manifest"),
        )
        .expect("write manifest");
        fs::write(
            &mapping_path,
            json!({
                "members": [
                    {"source_canister": ROOT, "target_canister": ROOT},
                    {"source_canister": CHILD, "target_canister": MAPPED_CHILD}
                ]
            })
            .to_string(),
        )
        .expect("write mapping");

        let options = RestorePlanOptions {
            manifest: Some(manifest_path),
            backup_dir: None,
            mapping: Some(mapping_path),
            out: None,
            require_verified: false,
            require_restore_ready: false,
        };

        let plan = plan_restore(&options).expect("plan restore");

        fs::remove_dir_all(root).expect("remove temp root");
        let members = plan.ordered_members();
        assert_eq!(members.len(), 2);
        assert_eq!(members[0].source_canister, ROOT);
        assert_eq!(members[1].target_canister, MAPPED_CHILD);
    }

    // Ensure restore-readiness gating happens after writing the plan artifact.
    #[test]
    fn run_restore_plan_require_restore_ready_writes_plan_then_fails() {
        let root = temp_dir("canic-cli-restore-plan-require-ready");
        fs::create_dir_all(&root).expect("create temp root");
        let manifest_path = root.join("manifest.json");
        let out_path = root.join("plan.json");

        fs::write(
            &manifest_path,
            serde_json::to_vec(&valid_manifest()).expect("serialize manifest"),
        )
        .expect("write manifest");

        let err = run([
            OsString::from("plan"),
            OsString::from("--manifest"),
            OsString::from(manifest_path.as_os_str()),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
            OsString::from("--require-restore-ready"),
        ])
        .expect_err("restore readiness should be enforced");

        assert!(out_path.exists());
        let plan: RestorePlan =
            serde_json::from_slice(&fs::read(&out_path).expect("read plan")).expect("decode plan");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(!plan.readiness_summary.ready);
        assert!(matches!(
            err,
            RestoreCommandError::RestoreNotReady {
                reasons,
                ..
            } if reasons == [
                "missing-module-hash",
                "missing-wasm-hash",
                "missing-snapshot-checksum"
            ]
        ));
    }

    // Ensure restore-readiness gating accepts plans with complete provenance.
    #[test]
    fn run_restore_plan_require_restore_ready_accepts_ready_plan() {
        let root = temp_dir("canic-cli-restore-plan-ready");
        fs::create_dir_all(&root).expect("create temp root");
        let manifest_path = root.join("manifest.json");
        let out_path = root.join("plan.json");

        fs::write(
            &manifest_path,
            serde_json::to_vec(&restore_ready_manifest()).expect("serialize manifest"),
        )
        .expect("write manifest");

        run([
            OsString::from("plan"),
            OsString::from("--manifest"),
            OsString::from(manifest_path.as_os_str()),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
            OsString::from("--require-restore-ready"),
        ])
        .expect("restore-ready plan should pass");

        let plan: RestorePlan =
            serde_json::from_slice(&fs::read(&out_path).expect("read plan")).expect("decode plan");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(plan.readiness_summary.ready);
        assert!(plan.readiness_summary.reasons.is_empty());
    }

    // Ensure restore status writes the initial planned execution journal.
    #[test]
    fn run_restore_status_writes_planned_status() {
        let root = temp_dir("canic-cli-restore-status");
        fs::create_dir_all(&root).expect("create temp root");
        let plan_path = root.join("restore-plan.json");
        let out_path = root.join("restore-status.json");
        let plan = RestorePlanner::plan(&restore_ready_manifest(), None).expect("build plan");

        fs::write(
            &plan_path,
            serde_json::to_vec(&plan).expect("serialize plan"),
        )
        .expect("write plan");

        run([
            OsString::from("status"),
            OsString::from("--plan"),
            OsString::from(plan_path.as_os_str()),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
        ])
        .expect("write restore status");

        let status: RestoreStatus =
            serde_json::from_slice(&fs::read(&out_path).expect("read restore status"))
                .expect("decode restore status");
        let status_json: serde_json::Value = serde_json::to_value(&status).expect("encode status");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(status.status_version, 1);
        assert_eq!(status.backup_id.as_str(), "backup-test");
        assert!(status.ready);
        assert!(status.readiness_reasons.is_empty());
        assert_eq!(status.member_count, 2);
        assert_eq!(status.phase_count, 1);
        assert_eq!(status.planned_snapshot_loads, 2);
        assert_eq!(status.planned_code_reinstalls, 2);
        assert_eq!(status.planned_verification_checks, 2);
        assert_eq!(status.phases[0].members[0].source_canister, ROOT);
        assert_eq!(status_json["phases"][0]["members"][0]["state"], "planned");
    }

    // Ensure restore apply dry-run writes ordered operations from plan and status.
    #[test]
    fn run_restore_apply_dry_run_writes_operations() {
        let root = temp_dir("canic-cli-restore-apply-dry-run");
        fs::create_dir_all(&root).expect("create temp root");
        let plan_path = root.join("restore-plan.json");
        let status_path = root.join("restore-status.json");
        let out_path = root.join("restore-apply-dry-run.json");
        let plan = RestorePlanner::plan(&restore_ready_manifest(), None).expect("build plan");
        let status = RestoreStatus::from_plan(&plan);

        fs::write(
            &plan_path,
            serde_json::to_vec(&plan).expect("serialize plan"),
        )
        .expect("write plan");
        fs::write(
            &status_path,
            serde_json::to_vec(&status).expect("serialize status"),
        )
        .expect("write status");

        run([
            OsString::from("apply"),
            OsString::from("--plan"),
            OsString::from(plan_path.as_os_str()),
            OsString::from("--status"),
            OsString::from(status_path.as_os_str()),
            OsString::from("--dry-run"),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
        ])
        .expect("write apply dry-run");

        let dry_run: RestoreApplyDryRun =
            serde_json::from_slice(&fs::read(&out_path).expect("read dry-run"))
                .expect("decode dry-run");
        let dry_run_json: serde_json::Value =
            serde_json::to_value(&dry_run).expect("encode dry-run");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(dry_run.dry_run_version, 1);
        assert_eq!(dry_run.backup_id.as_str(), "backup-test");
        assert!(dry_run.ready);
        assert!(dry_run.status_supplied);
        assert_eq!(dry_run.member_count, 2);
        assert_eq!(dry_run.phase_count, 1);
        assert_eq!(dry_run.rendered_operations, 8);
        assert_eq!(
            dry_run_json["phases"][0]["operations"][0]["operation"],
            "upload-snapshot"
        );
        assert_eq!(
            dry_run_json["phases"][0]["operations"][3]["operation"],
            "verify-member"
        );
        assert_eq!(
            dry_run_json["phases"][0]["operations"][3]["verification_kind"],
            "status"
        );
        assert_eq!(
            dry_run_json["phases"][0]["operations"][3]["verification_method"],
            serde_json::Value::Null
        );
    }

    // Ensure restore apply dry-run rejects status files from another plan.
    #[test]
    fn run_restore_apply_dry_run_rejects_mismatched_status() {
        let root = temp_dir("canic-cli-restore-apply-dry-run-mismatch");
        fs::create_dir_all(&root).expect("create temp root");
        let plan_path = root.join("restore-plan.json");
        let status_path = root.join("restore-status.json");
        let out_path = root.join("restore-apply-dry-run.json");
        let plan = RestorePlanner::plan(&restore_ready_manifest(), None).expect("build plan");
        let mut status = RestoreStatus::from_plan(&plan);
        status.backup_id = "other-backup".to_string();

        fs::write(
            &plan_path,
            serde_json::to_vec(&plan).expect("serialize plan"),
        )
        .expect("write plan");
        fs::write(
            &status_path,
            serde_json::to_vec(&status).expect("serialize status"),
        )
        .expect("write status");

        let err = run([
            OsString::from("apply"),
            OsString::from("--plan"),
            OsString::from(plan_path.as_os_str()),
            OsString::from("--status"),
            OsString::from(status_path.as_os_str()),
            OsString::from("--dry-run"),
            OsString::from("--out"),
            OsString::from(out_path.as_os_str()),
        ])
        .expect_err("mismatched status should fail");

        assert!(!out_path.exists());
        fs::remove_dir_all(root).expect("remove temp root");
        assert!(matches!(
            err,
            RestoreCommandError::RestoreApplyDryRun(RestoreApplyDryRunError::StatusPlanMismatch {
                field: "backup_id",
                ..
            })
        ));
    }

    // Build one valid manifest for restore planning tests.
    fn valid_manifest() -> FleetBackupManifest {
        FleetBackupManifest {
            manifest_version: 1,
            backup_id: "backup-test".to_string(),
            created_at: "2026-05-03T00:00:00Z".to_string(),
            tool: ToolMetadata {
                name: "canic".to_string(),
                version: "0.30.1".to_string(),
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
                    roles: vec!["root".to_string(), "app".to_string()],
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
                members: vec![
                    fleet_member("root", ROOT, None, IdentityMode::Fixed),
                    fleet_member("app", CHILD, Some(ROOT), IdentityMode::Relocatable),
                ],
            },
            verification: VerificationPlan::default(),
        }
    }

    // Build one manifest whose restore readiness metadata is complete.
    fn restore_ready_manifest() -> FleetBackupManifest {
        let mut manifest = valid_manifest();
        for member in &mut manifest.fleet.members {
            member.source_snapshot.module_hash = Some(HASH.to_string());
            member.source_snapshot.wasm_hash = Some(HASH.to_string());
            member.source_snapshot.checksum = Some(HASH.to_string());
        }
        manifest
    }

    // Build one valid manifest member.
    fn fleet_member(
        role: &str,
        canister_id: &str,
        parent_canister_id: Option<&str>,
        identity_mode: IdentityMode,
    ) -> FleetMember {
        FleetMember {
            role: role.to_string(),
            canister_id: canister_id.to_string(),
            parent_canister_id: parent_canister_id.map(str::to_string),
            subnet_canister_id: Some(ROOT.to_string()),
            controller_hint: None,
            identity_mode,
            restore_group: 1,
            verification_class: "basic".to_string(),
            verification_checks: vec![VerificationCheck {
                kind: "status".to_string(),
                method: None,
                roles: vec![role.to_string()],
            }],
            source_snapshot: SourceSnapshot {
                snapshot_id: format!("{role}-snapshot"),
                module_hash: None,
                wasm_hash: None,
                code_version: Some("v0.30.1".to_string()),
                artifact_path: format!("artifacts/{role}"),
                checksum_algorithm: "sha256".to_string(),
                checksum: None,
            },
        }
    }

    // Write a canonical backup layout whose journal checksums match the artifacts.
    fn write_verified_layout(root: &Path, layout: &BackupLayout, manifest: &FleetBackupManifest) {
        layout.write_manifest(manifest).expect("write manifest");

        let artifacts = manifest
            .fleet
            .members
            .iter()
            .map(|member| {
                let bytes = format!("{} artifact", member.role);
                let artifact_path = root.join(&member.source_snapshot.artifact_path);
                if let Some(parent) = artifact_path.parent() {
                    fs::create_dir_all(parent).expect("create artifact parent");
                }
                fs::write(&artifact_path, bytes.as_bytes()).expect("write artifact");
                let checksum = ArtifactChecksum::from_bytes(bytes.as_bytes());

                ArtifactJournalEntry {
                    canister_id: member.canister_id.clone(),
                    snapshot_id: member.source_snapshot.snapshot_id.clone(),
                    state: ArtifactState::Durable,
                    temp_path: None,
                    artifact_path: member.source_snapshot.artifact_path.clone(),
                    checksum_algorithm: checksum.algorithm,
                    checksum: Some(checksum.hash),
                    updated_at: "2026-05-03T00:00:00Z".to_string(),
                }
            })
            .collect();

        layout
            .write_journal(&DownloadJournal {
                journal_version: 1,
                backup_id: manifest.backup_id.clone(),
                discovery_topology_hash: Some(manifest.fleet.discovery_topology_hash.clone()),
                pre_snapshot_topology_hash: Some(manifest.fleet.pre_snapshot_topology_hash.clone()),
                operation_metrics: canic_backup::journal::DownloadOperationMetrics::default(),
                artifacts,
            })
            .expect("write journal");
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
