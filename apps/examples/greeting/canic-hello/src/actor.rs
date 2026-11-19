//! Explicit demo composition. Neither mount initializes a service or owns lifecycle.
#[cfg(target_arch = "wasm32")]
use ic_platform_runtime::system::time as now_ns;

use crate::{GREET, Greeting, LIMITS, greeting};

/// Greet a visitor in a read-only query.
#[canic::query(method = GREET, limits = LIMITS)]
fn greet(name: String) -> Greeting {
    greeting(&name)
}

/// Return the same greeting through a replicated update.
#[canic::update(method = GREET, limits = LIMITS)]
fn probe(name: String) -> Greeting {
    greeting(&name)
}

canic::mount_readiness_endpoint!(crate::actor::readiness);

fn readiness() -> canic::readiness::State {
    // This actor has no persistent state or lifecycle to open.
    canic::readiness::State::Ready
}

canic::mount_metrics_endpoint!(crate::actor::snapshot);

fn snapshot() -> canic::metrics::Snapshot {
    canic::metrics::Snapshot {
        observed_at_ns: now_ns(),
        timers: None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock before Unix epoch")
        .as_nanos()
        .try_into()
        .expect("timestamp outside u64 nanoseconds")
}
