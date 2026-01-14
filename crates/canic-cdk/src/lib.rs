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
pub use ic_canister_sig_creation as canister_sig_creation;
#[allow(deprecated)] // call is flagged as deprecated but functions inside arent
pub use ic_cdk::{
    api, call, eprintln, export_candid, futures, init, management_canister as mgmt, post_upgrade,
    println, query, trap, update,
};
pub use ic_cdk_timers as timers;
pub use ic_certified_map as certified_map;
pub use ic_signature_verification as signature_verification;
pub use icrc_ledger_types;

pub mod env;
pub mod spec;
pub mod structures;
pub mod types;
pub mod utils;
