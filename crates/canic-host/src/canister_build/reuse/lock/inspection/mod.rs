//! Module: canister_build::reuse::lock::inspection
//!
//! Responsibility: inspect the opened lock and a bounded Linux process snapshot.
//! Does not own: lock acquisition, process termination, or build health decisions.
//! Boundary: missing process visibility is unknown; arguments and environment are never read.

#[cfg(test)]
mod tests;

use super::{BuildLockOwner, read_owner};
use serde::{Deserialize, Serialize};
#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

/// Identity bound to a boot, PID namespace and process birth, rather than a reusable PID alone.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildProcessIdentity {
    pub boot_id: String,
    pub pid_namespace: String,
    pub start_ticks: u64,
}

/// Kernel observation in this inspector's process namespace; absence is not proof of unlock.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum KernelBuildLock {
    Held { pid: u32 },
    NotObserved,
    Unavailable,
}

/// Whether metadata can be bound to the observed holder without trusting PID reuse.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildProcessVisibility {
    Matched,
    IdentityMismatch,
    Unavailable,
}

/// Whitelisted process categories; arbitrary process names are not emitted.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildProcessKind {
    Canic,
    Cargo,
    Rustc,
    Sccache,
    Other,
}

/// Instantaneous scheduler state, not a build-progress or stall classification.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildProcessActivity {
    Running,
    Sleeping,
    Other,
}

/// Bounded process observation without command arguments or environment values.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BuildProcessObservation {
    pub pid: u32,
    pub start_ticks: u64,
    pub kind: BuildProcessKind,
    pub activity: BuildProcessActivity,
    pub cpu_ticks: u64,
}

/// Read-only best-effort snapshot; metadata and observed processes remain advisory.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BuildLockInspection {
    pub lock_path: PathBuf,
    pub recorded_owner: Option<BuildLockOwner>,
    pub kernel: KernelBuildLock,
    pub owner_visibility: BuildProcessVisibility,
    pub processes: Vec<BuildProcessObservation>,
    pub process_snapshot_complete: bool,
}

/// Inspect an existing regular lock without creating, replacing or locking it.
pub fn inspect_build_lock(path: &Path) -> io::Result<BuildLockInspection> {
    #[cfg(not(windows))]
    {
        use rustix::fs::{FileType, Mode, OFlags, fstat, open};
        let fd = open(
            path,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )?;
        if FileType::from_raw_mode(fstat(&fd)?.st_mode) != FileType::RegularFile {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "lock is not a regular file",
            ));
        }
        Ok(inspect_open_file(&fs::File::from(fd), path.to_path_buf()))
    }
    #[cfg(windows)]
    {
        let _ = path;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "lock inspection unavailable on this platform",
        ))
    }
}

pub(super) fn inspect_open_file(file: &fs::File, lock_path: PathBuf) -> BuildLockInspection {
    inspect_with_proc(file, lock_path, Path::new("/proc"))
}

fn inspect_with_proc(file: &fs::File, lock_path: PathBuf, proc_root: &Path) -> BuildLockInspection {
    let recorded_owner = read_owner(file);
    let kernel = kernel_holder(file, proc_root).unwrap_or(KernelBuildLock::Unavailable);
    let mut report = BuildLockInspection {
        lock_path,
        recorded_owner,
        kernel,
        owner_visibility: BuildProcessVisibility::Unavailable,
        processes: vec![],
        process_snapshot_complete: false,
    };
    if let Some(owner) = &report.recorded_owner
        && let Some(recorded_identity) = &owner.identity
        && let Some(observed_identity) = process_identity(proc_root, owner.pid)
    {
        if *recorded_identity != observed_identity {
            report.owner_visibility = BuildProcessVisibility::IdentityMismatch;
        } else if report.kernel == (KernelBuildLock::Held { pid: owner.pid }) {
            let (processes, complete) = process_tree(proc_root, owner.pid);
            // Reject observations crossing process exit/reuse while the snapshot was collected.
            if process_identity(proc_root, owner.pid).as_ref() == Some(recorded_identity) {
                report.owner_visibility = BuildProcessVisibility::Matched;
                report.processes = processes;
                report.process_snapshot_complete = complete;
            }
        } else if matches!(report.kernel, KernelBuildLock::Held { .. }) {
            report.owner_visibility = BuildProcessVisibility::IdentityMismatch;
        }
    }
    report
}

pub(super) fn current_identity() -> Option<BuildProcessIdentity> {
    process_identity(Path::new("/proc"), std::process::id())
}

fn bounded_text(path: &Path, limit: u64) -> Option<String> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .ok()?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() as u64 <= limit).then_some(())?;
    String::from_utf8(bytes).ok()
}

