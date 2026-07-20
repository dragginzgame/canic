use crate::{
    persistence::CommandLifetimeHandle,
    plan::{
        AuthorityEvidence, AuthorityProofSource, BackupExecutionPreflightReceipts, BackupPlan,
        ControlAuthorityReceipt, QuiescencePreflightReceipt, QuiescencePreflightTarget,
        SnapshotReadAuthorityReceipt, TopologyPreflightReceipt, TopologyPreflightTarget,
    },
    runner::{
        BackupRunnerCanisterStatus, BackupRunnerCommandError, BackupRunnerExecutor,
        BackupRunnerSnapshotReceipt,
    },
};

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// Test executor that records backup commands and returns exact typed receipts.
#[derive(Default)]
pub struct FakeBackupRunnerExecutor {
    pub commands: Vec<String>,
    pub fail_on: Option<FakeBackupRunnerFailure>,
}

/// Failure point supported by the shared backup runner test executor.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum FakeBackupRunnerFailure {
    Preflight,
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
        Ok(BackupRunnerCanisterStatus::Running)
    }

    fn stop_canister(
        &mut self,
        canister_id: &str,
        _command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError> {
        self.commands.push(format!("stop:{canister_id}"));
        Ok(())
    }

    fn start_canister(
        &mut self,
        canister_id: &str,
        _command_lifetime: CommandLifetimeHandle,
    ) -> Result<(), BackupRunnerCommandError> {
        self.commands.push(format!("start:{canister_id}"));
        Ok(())
    }

    fn create_snapshot(
        &mut self,
        canister_id: &str,
        _command_lifetime: CommandLifetimeHandle,
    ) -> Result<BackupRunnerSnapshotReceipt, BackupRunnerCommandError> {
        self.commands.push(format!("snapshot:{canister_id}"));
        if self.fail_on == Some(FakeBackupRunnerFailure::CreateSnapshot) {
            return Err(BackupRunnerCommandError::failed(
                "snapshot",
                "simulated snapshot failure",
            ));
        }
        Ok(BackupRunnerSnapshotReceipt {
            snapshot_id: "snap-app".to_string(),
            taken_at_timestamp: Some(1_778_709_681_897_818_005),
            total_size_bytes: Some(272_586_987),
        })
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
