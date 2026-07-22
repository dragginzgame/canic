//! Canic facade crate.
//!
//! This crate is the recommended dependency for downstream canister projects. It
//! re-exports the public Canic runtime surface and provides the common macro entry points:
//!
//! - `build!` for configured canisters and generated local sandbox/probe config
//! - `start!` for `lib.rs` (wire lifecycle hooks and export endpoints)
//!
//! For lower-level access, use the `api`, `dto`, and `memory` modules.
//! These surfaces are for configured canister role packages. Shared runtime
//! libraries should remain independent of Canic and use upstream crates such
//! as `candid` or `ic-cdk` directly when they need generic IC types or APIs.
//! Direct access to internal core modules is intentionally unsupported.

pub mod access;
pub mod api;
#[cfg(any(not(target_arch = "wasm32"), test))]
mod build_support;
pub mod dto;
pub mod ids;
mod instructions;
mod macros; // private implementation boundary
pub mod prelude;
pub mod protocol;

#[doc(hidden)]
pub mod __internal {
    // NOTE:
    // This module exists ONLY for macro expansion.
    // Do NOT re-export canic_core publicly.
    #[cfg(any(feature = "control-plane", feature = "wasm-store-canister"))]
    pub use canic_control_plane as control_plane;
    pub use canic_core as core;

    pub mod cdk {
        pub use canic_core::cdk::types::Principal;
        pub use canic_core::cdk::{
            export_candid, init, inspect_message, post_upgrade, query, update,
        };

        pub mod api {
            pub use canic_core::cdk::api::{
                canister_cycle_balance, canister_version, is_controller, msg_caller, time,
            };
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
        METRICS_TIER_CORE, METRICS_TIER_PLACEMENT, METRICS_TIER_PLATFORM, METRICS_TIER_RUNTIME,
        METRICS_TIER_SECURITY, METRICS_TIER_STORAGE, assert_canonical_role_contract_build,
        config_attaches_role, config_contains_role, config_declares_role, config_fleet_name,
        declared_package_metadata, declared_package_role,
        emit_root_wasm_store_bootstrap_release_set, manifest_declares_workspace,
        metrics_profile_tier_mask, read_config_source_or_default, required_package_metadata,
        required_package_role, role_normal_dependency_metrics_enabled,
    };
}

// -----------------------------------------------------------------------------
// Sub-crates
// -----------------------------------------------------------------------------
pub use canic_core::memory;

// -----------------------------------------------------------------------------
// Re-exports
// -----------------------------------------------------------------------------
pub use canic_core::dto::error::Error;
pub use canic_core::{impl_storable_bounded, impl_storable_unbounded};
pub use canic_macros::{canic_query, canic_update};

// -----------------------------------------------------------------------------
// Constants
// -----------------------------------------------------------------------------

pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CANIC_WASM_CHUNK_BYTES: usize = canic_core::CANIC_WASM_CHUNK_BYTES;
pub const CANIC_DEFAULT_UPDATE_INGRESS_MAX_BYTES: usize =
    canic_core::ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES;
