use super::*;
use crate::test_support::temp_dir;
use std::{
    io::BufRead,
    process::{Command, Stdio},
};

pub(super) fn context(root: PathBuf) -> WorkspaceBuildContext {
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
        let _lock = BuildLock::acquire(&context(root.into()), |_| Ok(())).unwrap();
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
            let owner = wait.inspection.recorded_owner.unwrap();
            assert_eq!(wait.inspection.lock_path, path);
            assert_eq!(owner.pid, child.id());
            assert_eq!(owner.workspace, root);
            assert_eq!(owner.profile, "fast");
            assert!(owner.started_at_unix_seconds > 0);
            observed = true;
            // Simulate a crash: no Drop cleanup, and no lock-file deletion.
            child.kill().unwrap();
        }
        Ok(())
    })
    .unwrap();
    child.wait().unwrap();
    assert!(observed);
    assert_eq!(read_owner(&lock.file).unwrap().pid, std::process::id());
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
    let lock = BuildLock::acquire(&context, |_| Ok(())).unwrap();
    for bytes in [vec![], b"not-json".to_vec(), vec![b'x'; OWNER_LIMIT + 1]] {
        lock.file.set_len(0).unwrap();
        let mut file = &lock.file;
        file.seek(SeekFrom::Start(0)).unwrap();
        file.write_all(&bytes).unwrap();
        assert!(read_owner(&lock.file).is_none());
    }
    drop(lock);
    let recovered = BuildLock::acquire(&context, |_| Ok(())).unwrap();
    assert!(read_owner(&recovered.file).is_some());
    drop(recovered);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cancelled_waiter_preserves_owner_inode_metadata_and_exclusion() {
    use std::os::unix::fs::MetadataExt;
    let root = temp_dir("lock-cancel");
    let context = context(root.clone());
    let owner = BuildLock::acquire(&context, |_| Ok(())).unwrap();
    let path = root.join(".canic/locks/complete-build-reuse.lock");
    let inode = owner.file.metadata().unwrap().ino();
    owner.phase(BuildLockPhase::BuildingArtifacts);
    let before = fs::read(&path).unwrap();
    let result = BuildLock::acquire(&context, |_| {
        Err(io::Error::from(io::ErrorKind::Interrupted))
    });
    assert!(matches!(result, Err(error) if error.kind() == io::ErrorKind::Interrupted));
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::metadata(&path).unwrap().ino(), inode);
    assert_eq!(
        read_owner(&owner.file).unwrap().phase,
        BuildLockPhase::BuildingArtifacts
    );
    let contender = fs::File::open(&path).unwrap();
    assert_eq!(
        rustix::fs::flock(
            &contender,
            rustix::fs::FlockOperation::NonBlockingLockExclusive
        ),
        Err(rustix::io::Errno::WOULDBLOCK)
    );
    drop(owner);
    let survivor = BuildLock::acquire(&context, |_| Ok(())).unwrap();
    assert_eq!(survivor.file.metadata().unwrap().ino(), inode);
    drop(survivor);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn owner_metadata_requires_explicit_nullable_identity_and_phase_updates_are_not_heartbeats() {
    let root = temp_dir("owner-shape");
    let lock = BuildLock::acquire(&context(root.clone()), |_| Ok(())).unwrap();
    let mut owner = read_owner(&lock.file).unwrap();
    owner.identity = None;
    let value = serde_json::to_value(&owner).unwrap();
    assert!(value["identity"].is_null());
    assert_eq!(
        serde_json::from_value::<BuildLockOwner>(value.clone()).unwrap(),
        owner
    );
    let mut missing = value;
    missing.as_object_mut().unwrap().remove("identity");
    assert!(serde_json::from_value::<BuildLockOwner>(missing).is_err());
    lock.phase(BuildLockPhase::BuildingArtifacts);
    let before = read_owner(&lock.file).unwrap();
    lock.phase(BuildLockPhase::BuildingArtifacts);
    assert_eq!(read_owner(&lock.file).unwrap(), before);
    drop(lock);
    fs::remove_dir_all(root).unwrap();
}
