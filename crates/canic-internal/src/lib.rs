//! Workspace-only helpers and fixtures.
//!
//! This crate is not published. It exists to host shared constants and small
//! helpers used by the workspace reference canisters and integration scaffolding.

pub mod canister;
pub mod reference;
#[cfg(any(not(target_arch = "wasm32"), test))]
pub mod release_set;
