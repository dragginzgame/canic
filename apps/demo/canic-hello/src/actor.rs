//! Explicit demo composition. Neither mount initializes a service or owns lifecycle.
canic::mount_greeting_endpoints!(crate::greeting);
canic::mount_metrics_endpoint!(crate::actor::snapshot);
canic::mount_readiness_endpoint!(crate::actor::readiness);

fn readiness() -> canic::readiness::State {
    // This actor has no persistent state or lifecycle to open.
    canic::readiness::State::Ready
}

fn snapshot() -> canic::metrics::Snapshot {
    canic::metrics::Snapshot {
        observed_at_ns: ic_platform_runtime::system::time(),
        // This stateless application has no timer provider; unavailable is not zero.
        timers: None,
    }
}
