use super::*;
use crate::test_support::temp_dir;
use std::{
    io::BufRead,
    process::{Command, Stdio},
};

fn context(root: PathBuf) -> WorkspaceBuildContext {
    WorkspaceBuildContext {
        role: "root".into(),
        profile: crate::canister_build::CanisterBuildProfile::Fast,
        environment: "local".into(),
        build_network: canic_core::ids::BuildNetwork::Local,
        workspace_root: root.clone(),
        icp_root: root.clone(),
        config_path: root.join("canic.toml"),
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: None,
    }
}

#[test]
fn contender_reports_owner_and_recovers_after_interrupted_ownership() {
    const CHILD: &str = "CANIC_TEST_LOCK_OWNER_ROOT";
    if let Some(root) = std::env::var_os(CHILD) {
        let _lock = BuildLock::acquire(&context(root.into()), |_| {}).unwrap();
        println!("OWNER_READY");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        return;
    }
    let root = temp_dir("build-owner");
    let context = context(root.clone());
    let path = root.join(".canic/locks/complete-build-reuse.lock");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            std::thread::current().name().unwrap(),
            "--nocapture",
        ])
        .env(CHILD, &root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = io::BufReader::new(child.stdout.take().unwrap());
    loop {
        let mut line = String::new();
        assert_ne!(stdout.read_line(&mut line).unwrap(), 0);
        if line.contains("OWNER_READY") {
            break;
        }
    }
    let mut observed = false;
    let lock = BuildLock::acquire(&context, |progress| {
        if let BuildReuseProgress::WaitingForLock(wait) = progress {
            let owner = wait.recorded_owner.unwrap();
            assert_eq!(wait.lock_path, path);
            assert_eq!(owner.pid, child.id());
            assert_eq!(owner.workspace, root);
            assert_eq!(owner.profile, "fast");
            assert!(owner.started_at_unix_seconds > 0);
            observed = true;
            // Simulate a crash: no Drop cleanup, and no lock-file deletion.
            child.kill().unwrap();
        }
    })
    .unwrap();
    child.wait().unwrap();
    assert!(observed);
    assert_eq!(read_owner(&lock.0).unwrap().pid, std::process::id());
    let contender = fs::File::open(&path).unwrap();
    assert_eq!(
        rustix::fs::flock(
            &contender,
            rustix::fs::FlockOperation::NonBlockingLockExclusive
        ),
        Err(rustix::io::Errno::WOULDBLOCK)
    );
    drop(lock);
    assert!(fs::read(&path).unwrap().is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_owner_metadata_is_bounded_and_never_lock_authority() {
    let root = temp_dir("build-owner-invalid");
    let context = context(root.clone());
    let lock = BuildLock::acquire(&context, |_| {}).unwrap();
    for bytes in [vec![], b"not-json".to_vec(), vec![b'x'; OWNER_LIMIT + 1]] {
        lock.0.set_len(0).unwrap();
        let mut file = &lock.0;
        file.seek(SeekFrom::Start(0)).unwrap();
        file.write_all(&bytes).unwrap();
        assert!(read_owner(&lock.0).is_none());
    }
    drop(lock);
    let recovered = BuildLock::acquire(&context, |_| {}).unwrap();
    assert!(read_owner(&recovered.0).is_some());
    drop(recovered);
    fs::remove_dir_all(root).unwrap();
}
