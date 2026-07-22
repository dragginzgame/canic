//! Module: ops::blob_storage::funding
//!
//! Responsibility: manage transient blob-storage funding execution state.
//! Does not own: Cashier calls, cycle math, stable storage, or endpoint authorization.
//! Boundary: workflow acquires the guard before awaiting external funding effects.
//! The guard relies on `Drop` to clear the transient lock on every return path.

use std::cell::Cell;

use thiserror::Error as ThisError;

thread_local! {
    static FUNDING_IN_PROGRESS: Cell<bool> = const { Cell::new(false) };
}

///
/// BlobStorageFundingOps
///
/// Zero-cost namespace for transient blob-storage funding controls.
///

pub struct BlobStorageFundingOps;

impl BlobStorageFundingOps {
    /// Acquire the transient funding guard.
    ///
    /// The guard is intentionally not persisted. An upgrade starts with no
    /// funding lock, matching the backend billing design.
    pub fn try_acquire() -> Result<BlobStorageFundingGuard, BlobStorageFundingInProgress> {
        FUNDING_IN_PROGRESS.with(|flag| {
            if flag.get() {
                Err(BlobStorageFundingInProgress)
            } else {
                flag.set(true);
                Ok(BlobStorageFundingGuard { _private: () })
            }
        })
    }

    #[cfg(test)]
    #[must_use]
    fn in_progress() -> bool {
        FUNDING_IN_PROGRESS.with(Cell::get)
    }
}

///
/// BlobStorageFundingGuard
///
/// RAII guard for one in-flight blob-storage funding operation.
///

#[must_use]
#[derive(Debug)]
pub struct BlobStorageFundingGuard {
    _private: (),
}

impl Drop for BlobStorageFundingGuard {
    fn drop(&mut self) {
        FUNDING_IN_PROGRESS.with(|flag| flag.set(false));
    }
}

///
/// BlobStorageFundingInProgress
///
/// Typed failure returned when another funding operation already holds the guard.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, ThisError)]
#[error("blob-storage funding is already in progress")]
pub struct BlobStorageFundingInProgress;

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static PANIC_HOOK_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn funding_guard_rejects_reentrant_acquire() {
        let guard = BlobStorageFundingOps::try_acquire().expect("first acquire succeeds");

        assert!(BlobStorageFundingOps::in_progress());
        assert_eq!(
            BlobStorageFundingOps::try_acquire().expect_err("second acquire should fail"),
            BlobStorageFundingInProgress
        );

        drop(guard);
    }

    #[test]
    fn funding_guard_releases_on_drop() {
        {
            let _guard = BlobStorageFundingOps::try_acquire().expect("acquire succeeds");
            assert!(BlobStorageFundingOps::in_progress());
        }

        assert!(!BlobStorageFundingOps::in_progress());
        let _guard = BlobStorageFundingOps::try_acquire().expect("drop releases guard");
    }

    #[test]
    fn funding_guard_releases_during_unwind() {
        let _panic_hook_guard = PANIC_HOOK_LOCK
            .lock()
            .expect("panic hook lock should not be poisoned");
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let result = std::panic::catch_unwind(|| {
            let _guard = BlobStorageFundingOps::try_acquire().expect("acquire succeeds");
            assert!(BlobStorageFundingOps::in_progress());
            panic!("simulated funding unwind");
        });
        std::panic::set_hook(previous_hook);

        assert!(result.is_err());
        assert!(!BlobStorageFundingOps::in_progress());
        let _guard = BlobStorageFundingOps::try_acquire().expect("unwind releases guard");
    }
}
