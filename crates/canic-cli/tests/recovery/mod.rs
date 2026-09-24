//! Executable inspection across real kernel-lock ownership and stale metadata.
//!
//! The subprocess owns a disposable lock; no application build is terminated.

use canic_host::canister_build::{BuildLockOwner, BuildLockPhase, BuildProcessIdentity};
use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const FIXTURE_ROOT: &str = "CANIC_CLI_LOCK_RECOVERY_ROOT";
const FIXTURE_ROLE: &str = "CANIC_CLI_LOCK_RECOVERY_ROLE";

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Process(Child);

impl Process {
    fn spawn(root: &Path, role: &str) -> Self {
        Self(
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "recovery::lock_owner_fixture", "--nocapture"])
                .env(FIXTURE_ROOT, root)
                .env(FIXTURE_ROLE, role)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .spawn()
                .unwrap(),
        )
    }

    fn wait_for(&mut self, marker: &Path) {
        let deadline = Instant::now() + Duration::from_secs(15);
        while !marker.exists() {
            assert!(self.0.try_wait().unwrap().is_none(), "fixture exited early");
            assert!(
                Instant::now() < deadline,
                "fixture did not reach {}",
                marker.display()
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn lock_owner_fixture() {
    let Some(root) = std::env::var_os(FIXTURE_ROOT).map(PathBuf::from) else {
        return;
    };
    let role = std::env::var(FIXTURE_ROLE).unwrap();
    fs::write(root.join(format!("{role}-started")), b"").unwrap();
    let lock = canic_host::durable_io::lock_file(&root.join("lock")).unwrap();
    let pid = std::process::id();
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
    let start_ticks = stat
        .rsplit_once(") ")
        .unwrap()
        .1
        .split_ascii_whitespace()
        .nth(19)
        .unwrap()
        .parse()
        .unwrap();
    let owner = BuildLockOwner {
        pid,
        profile: "fast".into(),
        workspace: root.clone(),
        started_at_unix_seconds: 1,
        identity: Some(BuildProcessIdentity {
            boot_id: fs::read_to_string("/proc/sys/kernel/random/boot_id")
                .unwrap()
                .trim()
                .into(),
            pid_namespace: fs::read_link("/proc/self/ns/pid")
                .unwrap()
                .to_str()
                .unwrap()
                .into(),
            start_ticks,
        }),
        phase: BuildLockPhase::BuildingArtifacts,
        phase_started_at_unix_seconds: 1,
    };
    fs::write(root.join("lock"), serde_json::to_vec(&owner).unwrap()).unwrap();
    fs::write(root.join(format!("{role}-ready")), b"").unwrap();
    // EOF requests normal release; the crash case kills only this owned fixture.
    std::io::stdin().read_line(&mut String::new()).unwrap();
    lock.set_len(0).unwrap();
}

fn inspect(lock: &Path) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_canic"))
        .args(["diagnostic", "build-lock", "--lock"])
        .arg(lock)
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn inspection_preserves_exclusion_through_stale_identity_crash_and_handoff() {
    inspect_recovery(true);
}

#[test]
fn inspection_preserves_exclusion_through_normal_owner_handoff() {
    inspect_recovery(false);
}

fn inspect_recovery(crash: bool) {
    let root = Scratch(std::env::temp_dir().join(format!(
        "canic-cli-lock-recovery-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    )));
    fs::create_dir(&root.0).unwrap();
    let lock = root.0.join("lock");
    let mut owner = Process::spawn(&root.0, "owner");
    owner.wait_for(&root.0.join("owner-ready"));
    let inode = fs::metadata(&lock).unwrap().ino();
    let original = fs::read(&lock).unwrap();
    let valid: BuildLockOwner = serde_json::from_slice(&original).unwrap();
    let matched = inspect(&lock);
    assert_eq!(matched["kernel"]["pid"], owner.0.id());
    assert_eq!(matched["owner_visibility"], "matched");

    let mut wrong_birth = valid.clone();
    wrong_birth.identity.as_mut().unwrap().start_ticks += 1;
    let mut wrong_namespace = valid.clone();
    wrong_namespace.identity.as_mut().unwrap().pid_namespace = "pid:[0]".into();
    let mut unknown = valid;
    unknown.identity = None;
    for (metadata, expected) in [
        (
            serde_json::to_vec(&wrong_birth).unwrap(),
            "identity_mismatch",
        ),
        (
            serde_json::to_vec(&wrong_namespace).unwrap(),
            "identity_mismatch",
        ),
        (serde_json::to_vec(&unknown).unwrap(), "unavailable"),
        (b"malformed".to_vec(), "unavailable"),
    ] {
        fs::write(&lock, &metadata).unwrap();
        let report = inspect(&lock);
        assert_eq!(report["kernel"]["pid"], owner.0.id());
        assert_eq!(report["owner_visibility"], expected);
        assert_eq!(report["processes"], serde_json::json!([]));
        assert_eq!(report["process_snapshot_complete"], false);
        assert_eq!(fs::read(&lock).unwrap(), metadata);
        let contender = fs::File::open(&lock).unwrap();
        assert_eq!(
            rustix::fs::flock(
                &contender,
                rustix::fs::FlockOperation::NonBlockingLockExclusive
            ),
            Err(rustix::io::Errno::WOULDBLOCK)
        );
    }
    fs::write(&lock, &original).unwrap();
    let mut survivor = Process::spawn(&root.0, "survivor");
    survivor.wait_for(&root.0.join("survivor-started"));
    assert!(!root.0.join("survivor-ready").exists());
    assert_eq!(inspect(&lock)["kernel"]["pid"], owner.0.id());
    if crash {
        owner.0.kill().unwrap();
        assert!(!owner.0.wait().unwrap().success());
    } else {
        drop(owner.0.stdin.take());
        assert!(owner.0.wait().unwrap().success());
    }
    survivor.wait_for(&root.0.join("survivor-ready"));
    let handed_off = inspect(&lock);
    assert_eq!(handed_off["kernel"]["pid"], survivor.0.id());
    assert_eq!(handed_off["recorded_owner"]["pid"], survivor.0.id());
    assert_eq!(handed_off["owner_visibility"], "matched");
    assert_eq!(fs::metadata(&lock).unwrap().ino(), inode);
    drop(survivor.0.stdin.take());
    assert!(survivor.0.wait().unwrap().success());
    assert!(fs::read(&lock).unwrap().is_empty());
    assert_eq!(inspect(&lock)["kernel"]["status"], "not_observed");
    // An exited owner's retained metadata cannot become a matched process.
    fs::write(&lock, &original).unwrap();
    let abandoned = inspect(&lock);
    assert_eq!(abandoned["kernel"]["status"], "not_observed");
    assert_ne!(abandoned["owner_visibility"], "matched");
    assert_eq!(abandoned["processes"], serde_json::json!([]));
    assert_eq!(abandoned["process_snapshot_complete"], false);
    assert_eq!(fs::read(&lock).unwrap(), original);
}
