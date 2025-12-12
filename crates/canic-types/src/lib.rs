//! Shared type wrappers and aliases used across the ops and endpoint layers.
//!
//! These helpers centralize candid-friendly structs plus bounded/principal
//! utilities so consumers can `use canic::core::types::*` without reaching into
//! submodules.

pub use canic_cdk as cdk;
pub use canic_utils as utils;
pub use canic_utils::macros::*;

mod account;
mod cycles;
mod page;
mod string;
mod ulid;
mod wasm;

pub use account::*;
pub use cycles::*;
pub use page::*;
pub use string::*;
pub use ulid::*;
pub use wasm::*;

// common aliases
pub use canic_cdk::candid::{Int, Nat, Principal};
pub use serde_bytes::ByteBuf;
