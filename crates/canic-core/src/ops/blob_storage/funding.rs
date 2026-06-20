//! Module: ops::blob_storage::funding
//!
//! Responsibility: manage transient blob-storage funding execution state.
//! Does not own: Cashier calls, cycle math, stable storage, or endpoint authorization.
//! Boundary: API/workflow acquires the guard before awaiting external funding effects.

use std::{cell::Cell, error::Error, fmt};

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
    /// funding lock, matching the 0.70 backend billing design.
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlobStorageFundingInProgress;

impl fmt::Display for BlobStorageFundingInProgress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("blob-storage funding is already in progress")
    }
}

impl Error for BlobStorageFundingInProgress {}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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
}
