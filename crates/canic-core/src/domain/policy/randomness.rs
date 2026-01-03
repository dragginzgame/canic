use crate::config::schema::RandomnessConfig;
use std::time::Duration;

///
/// Randomness scheduling policy.
///

#[must_use]
pub const fn schedule(cfg: &RandomnessConfig) -> Option<Duration> {
    if !cfg.enabled {
        return None;
    }

    let interval_secs = cfg.reseed_interval_secs;
    if interval_secs == 0 {
        return None;
    }

    Some(Duration::from_secs(interval_secs))
}
