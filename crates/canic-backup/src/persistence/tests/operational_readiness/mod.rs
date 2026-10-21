//! Module: persistence::tests::operational_readiness
//!
//! Responsibility: execute retained document-publication and verification crash cases.
//! Does not own: the case manifest or production recovery policy.
//! Boundary: binds deterministic process loss to real durable layout operations.

use super::*;
use crate::{
    operational_readiness::manifest::assert_case_defined,
    persistence::{DurableWriteBarrier, write_json_durable_at_barriers},
    test_support::{hold_at_acknowledged_barrier, kill_child_at_acknowledged_barrier},
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
            hold_at_acknowledged_barrier(&root, std::time::Duration::from_secs(5));
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
        hold_at_acknowledged_barrier(&handshake_root, std::time::Duration::from_secs(5));
    }
    if barrier_name == "during-checksum" {
        let file = fs::File::open(root.join("artifacts/root"))
            .expect("open verification artifact in crash child");
        let mut reader = InterruptingReader::new(file, handshake_root.clone());
        crate::artifacts::checksum_reader(&mut reader).expect("checksum artifact in crash child");
    }
    BackupLayout::new(root)
        .verify_integrity()
        .expect("verify layout in crash child");
    if barrier_name == "after-result" {
        hold_at_acknowledged_barrier(&handshake_root, std::time::Duration::from_secs(5));
    }
    panic!("unsupported or unarmed verification barrier: {barrier_name}");
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
            hold_at_acknowledged_barrier(&self.handshake_root, std::time::Duration::from_secs(5));
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

    kill_child_at_acknowledged_barrier(&mut child, &root, std::time::Duration::from_secs(5))
        .expect("complete crash barrier handshake");

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
        .write_journal(&journal_with_checksum(checksum.hash().to_owned()))
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

    kill_child_at_acknowledged_barrier(
        &mut child,
        &handshake_root,
        std::time::Duration::from_secs(5),
    )
    .expect("complete crash barrier handshake");
    let recovered = layout
        .verify_integrity()
        .expect("restart repeats complete verification");
    let after = snapshot_layout_tree(&root);

    assert!(recovered.verified);
    assert_eq!(after, before);
    fs::remove_dir_all(root).expect("remove verification layout");
    fs::remove_dir_all(handshake_root).expect("remove verification handshake root");
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
