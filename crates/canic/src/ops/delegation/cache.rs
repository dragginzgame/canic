//! Cache maintenance helpers for delegation sessions.

use crate::{Log, log, state::delegation::DelegationCache};

/// Remove expired cache entries and log when the count changes.
pub fn cleanup_sessions() {
    let (before, after) = DelegationCache::cleanup_expired();
    if before != after {
        log!(
            Log::Info,
            "Cleaned up cache sessions, before: {before}, after: {after}"
        );
    }
}
