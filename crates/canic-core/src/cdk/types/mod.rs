//! Module: cdk::types
//!
//! Responsibility: common IC-facing value types re-exported through Canic CDK.
//! Does not own: CDK API wrappers, stable structures, or serialization policy.
//! Boundary: centralizes type aliases and wrappers used by Canic-facing code.

pub mod cycles;
pub mod string;

pub use cycles::*;
pub use string::*;

pub use candid::{Int, Nat, Principal};
pub use icrc_ledger_types::icrc1::account::{Account, Subaccount};
