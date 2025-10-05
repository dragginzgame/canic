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
// Subaccount - this is the wrapped one from ic_ledger_types
// as the one in icrc_ is a type alias
//

pub use crate::cdk::candid::{Int, Nat, Principal};
pub use ic_ledger_types::Subaccount;
pub use icrc_ledger_types::icrc1::account::Account;
pub use serde_bytes::ByteBuf;
