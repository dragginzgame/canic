//! Pure domain value re-exports used by policy and decision helpers.
//!
//! This module gives pure domain code a non-runtime namespace for shared value
//! types that are also used at IC/CDK boundaries. Re-exporting the same types
//! keeps serialized shapes and equality semantics unchanged.

pub use crate::cdk::types::{BoundedString64, Principal};
