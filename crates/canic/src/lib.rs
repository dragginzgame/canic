//! Canic facade crate.
//!
//! This crate is the recommended dependency for downstream canister projects. It
//! re-exports the public Canic runtime surface and provides the common macro entry points:
//!
//! - `build!` / `build_root!` for strict configured canisters
//! - `build_standalone!` for sandbox/probe canisters with generated minimal config
//! - `start!` / `start_root!` for `lib.rs` (wire lifecycle hooks and export endpoints)
//!
//! For lower-level access, use the `api`, `cdk`, and `memory` modules.
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
    #[cfg(feature = "control-plane")]
    pub use canic_control_plane as control_plane;
    pub use canic_core as core;

    pub mod instructions {
        pub use crate::instructions::format_instructions;
    }
}

#[doc(hidden)]
#[cfg(any(not(target_arch = "wasm32"), test))]
pub mod __build {
    pub use crate::build_support::{
        emit_root_wasm_store_bootstrap_release_set, read_config_source_or_default,
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
pub use canic_macros::{canic_query, canic_update};

// -----------------------------------------------------------------------------
// Constants
// -----------------------------------------------------------------------------

pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CANIC_WASM_CHUNK_BYTES: usize = canic_core::CANIC_WASM_CHUNK_BYTES;
pub const CANIC_DEFAULT_UPDATE_INGRESS_MAX_BYTES: usize =
    canic_core::ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES;
