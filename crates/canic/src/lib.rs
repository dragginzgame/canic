//! Canic facade crate.
//!
//! This crate is the recommended dependency for downstream canister projects. It
//! re-exports the core Canic stack and provides the common macro entry points:
//! - `build!` / `build_root!` for `build.rs` (validate/embed `canic.toml`)
//! - `start!` / `start_root!` for `lib.rs` (wire lifecycle hooks and export endpoints)
//!
//! For lower-level access, use the `api`, `cdk`, `memory`, and `utils` modules.
//! Direct access to internal core modules is intentionally unsupported.

mod macros;

#[doc(hidden)]
pub mod __internal {
    pub use canic_core as core;
}
#[doc(hidden)]
pub use __internal::core;

// -----------------------------------------------------------------------------
// Sub-crates
// -----------------------------------------------------------------------------
pub use canic_cdk as cdk;
pub use canic_utils as utils;

// -----------------------------------------------------------------------------
// Re-exports
// -----------------------------------------------------------------------------
pub use canic_core::PublicError;
pub use canic_memory::{
    eager_init, eager_static, ic_memory, ic_memory_range, impl_storable_bounded,
    impl_storable_unbounded,
};

// -----------------------------------------------------------------------------
// Constants
// -----------------------------------------------------------------------------

pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// -----------------------------------------------------------------------------
// Prelude
// -----------------------------------------------------------------------------

///
/// Opinionated prelude for Canic canister crates.
///
/// Prefer importing from the prelude in your canister `lib.rs` to keep endpoint
/// modules small and consistent. Library crates and shared modules should
/// generally import from specific paths instead of pulling in the entire prelude.
///

pub mod prelude {
    pub use crate::cdk::{
        api::{canister_self, msg_caller},
        candid::CandidType,
        export_candid,
    };

    pub use canic_dsl::{canic_query, canic_update};

    // Explicit, curated exports (no `core::` paths)
    pub use crate::api::{
        Call, CanisterRole, auth_require_all, auth_require_any, is_child, is_controller, is_parent,
        is_root, log, perf, timer, timer_interval,
    };
}

pub mod api {
    pub use crate::__internal::core::{
        access::auth::{is_child, is_controller, is_parent, is_root},
        api::{
            ic::call::{Call, CallBuilder, CallResult},
            placement::{scaling::ScalingApi, sharding::ShardingApi},
            rpc::RpcApi,
            wasm::WasmApi,
        },
        ids::CanisterRole,
        log, perf,
    };

    pub use crate::{auth_require_all, auth_require_any, timer, timer_interval};
}
