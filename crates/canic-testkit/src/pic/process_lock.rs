use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process,
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

const PIC_PROCESS_LOCK_DIR_NAME: &str = "canic-pocket-ic.lock";
const PIC_PROCESS_LOCK_RETRY_DELAY: Duration = Duration::from_millis(100);
const PIC_PROCESS_LOCK_LOG_AFTER: Duration = Duration::from_secs(1);
static PIC_PROCESS_LOCK_STATE: Mutex<ProcessLockState> = Mutex::new(ProcessLockState {
    ref_count: 0,
    process_lock: None,
});

struct ProcessLockGuard {
    path: PathBuf,
}

struct ProcessLockOwner {
    pid: u32,
    start_ticks: Option<u64>,
}

struct ProcessLockState {
    ref_count: usize,
    process_lock: Option<ProcessLockGuard>,
}

///
/// PicSerialGuardError
///

#[derive(Debug)]
pub enum PicSerialGuardError {
    LockParentUnavailable { path: PathBuf, source: io::Error },
    LockUnavailable { path: PathBuf, source: io::Error },
    LockOwnerRecordFailed { path: PathBuf, source: io::Error },
}

///
/// PicSerialGuard
///

pub struct PicSerialGuard {
    _private: (),
}

/// Acquire the shared PocketIC serialization guard for the current process.
#[must_use]
pub fn acquire_pic_serial_guard() -> PicSerialGuard {
    try_acquire_pic_serial_guard()
        .unwrap_or_else(|err| panic!("failed to acquire PocketIC serial guard: {err}"))
}

/// Acquire the shared PocketIC serialization guard for the current process.
pub fn try_acquire_pic_serial_guard() -> Result<PicSerialGuard, PicSerialGuardError> {
    let mut state = PIC_PROCESS_LOCK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    if state.ref_count == 0 {
        state.process_lock = Some(acquire_process_lock()?);
    }
    state.ref_count += 1;

    Ok(PicSerialGuard { _private: () })
}

impl std::fmt::Display for PicSerialGuardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LockParentUnavailable { path, source } => write!(
                f,
                "failed to create PocketIC lock parent at {}: {source}",
                path.display()
            ),
            Self::LockUnavailable { path, source } => write!(
                f,
                "failed to create PocketIC process lock dir at {}: {source}",
                path.display()
            ),
            Self::LockOwnerRecordFailed { path, source } => write!(
                f,
                "failed to record PocketIC process lock owner at {}: {source}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for PicSerialGuardError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::LockParentUnavailable { source, .. }
            | Self::LockUnavailable { source, .. }
            | Self::LockOwnerRecordFailed { source, .. } => Some(source),
        }
    }
}

impl Drop for ProcessLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

impl Drop for PicSerialGuard {
    fn drop(&mut self) {
        let mut state = PIC_PROCESS_LOCK_STATE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        state.ref_count = state
            .ref_count
            .checked_sub(1)
            .expect("PocketIC serial guard refcount underflow");
        if state.ref_count == 0 {
            state.process_lock.take();
        }
    }
}

// Acquire the shared filesystem lock that serializes PocketIC usage per host.
fn acquire_process_lock() -> Result<ProcessLockGuard, PicSerialGuardError> {
    let lock_dir = process_lock_dir();
    ensure_process_lock_parent(&lock_dir)?;
    let started_waiting = Instant::now();
    let mut logged_wait = false;

    loop {
        match fs::create_dir(&lock_dir) {
            Ok(()) => {
                if let Err(source) = fs::write(
                    process_lock_owner_path(&lock_dir),
                    render_process_lock_owner(),
                ) {
                    let _ = fs::remove_dir(&lock_dir);
                    return Err(PicSerialGuardError::LockOwnerRecordFailed {
                        path: lock_dir,
                        source,
                    });
                }

                if logged_wait {
                    eprintln!(
                        "[canic_testkit::pic] acquired cross-process PocketIC lock at {}",
                        lock_dir.display()
                    );
                }

                return Ok(ProcessLockGuard { path: lock_dir });
            }
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                if process_lock_is_stale(&lock_dir) && clear_stale_process_lock(&lock_dir).is_ok() {
                    continue;
                }

                if !logged_wait && started_waiting.elapsed() >= PIC_PROCESS_LOCK_LOG_AFTER {
                    eprintln!(
                        "[canic_testkit::pic] waiting for cross-process PocketIC lock at {}",
                        lock_dir.display()
                    );
                    logged_wait = true;
                }

                thread::sleep(PIC_PROCESS_LOCK_RETRY_DELAY);
            }
            Err(source) => {
                return Err(PicSerialGuardError::LockUnavailable {
                    path: lock_dir,
                    source,
                });
            }
        }
    }
}

// Resolve the cross-process PocketIC lock path from the active temp root.
fn process_lock_dir() -> PathBuf {
    process_lock_dir_from_temp_root(&env::temp_dir())
}

// Resolve the cross-process PocketIC lock path for one explicit temp root.
fn process_lock_dir_from_temp_root(temp_root: &Path) -> PathBuf {
    temp_root.join(PIC_PROCESS_LOCK_DIR_NAME)
}

