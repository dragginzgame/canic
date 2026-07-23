//! Module: cdk::utils
//!
//! Responsibility: pure hash and hexadecimal helpers shared across the Canic stack.
//! Does not own: IC runtime APIs, serialization, or stable structures.
//! Boundary: deterministic byte utilities used by runtime and host crates.

pub mod hash;
