//! Module: macros::endpoints::bundles
//! Responsibility: compose endpoint emitter macros into default runtime surfaces.
//! Does not own: endpoint implementations, lifecycle hooks, or Candid export order.
//! Boundary: preserves `start!` capability composition through explicit macro bundles.

/// Emit the default shared runtime endpoint surface for all Canic canisters.
#[macro_export]
macro_rules! canic_bundle_shared_runtime_endpoints {
    () => {
        $crate::canic_emit_lifecycle_core_endpoints!();
        #[cfg(canic_memory_ledger_enabled)]
        $crate::canic_emit_memory_ledger_diagnostic_endpoint!();
        $crate::canic_bundle_discovery_endpoints!();
        $crate::canic_bundle_observability_endpoints!();
        #[cfg(not(canic_disable_bundle_metrics))]
        $crate::canic_emit_metrics_endpoints!();
        #[cfg(not(canic_disable_bundle_cycle_tracker))]
        $crate::canic_emit_cycle_tracker_endpoints!();
        #[cfg(not(canic_disable_bundle_auth_attestation))]
        $crate::canic_emit_auth_attestation_endpoints!();
        $crate::canic_bundle_topology_views_endpoints!();
    };
}

/// Emit the root-only runtime endpoint surface.
#[macro_export]
macro_rules! canic_bundle_root_only_endpoints {
    () => {
        $crate::canic_emit_root_admin_endpoints!();
        $crate::canic_emit_root_auth_attestation_endpoints!();
        $crate::canic_emit_root_wasm_store_endpoints!();
    };
}

/// Emit the non-root-only runtime endpoint surface.
#[macro_export]
macro_rules! canic_bundle_nonroot_only_endpoints {
    () => {
        #[cfg(not(canic_disable_bundle_nonroot_sync_topology))]
        $crate::canic_emit_nonroot_sync_topology_endpoints!();
        #[cfg(canic_delegated_token_issuer)]
        $crate::canic_emit_nonroot_auth_attestation_endpoints!();
    };
}

/// Emit the canonical subnet-local wasm-store runtime endpoint surface.
#[macro_export]
macro_rules! canic_bundle_wasm_store_runtime_endpoints {
    () => {
        $crate::canic_emit_lifecycle_core_endpoints!();
        #[cfg(canic_memory_ledger_enabled)]
        $crate::canic_emit_memory_ledger_diagnostic_endpoint!();
        $crate::canic_bundle_discovery_endpoints!();
        #[cfg(not(canic_disable_bundle_cycle_tracker))]
        $crate::canic_emit_cycle_tracker_endpoints!();
        #[cfg(not(canic_disable_bundle_auth_attestation))]
        $crate::canic_emit_auth_attestation_endpoints!();
        #[cfg(not(canic_disable_bundle_nonroot_sync_topology))]
        $crate::canic_emit_nonroot_sync_topology_endpoints!();
        #[cfg(canic_delegated_token_issuer)]
        $crate::canic_emit_nonroot_auth_attestation_endpoints!();
        $crate::canic_emit_local_wasm_store_endpoints!();
    };
}
