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
    // NOTE:
    // This module exists ONLY for macro expansion.
    // Do NOT re-export canic_core publicly.
    pub use canic_core as core;
}

// -----------------------------------------------------------------------------
// Public data contracts
// -----------------------------------------------------------------------------
// DTOs, IDs, and protocol definitions are stable, versioned contracts intended
// for downstream use (candid, RPC, tests, tooling).
pub use canic_core::{dto, ids, protocol};

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
    pub use crate::{
        api::{
            access::{
                auth::{
                    auth_require_all, auth_require_any, caller_is_child, caller_is_controller,
                    caller_is_parent, caller_is_root,
                },
                env::{self_is_prime_root, self_is_prime_subnet, self_is_root},
            },
            canister::CanisterRole,
            ic::Call,
            ops::{log, perf},
            timer::{timer, timer_interval},
        },
        cdk::{candid::CandidType, export_candid},
    };

    pub use canic_dsl::{canic_query, canic_update};
}

///
/// Structured public runtime API surface.
///
/// This module groups Canic’s runtime capabilities by intent (auth, calls,
/// canister topology, observability, scheduling) rather than mirroring internal
/// core layout.
///

pub mod api {
    // ─────────────────────────────
    // Access & authorization
    // ─────────────────────────────
    pub mod access {
        pub mod auth {
            pub use crate::__internal::core::access::auth::{
                is_child as caller_is_child, is_controller as caller_is_controller,
                is_parent as caller_is_parent, is_root as caller_is_root,
            };

            pub use crate::{auth_require_all, auth_require_any};
        }

        pub mod env {
            pub use crate::__internal::core::access::env::{
                is_prime_root as self_is_prime_root, is_prime_subnet as self_is_prime_subnet,
                is_root as self_is_root,
            };
        }

        pub mod guard {
            pub use crate::__internal::core::access::guard::{guard_app_query, guard_app_update};
        }
    }

    /// IC primitives
    pub mod ic {
        pub use crate::__internal::core::api::ic::{
            call::{Call, CallBuilder, CallResult},
            http::HttpApi,
        };
    }

    /// Canister lifecycle, placement, and topology
    pub mod canister {
        pub use crate::__internal::core::ids::CanisterRole;

        pub mod placement {
            pub use crate::__internal::core::api::placement::{
                scaling::ScalingApi, sharding::ShardingApi,
            };
        }

        pub mod wasm {
            pub use crate::__internal::core::api::wasm::WasmApi;
        }
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
