//! Module: persistence::journal_lock
//!
//! Responsibility: serialize mutation of a persisted journal with a sidecar lock.
//! Does not own: journal validation, workflow policy, or domain error projection.

use std::{fs, io, path::Path, path::PathBuf};

use super::file_lock::{self, FileLockError};

#[derive(Debug)]
pub enum JournalLockError {
    Locked { lock_path: String },
    UnsafeEntry { lock_path: String, kind: String },
    Io(io::Error),
}

impl From<io::Error> for JournalLockError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug)]
pub struct JournalLock {
    #[cfg(unix)]
    file: fs::File,
}

impl JournalLock {
    pub(crate) fn acquire(journal_path: &Path) -> Result<Self, JournalLockError> {
        let path = journal_lock_path(journal_path);

        #[cfg(unix)]
        {
            acquire_supported(path)
        }

        #[cfg(not(unix))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!(
                    "journal locking is unsupported on this host: {}",
                    path.display()
                ),
            )
            .into())
        }
    }
}

#[cfg(unix)]
impl Drop for JournalLock {
    fn drop(&mut self) {
        file_lock::unlock(&self.file);
    }
}

#[cfg(unix)]
fn acquire_supported(path: PathBuf) -> Result<JournalLock, JournalLockError> {
    match file_lock::acquire(&path) {
        Ok(file) => Ok(JournalLock { file }),
        Err(FileLockError::Locked) => Err(JournalLockError::Locked {
            lock_path: path.to_string_lossy().to_string(),
        }),
        Err(FileLockError::UnsafeEntry { kind }) => Err(JournalLockError::UnsafeEntry {
            lock_path: path.to_string_lossy().to_string(),
            kind,
        }),
        Err(FileLockError::Io(error)) => Err(JournalLockError::Io(error)),
    }
}

fn journal_lock_path(path: &Path) -> PathBuf {
    let mut lock_path = path.as_os_str().to_os_string();
    lock_path.push(".lock");
    PathBuf::from(lock_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_path;

    #[cfg(unix)]
    use rustix::io::{FdFlags, fcntl_getfd};
    #[cfg(unix)]
    use std::{os::unix::fs::symlink, process::Command, thread, time::Duration};

    const CHILD_JOURNAL_ENV: &str = "CANIC_TEST_JOURNAL_LOCK_CHILD_PATH";
    const CHILD_READY_ENV: &str = "CANIC_TEST_JOURNAL_LOCK_CHILD_READY";

    #[cfg(unix)]
    #[test]
    fn lock_excludes_a_second_owner_and_reacquires_after_drop() {
        let journal_path = temp_path("canic-backup-journal-lock");
        let lock_path = journal_lock_path(&journal_path);
        let lock = JournalLock::acquire(&journal_path).expect("acquire lock");

        assert!(lock_path.is_file());
        assert!(
            fcntl_getfd(&lock.file)
                .expect("read lock descriptor flags")
                .contains(FdFlags::CLOEXEC)
        );
        std::assert_matches!(
            JournalLock::acquire(&journal_path),
            Err(JournalLockError::Locked { lock_path: actual })
                if actual == lock_path.to_string_lossy()
        );

        drop(lock);
        assert!(lock_path.is_file());
        let reacquired = JournalLock::acquire(&journal_path).expect("reacquire lock");
        drop(reacquired);
        fs::remove_file(lock_path).expect("remove lock path");
    }

    #[cfg(unix)]
    #[test]
    fn lock_rejects_symlink_and_special_entry_substitution() {
        let journal_path = temp_path("canic-backup-journal-lock-unsafe");
        let lock_path = journal_lock_path(&journal_path);
        let target = temp_path("canic-backup-journal-lock-target");
        symlink(&target, &lock_path).expect("create lock symlink");

        std::assert_matches!(
            JournalLock::acquire(&journal_path),
            Err(JournalLockError::UnsafeEntry { lock_path: actual, kind })
                if actual == lock_path.to_string_lossy() && kind == "Symlink"
        );
        fs::remove_file(&lock_path).expect("remove lock symlink");
        fs::create_dir(&lock_path).expect("create lock directory");
        std::assert_matches!(
            JournalLock::acquire(&journal_path),
            Err(JournalLockError::UnsafeEntry { lock_path: actual, kind })
                if actual == lock_path.to_string_lossy() && kind == "Directory"
        );
        fs::remove_dir(lock_path).expect("remove lock directory");
    }

    #[cfg(unix)]
    #[test]
    fn process_death_releases_lock_without_sidecar_removal() {
        let journal_path = temp_path("canic-backup-journal-lock-process-death");
        let ready_path = temp_path("canic-backup-journal-lock-ready");
        let mut child = Command::new(std::env::current_exe().expect("resolve test executable"))
            .args([
                "--exact",
                "persistence::journal_lock::tests::process_death_child_holds_lock",
                "--nocapture",
            ])
            .env(CHILD_JOURNAL_ENV, &journal_path)
            .env(CHILD_READY_ENV, &ready_path)
            .spawn()
            .expect("spawn lock owner");

        for _ in 0..500 {
            if ready_path.is_file() {
                break;
            }
            assert!(
                child.try_wait().expect("inspect child").is_none(),
                "lock owner exited before signaling readiness"
            );
            thread::sleep(Duration::from_millis(10));
        }
        assert!(ready_path.is_file(), "lock owner did not become ready");
        std::assert_matches!(
            JournalLock::acquire(&journal_path),
            Err(JournalLockError::Locked { .. })
        );

        child.kill().expect("kill lock owner without unwinding");
        child.wait().expect("reap lock owner");
        let lock = JournalLock::acquire(&journal_path).expect("reacquire after process death");

        drop(lock);
        fs::remove_file(journal_lock_path(&journal_path)).expect("remove lock path");
        fs::remove_file(ready_path).expect("remove ready marker");
    }

    #[cfg(unix)]
    #[test]
    fn process_death_child_holds_lock() {
        let Some(journal_path) = std::env::var_os(CHILD_JOURNAL_ENV) else {
            return;
        };
        let ready_path = std::env::var_os(CHILD_READY_ENV).expect("child ready path");
        let _lock = JournalLock::acquire(Path::new(&journal_path)).expect("child acquire lock");
        fs::write(ready_path, b"ready\n").expect("signal child readiness");
        loop {
            thread::sleep(Duration::from_secs(1));
        }
    }
}