// Create the temp-root parent chain before trying to create the lock directory itself.
fn ensure_process_lock_parent(lock_dir: &Path) -> Result<(), PicSerialGuardError> {
    let parent = lock_dir.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|source| PicSerialGuardError::LockParentUnavailable {
        path: parent.to_path_buf(),
        source,
    })
}

fn process_lock_owner_path(lock_dir: &Path) -> PathBuf {
    lock_dir.join("owner")
}

fn clear_stale_process_lock(lock_dir: &Path) -> io::Result<()> {
    match fs::remove_dir_all(lock_dir) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

fn process_lock_is_stale(lock_dir: &Path) -> bool {
    process_lock_is_stale_with_proc_root(lock_dir, Path::new("/proc"))
}

fn process_lock_is_stale_with_proc_root(lock_dir: &Path, proc_root: &Path) -> bool {
    let Some(owner) = read_process_lock_owner(&process_lock_owner_path(lock_dir)) else {
        return true;
    };

    let proc_dir = proc_root.join(owner.pid.to_string());
    if !proc_dir.exists() {
        return true;
    }

    match owner.start_ticks {
        Some(expected_ticks) => {
            read_process_start_ticks(proc_root, owner.pid) != Some(expected_ticks)
        }
        None => false,
    }
}

fn render_process_lock_owner() -> String {
    let owner = current_process_lock_owner();
    match owner.start_ticks {
        Some(start_ticks) => format!("pid={}\nstart_ticks={start_ticks}\n", owner.pid),
        None => format!("pid={}\n", owner.pid),
    }
}

fn current_process_lock_owner() -> ProcessLockOwner {
    ProcessLockOwner {
        pid: process::id(),
        start_ticks: read_process_start_ticks(Path::new("/proc"), process::id()),
    }
}

fn read_process_lock_owner(path: &Path) -> Option<ProcessLockOwner> {
    let text = fs::read_to_string(path).ok()?;
    parse_process_lock_owner(&text)
}

fn parse_process_lock_owner(text: &str) -> Option<ProcessLockOwner> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut pid = None;
    let mut start_ticks = None;
    for line in trimmed.lines() {
        if let Some(value) = line.strip_prefix("pid=") {
            pid = value.trim().parse::<u32>().ok();
        } else if let Some(value) = line.strip_prefix("start_ticks=") {
            start_ticks = value.trim().parse::<u64>().ok();
        }
    }

    Some(ProcessLockOwner {
        pid: pid?,
        start_ticks,
    })
}

fn read_process_start_ticks(proc_root: &Path, pid: u32) -> Option<u64> {
    let stat_path = proc_root.join(pid.to_string()).join("stat");
    let stat = fs::read_to_string(stat_path).ok()?;
    let close_paren = stat.rfind(')')?;
    let rest = stat.get(close_paren + 2..)?;
    let fields = rest.split_whitespace().collect::<Vec<_>>();
    fields.get(19)?.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::{
        clear_stale_process_lock, ensure_process_lock_parent, parse_process_lock_owner,
        process_lock_dir_from_temp_root, process_lock_is_stale_with_proc_root,
        process_lock_owner_path,
    };
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn unique_lock_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("canic-pocket-ic-test-lock-{nanos}"))
    }

    #[test]
    fn stale_process_lock_is_detected_and_removed() {
        let lock_dir = unique_lock_dir();
        fs::create_dir(&lock_dir).expect("create lock dir");
        fs::write(process_lock_owner_path(&lock_dir), "999999").expect("write stale owner");

        assert!(process_lock_is_stale_with_proc_root(
            &lock_dir,
            std::path::Path::new("/proc")
        ));
        clear_stale_process_lock(&lock_dir).expect("remove stale lock dir");
        assert!(!lock_dir.exists());
    }

    #[test]
    fn owner_parser_rejects_pid_only_format() {
        assert!(parse_process_lock_owner("12345\n").is_none());
    }

    #[test]
    fn stale_process_lock_detects_pid_reuse_via_start_ticks() {
        let root = unique_lock_dir();
        let lock_dir = root.join("lock");
        let proc_root = root.join("proc");
        let proc_pid = proc_root.join("77");
        fs::create_dir_all(&lock_dir).expect("create lock dir");
        fs::create_dir_all(&proc_pid).expect("create proc pid dir");
        fs::write(
            process_lock_owner_path(&lock_dir),
            "pid=77\nstart_ticks=41\n",
        )
        .expect("write owner");
        fs::write(
            proc_pid.join("stat"),
            "77 (cargo) S 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 99 0 0\n",
        )
        .expect("write proc stat");

        assert!(process_lock_is_stale_with_proc_root(&lock_dir, &proc_root));
    }

    #[test]
    fn ensure_process_lock_parent_creates_missing_temp_root_chain() {
        let root = unique_lock_dir();
        let temp_root = root.join("repo-local").join("tmp");
        let lock_dir = process_lock_dir_from_temp_root(&temp_root);

        ensure_process_lock_parent(&lock_dir).expect("create temp-root parent chain");

        assert!(temp_root.exists());
    }
}
