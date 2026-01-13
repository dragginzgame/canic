//! Cascade propagation.
//!
//! Pushes environment/topology/state changes from root to child canisters.
//! This is orchestration logic (fanout), not storage and not placement strategy.

pub mod snapshot;
pub mod state;
pub mod topology;

use crate::{log, log::Topic};

const SYNC_CALL_WARN_THRESHOLD: usize = 10;

///
/// Helpers
///

fn warn_if_large(label: &str, count: usize) {
    if count > SYNC_CALL_WARN_THRESHOLD {
        log!(
            Topic::Sync,
            Warn,
            "sync: large {}: {} entries",
            label,
            count
        );
    }
}
