//! Native link proof for real managed mounts; no IC calls or database engine.
mod metrics_support;
use metrics_support::{self as __icydb_facade, generated as __icydb_generated};

canic_icydb::mount! {
    managed;
    METRICS_DECLARATIONS
}

// Disabled endpoints must leave these exact export identities available.
#[cfg(any(not(canic_capability_observability_metrics), not(METRICS_MOUNTED)))]
#[unsafe(export_name = "canister_query.icydb_metrics")]
fn disabled_read_identity() {}
#[cfg(any(not(canic_capability_observability_metrics), not(METRICS_MOUNTED)))]
#[unsafe(export_name = "canister_update.icydb_metrics_reset")]
fn disabled_reset_identity() {}

unsafe extern "C" {
    #[link_name = "canister_query.icydb_metrics"]
    fn selected_read();
    #[link_name = "canister_update.icydb_metrics_reset"]
    fn selected_reset();
}

#[test]
fn selected_metrics_exports_link_without_executing_platform_calls() {
    assert_eq!(cfg!(canic_capability_observability_metrics), EXPECTED_CAPABILITY);
    // Enabled builds link the real exports; disabled builds collide if either
    // real export leaks past the role policy. Never call either IC entrypoint.
    std::hint::black_box(selected_read as unsafe extern "C" fn());
    std::hint::black_box(selected_reset as unsafe extern "C" fn());
    metrics_support::assert_consumer_wire_contract();
}
