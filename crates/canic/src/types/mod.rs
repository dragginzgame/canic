//!
//! Shared type wrappers and aliases used across the ops and endpoint layers.
//!
//! These helpers centralize candid-friendly structs plus bounded/principal
//! utilities so consumers can `use canic::types::*` without reaching into
//! submodules.
//!

mod canister;
mod cycles;
mod string;
mod wasm;

pub use canister::*;
pub use cycles::*;
pub use string::*;
pub use wasm::*;

//
// common types
//

pub use crate::cdk::candid::{Int, Nat, Principal};
pub use icrc_ledger_types::icrc1::account::{Account, Subaccount};
pub use serde_bytes::ByteBuf;
