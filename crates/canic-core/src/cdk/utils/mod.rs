//! Module: cdk::utils
//!
//! Responsibility: small runtime-safe helpers shared across the Canic stack.
//! Does not own: canister API wrappers, serialization, or stable structures.
//! Boundary: exposes pure utility functions used by CDK-adjacent modules.

pub mod hash;
pub mod time;
