mod apply;
mod options;
mod plan;
mod run;

use super::io::{
    require_prepared_journal_path, require_prepared_plan_path, verify_selected_journal_backup_root,
};
use super::*;
use crate::test_support::temp_dir;
use canic_backup::{
    artifacts::ArtifactChecksum,
    journal::{ArtifactJournalEntry, ArtifactState, DownloadJournal},
    manifest::{
        BackupUnit, BackupUnitKind, ConsistencySection, FleetBackupManifest, FleetMember,
        FleetSection, IdentityMode, SourceMetadata, SourceSnapshot, ToolMetadata,
        VerificationCheck, VerificationPlan,
    },
    persistence::BackupLayout,
    restore::{RestoreApplyDryRun, RestoreApplyJournal, RestorePlanner},
};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

const ROOT: &str = "aaaaa-aa";
const CHILD: &str = "renrk-eyaaa-aaaaa-aaada-cai";
const MAPPED_CHILD: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

///
/// RestoreCliFixture
///

struct RestoreCliFixture {
    root: PathBuf,
    journal_path: PathBuf,
    out_path: PathBuf,
}

impl RestoreCliFixture {
    // Create a temp restore CLI fixture with canonical journal and output paths.
    fn new(prefix: &str, out_file: &str) -> Self {
        let root = temp_dir(prefix);
        fs::create_dir_all(&root).expect("create temp root");

        Self {
            journal_path: root.join("restore-apply-journal.json"),
            out_path: root.join(out_file),
            root,
        }
    }

    // Persist a restore apply journal at the fixture journal path.
    fn write_journal(&self, journal: &RestoreApplyJournal) {
        fs::write(
            &self.journal_path,
            serde_json::to_vec(journal).expect("serialize journal"),
        )
        .expect("write journal");
    }

    // Run restore-run against the fixture journal and output paths.
    fn run_restore_run(&self, extra: &[&str]) -> Result<(), RestoreCommandError> {
        self.run_journal_command("run", extra)
    }

    // Read the fixture output as a typed JSON value.
    fn read_out<T>(&self, label: &str) -> T
    where
        T: serde::de::DeserializeOwned,
    {
        serde_json::from_slice(&fs::read(&self.out_path).expect(label)).expect(label)
    }

    // Build and run one journal-backed restore CLI command.
    fn run_journal_command(
        &self,
        command: &str,
        extra: &[&str],
    ) -> Result<(), RestoreCommandError> {
        let mut args = vec![
            OsString::from(command),
            OsString::from("--journal"),
            OsString::from(self.journal_path.as_os_str()),
            OsString::from("--out"),
            OsString::from(self.out_path.as_os_str()),
        ];
        args.extend(extra.iter().map(OsString::from));
        run(args)
    }
}

impl Drop for RestoreCliFixture {
    // Remove the fixture directory after each test completes.
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

// Write a tiny fake icp executable that reports one uploaded snapshot ID.
#[cfg(unix)]
fn write_fake_icp_upload(root: &Path, uploaded_snapshot_id: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = root.join("icp-upload-ok");
    fs::write(
        &path,
        format!("#!/bin/sh\nprintf 'Uploaded snapshot: {uploaded_snapshot_id}\\n'\n"),
    )
    .expect("write fake icp");
    let mut permissions = fs::metadata(&path)
        .expect("fake icp metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make fake icp executable");
    path
}

// Write a fake icp executable that succeeds without reporting an uploaded snapshot ID.
#[cfg(unix)]
fn write_fake_icp_upload_without_id(root: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = root.join("icp-upload-missing-id");
    fs::write(&path, "#!/bin/sh\nprintf 'Upload completed\\n'\n").expect("write fake icp");
    let mut permissions = fs::metadata(&path)
        .expect("fake icp metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make fake icp executable");
    path
}

// Build one manually ready apply journal for runner-focused CLI tests.
fn ready_apply_journal() -> RestoreApplyJournal {
    let plan = RestorePlanner::plan(&restore_ready_manifest(), None).expect("build plan");
    let dry_run = RestoreApplyDryRun::from_plan(&plan);
    let mut journal = RestoreApplyJournal::from_dry_run(&dry_run);

    journal.ready = true;
    journal.blocked_reasons = Vec::new();
    journal.backup_root = Some("/tmp/canic-cli-restore-artifacts".to_string());
    for operation in &mut journal.operations {
        operation.state = canic_backup::restore::RestoreApplyOperationState::Ready;
        operation.blocking_reasons = Vec::new();
    }
    journal.blocked_operations = 0;
    journal.ready_operations = journal.operation_count;
    journal.validate().expect("journal should validate");
    journal
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
            backup_units: vec![BackupUnit {
                unit_id: "fleet".to_string(),
                kind: BackupUnitKind::Subtree,
                roles: vec!["root".to_string(), "app".to_string()],
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
        verification_checks: vec![VerificationCheck {
            kind: "status".to_string(),
            roles: vec![role.to_string()],
        }],
        source_snapshot: SourceSnapshot {
            snapshot_id: format!("{role}-snapshot"),
            module_hash: None,
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

// Write artifact bytes and update the manifest checksums for apply validation.
fn write_manifest_artifacts(root: &Path, manifest: &mut FleetBackupManifest) {
    for member in &mut manifest.fleet.members {
        let bytes = format!("{} apply artifact", member.role);
        let artifact_path = root.join(&member.source_snapshot.artifact_path);
        if let Some(parent) = artifact_path.parent() {
            fs::create_dir_all(parent).expect("create artifact parent");
        }
        fs::write(&artifact_path, bytes.as_bytes()).expect("write artifact");
        let checksum = ArtifactChecksum::from_bytes(bytes.as_bytes());
        member.source_snapshot.checksum = Some(checksum.hash);
    }
}

// Derive the runner sidecar lock path for assertions.
fn journal_lock_path(path: &Path) -> PathBuf {
    let mut lock_path = path.as_os_str().to_os_string();
    lock_path.push(".lock");
    PathBuf::from(lock_path)
}
