//! Module: persistence::tests::operational_readiness
//!
//! Responsibility: execute persistence-owned 0.94 crash and verification cases.
//! Does not own: the frozen case manifest or production recovery policy.
//! Boundary: binds deterministic process loss to real durable layout operations.

#[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
mod artifact_publication;
#[cfg(unix)]
mod checksum_effect;
#[cfg(unix)]
mod checksum_transition;
#[cfg(unix)]
mod command_in_flight;
#[cfg(unix)]
mod download_effect;
#[cfg(unix)]
mod download_transition;
#[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
mod durable_transition;
mod lifecycle_effect;
#[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
mod manifest_publication;
mod pending_claim;
#[cfg(unix)]
mod response_loss;
#[cfg(unix)]
mod snapshot_create;
#[cfg(unix)]
mod terminal_transition;

use super::*;
use crate::{
    operational_readiness::manifest::assert_case_defined,
    persistence::{DurableWriteBarrier, write_json_durable_at_barriers},
    plan::{AuthorityEvidence, BackupOperationKind},
    runner::{BackupRunnerConfig, BackupRunnerExecutor, backup_run_execute_with_executor},
    test_support::{
        FakeBackupRunnerExecutor, hold_at_acknowledged_barrier, kill_child_at_acknowledged_barrier,
        wait_for_child_path, wait_for_path,
    },
};

#[cfg(unix)]
use std::{
    io::{self, Read},
    process::Command,
};

#[cfg(unix)]
const CRASH_CHILD_ROOT_ENV: &str = "CANIC_TEST_DURABLE_WRITE_ROOT";
#[cfg(unix)]
const CRASH_CHILD_BARRIER_ENV: &str = "CANIC_TEST_DURABLE_WRITE_BARRIER";
#[cfg(unix)]
const VERIFY_CHILD_ROOT_ENV: &str = "CANIC_TEST_VERIFY_CRASH_ROOT";
#[cfg(unix)]
const VERIFY_CHILD_BARRIER_ENV: &str = "CANIC_TEST_VERIFY_CRASH_BARRIER";
#[cfg(unix)]
const VERIFY_CHILD_HANDSHAKE_ENV: &str = "CANIC_TEST_VERIFY_CRASH_HANDSHAKE";
#[cfg(unix)]
const PREFLIGHT_CHILD_ROOT_ENV: &str = "CANIC_TEST_PREFLIGHT_CRASH_ROOT";
#[cfg(unix)]
const PREFLIGHT_CHILD_DOCUMENT_ENV: &str = "CANIC_TEST_PREFLIGHT_CRASH_DOCUMENT";
#[cfg(unix)]
const PREFLIGHT_CHILD_BARRIER_ENV: &str = "CANIC_TEST_PREFLIGHT_CRASH_BARRIER";
#[cfg(unix)]
const PREFLIGHT_CHILD_HANDSHAKE_ENV: &str = "CANIC_TEST_PREFLIGHT_CRASH_HANDSHAKE";

#[cfg(unix)]
#[test]
fn execution_journal_publication_survives_process_death_on_both_write_sides() {
    let Some(root) = std::env::var_os(CRASH_CHILD_ROOT_ENV) else {
        for (barrier_name, case_id) in [
            (
                "before-rename",
                "CANIC-094-B01/execution-journal-publication/before-durable-write",
            ),
            (
                "after-directory-sync",
                "CANIC-094-B01/execution-journal-publication/after-durable-write",
            ),
        ] {
            assert_case_defined(case_id);
            prove_execution_journal_publication_barrier(barrier_name);
        }
        return;
    };

    let barrier_name = std::env::var(CRASH_CHILD_BARRIER_ENV).expect("child barrier name");
    let target = match barrier_name.as_str() {
        "before-rename" => DurableWriteBarrier::BeforeRename,
        "after-directory-sync" => DurableWriteBarrier::AfterDirectorySync,
        _ => panic!("unsupported durable-write barrier: {barrier_name}"),
    };
    let root = std::path::PathBuf::from(root);
    let layout = BackupLayout::new(root.clone());

    write_json_durable_at_barriers(
        &layout.execution_journal_path(),
        &valid_execution_journal(),
        |barrier| {
            if barrier != target {
                return;
            }
            hold_at_acknowledged_barrier(&root);
        },
    )
    .expect("write execution journal in crash child");
}

