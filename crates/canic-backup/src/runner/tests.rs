use super::*;
use crate::{
    execution::BackupExecutionOperationState,
    journal::{ArtifactJournalEntry, ArtifactState, DownloadJournal, DownloadOperationMetrics},
    manifest::{BackupUnitKind, IdentityMode},
    persistence::BackupLayout,
    plan::{
        AuthorityEvidence, AuthorityProofSource, BackupExecutionPreflightReceipts,
        BackupOperationKind, BackupPlan, BackupPlanBuildInput, BackupScopeKind, ControlAuthority,
        ControlAuthorityReceipt, QuiescencePolicy, QuiescencePreflightReceipt,
        QuiescencePreflightTarget, SnapshotReadAuthority, SnapshotReadAuthorityReceipt,
        TopologyPreflightReceipt, TopologyPreflightTarget, build_backup_plan,
    },
    registry::RegistryEntry,
    test_support::temp_dir,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

const ROOT: &str = "aaaaa-aa";
const APP: &str = "renrk-eyaaa-aaaaa-aaada-cai";
const WORKER: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[test]
fn runner_rejects_stale_download_journal_topology_before_snapshot_creation() {
    let root = prepared_layout("canic-backup-runner-stale-download-topology");
    let layout = BackupLayout::new(root.clone());
    let stale_hash = "f".repeat(64);
    layout
        .write_journal(&download_journal("run-test", &stale_hash, "stale-snapshot"))
        .expect("write stale download journal");

    let mut executor = FakeExecutor::default();
    let err = backup_run_execute_with_executor(&runner_config(root.clone(), None), &mut executor)
        .expect_err("stale topology receipt must reject");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupRunnerError::DownloadJournalTopologyMismatch {
            field: "discovery_topology_hash",
            expected,
            actual,
        } if expected == HASH && actual == stale_hash
    );
    assert!(
        executor
            .commands
            .iter()
            .all(|command| !command.starts_with("snapshot:"))
    );
}

#[test]
fn runner_rejects_download_journal_for_another_backup_before_snapshot_creation() {
    let root = prepared_layout("canic-backup-runner-other-download-backup");
    let layout = BackupLayout::new(root.clone());
    layout
        .write_journal(&download_journal("other-backup", HASH, "other-snapshot"))
        .expect("write foreign download journal");

    let mut executor = FakeExecutor::default();
    let err = backup_run_execute_with_executor(&runner_config(root.clone(), None), &mut executor)
        .expect_err("foreign download journal must reject");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupRunnerError::DownloadJournalBackupIdMismatch { expected, actual }
            if expected == "run-test" && actual == "other-backup"
    );
    assert!(
        executor
            .commands
            .iter()
            .all(|command| !command.starts_with("snapshot:"))
    );
}

#[test]
fn runner_recovers_created_snapshot_from_bound_download_journal() {
    let root = prepared_layout("canic-backup-runner-created-snapshot-recovery");
    let layout = BackupLayout::new(root.clone());
    layout
        .write_journal(&download_journal("run-test", HASH, "existing-snapshot"))
        .expect("write interrupted download journal");

    let mut executor = FakeExecutor::default();
    let response =
        backup_run_execute_with_executor(&runner_config(root.clone(), None), &mut executor)
            .expect("resume from recorded snapshot");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(response.complete);
    assert!(
        executor
            .commands
            .iter()
            .all(|command| !command.starts_with("snapshot:"))
    );
    assert!(
        executor
            .commands
            .iter()
            .any(|command| command == &format!("download:{APP}:existing-snapshot"))
    );
}

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
    let response =
        backup_run_execute_with_executor(&runner_config(root.clone(), None), &mut executor)
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

// Ensure root-omitted deployment backups describe each disconnected branch separately.
#[test]
fn runner_finalizes_root_omitted_deployment_plan_as_multiple_backup_units() {
    let root = temp_dir("canic-backup-runner-root-omitted-deployment");
    let layout = BackupLayout::new(root.clone());
    let plan = root_omitted_deployment_plan();
    let journal = BackupExecutionJournal::from_plan(&plan).expect("execution journal");
    layout.write_backup_plan(&plan).expect("write plan");
    layout
        .write_execution_journal(&journal)
        .expect("write execution journal");

    let mut executor = FakeExecutor::default();
    let response =
        backup_run_execute_with_executor(&runner_config(root.clone(), None), &mut executor)
            .expect("run backup");
    let manifest = layout.read_manifest().expect("read manifest");
    manifest.validate().expect("valid manifest");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(response.complete);
    assert_eq!(manifest.consistency.backup_units.len(), 2);
    assert_eq!(
        manifest.consistency.backup_units[0].unit_id,
        "backup-selection-1"
    );
    assert_eq!(
        manifest.consistency.backup_units[0].kind,
        BackupUnitKind::Single
    );
    assert_eq!(
        manifest.consistency.backup_units[0].roles,
        vec!["app".to_string()]
    );
    assert_eq!(
        manifest.consistency.backup_units[1].unit_id,
        "backup-selection-2"
    );
    assert_eq!(
        manifest.consistency.backup_units[1].kind,
        BackupUnitKind::Single
    );
    assert_eq!(
        manifest.consistency.backup_units[1].roles,
        vec!["worker".to_string()]
    );
}

