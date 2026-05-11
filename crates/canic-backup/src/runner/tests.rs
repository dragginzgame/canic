use super::*;
use crate::{
    discovery::RegistryEntry,
    manifest::IdentityMode,
    persistence::BackupLayout,
    plan::{
        AuthorityEvidence, AuthorityProofSource, BackupExecutionPreflightReceipts, BackupPlan,
        BackupPlanBuildInput, BackupScopeKind, ControlAuthority, ControlAuthorityReceipt,
        QuiescencePolicy, QuiescencePreflightReceipt, QuiescencePreflightTarget,
        SnapshotReadAuthority, SnapshotReadAuthorityReceipt, TopologyPreflightReceipt,
        TopologyPreflightTarget, build_backup_plan,
    },
    test_support::temp_dir,
};
use std::{fs, path::Path};

const ROOT: &str = "aaaaa-aa";
const APP: &str = "renrk-eyaaa-aaaaa-aaada-cai";
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

// Ensure the backup runner executes a persisted plan into a verified backup layout.
#[test]
fn runner_executes_plan_and_finalizes_manifest() {
    let root = temp_dir("canic-backup-runner");
    let layout = BackupLayout::new(root.clone());
    let plan = plan();
    let journal = BackupExecutionJournal::from_plan(&plan).expect("execution journal");
    layout.write_backup_plan(&plan).expect("write plan");
    layout
        .write_execution_journal(&journal)
        .expect("write execution journal");

    let mut executor = FakeExecutor::default();
    let response = backup_run_execute_with_executor(
        &BackupRunnerConfig {
            out: root.clone(),
            max_steps: None,
            updated_at: Some("unix:10".to_string()),
            tool_name: "canic".to_string(),
            tool_version: "0.34.3".to_string(),
        },
        &mut executor,
    )
    .expect("run backup");
    let integrity = layout.verify_integrity().expect("verify finalized layout");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(response.complete);
    assert_eq!(response.executed_operation_count, 6);
    assert_eq!(integrity.backup_id, "run-test");
    assert_eq!(integrity.durable_artifacts, 1);
    assert_eq!(
        executor.commands,
        vec![
            format!("status:{APP}"),
            format!("stop:{APP}"),
            format!("snapshot:{APP}"),
            format!("start:{APP}"),
            format!("download:{APP}:snap-app"),
        ]
    );
}

#[derive(Default)]
struct FakeExecutor {
    commands: Vec<String>,
}

impl BackupRunnerExecutor for FakeExecutor {
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
                .map(|target| ControlAuthorityReceipt {
                    plan_id: plan.plan_id.clone(),
                    preflight_id: preflight_id.to_string(),
                    target_canister_id: target.canister_id.clone(),
                    authority: ControlAuthority::operator_controller(AuthorityEvidence::Proven),
                    proof_source: AuthorityProofSource::ManagementStatus,
                    validated_at: validated_at.to_string(),
                    expires_at: expires_at.to_string(),
                    message: None,
                })
                .collect(),
            snapshot_read_authority: plan
                .targets
                .iter()
                .map(|target| SnapshotReadAuthorityReceipt {
                    plan_id: plan.plan_id.clone(),
                    preflight_id: preflight_id.to_string(),
                    target_canister_id: target.canister_id.clone(),
                    authority: SnapshotReadAuthority::operator_controller(
                        AuthorityEvidence::Proven,
                    ),
                    proof_source: AuthorityProofSource::ManagementStatus,
                    validated_at: validated_at.to_string(),
                    expires_at: expires_at.to_string(),
                    message: None,
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

    fn stop_canister(&mut self, canister_id: &str) -> Result<(), BackupRunnerCommandError> {
        self.commands.push(format!("stop:{canister_id}"));
        Ok(())
    }

    fn start_canister(&mut self, canister_id: &str) -> Result<(), BackupRunnerCommandError> {
        self.commands.push(format!("start:{canister_id}"));
        Ok(())
    }

    fn create_snapshot(&mut self, canister_id: &str) -> Result<String, BackupRunnerCommandError> {
        self.commands.push(format!("snapshot:{canister_id}"));
        Ok("snap-app".to_string())
    }

    fn download_snapshot(
        &mut self,
        canister_id: &str,
        snapshot_id: &str,
        artifact_path: &Path,
    ) -> Result<(), BackupRunnerCommandError> {
        self.commands
            .push(format!("download:{canister_id}:{snapshot_id}"));
        fs::create_dir_all(artifact_path)
            .map_err(|err| BackupRunnerCommandError::failed("io", err.to_string()))?;
        fs::write(artifact_path.join("snapshot.bin"), b"app snapshot")
            .map_err(|err| BackupRunnerCommandError::failed("io", err.to_string()))?;
        Ok(())
    }
}

fn plan() -> BackupPlan {
    build_backup_plan(BackupPlanBuildInput {
        plan_id: "plan-test".to_string(),
        run_id: "run-test".to_string(),
        fleet: "demo".to_string(),
        network: "local".to_string(),
        root_canister_id: ROOT.to_string(),
        selected_canister_id: Some(APP.to_string()),
        selected_scope_kind: BackupScopeKind::Subtree,
        include_descendants: true,
        topology_hash_before_quiesce: HASH.to_string(),
        registry: &[
            RegistryEntry {
                pid: ROOT.to_string(),
                role: Some("root".to_string()),
                kind: Some("root".to_string()),
                parent_pid: None,
                module_hash: None,
            },
            RegistryEntry {
                pid: APP.to_string(),
                role: Some("app".to_string()),
                kind: Some("singleton".to_string()),
                parent_pid: Some(ROOT.to_string()),
                module_hash: Some(HASH.to_string()),
            },
        ],
        control_authority: ControlAuthority::operator_controller(AuthorityEvidence::Proven),
        snapshot_read_authority: SnapshotReadAuthority::operator_controller(
            AuthorityEvidence::Proven,
        ),
        quiescence_policy: QuiescencePolicy::CrashConsistent,
        identity_mode: IdentityMode::Relocatable,
    })
    .expect("backup plan")
}