#[cfg(unix)]
#[test]
fn verification_survives_process_death_without_mutating_layout() {
    let Some(root) = std::env::var_os(VERIFY_CHILD_ROOT_ENV) else {
        for (barrier_name, case_id) in [
            (
                "before-validation",
                "CANIC-094-V01/before-document-validation/interrupted",
            ),
            (
                "during-checksum",
                "CANIC-094-V02/during-artifact-checksum/interrupted",
            ),
            (
                "after-result",
                "CANIC-094-V03/after-result-before-output/interrupted",
            ),
        ] {
            assert_case_defined(case_id);
            prove_verification_barrier(barrier_name);
        }
        return;
    };

    let root = std::path::PathBuf::from(root);
    let handshake_root = std::path::PathBuf::from(
        std::env::var_os(VERIFY_CHILD_HANDSHAKE_ENV).expect("verify handshake root"),
    );
    let barrier_name = std::env::var(VERIFY_CHILD_BARRIER_ENV).expect("verify barrier name");
    if barrier_name == "before-validation" {
        hold_at_acknowledged_barrier(&handshake_root);
    }
    if barrier_name == "during-checksum" {
        let file = fs::File::open(root.join("artifacts/root"))
            .expect("open verification artifact in crash child");
        let mut reader = InterruptingReader::new(file, handshake_root.clone());
        ArtifactChecksum::from_reader(&mut reader).expect("checksum artifact in crash child");
    }
    BackupLayout::new(root)
        .verify_integrity()
        .expect("verify layout in crash child");
    if barrier_name == "after-result" {
        hold_at_acknowledged_barrier(&handshake_root);
    }
    panic!("unsupported or unarmed verification barrier: {barrier_name}");
}

#[cfg(unix)]
#[test]
fn preflight_publications_survive_process_death_without_starting_mutation() {
    let Some(root) = std::env::var_os(PREFLIGHT_CHILD_ROOT_ENV) else {
        for (document, barrier_name, case_id) in [
            (
                "plan",
                "before-rename",
                "CANIC-094-B02/preflight-applied-plan-publication/before-durable-write",
            ),
            (
                "plan",
                "after-directory-sync",
                "CANIC-094-B02/preflight-applied-plan-publication/after-durable-write",
            ),
            (
                "journal",
                "before-rename",
                "CANIC-094-B03/preflight-acceptance/before-durable-write",
            ),
            (
                "journal",
                "after-directory-sync",
                "CANIC-094-B03/preflight-acceptance/after-durable-write",
            ),
        ] {
            assert_case_defined(case_id);
            prove_preflight_publication_barrier(document, barrier_name);
        }
        return;
    };

    let root = std::path::PathBuf::from(root);
    let document = std::env::var(PREFLIGHT_CHILD_DOCUMENT_ENV).expect("preflight document");
    let barrier_name = std::env::var(PREFLIGHT_CHILD_BARRIER_ENV).expect("preflight barrier name");
    let handshake_root = std::path::PathBuf::from(
        std::env::var_os(PREFLIGHT_CHILD_HANDSHAKE_ENV).expect("preflight handshake root"),
    );
    let layout = BackupLayout::new(root);
    let initial_plan = declared_backup_plan();
    let (updated_plan, accepted_journal) = accepted_preflight_documents(&initial_plan);

    match document.as_str() {
        "plan" => write_document_at_barrier(
            &layout.backup_plan_path(),
            &updated_plan,
            &barrier_name,
            &handshake_root,
        ),
        "journal" => write_document_at_barrier(
            &layout.execution_journal_path(),
            &accepted_journal,
            &barrier_name,
            &handshake_root,
        ),
        _ => panic!("unsupported preflight crash document: {document}"),
    }
}

#[cfg(unix)]
struct InterruptingReader {
    inner: fs::File,
    handshake_root: std::path::PathBuf,
    reached_barrier: bool,
}

#[cfg(unix)]
impl InterruptingReader {
    fn new(inner: fs::File, handshake_root: std::path::PathBuf) -> Self {
        Self {
            inner,
            handshake_root,
            reached_barrier: false,
        }
    }
}

#[cfg(unix)]
impl Read for InterruptingReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buffer)?;
        if read > 0 && !self.reached_barrier {
            self.reached_barrier = true;
            hold_at_acknowledged_barrier(&self.handshake_root);
        }
        Ok(read)
    }
}

