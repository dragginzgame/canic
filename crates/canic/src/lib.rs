//! Canic facade crate.
//!
//! This crate is the recommended dependency for downstream canister projects. It
//! re-exports the public Canic runtime surface and provides the common macro entry points:
//!
//! - `build!` for configured canisters and generated local sandbox/probe config
//! - `start!` for configured canister lifecycle hooks and endpoints
//! - `start_fleet_coordinator!` for Canic's canonical Coordinator artifact source
//! - `finish!` after all endpoint definitions to complete the canister module
//!
//! Application canisters using `start!` define async `canic_setup()`,
//! `canic_install(args: Option<Vec<u8>>)` and `canic_upgrade()` hooks, each
//! returning `()`. Canic defers these hooks until after restoration.
//!
//! For lower-level access, use the `api`, `dto`, and `memory` modules.
//! These surfaces are for configured canister role packages. Shared runtime
//! libraries should remain independent of Canic and use upstream crates such
//! as `ic-platform` for generic IC types and supported runtime APIs. Direct
//! Platform owns the Candid/CDK boundary documented in
//! `docs/governance/platform-boundary.md`.
//! Direct access to internal core modules is intentionally unsupported.

#![deny(unreachable_pub)]

pub mod access;
pub mod api;
#[cfg(any(not(target_arch = "wasm32"), test))]
mod build_support;
pub mod diagnostics;
pub mod dto;
pub mod endpoint;
pub mod fleet_admission;
pub mod ids;
mod instructions;
mod macros; // private implementation boundary
pub mod prelude;
pub mod protocol;
#[cfg(all(feature = "testing", not(target_arch = "wasm32")))]
pub mod testing;

#[doc(hidden)]
pub mod __internal {
    pub use canic_macros::__canic_lifecycle;
    // NOTE:
    // Public only to support macro expansion in consuming crates.
    // These implementation dependencies are not the supported application API.
    pub use canic_contracts as contracts;
    #[cfg(any(
        feature = "control-plane",
        feature = "fleet-coordinator-canister",
        feature = "wasm-store-canister"
    ))]
    pub use canic_control_plane as control_plane;
    pub use canic_core as core;
    pub use ic_platform;
    pub use ic_wire;
    pub use serde;

    pub mod cdk {
        pub use ic_platform::cdk::{futures, init, inspect_message, post_upgrade, query, update};
        pub use ic_platform::principal::ExternalPrincipal as Principal;

        pub mod api {
            pub use ic_platform::cdk::api::{canister_cycle_balance, canister_version};
            pub use ic_platform::cdk::api::{
                is_controller, msg_caller, msg_reject, msg_reply, time,
            };
        }

        pub mod raw {
            pub use ic_platform::ic0::{msg_arg_data_copy, msg_arg_data_size};
        }
    }

    pub mod instructions {
        pub use crate::instructions::format_instructions;
    }
}

#[doc(hidden)]
#[cfg(any(not(target_arch = "wasm32"), test))]
pub mod __build {
    pub use crate::build_support::{
        CANIC_CUSTOM_CFG_NAMES, METRICS_TIER_CORE, METRICS_TIER_PLACEMENT, METRICS_TIER_PLATFORM,
        METRICS_TIER_RUNTIME, METRICS_TIER_SECURITY, METRICS_TIER_STORAGE,
        assert_canonical_role_contract_build, compile_role_build_sources, config_app_id,
        config_declares_role, configured_role_metrics_tier_mask, read_config_source_or_default,
        required_package_metadata, write_build_source_if_changed,
    };
}

// -----------------------------------------------------------------------------
// Sub-crates
// -----------------------------------------------------------------------------
pub mod memory;

// -----------------------------------------------------------------------------
// Re-exports
// -----------------------------------------------------------------------------
pub use canic_contracts::dto::error::Error;
pub use canic_core::{impl_storable_bounded, impl_storable_unbounded};
pub use canic_macros::{candid_query, candid_update, query, raw_query, raw_update, update};
pub use diagnostics::{DiagnosticCode, RegisteredDiagnosticCode};

// -----------------------------------------------------------------------------
// Constants
// -----------------------------------------------------------------------------

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CANIC_WASM_CHUNK_BYTES: usize = canic_core::CANIC_WASM_CHUNK_BYTES;
pub const CANIC_DEFAULT_UPDATE_INGRESS_MAX_BYTES: usize =
    canic_core::ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES;
