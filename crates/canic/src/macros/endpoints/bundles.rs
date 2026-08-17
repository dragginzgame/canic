//! Module: macros::endpoints::bundles
//!
//! Responsibility: compose endpoint emitter macros into default runtime surfaces.
//! Does not own: endpoint implementations, lifecycle hooks, or Candid export order.
//! Boundary: preserves `start!` capability composition through explicit macro bundles.

/// Emit the root-only runtime endpoint surface.
#[macro_export]
macro_rules! canic_bundle_root_only_endpoints {
    () => {
        $crate::canic_emit_root_command_endpoint!();
        $crate::canic_emit_root_status_endpoint!();
    };
}

/// Emit the canonical Fleet Subnet Root-local wasm-store endpoint surface.
#[macro_export]
macro_rules! canic_bundle_wasm_store_runtime_endpoints {
    () => {
        $crate::canic_emit_local_wasm_store_endpoints!();
    };
}
