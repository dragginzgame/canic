//! Canic facade crate.
//!
//! This crate is the recommended dependency for downstream canister projects. It
//! re-exports the public Canic runtime surface and provides the common macro entry points:
//!
//! - `build!` / `build_root!` for `build.rs` (validate/embed `canic.toml`)
//! - `start!` / `start_root!` for `lib.rs` (wire lifecycle hooks and export endpoints)
//!
//! For lower-level access, use the `api`, `cdk`, `memory`, and `utils` modules.
//! Direct access to internal core modules is intentionally unsupported.

mod macros; // private implementation boundary

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
pub use canic_dsl as dsl;
pub use canic_memory as memory;
pub use canic_utils as utils;

// -----------------------------------------------------------------------------
// Re-exports
// -----------------------------------------------------------------------------
pub use canic_core::dto::error::Error;
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

    // Flat, opinionated imports
    pub use crate::api::{
        auth::{auth_require_all, auth_require_any, is_child, is_controller, is_parent, is_root},
        call::Call,
        canister::CanisterRole,
        ops::{log, perf},
        timer::{timer, timer_interval},
    };
}

///
/// Structured public runtime API surface.
///
/// This module groups Canic’s runtime capabilities by intent (auth, calls,
/// canister topology, observability, scheduling) rather than mirroring internal
/// core layout.
///

pub mod api {
    /// Authentication and caller/context inspection
    pub mod auth {
        pub use crate::__internal::core::access::auth::{
            is_child, is_controller, is_parent, is_root,
        };

        pub use crate::{auth_require_all, auth_require_any};
    }

    /// Inter-canister call primitives
    pub mod call {
        pub use crate::__internal::core::api::ic::call::{Call, CallBuilder, CallResult};
    }

    /// Canister lifecycle, placement, and topology
    pub mod canister {
        pub use crate::__internal::core::ids::CanisterRole;

        pub use crate::__internal::core::api::{
            placement::{scaling::ScalingApi, sharding::ShardingApi},
            wasm::WasmApi,
        };
    }

    /// RPC abstractions (non-IC-specific)
    pub mod rpc {
        pub use crate::__internal::core::api::rpc::RpcApi;
    }

    /// Observability and operational helpers
    pub mod ops {
        pub use crate::__internal::core::{log, perf};
    }

    /// Timers and scheduling helpers
    pub mod timer {
        pub use crate::{timer, timer_interval};
    }
}