// Ensure max-step capped runs can resume without replaying preflight or completed operations.
#[test]
fn runner_resumes_after_max_steps_without_replaying_completed_work() {
    let root = prepared_layout("canic-backup-runner-resume");

    let mut first_executor = FakeExecutor::default();
    let first = backup_run_execute_with_executor(
        &runner_config(root.clone(), Some(2)),
        &mut first_executor,
    )
    .expect("first capped run");

    assert!(!first.complete);
    assert!(first.max_steps_reached);
    assert_eq!(first.executed_operation_count, 2);
    assert!(first.execution.preflight_accepted);
    assert!(first.execution.restart_required);
    assert_eq!(
        first_executor.commands,
        vec![
            format!("status:{APP}"),
            format!("stop:{APP}"),
            format!("snapshot:{APP}"),
        ]
    );

    let mut second_executor = FakeExecutor::default();
    let second =
        backup_run_execute_with_executor(&runner_config(root.clone(), None), &mut second_executor)
            .expect("resume run");
    let integrity = BackupLayout::new(root.clone())
        .verify_integrity()
        .expect("verify resumed layout");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(second.complete);
    assert!(!second.max_steps_reached);
    assert_eq!(second.executed_operation_count, 4);
    assert_eq!(second.execution.failed_operations, 0);
    assert_eq!(integrity.durable_artifacts, 1);
    assert_eq!(
        second_executor.commands,
        vec![format!("start:{APP}"), format!("download:{APP}:snap-app"),]
    );
}

// Ensure a published artifact is reverified after interruption before its durable journal update.
#[test]
fn runner_recovers_artifact_published_before_durable_journal_transition() {
    let root = prepared_layout("canic-backup-runner-published-artifact-resume");
    let layout = BackupLayout::new(root.clone());

    let mut first_executor = FakeExecutor::default();
    let first = backup_run_execute_with_executor(
        &runner_config(root.clone(), Some(5)),
        &mut first_executor,
    )
    .expect("run through artifact verification");
    assert!(!first.complete);
    assert_eq!(first.executed_operation_count, 5);

    let interrupted_journal = layout.read_journal().expect("read interrupted journal");
    let interrupted_entry = &interrupted_journal.artifacts[0];
    assert_eq!(
        interrupted_entry.state,
        crate::journal::ArtifactState::ChecksumVerified
    );
    let temporary = PathBuf::from(
        interrupted_entry
            .temp_path
            .as_deref()
            .expect("verified artifact temp path"),
    );
    let canonical = root.join(&interrupted_entry.artifact_path);
    fs::rename(&temporary, &canonical).expect("simulate publication before journal transition");

    let mut resumed_executor = FakeExecutor::default();
    let resumed =
        backup_run_execute_with_executor(&runner_config(root.clone(), None), &mut resumed_executor)
            .expect("recover published artifact");
    let recovered_journal = layout.read_journal().expect("read recovered journal");
    let recovered_entry = &recovered_journal.artifacts[0];
    let integrity = layout.verify_integrity().expect("verify recovered layout");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(resumed.complete);
    assert_eq!(resumed.executed_operation_count, 1);
    assert!(resumed_executor.commands.is_empty());
    assert_eq!(
        recovered_entry.state,
        crate::journal::ArtifactState::Durable
    );
    assert!(recovered_entry.temp_path.is_none());
    assert_eq!(integrity.durable_artifacts, 1);
}

