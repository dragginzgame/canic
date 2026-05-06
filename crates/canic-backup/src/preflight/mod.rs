use crate::{
    journal::{DownloadOperationMetrics, JournalResumeReport},
    manifest::{FleetBackupManifest, manifest_validation_summary},
    persistence::{
        BackupInspectionReport, BackupIntegrityReport, BackupLayout, BackupProvenanceReport,
        PersistenceError,
    },
    restore::{RestoreMapping, RestorePlan, RestorePlanError, RestorePlanner, RestoreStatus},
};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

///
/// BackupPreflightConfig
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupPreflightConfig {
    pub backup_dir: PathBuf,
    pub out_dir: PathBuf,
    pub mapping: Option<PathBuf>,
}

///
/// BackupPreflightReport
///

#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "preflight reports intentionally mirror machine-readable JSON status flags"
)]
pub struct BackupPreflightReport {
    pub status: String,
    pub backup_id: String,
    pub backup_dir: String,
    pub source_environment: String,
    pub source_root_canister: String,
    pub topology_hash: String,
    pub mapping_path: Option<String>,
    pub journal_complete: bool,
    pub journal_operation_metrics: DownloadOperationMetrics,
    pub inspection_status: String,
    pub provenance_status: String,
    pub backup_id_status: String,
    pub topology_receipts_status: String,
    pub topology_mismatch_count: usize,
    pub integrity_verified: bool,
    pub manifest_design_v1_ready: bool,
    pub manifest_members: usize,
    pub backup_unit_count: usize,
    pub restore_plan_members: usize,
    pub restore_mapping_supplied: bool,
    pub restore_all_sources_mapped: bool,
    pub restore_fixed_members: usize,
    pub restore_relocatable_members: usize,
    pub restore_in_place_members: usize,
    pub restore_mapped_members: usize,
    pub restore_remapped_members: usize,
    pub restore_ready: bool,
    pub restore_readiness_reasons: Vec<String>,
    pub restore_all_members_have_module_hash: bool,
    pub restore_all_members_have_wasm_hash: bool,
    pub restore_all_members_have_code_version: bool,
    pub restore_all_members_have_checksum: bool,
    pub restore_members_with_module_hash: usize,
    pub restore_members_with_wasm_hash: usize,
    pub restore_members_with_code_version: usize,
    pub restore_members_with_checksum: usize,
    pub restore_verification_required: bool,
    pub restore_all_members_have_checks: bool,
    pub restore_fleet_checks: usize,
    pub restore_member_check_groups: usize,
    pub restore_member_checks: usize,
    pub restore_members_with_checks: usize,
    pub restore_total_checks: usize,
    pub restore_planned_snapshot_uploads: usize,
    pub restore_planned_snapshot_loads: usize,
    pub restore_planned_code_reinstalls: usize,
    pub restore_planned_verification_checks: usize,
    pub restore_planned_operations: usize,
    pub restore_planned_phases: usize,
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
    pub restore_status_path: String,
    pub preflight_summary_path: String,
}

///
/// BackupPreflightError
///

#[derive(Debug, ThisError)]
pub enum BackupPreflightError {
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

    #[error(transparent)]
    RestorePlan(#[from] RestorePlanError),
}

///
/// PreflightArtifactPaths
///

struct PreflightArtifactPaths {
    manifest_validation: PathBuf,
    backup_status: PathBuf,
    backup_inspection: PathBuf,
    backup_provenance: PathBuf,
    backup_integrity: PathBuf,
    restore_plan: PathBuf,
    restore_status: PathBuf,
    preflight_summary: PathBuf,
}

///
/// PreflightReportInput
///

struct PreflightReportInput<'a> {
    config: &'a BackupPreflightConfig,
    manifest: &'a FleetBackupManifest,
    status: &'a JournalResumeReport,
    inspection: &'a BackupInspectionReport,
    provenance: &'a BackupProvenanceReport,
    integrity: &'a BackupIntegrityReport,
    restore_plan: &'a RestorePlan,
    paths: &'a PreflightArtifactPaths,
}

///
/// PreflightArtifactInput
///

struct PreflightArtifactInput<'a> {
    paths: &'a PreflightArtifactPaths,
    manifest: &'a FleetBackupManifest,
    status: &'a JournalResumeReport,
    inspection: &'a BackupInspectionReport,
    provenance: &'a BackupProvenanceReport,
    integrity: &'a BackupIntegrityReport,
    restore_plan: &'a RestorePlan,
    restore_status: &'a RestoreStatus,
}