fn process_identity(proc_root: &Path, pid: u32) -> Option<BuildProcessIdentity> {
    let process = read_process(proc_root, pid)?;
    let boot_id = bounded_text(&proc_root.join("sys/kernel/random/boot_id"), 64)?
        .trim()
        .to_string();
    if boot_id.len() != 36
        || !boot_id
            .bytes()
            .all(|ch| ch.is_ascii_hexdigit() || ch == b'-')
    {
        return None;
    }
    let pid_namespace = fs::read_link(proc_root.join(format!("{pid}/ns/pid")))
        .ok()?
        .into_os_string()
        .into_string()
        .ok()?;
    let inspector_namespace = fs::read_link(proc_root.join("self/ns/pid")).ok()?;
    if Path::new(&pid_namespace) != inspector_namespace || pid_namespace.len() > 64 {
        return None;
    }
    Some(BuildProcessIdentity {
        boot_id,
        pid_namespace,
        start_ticks: process.start_ticks,
    })
}

fn read_process(proc_root: &Path, pid: u32) -> Option<BuildProcessObservation> {
    parse_process(
        pid,
        &bounded_text(&proc_root.join(format!("{pid}/stat")), 4096)?,
    )
}

fn parse_process(pid: u32, stat: &str) -> Option<BuildProcessObservation> {
    let (identity, fields) = stat.rsplit_once(") ")?;
    let (number, name) = identity.split_once(" (")?;
    if number.parse::<u32>().ok()? != pid {
        return None;
    }
    let fields = fields.split_ascii_whitespace().collect::<Vec<_>>();
    Some(BuildProcessObservation {
        pid,
        start_ticks: fields.get(19)?.parse().ok()?,
        kind: match name {
            "canic" => BuildProcessKind::Canic,
            "cargo" => BuildProcessKind::Cargo,
            "rustc" => BuildProcessKind::Rustc,
            "sccache" => BuildProcessKind::Sccache,
            _ => BuildProcessKind::Other,
        },
        activity: match *fields.first()? {
            "R" => BuildProcessActivity::Running,
            "S" | "D" | "I" => BuildProcessActivity::Sleeping,
            _ => BuildProcessActivity::Other,
        },
        cpu_ticks: fields
            .get(11)?
            .parse::<u64>()
            .ok()?
            .checked_add(fields.get(12)?.parse().ok()?)?,
    })
}

// Global budgets bound traversal even for large compiler process trees.
fn process_tree(proc_root: &Path, owner: u32) -> (Vec<BuildProcessObservation>, bool) {
    let mut pending = vec![owner];
    let mut seen = std::collections::BTreeSet::new();
    let mut result = Vec::new();
    let mut complete = true;
    let mut threads_left = 64_usize;
    while let Some(pid) = pending.pop() {
        if !seen.insert(pid) {
            continue;
        }
        if seen.len() > 32 {
            complete = false;
            break;
        }
        let Some(before) = read_process(proc_root, pid) else {
            complete = false;
            continue;
        };
        let Ok(tasks) = fs::read_dir(proc_root.join(format!("{pid}/task"))) else {
            complete = false;
            result.push(before);
            continue;
        };
        let mut children = Vec::new();
        for task in tasks {
            if threads_left == 0 {
                complete = false;
                break;
            }
            threads_left -= 1;
            let Some(text) = task
                .ok()
                .and_then(|task| bounded_text(&task.path().join("children"), 4096))
            else {
                complete = false;
                continue;
            };
            for child in text.split_ascii_whitespace() {
                if children.len() >= 32 {
                    complete = false;
                    break;
                }
                if let Ok(child) = child.parse::<u32>() {
                    children.push(child);
                } else {
                    complete = false;
                }
            }
        }
        if read_process(proc_root, pid).is_some_and(|after| after.start_ticks == before.start_ticks)
        {
            result.push(before);
            pending.extend(children);
        } else {
            complete = false;
        }
    }
    result.sort_by_key(|process| process.pid);
    (result, complete)
}

#[cfg(target_os = "linux")]
fn kernel_holder(file: &fs::File, proc_root: &Path) -> Option<KernelBuildLock> {
    let metadata = file.metadata().ok()?;
    let locks = bounded_text(&proc_root.join("locks"), 1024 * 1024)?;
    Some(parse_kernel_holder(&locks, metadata.dev(), metadata.ino()))
}

#[cfg(not(target_os = "linux"))]
fn kernel_holder(_file: &fs::File, _proc_root: &Path) -> Option<KernelBuildLock> {
    None
}

#[cfg(target_os = "linux")]
fn parse_kernel_holder(locks: &str, device: u64, inode: u64) -> KernelBuildLock {
    for line in locks.lines() {
        let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
        if fields.get(1) != Some(&"FLOCK") || fields.get(3) != Some(&"WRITE") {
            continue;
        }
        let Some(key) = fields.get(5) else {
            continue;
        };
        let parts = key.split(':').collect::<Vec<_>>();
        if parts.len() != 3 {
            continue;
        }
        let device_matches = u32::from_str_radix(parts[0], 16).ok()
            == Some(rustix::fs::major(device))
            && u32::from_str_radix(parts[1], 16).ok() == Some(rustix::fs::minor(device));
        if device_matches
            && parts[2].parse::<u64>().ok() == Some(inode)
            && let Some(pid) = fields
                .get(4)
                .and_then(|pid| pid.parse::<u32>().ok())
                .filter(|pid| *pid > 0)
        {
            return KernelBuildLock::Held { pid };
        }
    }
    KernelBuildLock::NotObserved
}