// Ensure recovery never adopts a published directory whose bytes differ from the journal checksum.
#[test]
fn runner_rejects_unverifiable_post_publication_artifact() {
    let root = prepared_layout("canic-backup-runner-unverifiable-artifact");
    let layout = BackupLayout::new(root.clone());

    let mut first_executor = FakeExecutor::default();
    backup_run_execute_with_executor(&runner_config(root.clone(), Some(5)), &mut first_executor)
        .expect("run through artifact verification");
    let interrupted_journal = layout.read_journal().expect("read interrupted journal");
    let interrupted_entry = &interrupted_journal.artifacts[0];
    let temporary = PathBuf::from(
        interrupted_entry
            .temp_path
            .as_deref()
            .expect("verified artifact temp path"),
    );
    let canonical = root.join(&interrupted_entry.artifact_path);
    fs::rename(&temporary, &canonical).expect("simulate interrupted publication");
    fs::write(canonical.join("snapshot.bin"), b"different snapshot")
        .expect("change published artifact");

    let mut resumed_executor = FakeExecutor::default();
    let error =
        backup_run_execute_with_executor(&runner_config(root.clone(), None), &mut resumed_executor)
            .expect_err("changed published artifact must reject");
    let rejected_journal = layout.read_journal().expect("read rejected journal");

    std::assert_matches!(
        error,
        BackupRunnerError::Persistence(crate::persistence::PersistenceError::Checksum(
            crate::artifacts::ArtifactChecksumError::ChecksumMismatch { .. }
        ))
    );
    assert_eq!(
        rejected_journal.artifacts[0].state,
        crate::journal::ArtifactState::ChecksumVerified
    );
    assert!(rejected_journal.artifacts[0].temp_path.is_some());

    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure command failures are durably journaled and can be retried without replaying prior work.
#[test]
fn runner_records_failed_operation_and_retries_from_that_operation() {
    let root = prepared_layout("canic-backup-runner-retry");

    let mut failing_executor = FakeExecutor {
        fail_on: Some(FakeFailure::CreateSnapshot),
        ..FakeExecutor::default()
    };
    let err =
        backup_run_execute_with_executor(&runner_config(root.clone(), None), &mut failing_executor)
            .expect_err("snapshot failure aborts run");
    let failed_journal = BackupLayout::new(root.clone())
        .read_execution_journal()
        .expect("read failed execution journal");
    let failed_summary = failed_journal.resume_summary();

    std::assert_matches!(
        err,
        BackupRunnerError::CommandFailed {
            sequence: 5,
            status,
            message,
        } if status == "snapshot" && message == "simulated snapshot failure"
    );
    assert!(failed_summary.restart_required);
    assert_eq!(failed_summary.failed_operations, 1);
    assert_eq!(
        failed_summary.next_operation.expect("failed op").state,
        BackupExecutionOperationState::Failed
    );
    assert_eq!(
        failing_executor.commands,
        vec![
            format!("status:{APP}"),
            format!("stop:{APP}"),
            format!("snapshot:{APP}"),
        ]
    );

    let mut retry_executor = FakeExecutor::default();
    let response =
        backup_run_execute_with_executor(&runner_config(root.clone(), None), &mut retry_executor)
            .expect("retry run");
    let integrity = BackupLayout::new(root.clone())
        .verify_integrity()
        .expect("verify retry layout");

    fs::remove_dir_all(root).expect("remove temp root");
    assert!(response.complete);
    assert_eq!(response.execution.failed_operations, 0);
    assert_eq!(integrity.durable_artifacts, 1);
    assert_eq!(
        retry_executor.commands,
        vec![
            format!("snapshot:{APP}"),
            format!("start:{APP}"),
            format!("download:{APP}:snap-app"),
        ]
    );
}

// Ensure a second runner cannot mutate a backup while the execution journal is locked.
#[test]
fn runner_rejects_locked_execution_journal_before_running_commands() {
    let root = prepared_layout("canic-backup-runner-lock");
    let layout = BackupLayout::new(root.clone());
    fs::write(execution_journal_lock_path(&layout), b"pid=1\n").expect("write lock");

    let mut executor = FakeExecutor::default();
    let err = backup_run_execute_with_executor(&runner_config(root.clone(), None), &mut executor)
        .expect_err("locked journal rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupRunnerError::JournalLocked { lock_path } if lock_path.ends_with("backup-execution-journal.json.lock")
    );
    assert!(executor.commands.is_empty());
}

// Ensure failed preflight evidence does not accept the preflight or unblock mutation.
#[test]
fn runner_preflight_failure_leaves_mutation_blocked() {
    let root = prepared_layout("canic-backup-runner-preflight-failure");

    let mut executor = FakeExecutor {
        fail_on: Some(FakeFailure::Preflight),
        ..FakeExecutor::default()
    };
    let err = backup_run_execute_with_executor(&runner_config(root.clone(), None), &mut executor)
        .expect_err("preflight failure rejects");
    let journal = BackupLayout::new(root.clone())
        .read_execution_journal()
        .expect("read execution journal");
    let summary = journal.resume_summary();

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupRunnerError::PreflightFailed {
            status,
            message,
        } if status == "preflight" && message == "simulated preflight failure"
    );
    assert_eq!(executor.commands, vec![format!("status:{APP}")]);
    assert!(!summary.preflight_accepted);
    assert_eq!(summary.completed_operations, 0);
    assert_eq!(summary.failed_operations, 0);
    assert!(
        journal
            .operations
            .iter()
            .filter(|operation| {
                matches!(
                    operation.kind,
                    BackupOperationKind::Stop
                        | BackupOperationKind::CreateSnapshot
                        | BackupOperationKind::Start
                        | BackupOperationKind::DownloadSnapshot
                )
            })
            .all(|operation| operation.state == BackupExecutionOperationState::Blocked)
    );
}

