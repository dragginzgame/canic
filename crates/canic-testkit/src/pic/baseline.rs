use candid::Principal;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    sync::{Mutex, MutexGuard},
};

use super::{Pic, PicSerialGuard, acquire_pic_serial_guard, startup};

struct ControllerSnapshot {
    snapshot_id: Vec<u8>,
    sender: Option<Principal>,
}

///
/// ControllerSnapshots
///

pub struct ControllerSnapshots(HashMap<Principal, ControllerSnapshot>);

///
/// CachedPicBaseline
///

pub struct CachedPicBaseline<T> {
    pic: Pic,
    snapshots: ControllerSnapshots,
    metadata: T,
    _serial_guard: PicSerialGuard,
}

///
/// CachedPicBaselineGuard
///

pub struct CachedPicBaselineGuard<'a, T> {
    guard: MutexGuard<'a, Option<CachedPicBaseline<T>>>,
}

enum CachedBaselineRestoreFailure {
    DeadInstanceTransport,
    Panic(Box<dyn std::any::Any + Send>),
}

/// Acquire one process-local cached PocketIC baseline, building it on first use.
fn acquire_cached_pic_baseline<T, F>(
    slot: &'static Mutex<Option<CachedPicBaseline<T>>>,
    build: F,
) -> (CachedPicBaselineGuard<'static, T>, bool)
where
    F: FnOnce() -> CachedPicBaseline<T>,
{
    let mut guard = slot
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let cache_hit = guard.is_some();

    if !cache_hit {
        *guard = Some(build());
    }

    (CachedPicBaselineGuard { guard }, cache_hit)
}

/// Restore one cached PocketIC baseline, rebuilding it if the owned PocketIC
/// instance has died between tests.
pub fn restore_or_rebuild_cached_pic_baseline<T, B, R>(
    slot: &'static Mutex<Option<CachedPicBaseline<T>>>,
    build: B,
    restore: R,
) -> (CachedPicBaselineGuard<'static, T>, bool)
where
    B: Fn() -> CachedPicBaseline<T>,
    R: Fn(&CachedPicBaseline<T>),
{
    let (baseline, cache_hit) = acquire_cached_pic_baseline(slot, &build);
    if !cache_hit {
        return (baseline, false);
    }

    match try_restore_cached_pic_baseline(&baseline, restore) {
        Ok(()) => return (baseline, true),
        Err(CachedBaselineRestoreFailure::DeadInstanceTransport) => {}
        Err(CachedBaselineRestoreFailure::Panic(payload)) => {
            resume_unwind(payload);
        }
    }

    drop(baseline);
    drop_stale_cached_pic_baseline(slot);

    let (rebuilt, _cache_hit) = acquire_cached_pic_baseline(slot, build);
    (rebuilt, false)
}

// Attempt one cached baseline restore and classify only the one recovery path
// we intentionally swallow: a dead PocketIC transport instance.
fn try_restore_cached_pic_baseline<T, R>(
    baseline: &CachedPicBaseline<T>,
    restore: R,
) -> Result<(), CachedBaselineRestoreFailure>
where
    R: Fn(&CachedPicBaseline<T>),
{
    match catch_unwind(AssertUnwindSafe(|| restore(baseline))) {
        Ok(()) => Ok(()),
        Err(payload) => {
            if startup::panic_is_dead_instance_transport(payload.as_ref()) {
                Err(CachedBaselineRestoreFailure::DeadInstanceTransport)
            } else {
                Err(CachedBaselineRestoreFailure::Panic(payload))
            }
        }
    }
}

/// Remove one dead cached baseline and swallow teardown panics from a broken
/// PocketIC instance so callers can rebuild cleanly.
fn drop_stale_cached_pic_baseline<T>(slot: &'static Mutex<Option<CachedPicBaseline<T>>>) {
    let stale = {
        let mut slot = slot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        slot.take()
    };

    if let Some(stale) = stale {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            drop(stale);
        }));
    }
}

impl<T> Deref for CachedPicBaselineGuard<'_, T> {
    type Target = CachedPicBaseline<T>;

    fn deref(&self) -> &Self::Target {
        self.guard
            .as_ref()
            .expect("cached PocketIC baseline must exist")
    }
}

impl<T> DerefMut for CachedPicBaselineGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard
            .as_mut()
            .expect("cached PocketIC baseline must exist")
    }
}

impl<T> CachedPicBaseline<T> {
    /// Capture one immutable cached baseline from the current PocketIC instance.
    pub fn capture<I>(
        pic: Pic,
        controller_id: Principal,
        canister_ids: I,
        metadata: T,
    ) -> Option<Self>
    where
        I: IntoIterator<Item = Principal>,
    {
        let snapshots = pic.capture_controller_snapshots(controller_id, canister_ids)?;

        Some(Self {
            pic,
            snapshots,
            metadata,
            _serial_guard: acquire_pic_serial_guard(),
        })
    }

    /// Restore the captured snapshot set back into the owned PocketIC instance.
    pub fn restore(&self, controller_id: Principal) {
        self.pic
            .restore_controller_snapshots(controller_id, &self.snapshots);
    }

    /// Borrow the owned PocketIC instance behind this cached baseline.
    #[must_use]
    pub const fn pic(&self) -> &Pic {
        &self.pic
    }

    /// Mutably borrow the owned PocketIC instance behind this cached baseline.
    #[must_use]
    pub const fn pic_mut(&mut self) -> &mut Pic {
        &mut self.pic
    }

    /// Borrow the captured metadata associated with this cached baseline.
    #[must_use]
    pub const fn metadata(&self) -> &T {
        &self.metadata
    }

    /// Mutably borrow the captured metadata associated with this cached baseline.
    #[must_use]
    pub const fn metadata_mut(&mut self) -> &mut T {
        &mut self.metadata
    }
}

impl ControllerSnapshots {
    pub(super) fn new(snapshots: HashMap<Principal, (Vec<u8>, Option<Principal>)>) -> Self {
        Self(
            snapshots
                .into_iter()
                .map(|(canister_id, (snapshot_id, sender))| {
                    (
                        canister_id,
                        ControllerSnapshot {
                            snapshot_id,
                            sender,
                        },
                    )
                })
                .collect(),
        )
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (Principal, &[u8], Option<Principal>)> + '_ {
        self.0.iter().map(|(canister_id, snapshot)| {
            (
                *canister_id,
                snapshot.snapshot_id.as_slice(),
                snapshot.sender,
            )
        })
    }
}
