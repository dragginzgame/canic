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
pub use canic_dsl_macros::{canic_query, canic_update};
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
            access::{AuthAccessApi, EnvAccessApi, GuardAccessApi, RuleAccessApi},
            canister::CanisterRole,
            ic::Call,
            ops::{log, perf},
            timer::{timer, timer_interval},
        },
        cdk::{
            api::{canister_self, msg_caller},
            candid::CandidType,
            export_candid,
        },
    };

    pub use canic_dsl::access::{auth::*, env::*, guard::*};
    pub use canic_dsl_macros::{canic_query, canic_update};
}

///
/// Structured public runtime API surface.
///
/// This module groups Canic’s runtime capabilities by intent (auth, calls,
/// canister topology, observability, scheduling) rather than mirroring internal
/// core layout.
///

pub mod api {
    /// Access & authorization
    pub mod access {
        pub use crate::__internal::core::api::access::{
            auth::AuthAccessApi, env::EnvAccessApi, guard::GuardAccessApi, rule::RuleAccessApi,
        };
    }

    /// IC primitives (calls, HTTP, crypto, network, system APIs)
    pub mod ic {
        pub use crate::__internal::core::api::ic::call::{Call, CallBuilder, CallResult};

        pub mod http {
            pub use crate::__internal::core::api::ic::http::HttpApi;
        }

        pub mod network {
            pub use crate::__internal::core::api::ic::network::NetworkApi;
        }

        pub mod signature {
            pub use crate::__internal::core::api::ic::signature::SignatureApi;
        }
    }

    /// Canister lifecycle, placement, and topology
    pub mod canister {
        pub use crate::__internal::core::ids::CanisterRole;

        pub mod children {
            pub use crate::__internal::core::api::topology::children::CanisterChildrenApi;
        }

        pub mod directory {
            pub use crate::__internal::core::api::topology::directory::{
                AppDirectoryApi, SubnetDirectoryApi,
            };
        }

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

    /// Protocol (protocol runtime services)
    pub mod protocol {
        pub mod icrc21 {
            pub use crate::__internal::core::dispatch::icrc21::Icrc21Dispatcher;
        }
    }

    /// Timers and scheduling helpers
    pub mod timer {
        pub use crate::{timer, timer_interval};
    }
}
