//! Thin facade over the Canic stack re-exporting the core crate and helpers.

///
/// RE-EXPORTS
///
pub use canic_core::{Error, build, build_root, log, log::Level, start, start_root};
pub use canic_memory::{
    eager_init, eager_static, ic_memory, ic_memory_range, impl_storable_bounded,
    impl_storable_unbounded,
};

///
/// SUB-CRATES
///
pub use canic_cdk as cdk;
pub use canic_core as core;
pub use canic_macros as macros;
pub use canic_types as types;
pub use canic_utils as utils;

///
/// CONSTANTS
///

pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

///
/// Prelude
/// should only be used in the Actor file
///

pub mod prelude {
    pub use crate::{
        Error as CanicError,
        cdk::{
            api::{canister_self, msg_caller},
            candid::CandidType,
            export_candid, init,
        },
        core::{
            access::auth::{is_controller, is_parent, is_root},
            auth_require_all, auth_require_any,
            ids::CanisterRole,
            log,
            ops::ic::{call::Call, http::Http},
            perf, perf_scope, timer, timer_interval,
        },
        macros::{canic_query, canic_update},
    };
}