/// Run all no-mutation backup checks and write the standard preflight bundle.
pub fn run_backup_preflight(
    config: &BackupPreflightConfig,
) -> Result<BackupPreflightReport, BackupPreflightError> {
    fs::create_dir_all(&config.out_dir)?;

    let layout = BackupLayout::new(config.backup_dir.clone());
    let manifest = layout.read_manifest()?;
    let status = layout.read_journal()?.resume_report();
    ensure_complete_status(&status)?;
    let inspection = layout.inspect()?;
    let provenance = layout.provenance()?;
    let integrity = layout.verify_integrity()?;
    let mapping = config.mapping.as_ref().map(read_mapping).transpose()?;
    let restore_plan = RestorePlanner::plan(&manifest, mapping.as_ref())?;
    let restore_status = RestoreStatus::from_plan(&restore_plan);
    let paths = preflight_artifact_paths(&config.out_dir);

    write_preflight_artifacts(PreflightArtifactInput {
        paths: &paths,
        manifest: &manifest,
        status: &status,
        inspection: &inspection,
        provenance: &provenance,
        integrity: &integrity,
        restore_plan: &restore_plan,
        restore_status: &restore_status,
    })?;
    let report = build_preflight_report(PreflightReportInput {
        config,
        manifest: &manifest,
        status: &status,
        inspection: &inspection,
        provenance: &provenance,
        integrity: &integrity,
        restore_plan: &restore_plan,
        paths: &paths,
    });
    write_json_value_file(&paths.preflight_summary, &preflight_summary_value(&report))?;
    Ok(report)
}

// Ensure a journal status report has no remaining resume work.
fn ensure_complete_status(report: &JournalResumeReport) -> Result<(), BackupPreflightError> {
    if report.is_complete {
        return Ok(());
    }

    Err(BackupPreflightError::IncompleteJournal {
        backup_id: report.backup_id.clone(),
        total_artifacts: report.total_artifacts,
        pending_artifacts: report.pending_artifacts,
    })
}

// Build the standard preflight artifact path set under one output directory.
fn preflight_artifact_paths(out_dir: &Path) -> PreflightArtifactPaths {
    PreflightArtifactPaths {
        manifest_validation: out_dir.join("manifest-validation.json"),
        backup_status: out_dir.join("backup-status.json"),
        backup_inspection: out_dir.join("backup-inspection.json"),
        backup_provenance: out_dir.join("backup-provenance.json"),
        backup_integrity: out_dir.join("backup-integrity.json"),
        restore_plan: out_dir.join("restore-plan.json"),
        restore_status: out_dir.join("restore-status.json"),
        preflight_summary: out_dir.join("preflight-summary.json"),
    }
}

// Write the standard preflight artifacts before emitting the compact summary.
fn write_preflight_artifacts(
    input: PreflightArtifactInput<'_>,
) -> Result<(), BackupPreflightError> {
    write_json_value_file(
        &input.paths.manifest_validation,
        &manifest_validation_summary(input.manifest),
    )?;
    fs::write(
        &input.paths.backup_status,
        serde_json::to_vec_pretty(&input.status)?,
    )?;
    fs::write(
        &input.paths.backup_inspection,
        serde_json::to_vec_pretty(&input.inspection)?,
    )?;
    fs::write(
        &input.paths.backup_provenance,
        serde_json::to_vec_pretty(&input.provenance)?,
    )?;
    fs::write(
        &input.paths.backup_integrity,
        serde_json::to_vec_pretty(&input.integrity)?,
    )?;
    fs::write(
        &input.paths.restore_plan,
        serde_json::to_vec_pretty(&input.restore_plan)?,
    )?;
    fs::write(
        &input.paths.restore_status,
        serde_json::to_vec_pretty(&input.restore_status)?,
    )?;
    Ok(())
}

