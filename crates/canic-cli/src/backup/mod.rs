use canic_backup::{
    journal::JournalResumeReport,
    manifest::{BackupUnitKind, ConsistencyMode, FleetBackupManifest},
    persistence::{
        BackupInspectionReport, BackupIntegrityReport, BackupLayout, BackupProvenanceReport,
        PersistenceError,
    },
    restore::{RestoreMapping, RestorePlanError, RestorePlanner},
};
use serde_json::json;
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

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Persistence(#[from] PersistenceError),

    #[error(transparent)]
    RestorePlan(#[from] RestorePlanError),
}

///
/// BackupPreflightOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupPreflightOptions {
    pub dir: PathBuf,
    pub out_dir: PathBuf,
    pub mapping: Option<PathBuf>,
}

impl BackupPreflightOptions {
    /// Parse backup preflight options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut dir = None;
        let mut out_dir = None;
        let mut mapping = None;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| BackupCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--dir" => dir = Some(PathBuf::from(next_value(&mut args, "--dir")?)),
                "--out-dir" => out_dir = Some(PathBuf::from(next_value(&mut args, "--out-dir")?)),
                "--mapping" => mapping = Some(PathBuf::from(next_value(&mut args, "--mapping")?)),
                "--help" | "-h" => return Err(BackupCommandError::Usage(usage())),
                _ => return Err(BackupCommandError::UnknownOption(arg)),
            }
        }

        Ok(Self {
            dir: dir.ok_or(BackupCommandError::MissingOption("--dir"))?,
            out_dir: out_dir.ok_or(BackupCommandError::MissingOption("--out-dir"))?,
            mapping,
        })
    }
}

///
/// BackupPreflightReport
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupPreflightReport {
    pub status: String,
    pub backup_id: String,
    pub backup_dir: String,
    pub source_environment: String,
    pub source_root_canister: String,
    pub topology_hash: String,
    pub mapping_path: Option<String>,
    pub journal_complete: bool,
    pub inspection_status: String,
    pub provenance_status: String,
    pub backup_id_status: String,
    pub topology_receipts_status: String,
    pub topology_mismatch_count: usize,
    pub integrity_verified: bool,
    pub manifest_members: usize,
    pub backup_unit_count: usize,
    pub restore_plan_members: usize,
    pub restore_fixed_members: usize,
    pub restore_relocatable_members: usize,
    pub restore_in_place_members: usize,
    pub restore_mapped_members: usize,
    pub restore_remapped_members: usize,
    pub restore_fleet_checks: usize,
    pub restore_member_check_groups: usize,
    pub restore_member_checks: usize,
    pub restore_members_with_checks: usize,
    pub restore_total_checks: usize,
    pub restore_phase_count: usize,
    pub restore_dependency_free_members: usize,
    pub restore_in_group_parent_edges: usize,
    pub restore_cross_group_parent_edges: usize,
    pub manifest_validation_path: String,
    pub backup_status_path: String,
    pub backup_inspection_path: String,
    pub backup_provenance_path: String,
    pub backup_integrity_path: String,
    pub restore_plan_path: String,
    pub preflight_summary_path: String,
}

///
/// BackupInspectOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupInspectOptions {
    pub dir: PathBuf,
    pub out: Option<PathBuf>,
    pub require_ready: bool,
}

impl BackupInspectOptions {
    /// Parse backup inspection options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut dir = None;
        let mut out = None;
        let mut require_ready = false;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| BackupCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--dir" => dir = Some(PathBuf::from(next_value(&mut args, "--dir")?)),
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--require-ready" => require_ready = true,
                "--help" | "-h" => return Err(BackupCommandError::Usage(usage())),
                _ => return Err(BackupCommandError::UnknownOption(arg)),
            }
        }

        Ok(Self {
            dir: dir.ok_or(BackupCommandError::MissingOption("--dir"))?,
            out,
            require_ready,
        })
    }
}

///
/// BackupProvenanceOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupProvenanceOptions {
    pub dir: PathBuf,
    pub out: Option<PathBuf>,
    pub require_consistent: bool,
}