#[cfg(unix)]
fn prove_execution_journal_publication_barrier(barrier_name: &str) {
    let root = temp_dir(&format!("canic-backup-execution-journal-{barrier_name}"));
    fs::create_dir_all(&root).expect("create crash layout");
    let layout = BackupLayout::new(root.clone());
    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "persistence::tests::operational_readiness::execution_journal_publication_survives_process_death_on_both_write_sides",
            "--nocapture",
        ])
        .env(CRASH_CHILD_ROOT_ENV, &root)
        .env(CRASH_CHILD_BARRIER_ENV, barrier_name)
        .spawn()
        .expect("spawn durable-write child");

    kill_child_at_acknowledged_barrier(&mut child, &root);

    let expected = valid_execution_journal();
    if barrier_name == "before-rename" {
        assert!(!layout.execution_journal_path().exists());
        layout
            .write_execution_journal(&expected)
            .expect("restart publishes complete execution journal");
    }
    let recovered = layout
        .read_execution_journal()
        .expect("restart reads complete execution journal");

    assert_eq!(recovered, expected);
    fs::remove_dir_all(root).expect("remove crash layout");
}

#[cfg(unix)]
fn prove_verification_barrier(barrier_name: &str) {
    let root = temp_dir(&format!("canic-backup-verification-{barrier_name}"));
    let handshake_root = temp_dir(&format!(
        "canic-backup-verification-handshake-{barrier_name}"
    ));
    fs::create_dir_all(&handshake_root).expect("create verification handshake root");
    let layout = BackupLayout::new(root.clone());
    let checksum = write_artifact(&root, b"root artifact");
    layout
        .publish_manifest(&valid_manifest())
        .expect("write manifest");
    layout
        .write_journal(&journal_with_checksum(checksum.hash))
        .expect("write journal");
    let before = snapshot_layout_tree(&root);
    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "persistence::tests::operational_readiness::verification_survives_process_death_without_mutating_layout",
            "--nocapture",
        ])
        .env(VERIFY_CHILD_ROOT_ENV, &root)
        .env(VERIFY_CHILD_BARRIER_ENV, barrier_name)
        .env(VERIFY_CHILD_HANDSHAKE_ENV, &handshake_root)
        .spawn()
        .expect("spawn verification child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake_root);
    let recovered = layout
        .verify_integrity()
        .expect("restart repeats complete verification");
    let after = snapshot_layout_tree(&root);

    assert!(recovered.verified);
    assert_eq!(after, before);
    fs::remove_dir_all(root).expect("remove verification layout");
    fs::remove_dir_all(handshake_root).expect("remove verification handshake root");
}

#[cfg(unix)]
fn prove_preflight_publication_barrier(document: &str, barrier_name: &str) {
    let root = temp_dir(&format!("canic-backup-preflight-{document}-{barrier_name}"));
    let handshake_root = temp_dir(&format!(
        "canic-backup-preflight-handshake-{document}-{barrier_name}"
    ));
    fs::create_dir_all(&handshake_root).expect("create preflight handshake root");
    let layout = BackupLayout::new(root.clone());
    let initial_plan = declared_backup_plan();
    let initial_journal =
        BackupExecutionJournal::from_plan(&initial_plan).expect("initial execution journal");
    let (updated_plan, accepted_journal) = accepted_preflight_documents(&initial_plan);
    layout
        .write_backup_plan(if document == "journal" {
            &updated_plan
        } else {
            &initial_plan
        })
        .expect("write pre-crash backup plan");
    layout
        .write_execution_journal(&initial_journal)
        .expect("write pre-crash execution journal");
    let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .args([
            "--exact",
            "persistence::tests::operational_readiness::preflight_publications_survive_process_death_without_starting_mutation",
            "--nocapture",
        ])
        .env(PREFLIGHT_CHILD_ROOT_ENV, &root)
        .env(PREFLIGHT_CHILD_DOCUMENT_ENV, document)
        .env(PREFLIGHT_CHILD_BARRIER_ENV, barrier_name)
        .env(PREFLIGHT_CHILD_HANDSHAKE_ENV, &handshake_root)
        .spawn()
        .expect("spawn preflight publication child");

    kill_child_at_acknowledged_barrier(&mut child, &handshake_root);
    let observed_plan = layout.read_backup_plan().expect("read plan after crash");
    let observed_journal = layout
        .read_execution_journal()
        .expect("read journal after crash");

    match (document, barrier_name) {
        ("plan", "before-rename") => assert_eq!(observed_plan, initial_plan),
        ("plan", "after-directory-sync") | ("journal", _) => {
            assert_eq!(observed_plan, updated_plan);
        }
        _ => panic!("unsupported preflight proof case: {document}/{barrier_name}"),
    }
    if document == "journal" && barrier_name == "after-directory-sync" {
        assert_eq!(observed_journal, accepted_journal);
    } else {
        assert_eq!(observed_journal, initial_journal);
        assert_mutation_is_blocked(&observed_journal);
    }

    let mut executor = FakeBackupRunnerExecutor::default();
    let response = backup_run_execute_with_executor(
        &BackupRunnerConfig {
            out: root.clone(),
            max_steps: Some(0),
            updated_at: Some("unix:10".to_string()),
            tool_name: "canic".to_string(),
            tool_version: "test".to_string(),
        },
        &mut executor,
    )
    .expect("restart reconciles preflight publication");
    let final_plan = layout.read_backup_plan().expect("read reconciled plan");
    let final_journal = layout
        .read_execution_journal()
        .expect("read reconciled journal");

    assert!(!response.complete);
    assert!(response.max_steps_reached);
    assert_eq!(response.executed_operation_count, 0);
    assert_eq!(final_plan, updated_plan);
    assert_eq!(final_journal, accepted_journal);
    assert!(
        executor
            .commands
            .iter()
            .all(|command| command.starts_with("status:"))
    );
    if document == "journal" && barrier_name == "after-directory-sync" {
        assert!(executor.commands.is_empty());
    } else {
        assert_eq!(executor.commands.len(), initial_plan.targets.len());
    }

    fs::remove_dir_all(root).expect("remove preflight crash layout");
    fs::remove_dir_all(handshake_root).expect("remove preflight handshake root");
}