// Build the in-memory preflight report mirrored by preflight-summary.json.
fn build_preflight_report(input: PreflightReportInput<'_>) -> BackupPreflightReport {
    let identity = &input.restore_plan.identity_summary;
    let snapshot = &input.restore_plan.snapshot_summary;
    let verification = &input.restore_plan.verification_summary;
    let operation = &input.restore_plan.operation_summary;
    let ordering = &input.restore_plan.ordering_summary;

    BackupPreflightReport {
        status: "ready".to_string(),
        backup_id: input.manifest.backup_id.clone(),
        backup_dir: input.config.backup_dir.display().to_string(),
        source_environment: input.manifest.source.environment.clone(),
        source_root_canister: input.manifest.source.root_canister.clone(),
        topology_hash: input.manifest.fleet.topology_hash.clone(),
        mapping_path: input
            .config
            .mapping
            .as_ref()
            .map(|path| path.display().to_string()),
        journal_complete: input.status.is_complete,
        journal_operation_metrics: input.status.operation_metrics.clone(),
        inspection_status: readiness_status(input.inspection.ready_for_verify).to_string(),
        provenance_status: consistency_status(
            input.provenance.backup_id_matches && input.provenance.topology_receipts_match,
        )
        .to_string(),
        backup_id_status: match_status(input.provenance.backup_id_matches).to_string(),
        topology_receipts_status: match_status(input.provenance.topology_receipts_match)
            .to_string(),
        topology_mismatch_count: input.provenance.topology_receipt_mismatches.len(),
        integrity_verified: input.integrity.verified,
        manifest_design_v1_ready: input.manifest.design_conformance_report().design_v1_ready,
        manifest_members: input.manifest.fleet.members.len(),
        backup_unit_count: input.provenance.backup_unit_count,
        restore_plan_members: input.restore_plan.member_count,
        restore_mapping_supplied: identity.mapping_supplied,
        restore_all_sources_mapped: identity.all_sources_mapped,
        restore_fixed_members: identity.fixed_members,
        restore_relocatable_members: identity.relocatable_members,
        restore_in_place_members: identity.in_place_members,
        restore_mapped_members: identity.mapped_members,
        restore_remapped_members: identity.remapped_members,
        restore_ready: input.restore_plan.readiness_summary.ready,
        restore_readiness_reasons: input.restore_plan.readiness_summary.reasons.clone(),
        restore_all_members_have_module_hash: snapshot.all_members_have_module_hash,
        restore_all_members_have_wasm_hash: snapshot.all_members_have_wasm_hash,
        restore_all_members_have_code_version: snapshot.all_members_have_code_version,
        restore_all_members_have_checksum: snapshot.all_members_have_checksum,
        restore_members_with_module_hash: snapshot.members_with_module_hash,
        restore_members_with_wasm_hash: snapshot.members_with_wasm_hash,
        restore_members_with_code_version: snapshot.members_with_code_version,
        restore_members_with_checksum: snapshot.members_with_checksum,
        restore_verification_required: verification.verification_required,
        restore_all_members_have_checks: verification.all_members_have_checks,
        restore_fleet_checks: verification.fleet_checks,
        restore_member_check_groups: verification.member_check_groups,
        restore_member_checks: verification.member_checks,
        restore_members_with_checks: verification.members_with_checks,
        restore_total_checks: verification.total_checks,
        restore_planned_snapshot_uploads: operation
            .effective_planned_snapshot_uploads(input.restore_plan.member_count),
        restore_planned_snapshot_loads: operation.planned_snapshot_loads,
        restore_planned_code_reinstalls: operation.planned_code_reinstalls,
        restore_planned_verification_checks: operation.planned_verification_checks,
        restore_planned_operations: operation
            .effective_planned_operations(input.restore_plan.member_count),
        restore_planned_phases: operation.planned_phases,
        restore_phase_count: ordering.phase_count,
        restore_dependency_free_members: ordering.dependency_free_members,
        restore_in_group_parent_edges: ordering.in_group_parent_edges,
        restore_cross_group_parent_edges: ordering.cross_group_parent_edges,
        manifest_validation_path: input.paths.manifest_validation.display().to_string(),
        backup_status_path: input.paths.backup_status.display().to_string(),
        backup_inspection_path: input.paths.backup_inspection.display().to_string(),
        backup_provenance_path: input.paths.backup_provenance.display().to_string(),
        backup_integrity_path: input.paths.backup_integrity.display().to_string(),
        restore_plan_path: input.paths.restore_plan.display().to_string(),
        restore_status_path: input.paths.restore_status.display().to_string(),
        preflight_summary_path: input.paths.preflight_summary.display().to_string(),
    }
}

// Build the compact preflight summary emitted after all checks pass.
fn preflight_summary_value(report: &BackupPreflightReport) -> serde_json::Value {
    let mut summary = serde_json::Map::new();
    insert_preflight_source_summary(&mut summary, report);
    insert_preflight_restore_summary(&mut summary, report);
    insert_preflight_report_paths(&mut summary, report);
    serde_json::Value::Object(summary)
}

