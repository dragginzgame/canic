//!
//! Shared type wrappers and aliases used across the ops and endpoint layers.
//!
//! These helpers centralize candid-friendly structs plus bounded/principal
//! utilities so consumers can `use canic::ids::*` without reaching into
//! submodules.
//!

mod canister;
mod subnet;

pub use canister::*;
pub use subnet::*;
