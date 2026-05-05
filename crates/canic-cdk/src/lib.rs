///
/// Unified façade over the Internet Computer SDK
///
/// This module re-exports the core IC developer crates (`ic_cdk`, `candid`,
/// `ic_cdk_timers`, and management-canister APIs) under a single, stable
/// namespace.  The goal is to provide Canic users with a consistent import
/// surface regardless of how the underlying IC SDK evolves.
///
/// By collecting these crates into one place:
///
/// - downstream code can simply use `canic::cdk::*` instead of pulling in
///   several IC SDK crates directly;
/// - the Canic framework can update or reorganize its IC dependencies
///   without requiring changes in dependent canisters;
/// - the public API surface is easier to document, search, and version;
/// - consumers benefit from a curated, intentional subset of the IC SDK.
///
pub use candid;
pub use ic_cdk::{
    api, call, eprintln, export_candid, futures, init, inspect_message, post_upgrade, println,
    query, trap, update,
};
pub use ic_cdk_management_canister as mgmt;
pub use ic_cdk_timers as timers;
pub use icrc_ledger_types;

/// Export Candid only in debug builds.
#[macro_export]
macro_rules! export_candid_debug {
    () => {
        #[cfg(debug_assertions)]
        $crate::export_candid!();
    };
}

pub mod env;
pub mod spec;
pub mod structures;
pub mod types;
pub mod utils;
