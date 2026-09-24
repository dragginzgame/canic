use super::*;
use crate::test_support::temp_dir;

fn stat(pid: u32, name: &str, state: &str, start: u64, cpu: u64) -> String {
    let mut fields = vec!["0".to_string(); 20];
    fields[0] = state.into();
    fields[11] = cpu.to_string();
    fields[19] = start.to_string();
    format!("{pid} ({name}) {}", fields.join(" "))
}

#[test]
fn process_parser_bounds_identity_and_never_exposes_arbitrary_names() {
    let process = parse_process(42, &stat(42, "cargo", "S", 123, 7)).unwrap();
    assert_eq!(process.kind, BuildProcessKind::Cargo);
    assert_eq!(process.activity, BuildProcessActivity::Sleeping);
    assert_eq!(process.start_ticks, 123);
    assert_eq!(process.cpu_ticks, 7);
    assert_eq!(
        parse_process(42, &stat(42, "private ) token", "R", 123, 9))
            .unwrap()
            .kind,
        BuildProcessKind::Other
    );
    assert!(parse_process(43, &stat(42, "cargo", "S", 123, 7)).is_none());
    assert!(parse_process(42, "42 (cargo) S").is_none());
}

#[cfg(target_os = "linux")]
#[test]
fn inspection_binds_kernel_inode_namespace_and_birth_before_reporting_children() {
    use crate::canister_build::reuse::lock::{BuildLock, tests::context};
    use std::os::unix::fs::symlink;
    let root = temp_dir("lock-process-view");
    let lock = BuildLock::acquire(&context(root.clone()), |_| Ok(())).unwrap();
    let path = root.join(".canic/locks/complete-build-reuse.lock");
    let proc_root = root.join("proc");
    let pid = std::process::id();
    let identity = read_owner(&lock.file).unwrap().identity.unwrap();
    fs::create_dir_all(proc_root.join("sys/kernel/random")).unwrap();
    fs::create_dir_all(proc_root.join("self/ns")).unwrap();
    fs::write(
        proc_root.join("sys/kernel/random/boot_id"),
        &identity.boot_id,
    )
    .unwrap();
    symlink(&identity.pid_namespace, proc_root.join("self/ns/pid")).unwrap();
    for (process, kind, start) in [
        (pid, "canic", identity.start_ticks),
        (pid + 1, "cargo", 500),
    ] {
        fs::create_dir_all(proc_root.join(format!("{process}/ns"))).unwrap();
        fs::create_dir_all(proc_root.join(format!("{process}/task/{process}"))).unwrap();
        symlink(
            &identity.pid_namespace,
            proc_root.join(format!("{process}/ns/pid")),
        )
        .unwrap();
        fs::write(
            proc_root.join(format!("{process}/stat")),
            stat(process, kind, "S", start, 1),
        )
        .unwrap();
        fs::write(
            proc_root.join(format!("{process}/task/{process}/children")),
            "",
        )
        .unwrap();
    }
    fs::write(
        proc_root.join(format!("{pid}/task/{pid}/children")),
        (pid + 1).to_string(),
    )
    .unwrap();
    let metadata = lock.file.metadata().unwrap();
    fs::write(
        proc_root.join("locks"),
        format!(
            "1: FLOCK ADVISORY WRITE {pid} {:x}:{:x}:{} 0 EOF\n",
            rustix::fs::major(metadata.dev()),
            rustix::fs::minor(metadata.dev()),
            metadata.ino()
        ),
    )
    .unwrap();
    let inspect = || inspect_with_proc(&lock.file, path.clone(), &proc_root);
    let quiet = inspect();
    assert_eq!(quiet.owner_visibility, BuildProcessVisibility::Matched);
    assert!(quiet.process_snapshot_complete);
    assert!(
        quiet
            .processes
            .iter()
            .any(|child| child.kind == BuildProcessKind::Cargo
                && child.activity == BuildProcessActivity::Sleeping)
    );
    fs::write(
        proc_root.join(format!("{}/stat", pid + 1)),
        stat(pid + 1, "cargo", "R", 500, 25),
    )
    .unwrap();
    let active = inspect();
    assert_eq!(active.recorded_owner, quiet.recorded_owner);
    assert!(active.processes.iter().any(|child| child.cpu_ticks == 25));
    fs::write(
        proc_root.join(format!("{pid}/stat")),
        stat(pid, "canic", "S", identity.start_ticks + 1, 1),
    )
    .unwrap();
    let reused = inspect();
    assert_eq!(
        reused.owner_visibility,
        BuildProcessVisibility::IdentityMismatch
    );
    assert!(reused.processes.is_empty());
    fs::remove_file(proc_root.join(format!("{pid}/stat"))).unwrap();
    assert_eq!(
        inspect().owner_visibility,
        BuildProcessVisibility::Unavailable
    );
    fs::write(proc_root.join("locks"), "").unwrap();
    assert_eq!(inspect().kernel, KernelBuildLock::NotObserved);
    fs::remove_file(proc_root.join("locks")).unwrap();
    assert_eq!(inspect().kernel, KernelBuildLock::Unavailable);
    drop(lock);
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn inspection_is_read_only_and_rejects_symlinks_and_nonregular_files() {
    use std::os::unix::fs::symlink;
    let root = temp_dir("lock-read-only");
    let missing = root.join("missing/lock");
    assert_eq!(
        inspect_build_lock(&missing).unwrap_err().kind(),
        io::ErrorKind::NotFound
    );
    assert!(!root.join("missing").exists());
    fs::create_dir_all(&root).unwrap();
    let path = root.join("lock");
    fs::write(&path, b"invalid metadata").unwrap();
    let report = inspect_build_lock(&path).unwrap();
    assert!(report.recorded_owner.is_none());
    assert_eq!(fs::read(&path).unwrap(), b"invalid metadata");
    let alias = root.join("alias");
    symlink(&path, &alias).unwrap();
    assert!(inspect_build_lock(&alias).is_err());
    assert_eq!(
        inspect_build_lock(&root).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    fs::write(&path, vec![b'x'; 4097]).unwrap();
    assert!(inspect_build_lock(&path).unwrap().recorded_owner.is_none());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn process_snapshot_has_a_global_thread_budget_and_reports_truncation() {
    let root = temp_dir("lock-thread-budget");
    fs::create_dir_all(root.join("1/task")).unwrap();
    fs::write(root.join("1/stat"), stat(1, "cargo", "R", 2, 3)).unwrap();
    for thread in 0..65 {
        let task = root.join(format!("1/task/{thread}"));
        fs::create_dir(&task).unwrap();
        fs::write(task.join("children"), "").unwrap();
    }
    let (processes, complete) = process_tree(&root, 1);
    assert_eq!(processes.len(), 1);
    assert!(!complete);
    fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn kernel_parser_ignores_waiters_and_other_inodes() {
    let device = rustix::fs::makedev(8, 1);
    let locks = "1: -> FLOCK ADVISORY WRITE 123 08:01:50 0 EOF\n2: FLOCK ADVISORY WRITE 456 08:01:51 0 EOF\n3: FLOCK ADVISORY WRITE 789 08:01:50 0 EOF\n";
    assert_eq!(
        parse_kernel_holder(locks, device, 50),
        KernelBuildLock::Held { pid: 789 }
    );
    assert_eq!(
        parse_kernel_holder(locks, device, 52),
        KernelBuildLock::NotObserved
    );
}