// Insert one named JSON value into the compact preflight summary.
fn insert_summary_value(
    summary: &mut serde_json::Map<String, serde_json::Value>,
    key: &'static str,
    value: serde_json::Value,
) {
    summary.insert(key.to_string(), value);
}

// Insert a fixed group of named JSON values into the compact preflight summary.
fn insert_summary_values<const N: usize>(
    summary: &mut serde_json::Map<String, serde_json::Value>,
    values: [(&'static str, serde_json::Value); N],
) {
    for (key, value) in values {
        insert_summary_value(summary, key, value);
    }
}

// Insert backup source and validation status fields into the summary.
fn insert_preflight_source_summary(
    summary: &mut serde_json::Map<String, serde_json::Value>,
    report: &BackupPreflightReport,
) {
    insert_summary_values(
        summary,
        [
            ("status", json!(report.status)),
            ("backup_id", json!(report.backup_id)),
            ("backup_dir", json!(report.backup_dir)),
            ("source_environment", json!(report.source_environment)),
            ("source_root_canister", json!(report.source_root_canister)),
            ("topology_hash", json!(report.topology_hash)),
            ("mapping_path", json!(report.mapping_path)),
            ("journal_complete", json!(report.journal_complete)),
            (
                "journal_operation_metrics",
                json!(report.journal_operation_metrics),
            ),
            ("inspection_status", json!(report.inspection_status)),
            ("provenance_status", json!(report.provenance_status)),
            ("backup_id_status", json!(report.backup_id_status)),
            (
                "topology_receipts_status",
                json!(report.topology_receipts_status),
            ),
            (
                "topology_mismatch_count",
                json!(report.topology_mismatch_count),
            ),
            ("integrity_verified", json!(report.integrity_verified)),
            (
                "manifest_design_v1_ready",
                json!(report.manifest_design_v1_ready),
            ),
            ("manifest_members", json!(report.manifest_members)),
            ("backup_unit_count", json!(report.backup_unit_count)),
        ],
    );
}

// Insert restore planning summary fields into the compact preflight summary.
fn insert_preflight_restore_summary(
    summary: &mut serde_json::Map<String, serde_json::Value>,
    report: &BackupPreflightReport,
) {
    insert_summary_values(
        summary,
        [
            ("restore_plan_members", json!(report.restore_plan_members)),
            (
                "restore_mapping_supplied",
                json!(report.restore_mapping_supplied),
            ),
            (
                "restore_all_sources_mapped",
                json!(report.restore_all_sources_mapped),
            ),
        ],
    );
    insert_preflight_restore_identity_summary(summary, report);
    insert_preflight_restore_readiness_summary(summary, report);
    insert_preflight_restore_snapshot_summary(summary, report);
    insert_preflight_restore_verification_summary(summary, report);
    insert_preflight_restore_operation_summary(summary, report);
    insert_preflight_restore_ordering_summary(summary, report);
}

// Insert restore identity summary fields into the compact preflight summary.
fn insert_preflight_restore_identity_summary(
    summary: &mut serde_json::Map<String, serde_json::Value>,
    report: &BackupPreflightReport,
) {
    insert_summary_values(
        summary,
        [
            ("restore_fixed_members", json!(report.restore_fixed_members)),
            (
                "restore_relocatable_members",
                json!(report.restore_relocatable_members),
            ),
            (
                "restore_in_place_members",
                json!(report.restore_in_place_members),
            ),
            (
                "restore_mapped_members",
                json!(report.restore_mapped_members),
            ),
            (
                "restore_remapped_members",
                json!(report.restore_remapped_members),
            ),
        ],
    );
}

// Insert restore readiness summary fields into the compact preflight summary.
fn insert_preflight_restore_readiness_summary(
    summary: &mut serde_json::Map<String, serde_json::Value>,
    report: &BackupPreflightReport,
) {
    insert_summary_values(
        summary,
        [
            ("restore_ready", json!(report.restore_ready)),
            (
                "restore_readiness_reasons",
                json!(report.restore_readiness_reasons),
            ),
        ],
    );
}

// Insert restore snapshot summary fields into the compact preflight summary.
fn insert_preflight_restore_snapshot_summary(
    summary: &mut serde_json::Map<String, serde_json::Value>,
    report: &BackupPreflightReport,
) {
    insert_summary_values(
        summary,
        [
            (
                "restore_all_members_have_module_hash",
                json!(report.restore_all_members_have_module_hash),
            ),
            (
                "restore_all_members_have_wasm_hash",
                json!(report.restore_all_members_have_wasm_hash),
            ),
            (
                "restore_all_members_have_code_version",
                json!(report.restore_all_members_have_code_version),
            ),
            (
                "restore_all_members_have_checksum",
                json!(report.restore_all_members_have_checksum),
            ),
            (
                "restore_members_with_module_hash",
                json!(report.restore_members_with_module_hash),
            ),
            (
                "restore_members_with_wasm_hash",
                json!(report.restore_members_with_wasm_hash),
            ),
            (
                "restore_members_with_code_version",
                json!(report.restore_members_with_code_version),
            ),
            (
                "restore_members_with_checksum",
                json!(report.restore_members_with_checksum),
            ),
        ],
    );
}

// Insert restore verification summary fields into the compact preflight summary.
fn insert_preflight_restore_verification_summary(
    summary: &mut serde_json::Map<String, serde_json::Value>,
    report: &BackupPreflightReport,
) {
    insert_summary_values(
        summary,
        [
            (
                "restore_verification_required",
                json!(report.restore_verification_required),
            ),
            (
                "restore_all_members_have_checks",
                json!(report.restore_all_members_have_checks),
            ),
            ("restore_fleet_checks", json!(report.restore_fleet_checks)),
            (
                "restore_member_check_groups",
                json!(report.restore_member_check_groups),
            ),
            ("restore_member_checks", json!(report.restore_member_checks)),
            (
                "restore_members_with_checks",
                json!(report.restore_members_with_checks),
            ),
            ("restore_total_checks", json!(report.restore_total_checks)),
        ],
    );
}

// Insert restore operation summary fields into the compact preflight summary.
fn insert_preflight_restore_operation_summary(
    summary: &mut serde_json::Map<String, serde_json::Value>,
    report: &BackupPreflightReport,
) {
    insert_summary_values(
        summary,
        [
            (
                "restore_planned_snapshot_uploads",
                json!(report.restore_planned_snapshot_uploads),
            ),
            (
                "restore_planned_snapshot_loads",
                json!(report.restore_planned_snapshot_loads),
            ),
            (
                "restore_planned_code_reinstalls",
                json!(report.restore_planned_code_reinstalls),
            ),
            (
                "restore_planned_verification_checks",
                json!(report.restore_planned_verification_checks),
            ),
            (
                "restore_planned_operations",
                json!(report.restore_planned_operations),
            ),
            (
                "restore_planned_phases",
                json!(report.restore_planned_phases),
            ),
        ],
    );
}

// Insert restore ordering summary fields into the compact preflight summary.
fn insert_preflight_restore_ordering_summary(
    summary: &mut serde_json::Map<String, serde_json::Value>,
    report: &BackupPreflightReport,
) {
    insert_summary_values(
        summary,
        [
            ("restore_phase_count", json!(report.restore_phase_count)),
            (
                "restore_dependency_free_members",
                json!(report.restore_dependency_free_members),
            ),
            (
                "restore_in_group_parent_edges",
                json!(report.restore_in_group_parent_edges),
            ),
            (
                "restore_cross_group_parent_edges",
                json!(report.restore_cross_group_parent_edges),
            ),
        ],
    );
}

// Insert generated report paths into the compact preflight summary.
fn insert_preflight_report_paths(
    summary: &mut serde_json::Map<String, serde_json::Value>,
    report: &BackupPreflightReport,
) {
    insert_summary_values(
        summary,
        [
            (
                "manifest_validation_path",
                json!(report.manifest_validation_path),
            ),
            ("backup_status_path", json!(report.backup_status_path)),
            (
                "backup_inspection_path",
                json!(report.backup_inspection_path),
            ),
            (
                "backup_provenance_path",
                json!(report.backup_provenance_path),
            ),
            ("backup_integrity_path", json!(report.backup_integrity_path)),
            ("restore_plan_path", json!(report.restore_plan_path)),
            ("restore_status_path", json!(report.restore_status_path)),
            (
                "preflight_summary_path",
                json!(report.preflight_summary_path),
            ),
        ],
    );
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
fn read_mapping(path: &PathBuf) -> Result<RestoreMapping, BackupPreflightError> {
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(BackupPreflightError::from)
}

// Write one pretty JSON value artifact.
fn write_json_value_file(
    path: &PathBuf,
    value: &serde_json::Value,
) -> Result<(), BackupPreflightError> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}