impl BackupProvenanceOptions {
    /// Parse backup provenance options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut dir = None;
        let mut out = None;
        let mut require_consistent = false;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| BackupCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--dir" => dir = Some(PathBuf::from(next_value(&mut args, "--dir")?)),
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--require-consistent" => require_consistent = true,
                "--help" | "-h" => return Err(BackupCommandError::Usage(usage())),
                _ => return Err(BackupCommandError::UnknownOption(arg)),
            }
        }

        Ok(Self {
            dir: dir.ok_or(BackupCommandError::MissingOption("--dir"))?,
            out,
            require_consistent,
        })
    }
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
        "preflight" => {
            let options = BackupPreflightOptions::parse(args)?;
            backup_preflight(&options)?;
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
        "help" | "--help" | "-h" => Err(BackupCommandError::Usage(usage())),
        _ => Err(BackupCommandError::UnknownOption(command)),
    }
}

/// Run all no-mutation backup checks and write standard preflight artifacts.
pub fn backup_preflight(
    options: &BackupPreflightOptions,
) -> Result<BackupPreflightReport, BackupCommandError> {
    fs::create_dir_all(&options.out_dir)?;

    let layout = BackupLayout::new(options.dir.clone());
    let manifest = layout.read_manifest()?;
    let status = layout.read_journal()?.resume_report();
    ensure_complete_status(&status)?;
    let inspection = layout.inspect()?;
    let provenance = layout.provenance()?;
    let integrity = layout.verify_integrity()?;
    let mapping = options.mapping.as_ref().map(read_mapping).transpose()?;
    let restore_plan = RestorePlanner::plan(&manifest, mapping.as_ref())?;

    let manifest_validation_path = options.out_dir.join("manifest-validation.json");
    let backup_status_path = options.out_dir.join("backup-status.json");
    let backup_inspection_path = options.out_dir.join("backup-inspection.json");
    let backup_provenance_path = options.out_dir.join("backup-provenance.json");
    let backup_integrity_path = options.out_dir.join("backup-integrity.json");
    let restore_plan_path = options.out_dir.join("restore-plan.json");
    let preflight_summary_path = options.out_dir.join("preflight-summary.json");

    write_json_value_file(
        &manifest_validation_path,
        &manifest_validation_summary(&manifest),
    )?;
    fs::write(&backup_status_path, serde_json::to_vec_pretty(&status)?)?;
    fs::write(
        &backup_inspection_path,
        serde_json::to_vec_pretty(&inspection)?,
    )?;
    fs::write(
        &backup_provenance_path,
        serde_json::to_vec_pretty(&provenance)?,
    )?;
    fs::write(
        &backup_integrity_path,
        serde_json::to_vec_pretty(&integrity)?,
    )?;
    fs::write(
        &restore_plan_path,
        serde_json::to_vec_pretty(&restore_plan)?,
    )?;

    let report = BackupPreflightReport {
        status: "ready".to_string(),
        backup_id: manifest.backup_id.clone(),
        backup_dir: options.dir.display().to_string(),
        source_environment: manifest.source.environment.clone(),
        source_root_canister: manifest.source.root_canister.clone(),
        topology_hash: manifest.fleet.topology_hash.clone(),
        mapping_path: options
            .mapping
            .as_ref()
            .map(|path| path.display().to_string()),
        journal_complete: status.is_complete,
        inspection_status: readiness_status(inspection.ready_for_verify).to_string(),
        provenance_status: consistency_status(
            provenance.backup_id_matches && provenance.topology_receipts_match,
        )
        .to_string(),
        backup_id_status: match_status(provenance.backup_id_matches).to_string(),
        topology_receipts_status: match_status(provenance.topology_receipts_match).to_string(),
        topology_mismatch_count: provenance.topology_receipt_mismatches.len(),
        integrity_verified: integrity.verified,
        manifest_members: manifest.fleet.members.len(),
        backup_unit_count: provenance.backup_unit_count,
        restore_plan_members: restore_plan.member_count,
        restore_fixed_members: restore_plan.identity_summary.fixed_members,
        restore_relocatable_members: restore_plan.identity_summary.relocatable_members,
        restore_in_place_members: restore_plan.identity_summary.in_place_members,
        restore_mapped_members: restore_plan.identity_summary.mapped_members,
        restore_remapped_members: restore_plan.identity_summary.remapped_members,
        restore_fleet_checks: restore_plan.verification_summary.fleet_checks,
        restore_member_check_groups: restore_plan.verification_summary.member_check_groups,
        restore_member_checks: restore_plan.verification_summary.member_checks,
        restore_members_with_checks: restore_plan.verification_summary.members_with_checks,
        restore_total_checks: restore_plan.verification_summary.total_checks,
        restore_phase_count: restore_plan.ordering_summary.phase_count,
        restore_dependency_free_members: restore_plan.ordering_summary.dependency_free_members,
        restore_in_group_parent_edges: restore_plan.ordering_summary.in_group_parent_edges,
        restore_cross_group_parent_edges: restore_plan.ordering_summary.cross_group_parent_edges,
        manifest_validation_path: manifest_validation_path.display().to_string(),
        backup_status_path: backup_status_path.display().to_string(),
        backup_inspection_path: backup_inspection_path.display().to_string(),
        backup_provenance_path: backup_provenance_path.display().to_string(),
        backup_integrity_path: backup_integrity_path.display().to_string(),
        restore_plan_path: restore_plan_path.display().to_string(),
        preflight_summary_path: preflight_summary_path.display().to_string(),
    };

    write_json_value_file(&preflight_summary_path, &preflight_summary_value(&report))?;
    Ok(report)
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

/// Summarize a backup journal's resumable state.
pub fn backup_status(
    options: &BackupStatusOptions,
) -> Result<JournalResumeReport, BackupCommandError> {
    let layout = BackupLayout::new(options.dir.clone());
    let journal = layout.read_journal()?;
    Ok(journal.resume_report())
}

// Ensure a journal status report has no remaining resume work.
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

// Write the inspection report to stdout or a requested output file.
fn write_inspect_report(
    options: &BackupInspectOptions,
    report: &BackupInspectionReport,
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

// Write the provenance report to stdout or a requested output file.
fn write_provenance_report(
    options: &BackupProvenanceOptions,
    report: &BackupProvenanceReport,
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

// Write one pretty JSON value artifact, creating its parent directory when needed.
fn write_json_value_file(
    path: &PathBuf,
    value: &serde_json::Value,
) -> Result<(), BackupCommandError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let data = serde_json::to_vec_pretty(value)?;
    fs::write(path, data)?;
    Ok(())
}

// Build the compact preflight summary emitted after all checks pass.
fn preflight_summary_value(report: &BackupPreflightReport) -> serde_json::Value {
    json!({
        "status": report.status,
        "backup_id": report.backup_id,
        "backup_dir": report.backup_dir,
        "source_environment": report.source_environment,
        "source_root_canister": report.source_root_canister,
        "topology_hash": report.topology_hash,
        "mapping_path": report.mapping_path,
        "journal_complete": report.journal_complete,
        "inspection_status": report.inspection_status,
        "provenance_status": report.provenance_status,
        "backup_id_status": report.backup_id_status,
        "topology_receipts_status": report.topology_receipts_status,
        "topology_mismatch_count": report.topology_mismatch_count,
        "integrity_verified": report.integrity_verified,
        "manifest_members": report.manifest_members,
        "backup_unit_count": report.backup_unit_count,
        "restore_plan_members": report.restore_plan_members,
        "restore_fixed_members": report.restore_fixed_members,
        "restore_relocatable_members": report.restore_relocatable_members,
        "restore_in_place_members": report.restore_in_place_members,
        "restore_mapped_members": report.restore_mapped_members,
        "restore_remapped_members": report.restore_remapped_members,
        "restore_fleet_checks": report.restore_fleet_checks,
        "restore_member_check_groups": report.restore_member_check_groups,
        "restore_member_checks": report.restore_member_checks,
        "restore_members_with_checks": report.restore_members_with_checks,
        "restore_total_checks": report.restore_total_checks,
        "restore_phase_count": report.restore_phase_count,
        "restore_dependency_free_members": report.restore_dependency_free_members,
        "restore_in_group_parent_edges": report.restore_in_group_parent_edges,
        "restore_cross_group_parent_edges": report.restore_cross_group_parent_edges,
        "manifest_validation_path": report.manifest_validation_path,
        "backup_status_path": report.backup_status_path,
        "backup_inspection_path": report.backup_inspection_path,
        "backup_provenance_path": report.backup_provenance_path,
        "backup_integrity_path": report.backup_integrity_path,
        "restore_plan_path": report.restore_plan_path,
        "preflight_summary_path": report.preflight_summary_path,
    })
}

// Build the same compact validation summary emitted by manifest validation.
fn manifest_validation_summary(manifest: &FleetBackupManifest) -> serde_json::Value {
    json!({
        "status": "valid",
        "backup_id": manifest.backup_id,
        "members": manifest.fleet.members.len(),
        "backup_unit_count": manifest.consistency.backup_units.len(),
        "consistency_mode": consistency_mode_name(&manifest.consistency.mode),
        "topology_hash": manifest.fleet.topology_hash,
        "topology_hash_algorithm": manifest.fleet.topology_hash_algorithm,
        "topology_hash_input": manifest.fleet.topology_hash_input,
        "topology_validation_status": "validated",
        "backup_unit_kinds": backup_unit_kind_counts(manifest),
        "backup_units": manifest
            .consistency
            .backup_units
            .iter()
            .map(|unit| json!({
                "unit_id": unit.unit_id,
                "kind": backup_unit_kind_name(&unit.kind),
                "role_count": unit.roles.len(),
                "dependency_count": unit.dependency_closure.len(),
                "topology_validation": unit.topology_validation,
            }))
            .collect::<Vec<_>>(),
    })
}

// Count backup units by stable serialized kind name.
fn backup_unit_kind_counts(manifest: &FleetBackupManifest) -> serde_json::Value {
    let mut whole_fleet = 0;
    let mut control_plane_subset = 0;
    let mut subtree_rooted = 0;
    let mut flat = 0;
    for unit in &manifest.consistency.backup_units {
        match &unit.kind {
            BackupUnitKind::WholeFleet => whole_fleet += 1,
            BackupUnitKind::ControlPlaneSubset => control_plane_subset += 1,
            BackupUnitKind::SubtreeRooted => subtree_rooted += 1,
            BackupUnitKind::Flat => flat += 1,
        }
    }

    json!({
        "whole_fleet": whole_fleet,
        "control_plane_subset": control_plane_subset,
        "subtree_rooted": subtree_rooted,
        "flat": flat,
    })
}

// Return the stable serialized name for a consistency mode.
const fn consistency_mode_name(mode: &ConsistencyMode) -> &'static str {
    match mode {
        ConsistencyMode::CrashConsistent => "crash-consistent",
        ConsistencyMode::QuiescedUnit => "quiesced-unit",
    }
}

// Return the stable serialized name for a backup unit kind.
const fn backup_unit_kind_name(kind: &BackupUnitKind) -> &'static str {
    match kind {
        BackupUnitKind::WholeFleet => "whole-fleet",
        BackupUnitKind::ControlPlaneSubset => "control-plane-subset",
        BackupUnitKind::SubtreeRooted => "subtree-rooted",
        BackupUnitKind::Flat => "flat",
    }
}

