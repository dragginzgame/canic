//! Canic facade crate.
//!
//! This crate is the recommended dependency for downstream canister projects. It
//! re-exports the public Canic runtime surface and provides the common macro entry points:
//!
//! - `build!` / `build_root!` for `build.rs` (validate/embed `canic.toml`)
//! - `start!` / `start_root!` for `lib.rs` (wire lifecycle hooks and export endpoints)
//!
//! For lower-level access, use the `api`, `cdk`, and `memory` modules.
//! Direct access to internal core modules is intentionally unsupported.

#[cfg(feature = "control-plane")]
mod bootstrap;
#[cfg(any(not(target_arch = "wasm32"), test))]
mod build_support;
mod instructions;
mod macros; // private implementation boundary
pub mod protocol;

#[doc(hidden)]
pub mod __internal {
    // NOTE:
    // This module exists ONLY for macro expansion.
    // Do NOT re-export canic_core publicly.
    #[cfg(feature = "control-plane")]
    pub use canic_control_plane as control_plane;
    pub use canic_core as core;
    #[cfg(feature = "sharding")]
    pub use canic_sharding_runtime as sharding;

    #[cfg(feature = "control-plane")]
    pub mod bootstrap {
        pub use crate::bootstrap::root_wasm_store_wasm;
    }

    pub mod instructions {
        pub use crate::instructions::format_instructions;
    }
}

#[doc(hidden)]
#[cfg(any(not(target_arch = "wasm32"), test))]
pub mod __build {
    pub use crate::build_support::emit_root_release_bundle;
}

// -----------------------------------------------------------------------------
// Public data contracts
// -----------------------------------------------------------------------------
// DTOs and IDs are stable, versioned contracts intended for downstream use.
pub mod dto {
    pub use canic_core::dto::*;

    #[cfg(feature = "control-plane")]
    pub mod template {
        pub use canic_control_plane::dto::template::*;
    }
}

pub mod ids {
    pub use crate::__internal::core::ids::{
        AccessMetricKind, BuildNetwork, CanisterRole, EndpointCall, EndpointCallKind, EndpointId,
        IntentResourceKey, SubnetRole, SystemMetricKind, cap,
    };

    #[cfg(feature = "control-plane")]
    pub use canic_control_plane::ids::{
        TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion, WasmStoreBinding,
        WasmStoreGcMode, WasmStoreGcStatus,
    };
}

// -----------------------------------------------------------------------------
// Sub-crates
// -----------------------------------------------------------------------------
pub use canic_cdk as cdk;
pub use canic_memory as memory;

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
// Access predicates
// -----------------------------------------------------------------------------
pub mod access {
    pub use crate::__internal::core::access::{AccessError, app, auth, env};
}

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
            canister::CanisterRole,
            ic::Call,
            ops::{log, perf},
            timer::{timer, timer_interval},
        },
        cdk::{
            api::{canister_self, msg_caller},
            candid::CandidType,
        },
    };

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

    /// Delegation workflow helpers
    pub mod auth {
        pub use crate::__internal::core::api::auth::DelegationApi;
    }

    /// Environment queries
    pub mod env {
        pub use crate::__internal::core::api::env::EnvQuery;
    }

    /// IC primitives (calls, HTTP, crypto, network, system APIs)
    pub mod ic {
        pub use crate::__internal::core::api::ic::call::{
            Call, CallBuilder, CallResult, IntentKey, IntentReservation,
        };

        pub mod http {
            pub use crate::__internal::core::api::ic::http::HttpApi;
        }

        pub mod network {
            pub use crate::__internal::core::api::ic::network::NetworkApi;
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

        pub mod registry {
            pub use crate::__internal::core::api::topology::registry::{
                AppRegistryApi, SubnetRegistryApi,
            };
        }

        pub mod placement {
            pub use crate::__internal::core::api::placement::scaling::ScalingApi;

            #[cfg(feature = "sharding")]
            pub use crate::__internal::sharding::api::ShardingApi;
        }

        #[cfg(feature = "control-plane")]
        pub mod template {
            pub use canic_control_plane::api::template::WasmStoreApi as EmbeddedTemplateApi;
            pub use canic_control_plane::api::template::{
                WasmStoreApi, WasmStoreBootstrapApi, WasmStoreCanisterApi, WasmStorePublicationApi,
            };
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

    /// Runtime bootstrap helpers
    pub mod runtime {
        pub use crate::__internal::core::api::runtime::MemoryRuntimeApi;
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
