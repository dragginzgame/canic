use super::*;
use crate::{discovery::SnapshotTarget, journal::ArtifactState, persistence::BackupLayout};
use std::{
    error::Error as StdError,
    fmt, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const ROOT: &str = "aaaaa-aa";
const CHILD: &str = "renrk-eyaaa-aaaaa-aaada-cai";

// Ensure snapshot manifest construction stays in the backup domain crate.
#[test]
fn snapshot_manifest_includes_selection_and_artifacts() {
    let targets = vec![
        SnapshotTarget {
            canister_id: ROOT.to_string(),
            role: Some("root".to_string()),
            parent_canister_id: None,
        },
        SnapshotTarget {
            canister_id: CHILD.to_string(),
            role: Some("app".to_string()),
            parent_canister_id: Some(ROOT.to_string()),
        },
    ];
    let artifacts = targets
        .iter()
        .map(|target| SnapshotArtifact {
            canister_id: target.canister_id.clone(),
            snapshot_id: format!("snapshot-{}", target.role.as_deref().unwrap_or("unknown")),
            path: std::path::PathBuf::from(target.canister_id.clone()),
            checksum: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
        })
        .collect::<Vec<_>>();
    let topology_hash =
        topology_hash_for_targets(ROOT, &targets).expect("topology hash should build");

    let manifest = build_snapshot_manifest(SnapshotManifestInput {
        backup_id: "backup-test".to_string(),
        created_at: "unknown".to_string(),
        tool_name: "canic-cli".to_string(),
        tool_version: "0.31.0".to_string(),
        environment: "local".to_string(),
        root_canister: ROOT.to_string(),
        selected_canister: ROOT.to_string(),
        include_children: true,
        targets: &targets,
        artifacts: &artifacts,
        discovery_topology_hash: topology_hash.clone(),
        pre_snapshot_topology_hash: topology_hash,
    })
    .expect("snapshot manifest should build");

    assert_eq!(manifest.backup_id, "backup-test");
    assert_eq!(manifest.fleet.members.len(), 2);
    assert_eq!(
        manifest.fleet.members[1]
            .source_snapshot
            .checksum
            .as_deref(),
        Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
    );
    assert_eq!(
        manifest.consistency.backup_units[0].kind,
        BackupUnitKind::Subtree
    );
}

// Ensure topology drift is classified before snapshot mutation proceeds.
#[test]
fn topology_stability_rejects_drift() {
    let mut discovery = topology_hash_for_targets(
        ROOT,
        &[SnapshotTarget {
            canister_id: ROOT.to_string(),
            role: Some("root".to_string()),
            parent_canister_id: None,
        }],
    )
    .expect("topology hash should build");
    let pre_snapshot = discovery.clone();
    discovery.hash = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string();

    let err =
        ensure_topology_stable(&discovery, &pre_snapshot).expect_err("topology drift should fail");

    assert!(matches!(err, SnapshotManifestError::TopologyChanged { .. }));
}

// Ensure the backup crate owns snapshot journal, checksum, and manifest capture.
#[test]
fn download_snapshots_writes_manifest_and_durable_journal() {
    let root = temp_dir("canic-backup-download");
    let out = root.join("backup");
    let config = single_snapshot_config(out.clone());
    let mut driver = FakeSnapshotDriver;

    let result = download_snapshots(&config, &mut driver).expect("download snapshots");
    let layout = BackupLayout::new(out);
    let journal = layout.read_journal().expect("read journal");
    let manifest = layout.read_manifest().expect("read manifest");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(result.artifacts.len(), 1);
    assert!(result.planned_commands.is_empty());
    assert_eq!(journal.artifacts.len(), 1);
    assert_eq!(journal.operation_metrics.target_count, 1);
    assert_eq!(journal.operation_metrics.snapshot_create_started, 1);
    assert_eq!(journal.operation_metrics.snapshot_create_completed, 1);
    assert_eq!(journal.operation_metrics.snapshot_download_started, 1);
    assert_eq!(journal.operation_metrics.snapshot_download_completed, 1);
    assert_eq!(journal.operation_metrics.checksum_verify_started, 1);
    assert_eq!(journal.operation_metrics.checksum_verify_completed, 1);
    assert_eq!(journal.operation_metrics.artifact_finalize_started, 1);
    assert_eq!(journal.operation_metrics.artifact_finalize_completed, 1);
    assert_eq!(journal.artifacts[0].state, ArtifactState::Durable);
    assert!(journal.artifacts[0].checksum.is_some());
    assert_eq!(manifest.backup_id, journal.backup_id);
    assert_eq!(manifest.fleet.members.len(), 1);
    assert_eq!(manifest.fleet.members[0].canister_id, ROOT);
    assert_eq!(
        manifest.fleet.members[0].source_snapshot.snapshot_id,
        "snapshot-aaaaa-aa"
    );
    assert_eq!(
        manifest.fleet.members[0]
            .source_snapshot
            .checksum
            .as_deref(),
        journal.artifacts[0].checksum.as_deref()
    );
}

// Ensure dry-run planning returns commands without writing backup state.
#[test]
fn dry_run_returns_planned_commands_without_writing_manifest() {
    let root = temp_dir("canic-backup-dry-run");
    let out = root.join("backup");
    let mut config = single_snapshot_config(out.clone());
    config.dry_run = true;
    config.lifecycle = SnapshotLifecycleMode::StopAndResume;
    let mut driver = FakeSnapshotDriver;

    let result = download_snapshots(&config, &mut driver).expect("dry-run snapshots");

    fs::remove_dir_all(root).ok();
    assert_eq!(result.artifacts.len(), 1);
    assert_eq!(
        result.planned_commands,
        vec![
            "dfx canister stop aaaaa-aa",
            "dfx canister snapshot create aaaaa-aa",
            "dfx canister snapshot download aaaaa-aa <snapshot-id>",
            "dfx canister start aaaaa-aa"
        ]
    );
    assert!(!out.join("fleet-backup-manifest.json").exists());
}

///
/// FakeSnapshotDriver
///

#[derive(Default)]
struct FakeSnapshotDriver;

impl SnapshotDriver for FakeSnapshotDriver {
    /// Return no registry data because single-canister tests do not need it.
    fn registry_json(&mut self, _root: &str) -> Result<String, SnapshotDriverError> {
        Err(Box::new(FakeDriverError("registry unavailable")))
    }

    /// Return a deterministic fake snapshot id.
    fn create_snapshot(&mut self, canister_id: &str) -> Result<String, SnapshotDriverError> {
        Ok(format!("snapshot-{canister_id}"))
    }

    /// Record a successful fake stop operation.
    fn stop_canister(&mut self, _canister_id: &str) -> Result<(), SnapshotDriverError> {
        Ok(())
    }

    /// Record a successful fake start operation.
    fn start_canister(&mut self, _canister_id: &str) -> Result<(), SnapshotDriverError> {
        Ok(())
    }

    /// Write deterministic fake snapshot bytes into the artifact directory.
    fn download_snapshot(
        &mut self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
    ) -> Result<(), SnapshotDriverError> {
        fs::create_dir_all(artifact_path)?;
        fs::write(
            artifact_path.join("snapshot.txt"),
            format!("{canister_id}:{snapshot_id}\n"),
        )?;
        Ok(())
    }

    /// Render the fake dry-run create command.
    fn create_snapshot_command(&self, canister_id: &str) -> String {
        format!("dfx canister snapshot create {canister_id}")
    }

    /// Render the fake dry-run stop command.
    fn stop_canister_command(&self, canister_id: &str) -> String {
        format!("dfx canister stop {canister_id}")
    }

    /// Render the fake dry-run start command.
    fn start_canister_command(&self, canister_id: &str) -> String {
        format!("dfx canister start {canister_id}")
    }

    /// Render the fake dry-run download command.
    fn download_snapshot_command(
        &self,
        canister_id: &str,
        snapshot_id: &str,
        _artifact_path: &Path,
    ) -> String {
        format!("dfx canister snapshot download {canister_id} {snapshot_id}")
    }
}

///
/// FakeDriverError
///

#[derive(Debug)]
struct FakeDriverError(&'static str);

impl fmt::Display for FakeDriverError {
    /// Render the fake driver error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl StdError for FakeDriverError {}

// Build a single-canister snapshot config for orchestration tests.
fn single_snapshot_config(out: PathBuf) -> SnapshotDownloadConfig {
    SnapshotDownloadConfig {
        canister: ROOT.to_string(),
        out,
        root: None,
        include_children: false,
        recursive: false,
        dry_run: false,
        lifecycle: SnapshotLifecycleMode::StopBeforeSnapshot,
        backup_id: "backup-test".to_string(),
        created_at: "unknown".to_string(),
        tool_name: "canic-test".to_string(),
        tool_version: "0.31.0".to_string(),
        environment: "local".to_string(),
    }
}

// Build a unique temporary directory.
fn temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}
