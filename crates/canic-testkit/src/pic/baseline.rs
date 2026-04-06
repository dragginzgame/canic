use candid::Principal;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::{Mutex, MutexGuard},
};

use super::{Pic, PicSerialGuard, acquire_pic_serial_guard};

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
    pub pic: Pic,
    pub snapshots: ControllerSnapshots,
    pub metadata: T,
    _serial_guard: PicSerialGuard,
}

///
/// CachedPicBaselineGuard
///

pub struct CachedPicBaselineGuard<'a, T> {
    guard: MutexGuard<'a, Option<CachedPicBaseline<T>>>,
}

/// Acquire one process-local cached PocketIC baseline, building it on first use.
pub fn acquire_cached_pic_baseline<T, F>(
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