fn declared_backup_plan() -> BackupPlan {
    let mut plan = valid_backup_plan();
    for target in &mut plan.targets {
        target.control_authority.evidence = AuthorityEvidence::Declared;
        target.snapshot_read_authority.evidence = AuthorityEvidence::Declared;
    }
    plan.validate().expect("declared backup plan");
    plan
}

fn accepted_preflight_documents(initial_plan: &BackupPlan) -> (BackupPlan, BackupExecutionJournal) {
    let mut executor = FakeBackupRunnerExecutor::default();
    let receipts = executor
        .preflight_receipts(initial_plan, "preflight-run-001", "unix:10", "unix:310")
        .expect("build preflight receipts");
    let mut updated_plan = initial_plan.clone();
    updated_plan
        .apply_execution_preflight_receipts(&receipts, "unix:10")
        .expect("apply preflight receipts to plan");
    let mut accepted_journal =
        BackupExecutionJournal::from_plan(initial_plan).expect("execution journal");
    accepted_journal
        .accept_preflight_receipts_at(&receipts, Some("unix:10".to_string()))
        .expect("accept preflight receipts in journal");
    (updated_plan, accepted_journal)
}

fn assert_mutation_is_blocked(journal: &BackupExecutionJournal) {
    let stop = journal
        .operations
        .iter()
        .find(|operation| operation.kind == BackupOperationKind::Stop)
        .expect("stop operation");
    assert!(!journal.preflight_accepted);
    assert_eq!(stop.state, BackupExecutionOperationState::Blocked);
}

#[cfg(unix)]
fn write_document_at_barrier<T: serde::Serialize>(
    path: &Path,
    document: &T,
    barrier_name: &str,
    handshake_root: &Path,
) -> ! {
    let target = durable_write_barrier(barrier_name);
    write_json_durable_at_barriers(path, document, |barrier| {
        if barrier == target {
            hold_at_acknowledged_barrier(handshake_root);
        }
    })
    .expect("write document in crash child");
    panic!("durable-write child passed its armed barrier");
}

#[cfg(unix)]
fn durable_write_barrier(barrier_name: &str) -> DurableWriteBarrier {
    match barrier_name {
        "before-rename" => DurableWriteBarrier::BeforeRename,
        "after-directory-sync" => DurableWriteBarrier::AfterDirectorySync,
        _ => panic!("unsupported durable-write barrier: {barrier_name}"),
    }
}

fn snapshot_layout_tree(root: &Path) -> Vec<(std::path::PathBuf, &'static str, Vec<u8>)> {
    fn visit(
        root: &Path,
        directory: &Path,
        entries: &mut Vec<(std::path::PathBuf, &'static str, Vec<u8>)>,
    ) {
        for entry in fs::read_dir(directory).expect("read verification layout") {
            let entry = entry.expect("read verification layout entry");
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .expect("layout entry under root")
                .to_path_buf();
            let file_type = entry.file_type().expect("read layout entry type");
            if file_type.is_dir() {
                entries.push((relative, "directory", Vec::new()));
                visit(root, &path, entries);
            } else if file_type.is_file() {
                entries.push((
                    relative,
                    "file",
                    fs::read(path).expect("read verification layout file"),
                ));
            } else {
                panic!("verification fixture contains unsupported entry: {relative:?}");
            }
        }
    }

    let mut entries = Vec::new();
    visit(root, root, &mut entries);
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    entries
}