// Return the stable summary status for inspection readiness.
const fn readiness_status(ready: bool) -> &'static str {
    if ready { "ready" } else { "not-ready" }
}

// Return the stable summary status for provenance consistency.
const fn consistency_status(consistent: bool) -> &'static str {
    if consistent {
        "consistent"
    } else {
        "inconsistent"
    }
}

// Return the stable summary status for equality checks.
const fn match_status(matches: bool) -> &'static str {
    if matches { "matched" } else { "mismatched" }
}

// Read and decode an optional source-to-target restore mapping from disk.
fn read_mapping(path: &PathBuf) -> Result<RestoreMapping, BackupCommandError> {
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(BackupCommandError::from)
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
    "usage: canic backup preflight --dir <backup-dir> --out-dir <dir> [--mapping <file>]\n       canic backup inspect --dir <backup-dir> [--out <file>] [--require-ready]\n       canic backup provenance --dir <backup-dir> [--out <file>] [--require-consistent]\n       canic backup status --dir <backup-dir> [--out <file>] [--require-complete]\n       canic backup verify --dir <backup-dir> [--out <file>]"
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

    // Ensure backup preflight options parse the intended command shape.
    #[test]
    fn parses_backup_preflight_options() {
        let options = BackupPreflightOptions::parse([
            OsString::from("--dir"),
            OsString::from("backups/run"),
            OsString::from("--out-dir"),
            OsString::from("reports/run"),
            OsString::from("--mapping"),
            OsString::from("mapping.json"),
        ])
        .expect("parse options");

        assert_eq!(options.dir, PathBuf::from("backups/run"));
        assert_eq!(options.out_dir, PathBuf::from("reports/run"));
        assert_eq!(options.mapping, Some(PathBuf::from("mapping.json")));
    }

    // Ensure preflight writes the standard no-mutation report bundle.
    #[test]
    fn backup_preflight_writes_standard_reports() {
        let root = temp_dir("canic-cli-backup-preflight");
        let out_dir = root.join("reports");
        let backup_dir = root.join("backup");
        let layout = BackupLayout::new(backup_dir.clone());
        let checksum = write_artifact(&backup_dir, b"root artifact");

        layout
            .write_manifest(&valid_manifest())
            .expect("write manifest");
        layout
            .write_journal(&journal_with_checksum(checksum.hash))
            .expect("write journal");

        let options = BackupPreflightOptions {
            dir: backup_dir,
            out_dir: out_dir.clone(),
            mapping: None,
        };
        let report = backup_preflight(&options).expect("run preflight");

        assert_eq!(report.status, "ready");
        assert_eq!(report.backup_id, "backup-test");
        assert_eq!(report.source_environment, "local");
        assert_eq!(report.source_root_canister, ROOT);
        assert_eq!(report.topology_hash, HASH);
        assert_eq!(report.mapping_path, None);
        assert!(report.journal_complete);
        assert_eq!(report.inspection_status, "ready");
        assert_eq!(report.provenance_status, "consistent");
        assert_eq!(report.backup_id_status, "matched");
        assert_eq!(report.topology_receipts_status, "matched");
        assert_eq!(report.topology_mismatch_count, 0);
        assert!(report.integrity_verified);
        assert_eq!(report.manifest_members, 1);
        assert_eq!(report.backup_unit_count, 1);
        assert_eq!(report.restore_plan_members, 1);
        assert_preflight_report_restore_counts(&report);
        assert!(out_dir.join("manifest-validation.json").exists());
        assert!(out_dir.join("backup-status.json").exists());
        assert!(out_dir.join("backup-inspection.json").exists());
        assert!(out_dir.join("backup-provenance.json").exists());
        assert!(out_dir.join("backup-integrity.json").exists());
        assert!(out_dir.join("restore-plan.json").exists());
        assert!(out_dir.join("preflight-summary.json").exists());

        let summary: serde_json::Value = serde_json::from_slice(
            &fs::read(out_dir.join("preflight-summary.json")).expect("read summary"),
        )
        .expect("decode summary");
        let manifest_validation: serde_json::Value = serde_json::from_slice(
            &fs::read(out_dir.join("manifest-validation.json")).expect("read manifest summary"),
        )
        .expect("decode manifest summary");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_preflight_summary_matches_report(&summary, &report);
        assert_eq!(manifest_validation["backup_unit_count"], 1);
        assert_eq!(manifest_validation["consistency_mode"], "crash-consistent");
        assert_eq!(
            manifest_validation["topology_validation_status"],
            "validated"
        );
        assert_eq!(
            manifest_validation["backup_unit_kinds"]["subtree_rooted"],
            1
        );
        assert_eq!(
            manifest_validation["backup_units"][0]["kind"],
            "subtree-rooted"
        );
    }

    // Verify restore summary counts copied out of the generated restore plan.
    fn assert_preflight_report_restore_counts(report: &BackupPreflightReport) {
        assert_eq!(report.restore_fixed_members, 1);
        assert_eq!(report.restore_relocatable_members, 0);
        assert_eq!(report.restore_in_place_members, 1);
        assert_eq!(report.restore_mapped_members, 0);
        assert_eq!(report.restore_remapped_members, 0);
        assert_eq!(report.restore_fleet_checks, 0);
        assert_eq!(report.restore_member_check_groups, 0);
        assert_eq!(report.restore_member_checks, 1);
        assert_eq!(report.restore_members_with_checks, 1);
        assert_eq!(report.restore_total_checks, 1);
        assert_eq!(report.restore_phase_count, 1);
        assert_eq!(report.restore_dependency_free_members, 1);
        assert_eq!(report.restore_in_group_parent_edges, 0);
        assert_eq!(report.restore_cross_group_parent_edges, 0);
    }

    // Compare preflight summary JSON with the in-memory report.
    fn assert_preflight_summary_matches_report(
        summary: &serde_json::Value,
        report: &BackupPreflightReport,
    ) {
        assert_eq!(summary["status"], report.status);
        assert_eq!(summary["backup_id"], report.backup_id);
        assert_eq!(summary["source_environment"], report.source_environment);
        assert_eq!(summary["source_root_canister"], report.source_root_canister);
        assert_eq!(summary["topology_hash"], report.topology_hash);
        assert_eq!(summary["journal_complete"], report.journal_complete);
        assert_eq!(summary["inspection_status"], report.inspection_status);
        assert_eq!(summary["provenance_status"], report.provenance_status);
        assert_eq!(summary["backup_id_status"], report.backup_id_status);
        assert_eq!(
            summary["topology_receipts_status"],
            report.topology_receipts_status
        );
        assert_eq!(
            summary["topology_mismatch_count"],
            report.topology_mismatch_count
        );
        assert_eq!(summary["integrity_verified"], report.integrity_verified);
        assert_eq!(summary["manifest_members"], report.manifest_members);
        assert_eq!(summary["backup_unit_count"], report.backup_unit_count);
        assert_eq!(summary["restore_plan_members"], report.restore_plan_members);
        assert_eq!(
            summary["restore_fixed_members"],
            report.restore_fixed_members
        );
        assert_eq!(
            summary["restore_relocatable_members"],
            report.restore_relocatable_members
        );
        assert_eq!(
            summary["restore_in_place_members"],
            report.restore_in_place_members
        );
        assert_eq!(
            summary["restore_mapped_members"],
            report.restore_mapped_members
        );
        assert_eq!(
            summary["restore_remapped_members"],
            report.restore_remapped_members
        );
        assert_eq!(summary["restore_fleet_checks"], report.restore_fleet_checks);
        assert_eq!(
            summary["restore_member_check_groups"],
            report.restore_member_check_groups
        );
        assert_eq!(
            summary["restore_member_checks"],
            report.restore_member_checks
        );
        assert_eq!(
            summary["restore_members_with_checks"],
            report.restore_members_with_checks
        );
        assert_eq!(summary["restore_total_checks"], report.restore_total_checks);
        assert_eq!(summary["restore_phase_count"], report.restore_phase_count);
        assert_eq!(
            summary["restore_dependency_free_members"],
            report.restore_dependency_free_members
        );
        assert_eq!(
            summary["restore_in_group_parent_edges"],
            report.restore_in_group_parent_edges
        );
        assert_eq!(
            summary["restore_cross_group_parent_edges"],
            report.restore_cross_group_parent_edges
        );
        assert_eq!(
            summary["backup_inspection_path"],
            report.backup_inspection_path
        );
        assert_eq!(
            summary["backup_provenance_path"],
            report.backup_provenance_path
        );
    }

    // Ensure preflight stops on incomplete journals before claiming readiness.
    #[test]
    fn backup_preflight_rejects_incomplete_journal() {
        let root = temp_dir("canic-cli-backup-preflight-incomplete");
        let out_dir = root.join("reports");
        let backup_dir = root.join("backup");
        let layout = BackupLayout::new(backup_dir.clone());

        layout
            .write_manifest(&valid_manifest())
            .expect("write manifest");
        layout
            .write_journal(&created_journal())
            .expect("write journal");

        let options = BackupPreflightOptions {
            dir: backup_dir,
            out_dir,
            mapping: None,
        };

        let err = backup_preflight(&options).expect_err("incomplete journal should fail");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(matches!(
            err,
            BackupCommandError::IncompleteJournal {
                pending_artifacts: 1,
                total_artifacts: 1,
                ..
            }
        ));
    }

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

    // Ensure backup inspection options parse the intended command shape.
    #[test]
    fn parses_backup_inspect_options() {
        let options = BackupInspectOptions::parse([
            OsString::from("--dir"),
            OsString::from("backups/run"),
            OsString::from("--out"),
            OsString::from("inspect.json"),
            OsString::from("--require-ready"),
        ])
        .expect("parse options");

        assert_eq!(options.dir, PathBuf::from("backups/run"));
        assert_eq!(options.out, Some(PathBuf::from("inspect.json")));
        assert!(options.require_ready);
    }

    // Ensure backup provenance options parse the intended command shape.
    #[test]
    fn parses_backup_provenance_options() {
        let options = BackupProvenanceOptions::parse([
            OsString::from("--dir"),
            OsString::from("backups/run"),
            OsString::from("--out"),
            OsString::from("provenance.json"),
            OsString::from("--require-consistent"),
        ])
        .expect("parse options");

        assert_eq!(options.dir, PathBuf::from("backups/run"));
        assert_eq!(options.out, Some(PathBuf::from("provenance.json")));
        assert!(options.require_consistent);
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

    // Ensure backup inspection reports manifest and journal agreement.
    #[test]
    fn inspect_backup_reads_layout_metadata() {
        let root = temp_dir("canic-cli-backup-inspect");
        let layout = BackupLayout::new(root.clone());

        layout
            .write_manifest(&valid_manifest())
            .expect("write manifest");
        layout
            .write_journal(&journal_with_checksum(HASH.to_string()))
            .expect("write journal");

        let options = BackupInspectOptions {
            dir: root.clone(),
            out: None,
            require_ready: false,
        };
        let report = inspect_backup(&options).expect("inspect backup");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(report.backup_id, "backup-test");
        assert!(report.backup_id_matches);
        assert!(report.journal_complete);
        assert!(report.ready_for_verify);
        assert!(report.topology_receipt_mismatches.is_empty());
        assert_eq!(report.matched_artifacts, 1);
    }

    // Ensure backup provenance reports manifest and journal audit metadata.
    #[test]
    fn backup_provenance_reads_layout_metadata() {
        let root = temp_dir("canic-cli-backup-provenance");
        let layout = BackupLayout::new(root.clone());

        layout
            .write_manifest(&valid_manifest())
            .expect("write manifest");
        layout
            .write_journal(&journal_with_checksum(HASH.to_string()))
            .expect("write journal");

        let options = BackupProvenanceOptions {
            dir: root.clone(),
            out: None,
            require_consistent: false,
        };
        let report = backup_provenance(&options).expect("read provenance");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(report.backup_id, "backup-test");
        assert!(report.backup_id_matches);
        assert_eq!(report.source_environment, "local");
        assert_eq!(report.discovery_topology_hash, HASH);
        assert!(report.topology_receipts_match);
        assert!(report.topology_receipt_mismatches.is_empty());
        assert_eq!(report.backup_unit_count, 1);
        assert_eq!(report.member_count, 1);
        assert_eq!(report.backup_units[0].kind, "subtree-rooted");
        assert_eq!(report.members[0].canister_id, ROOT);
        assert_eq!(report.members[0].snapshot_id, "root-snapshot");
        assert_eq!(report.members[0].journal_state, Some("Durable".to_string()));
    }

    // Ensure require-consistent accepts matching provenance reports.
    #[test]
    fn require_consistent_accepts_matching_provenance() {
        let options = BackupProvenanceOptions {
            dir: PathBuf::from("unused"),
            out: None,
            require_consistent: true,
        };
        let report = ready_provenance_report();

        enforce_provenance_requirements(&options, &report)
            .expect("matching provenance should pass");
    }

    // Ensure require-consistent rejects backup ID or topology receipt drift.
    #[test]
    fn require_consistent_rejects_provenance_drift() {
        let options = BackupProvenanceOptions {
            dir: PathBuf::from("unused"),
            out: None,
            require_consistent: true,
        };
        let mut report = ready_provenance_report();
        report.backup_id_matches = false;
        report.journal_backup_id = "other-backup".to_string();
        report.topology_receipts_match = false;
        report.topology_receipt_mismatches.push(
            canic_backup::persistence::TopologyReceiptMismatch {
                field: "pre_snapshot_topology_hash".to_string(),
                manifest: HASH.to_string(),
                journal: None,
            },
        );

        let err = enforce_provenance_requirements(&options, &report)
            .expect_err("provenance drift should fail");

        assert!(matches!(
            err,
            BackupCommandError::ProvenanceNotConsistent {
                backup_id_matches: false,
                topology_receipts_match: false,
                topology_mismatches: 1,
                ..
            }
        ));
    }

    // Ensure require-ready accepts inspection reports ready for verification.
    #[test]
    fn require_ready_accepts_ready_inspection() {
        let options = BackupInspectOptions {
            dir: PathBuf::from("unused"),
            out: None,
            require_ready: true,
        };
        let report = ready_inspection_report();

        enforce_inspection_requirements(&options, &report).expect("ready inspection should pass");
    }

    // Ensure require-ready rejects inspection reports with metadata drift.
    #[test]
    fn require_ready_rejects_unready_inspection() {
        let options = BackupInspectOptions {
            dir: PathBuf::from("unused"),
            out: None,
            require_ready: true,
        };
        let mut report = ready_inspection_report();
        report.ready_for_verify = false;
        report
            .path_mismatches
            .push(canic_backup::persistence::ArtifactPathMismatch {
                canister_id: ROOT.to_string(),
                snapshot_id: "root-snapshot".to_string(),
                manifest: "artifacts/root".to_string(),
                journal: "artifacts/other-root".to_string(),
            });

        let err = enforce_inspection_requirements(&options, &report)
            .expect_err("unready inspection should fail");

        assert!(matches!(
            err,
            BackupCommandError::InspectionNotReady {
                path_mismatches: 1,
                ..
            }
        ));
    }

    // Ensure require-ready rejects topology receipt drift.
    #[test]
    fn require_ready_rejects_topology_receipt_drift() {
        let options = BackupInspectOptions {
            dir: PathBuf::from("unused"),
            out: None,
            require_ready: true,
        };
        let mut report = ready_inspection_report();
        report.ready_for_verify = false;
        report.topology_receipt_mismatches.push(
            canic_backup::persistence::TopologyReceiptMismatch {
                field: "discovery_topology_hash".to_string(),
                manifest: HASH.to_string(),
                journal: None,
            },
        );

        let err = enforce_inspection_requirements(&options, &report)
            .expect_err("topology receipt drift should fail");

        assert!(matches!(
            err,
            BackupCommandError::InspectionNotReady {
                topology_receipts_match: false,
                topology_mismatches: 1,
                ..
            }
        ));
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
                checksum: None,
            },
        }
    }

    // Build one durable journal with a caller-provided checksum.
    fn journal_with_checksum(checksum: String) -> DownloadJournal {
        DownloadJournal {
            journal_version: 1,
            backup_id: "backup-test".to_string(),
            discovery_topology_hash: Some(HASH.to_string()),
            pre_snapshot_topology_hash: Some(HASH.to_string()),
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
            discovery_topology_hash: Some(HASH.to_string()),
            pre_snapshot_topology_hash: Some(HASH.to_string()),
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

    // Build one ready inspection report for requirement tests.
    fn ready_inspection_report() -> BackupInspectionReport {
        BackupInspectionReport {
            backup_id: "backup-test".to_string(),
            manifest_backup_id: "backup-test".to_string(),
            journal_backup_id: "backup-test".to_string(),
            backup_id_matches: true,
            journal_complete: true,
            ready_for_verify: true,
            manifest_members: 1,
            journal_artifacts: 1,
            matched_artifacts: 1,
            topology_receipt_mismatches: Vec::new(),
            missing_journal_artifacts: Vec::new(),
            unexpected_journal_artifacts: Vec::new(),
            path_mismatches: Vec::new(),
            checksum_mismatches: Vec::new(),
        }
    }

    // Build one matching provenance report for requirement tests.
    fn ready_provenance_report() -> BackupProvenanceReport {
        BackupProvenanceReport {
            backup_id: "backup-test".to_string(),
            manifest_backup_id: "backup-test".to_string(),
            journal_backup_id: "backup-test".to_string(),
            backup_id_matches: true,
            manifest_version: 1,
            journal_version: 1,
            created_at: "2026-05-03T00:00:00Z".to_string(),
            tool_name: "canic".to_string(),
            tool_version: "0.30.12".to_string(),
            source_environment: "local".to_string(),
            source_root_canister: ROOT.to_string(),
            topology_hash_algorithm: "sha256".to_string(),
            topology_hash_input: "sorted(pid,parent_pid,role,module_hash)".to_string(),
            discovery_topology_hash: HASH.to_string(),
            pre_snapshot_topology_hash: HASH.to_string(),
            accepted_topology_hash: HASH.to_string(),
            journal_discovery_topology_hash: Some(HASH.to_string()),
            journal_pre_snapshot_topology_hash: Some(HASH.to_string()),
            topology_receipts_match: true,
            topology_receipt_mismatches: Vec::new(),
            backup_unit_count: 1,
            member_count: 1,
            consistency_mode: "crash-consistent".to_string(),
            backup_units: Vec::new(),
            members: Vec::new(),
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
