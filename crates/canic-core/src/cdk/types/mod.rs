//! Module: cdk::types
//!
//! Responsibility: common IC-facing value types used by Canic runtime code.
//! Does not own: CDK API wrappers, stable structures, or serialization policy.
//! Boundary: centralizes internal named value types and semantic DTO inputs.

pub mod string;

pub(crate) use canic_contracts::cycles::*;
pub use string::*;

pub use candid::{Int, Nat, Principal};
