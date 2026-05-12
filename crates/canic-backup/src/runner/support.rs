use super::BackupRunnerError;
use crate::timestamp::current_timestamp_marker;
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub(super) fn state_updated_at(updated_at: Option<&String>) -> String {
    updated_at.cloned().unwrap_or_else(current_timestamp_marker)
}

pub(super) fn timestamp_seconds(marker: &str) -> u64 {
    marker
        .strip_prefix("unix:")
        .and_then(|seconds| seconds.parse::<u64>().ok())
        .unwrap_or_else(current_unix_seconds)
}

pub(super) fn timestamp_marker(seconds: u64) -> String {
    format!("unix:{seconds}")
}

fn current_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

pub(super) struct BackupRunLock {
    path: PathBuf,
}

impl BackupRunLock {
    pub(super) fn acquire(journal_path: &Path) -> Result<Self, BackupRunnerError> {
        let path = journal_lock_path(journal_path);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                writeln!(file, "pid={}", std::process::id())?;
                Ok(Self { path })
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                Err(BackupRunnerError::JournalLocked {
                    lock_path: path.to_string_lossy().to_string(),
                })
            }
            Err(error) => Err(error.into()),
        }
    }
}

impl Drop for BackupRunLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn journal_lock_path(path: &Path) -> PathBuf {
    let mut lock_path = path.as_os_str().to_os_string();
    lock_path.push(".lock");
    PathBuf::from(lock_path)
}
