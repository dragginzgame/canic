//! Canic facade crate.
//!
//! This crate is the recommended dependency for downstream canister projects. It
//! re-exports the core Canic stack and provides the common macro entry points:
//! - `build!` / `build_root!` for `build.rs` (validate/embed `canic.toml`)
//! - `start!` / `start_root!` for `lib.rs` (wire lifecycle hooks and export endpoints)
//!
//! For lower-level access, use the `core`, `cdk`, `memory`, `types`, and `utils`
//! re-exports.

// -----------------------------------------------------------------------------
// Sub-crates
// -----------------------------------------------------------------------------
pub use canic_cdk as cdk;
pub use canic_core as core;
pub use canic_macros as macros;
pub use canic_utils as utils;

// -----------------------------------------------------------------------------
// Re-exports
// -----------------------------------------------------------------------------
pub use canic_core::{PublicError, build, build_root, log, log::Level, start, start_root};
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
        PublicError,
        cdk::{
            api::{canister_self, msg_caller},
            candid::CandidType,
            export_candid, init,
        },
        core::{
            access::auth::{is_child, is_controller, is_parent, is_root},
            auth_require_all, auth_require_any,
            ids::CanisterRole,
            log, perf, timer, timer_interval,
        },
        macros::{canic_query, canic_update},
    };
}