// Ensure a tampered temp path in a resumable download journal is not trusted.
#[test]
fn runner_rejects_unexpected_download_temp_path() {
    let root = prepared_layout("canic-backup-runner-temp-path");

    let mut first_executor = FakeExecutor::default();
    backup_run_execute_with_executor(&runner_config(root.clone(), Some(4)), &mut first_executor)
        .expect("run through download");

    let layout = BackupLayout::new(root.clone());
    let mut journal = layout.read_journal().expect("read download journal");
    journal.artifacts[0].temp_path = Some("/tmp/canic-backup-outside".to_string());
    layout
        .write_journal(&journal)
        .expect("write tampered journal");

    let mut second_executor = FakeExecutor::default();
    let err =
        backup_run_execute_with_executor(&runner_config(root.clone(), None), &mut second_executor)
            .expect_err("tampered temp path rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(err, BackupRunnerError::ArtifactTempPathMismatch { .. });
}

#[derive(Default)]
struct FakeExecutor {
    commands: Vec<String>,
    fail_on: Option<FakeFailure>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum FakeFailure {
    Preflight,
    CreateSnapshot,
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
        if self.fail_on == Some(FakeFailure::Preflight) {
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

    fn create_snapshot(
        &mut self,
        canister_id: &str,
    ) -> Result<BackupRunnerSnapshotReceipt, BackupRunnerCommandError> {
        self.commands.push(format!("snapshot:{canister_id}"));
        if self.fail_on == Some(FakeFailure::CreateSnapshot) {
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

fn prepared_layout(name: &str) -> PathBuf {
    let root = temp_dir(name);
    let layout = BackupLayout::new(root.clone());
    let plan = plan();
    let journal = BackupExecutionJournal::from_plan(&plan).expect("execution journal");
    layout.write_backup_plan(&plan).expect("write plan");
    layout
        .write_execution_journal(&journal)
        .expect("write execution journal");
    root
}

fn download_journal(backup_id: &str, topology_hash: &str, snapshot_id: &str) -> DownloadJournal {
    DownloadJournal {
        journal_version: 1,
        backup_id: backup_id.to_string(),
        discovery_topology_hash: topology_hash.to_string(),
        pre_snapshot_topology_hash: HASH.to_string(),
        operation_metrics: DownloadOperationMetrics::default(),
        artifacts: vec![ArtifactJournalEntry {
            canister_id: APP.to_string(),
            snapshot_id: snapshot_id.to_string(),
            state: ArtifactState::Created,
            temp_path: None,
            artifact_path: APP.to_string(),
            checksum_algorithm: "sha256".to_string(),
            checksum: None,
            updated_at: "unix:1".to_string(),
        }],
    }
}

fn execution_journal_lock_path(layout: &BackupLayout) -> PathBuf {
    let mut lock_path = layout.execution_journal_path().as_os_str().to_os_string();
    lock_path.push(".lock");
    PathBuf::from(lock_path)
}

fn runner_config(out: PathBuf, max_steps: Option<usize>) -> BackupRunnerConfig {
    BackupRunnerConfig {
        out,
        max_steps,
        updated_at: Some("unix:10".to_string()),
        tool_name: "canic".to_string(),
        tool_version: "test".to_string(),
    }
}

fn plan() -> BackupPlan {
    build_backup_plan(BackupPlanBuildInput {
        plan_id: "plan-test".to_string(),
        run_id: "run-test".to_string(),
        fleet: "demo".to_string(),
        environment: "local".to_string(),
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

fn root_omitted_deployment_plan() -> BackupPlan {
    build_backup_plan(BackupPlanBuildInput {
        plan_id: "plan-test".to_string(),
        run_id: "run-test".to_string(),
        fleet: "demo".to_string(),
        environment: "local".to_string(),
        root_canister_id: ROOT.to_string(),
        selected_canister_id: None,
        selected_scope_kind: BackupScopeKind::NonRootDeployment,
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
            RegistryEntry {
                pid: WORKER.to_string(),
                role: Some("worker".to_string()),
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
