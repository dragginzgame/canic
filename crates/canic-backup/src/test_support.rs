use crate::{
    persistence::CommandLifetimeHandle,
    plan::{
        AuthorityEvidence, AuthorityProofSource, BackupExecutionPreflightReceipts, BackupPlan,
        ControlAuthorityReceipt, QuiescencePreflightReceipt, QuiescencePreflightTarget,
        SnapshotReadAuthorityReceipt, TopologyPreflightReceipt, TopologyPreflightTarget,
    },
    runner::{
        BackupRunnerCanisterStatus, BackupRunnerCommandError, BackupRunnerExecutor,
        BackupRunnerSnapshot,
    },
};

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::{
    process::Child,
    thread,
    time::{Duration, Instant},
};

/// Test executor that records backup commands and returns exact typed receipts.
#[derive(Default)]
pub struct FakeBackupRunnerExecutor {
    pub commands: Vec<String>,
    pub fail_on: Option<FakeBackupRunnerFailure>,
    pub canister_statuses: BTreeMap<String, BackupRunnerCanisterStatus>,
    pub snapshots: BTreeMap<String, Vec<BackupRunnerSnapshot>>,
}

/// Failure point supported by the shared backup runner test executor.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum FakeBackupRunnerFailure {
    Preflight,
    SnapshotInventory,
    CreateSnapshot,
}

impl BackupRunnerExecutor for FakeBackupRunnerExecutor {
    fn preflight_receipts(
        &mut self,
        plan: &BackupPlan,
        preflight_id: &str,
        validated_at: &str,
        expires_at: &str,
    ) -> Result<BackupExecutionPreflightReceipts, BackupRunnerCommandError> {
        for target in &plan.targets {
            self.commands.push(format!("status:{}", target.canister_id));
        }
        if self.fail_on == Some(FakeBackupRunnerFailure::Preflight) {
            return Err(BackupRunnerCommandError::failed(
                "preflight",
                "simulated preflight failure",
            ));
        }
        Ok(BackupExecutionPreflightReceipts {
            plan_id: plan.plan_id.clone(),
            preflight_id: preflight_id.to_string(),
            validated_at: validated_at.to_string(),
            expires_at: expires_at.to_string(),
            topology: TopologyPreflightReceipt {
                plan_id: plan.plan_id.clone(),
                preflight_id: preflight_id.to_string(),
                topology_hash_before_quiesce: plan.topology_hash_before_quiesce.clone(),
                topology_hash_at_preflight: plan.topology_hash_before_quiesce.clone(),
                targets: plan
                    .targets
                    .iter()
                    .map(TopologyPreflightTarget::from)
                    .collect(),
                validated_at: validated_at.to_string(),
                expires_at: expires_at.to_string(),
                message: None,
            },
            control_authority: plan
                .targets
                .iter()
                .map(|target| {
                    let mut authority = target.control_authority.clone();
                    authority.evidence = AuthorityEvidence::Proven;
                    ControlAuthorityReceipt {
                        plan_id: plan.plan_id.clone(),
                        preflight_id: preflight_id.to_string(),
                        target_canister_id: target.canister_id.clone(),
                        authority,
                        proof_source: AuthorityProofSource::ManagementStatus,
                        validated_at: validated_at.to_string(),
                        expires_at: expires_at.to_string(),
                        message: None,
                    }
                })
                .collect(),
            snapshot_read_authority: plan
                .targets
                .iter()
                .map(|target| {
                    let mut authority = target.snapshot_read_authority.clone();
                    authority.evidence = AuthorityEvidence::Proven;
                    SnapshotReadAuthorityReceipt {
                        plan_id: plan.plan_id.clone(),
                        preflight_id: preflight_id.to_string(),
                        target_canister_id: target.canister_id.clone(),
                        authority,
                        proof_source: AuthorityProofSource::ManagementStatus,
                        validated_at: validated_at.to_string(),
                        expires_at: expires_at.to_string(),
                        message: None,
                    }
                })
                .collect(),
            quiescence: QuiescencePreflightReceipt {
                plan_id: plan.plan_id.clone(),
                preflight_id: preflight_id.to_string(),
                quiescence_policy: plan.quiescence_policy.clone(),
                accepted: true,
                targets: plan
                    .targets
                    .iter()
                    .map(QuiescencePreflightTarget::from)
                    .collect(),
                validated_at: validated_at.to_string(),
                expires_at: expires_at.to_string(),
                message: None,
            },
        })
    }

    fn canister_status(
        &mut self,
        canister_id: &str,
    ) -> Result<BackupRunnerCanisterStatus, BackupRunnerCommandError> {
        self.commands.push(format!("status:{canister_id}"));
        Ok(self
            .canister_statuses
            .get(canister_id)
            .copied()
            .unwrap_or(BackupRunnerCanisterStatus::Running))
    }

