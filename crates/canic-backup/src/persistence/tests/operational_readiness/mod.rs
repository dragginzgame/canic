//! Module: persistence::tests::operational_readiness
//!
//! Responsibility: execute persistence-owned 0.94 crash and verification cases.
//! Does not own: the frozen case manifest or production recovery policy.
//! Boundary: binds deterministic process loss to real durable layout operations.

use super::*;
use crate::{
    operational_readiness::manifest::assert_case_defined,
    persistence::json::{DurableWriteBarrier, write_json_durable_at_barriers},
};

#[cfg(unix)]
use std::{
    io::{self, Read},
    process::{Child, Command},
    thread,
    time::{Duration, Instant},
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
        .write_manifest(&valid_manifest())
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
fn kill_child_at_acknowledged_barrier(child: &mut Child, root: &Path) {
    let ready_path = root.join("barrier-ready");
    let acknowledge_path = root.join("barrier-acknowledged");
    let armed_path = root.join("barrier-armed");
    wait_for_child_path(child, &ready_path, "child barrier");
    fs::write(&acknowledge_path, b"acknowledged\n").expect("acknowledge child barrier");
    wait_for_child_path(child, &armed_path, "armed child barrier");
    child.kill().expect("kill child at acknowledged barrier");
    child.wait().expect("reap killed child");
}

#[cfg(unix)]
fn hold_at_acknowledged_barrier(root: &Path) -> ! {
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
fn wait_for_child_path(child: &mut Child, path: &Path, description: &str) {
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
fn wait_for_path(path: &Path, description: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !path.is_file() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {description}"
        );
        thread::sleep(Duration::from_millis(10));
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
