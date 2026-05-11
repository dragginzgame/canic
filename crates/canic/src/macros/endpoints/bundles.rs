// -----------------------------------------------------------------------------
// Endpoint bundle macros
// -----------------------------------------------------------------------------

// Macros that generate public IC endpoints for Canic canisters.
// These emitters and bundles define the compile-time capability surface for
// `start!` and `start_root!`. The default compositions intentionally preserve
// the current feature set; bundle boundaries exist to make linker policy
// explicit.

// Bundle composer for the default shared runtime surface on all Canic canisters.
#[macro_export]
macro_rules! canic_bundle_shared_runtime_endpoints {
    () => {
        $crate::canic_emit_lifecycle_core_endpoints!();
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

// Bundle composer for the root-only runtime surface.
#[macro_export]
macro_rules! canic_bundle_root_only_endpoints {
    () => {
        $crate::canic_emit_root_admin_endpoints!();
        $crate::canic_emit_root_auth_attestation_endpoints!();
        $crate::canic_emit_root_wasm_store_endpoints!();
    };
}

// Bundle composer for the non-root-only runtime surface.
#[macro_export]
macro_rules! canic_bundle_nonroot_only_endpoints {
    () => {
        #[cfg(not(canic_disable_bundle_nonroot_sync_topology))]
        $crate::canic_emit_nonroot_sync_topology_endpoints!();
        $crate::canic_emit_nonroot_auth_attestation_endpoints!();
    };
}

// Bundle composer for the canonical subnet-local wasm_store runtime surface.
#[macro_export]
macro_rules! canic_bundle_wasm_store_runtime_endpoints {
    () => {
        $crate::canic_emit_lifecycle_core_endpoints!();
        $crate::canic_bundle_discovery_endpoints!();
        #[cfg(not(canic_disable_bundle_cycle_tracker))]
        $crate::canic_emit_cycle_tracker_endpoints!();
        #[cfg(not(canic_disable_bundle_auth_attestation))]
        $crate::canic_emit_auth_attestation_endpoints!();
        #[cfg(not(canic_disable_bundle_nonroot_sync_topology))]
        $crate::canic_emit_nonroot_sync_topology_endpoints!();
        $crate::canic_emit_nonroot_auth_attestation_endpoints!();
        $crate::canic_emit_local_wasm_store_endpoints!();
    };
}