    fn snapshot_inventory(
        &mut self,
        canister_id: &str,
    ) -> Result<Vec<BackupRunnerSnapshot>, BackupRunnerCommandError> {
        self.commands.push(format!("snapshot-list:{canister_id}"));
        if self.fail_on == Some(FakeBackupRunnerFailure::SnapshotInventory) {
            return Err(BackupRunnerCommandError::failed(
                "snapshot-list",
                "simulated snapshot inventory failure",
            ));
        }
        Ok(self.snapshots.get(canister_id).cloned().unwrap_or_default())
    }

    fn stop_canister(
        &mut self,
        canister_id: &str,
        _command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError> {
        self.commands.push(format!("stop:{canister_id}"));
        self.canister_statuses
            .insert(canister_id.to_string(), BackupRunnerCanisterStatus::Stopped);
        Ok(())
    }

    fn start_canister(
        &mut self,
        canister_id: &str,
        _command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError> {
        self.commands.push(format!("start:{canister_id}"));
        self.canister_statuses
            .insert(canister_id.to_string(), BackupRunnerCanisterStatus::Running);
        Ok(())
    }

    fn create_snapshot(
        &mut self,
        canister_id: &str,
        _command_lifetime: CommandLifetimeHandle,
    ) -> Result<BackupRunnerSnapshot, BackupRunnerCommandError> {
        self.commands.push(format!("snapshot:{canister_id}"));
        if self.fail_on == Some(FakeBackupRunnerFailure::CreateSnapshot) {
            return Err(BackupRunnerCommandError::failed(
                "snapshot",
                "simulated snapshot failure",
            ));
        }
        let snapshot = BackupRunnerSnapshot {
            snapshot_id: "snap-app".to_string(),
            taken_at_timestamp: Some(1_778_709_681_897_818_005),
            total_size_bytes: Some(272_586_987),
        };
        self.snapshots
            .entry(canister_id.to_string())
            .or_default()
            .push(snapshot.clone());
        Ok(snapshot)
    }

    fn download_snapshot(
        &mut self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
        _command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError> {
        self.commands
            .push(format!("download:{canister_id}:{snapshot_id}"));
        fs::create_dir_all(artifact_path)
            .map_err(|error| BackupRunnerCommandError::failed("io", error.to_string()))?;
        fs::write(artifact_path.join("snapshot.bin"), b"app snapshot")
            .map_err(|error| BackupRunnerCommandError::failed("io", error.to_string()))?;
        Ok(())
    }
}

// Build a unique temporary directory path for tests that create their own layout.
pub fn temp_dir(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(unique_name(prefix))
}

// Build a unique temporary file path for tests that only need one artifact.
pub fn temp_path(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(unique_name(prefix))
}

// Include process and timestamp data so parallel test runs do not collide.
fn unique_name(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    format!("{prefix}-{}-{nanos}", std::process::id())
}

/// Kill one test child only after both sides acknowledge the named crash barrier.
#[cfg(unix)]
pub fn kill_child_at_acknowledged_barrier(child: &mut Child, root: &Path) {
    let ready_path = root.join("barrier-ready");
    let acknowledge_path = root.join("barrier-acknowledged");
    let armed_path = root.join("barrier-armed");
    wait_for_child_path(child, &ready_path, "child barrier");
    fs::write(&acknowledge_path, b"acknowledged\n").expect("acknowledge child barrier");
    wait_for_child_path(child, &armed_path, "armed child barrier");
    child.kill().expect("kill child at acknowledged barrier");
    child.wait().expect("reap killed child");
}

/// Signal that a test child reached its barrier, then wait to be killed.
#[cfg(unix)]
pub fn hold_at_acknowledged_barrier(root: &Path) -> ! {
    let ready_path = root.join("barrier-ready");
    let acknowledge_path = root.join("barrier-acknowledged");
    let armed_path = root.join("barrier-armed");
    fs::write(&ready_path, b"ready\n").expect("signal child barrier");
    wait_for_path(&acknowledge_path, "parent barrier acknowledgement");
    fs::write(&armed_path, b"armed\n").expect("arm child crash");
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}

#[cfg(unix)]
pub fn wait_for_child_path(child: &mut Child, path: &Path, description: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !path.is_file() {
        assert!(
            child.try_wait().expect("inspect crash child").is_none(),
            "crash child exited before {description}"
        );
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {description}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(unix)]
pub fn wait_for_path(path: &Path, description: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !path.is_file() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {description}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}
