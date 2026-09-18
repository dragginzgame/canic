//! Complete-build lock diagnostics, without changing kernel lock authority.
//!
//! Owner metadata is advisory, bounded and never a reason to break a lock.

#[cfg(all(test, unix))]
mod tests;

use super::{BuildReuseProgress, WorkspaceBuildContext};
use crate::durable_io::lock_file_with_progress;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read, Seek, SeekFrom, Write},
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const OWNER_LIMIT: usize = 4096;

/// Last recorded build owner; a crash or publication race may leave stale metadata.
/// This diagnostic contains no command arguments or environment values.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildLockOwner {
    pub pid: u32,
    pub profile: String,
    pub started_at_unix_seconds: u64,
    pub workspace: PathBuf,
}

/// Current contention and optional advisory owner on the same opened lock inode.
#[derive(Clone, Debug)]
pub struct BuildLockWait {
    pub elapsed: Duration,
    pub lock_path: PathBuf,
    pub recorded_owner: Option<BuildLockOwner>,
}

/// Holds the existing kernel lock; clears advisory data before releasing it.
pub(super) struct BuildLock(fs::File);

impl BuildLock {
    pub(super) fn acquire(
        context: &WorkspaceBuildContext,
        mut progress: impl FnMut(BuildReuseProgress),
    ) -> io::Result<Self> {
        let lock_path = context
            .icp_root
            .join(".canic/locks/complete-build-reuse.lock");
        let started = Instant::now();
        let result = lock_file_with_progress(&lock_path, |file, elapsed| {
            progress(BuildReuseProgress::WaitingForLock(BuildLockWait {
                elapsed,
                lock_path: lock_path.clone(),
                recorded_owner: read_owner(file),
            }));
        });
        progress(BuildReuseProgress::LockFinished(started.elapsed()));
        let lock = Self(result?);
        // Diagnostics fail soft. Neither stale nor missing data changes exclusion.
        let _ = lock.publish(context);
        Ok(lock)
    }

    fn publish(&self, context: &WorkspaceBuildContext) -> io::Result<()> {
        self.0.set_len(0)?;
        let owner = BuildLockOwner {
            pid: std::process::id(),
            profile: context.profile.target_dir_name().into(),
            started_at_unix_seconds: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(io::Error::other)?
                .as_secs(),
            workspace: context.workspace_root.clone(),
        };
        let bytes = serde_json::to_vec(&owner)?;
        if bytes.len() > OWNER_LIMIT {
            return Ok(());
        }
        let mut file = &self.0;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(&bytes)
    }
}

impl Drop for BuildLock {
    fn drop(&mut self) {
        // Never unlink or replace the inode: contenders already have it open.
        let _ = self.0.set_len(0);
    }
}

fn read_owner(mut file: &fs::File) -> Option<BuildLockOwner> {
    if file.metadata().ok()?.len() > OWNER_LIMIT as u64 {
        return None;
    }
    file.seek(SeekFrom::Start(0)).ok()?;
    let mut bytes = Vec::new();
    file.take(OWNER_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > OWNER_LIMIT {
        return None;
    }
    let owner: BuildLockOwner = serde_json::from_slice(&bytes).ok()?;
    (owner.pid > 0 && matches!(owner.profile.as_str(), "debug" | "fast" | "release"))
        .then_some(owner)
}
