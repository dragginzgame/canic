//! Module: canister_build::reuse::lock
//!
//! Responsibility: retain kernel exclusion and publish bounded advisory build phases.
//! Does not own: cache admission or recovery authority.
//! Boundary: metadata failure never changes exclusion; the lock inode is never replaced.

mod inspection;
mod serialization;
#[cfg(all(test, unix))]
mod tests;

use super::{BuildReuseProgress, WorkspaceBuildContext};
use crate::durable_io::lock_file_with_progress;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub use inspection::{
    BuildLockInspection, BuildProcessActivity, BuildProcessIdentity, BuildProcessKind,
    BuildProcessObservation, BuildProcessVisibility, KernelBuildLock, inspect_build_lock,
};

const OWNER_LIMIT: usize = 4096;

/// Last entered build phase; a timestamp is not a heartbeat or proof of forward progress.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildLockPhase {
    PreparingInputs,
    VerifyingOutputs,
    BuildingArtifacts,
    VerifyingAndPublishing,
}

/// Last recorded owner; stale or partial metadata never authorizes breaking a lock.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildLockOwner {
    pub pid: u32,
    pub profile: String,
    pub started_at_unix_seconds: u64,
    pub workspace: PathBuf,
    #[serde(deserialize_with = "serialization::required_option")]
    pub identity: Option<BuildProcessIdentity>,
    pub phase: BuildLockPhase,
    pub phase_started_at_unix_seconds: u64,
}

/// Contention observed on the opened inode, with fallible read-only diagnostics.
#[derive(Clone, Debug)]
pub struct BuildLockWait {
    pub elapsed: Duration,
    pub inspection: BuildLockInspection,
}

/// Holds the kernel lock and clears advisory data before releasing it.
pub(super) struct BuildLock {
    file: fs::File,
    owner: Mutex<BuildLockOwner>,
}

impl BuildLock {
    pub(super) fn acquire(
        context: &WorkspaceBuildContext,
        mut progress: impl FnMut(BuildReuseProgress) -> io::Result<()>,
    ) -> io::Result<Self> {
        let lock_path = context
            .icp_root
            .join(".canic/locks/complete-build-reuse.lock");
        let started = Instant::now();
        let file = lock_file_with_progress(&lock_path, |file, elapsed| {
            progress(BuildReuseProgress::WaitingForLock(BuildLockWait {
                elapsed,
                inspection: inspection::inspect_open_file(file, lock_path.clone()),
            }))
        })?;
        // A pending cancellation must not start another build or clear another owner's metadata.
        progress(BuildReuseProgress::LockFinished(started.elapsed()))?;
        let now = unix_seconds();
        let owner = BuildLockOwner {
            pid: std::process::id(),
            profile: context.profile.target_dir_name().into(),
            started_at_unix_seconds: now,
            workspace: context.workspace_root.clone(),
            identity: inspection::current_identity(),
            phase: BuildLockPhase::PreparingInputs,
            phase_started_at_unix_seconds: now,
        };
        let lock = Self {
            file,
            owner: Mutex::new(owner),
        };
        if let Ok(owner) = lock.owner.lock() {
            let _ = lock.publish(&owner);
        }
        Ok(lock)
    }

    pub(super) fn phase(&self, phase: BuildLockPhase) {
        if let Ok(mut owner) = self.owner.lock()
            && owner.phase != phase
        {
            owner.phase = phase;
            owner.phase_started_at_unix_seconds = unix_seconds();
            let _ = self.publish(&owner);
        }
    }

    fn publish(&self, owner: &BuildLockOwner) -> io::Result<()> {
        self.file.set_len(0)?;
        let bytes = serde_json::to_vec(owner)?;
        if bytes.len() > OWNER_LIMIT {
            return Ok(());
        }
        let mut file = &self.file;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(&bytes)
    }
}

impl Drop for BuildLock {
    fn drop(&mut self) {
        let _ = self.file.set_len(0);
    }
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_secs())
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
