use crate::{Log, log, state::delegation::DelegationCache};

pub fn cleanup_sessions() {
    let (before, after) = DelegationCache::cleanup_expired();
    if before != after {
        log!(
            Log::Info,
            "Cleaned up cache sessions, before: {before}, after: {after}"
        );
    }
}
