//! Host-side fleet backup and restore primitives for Canic.
//!
//! This crate intentionally keeps backup-run provenance and restore contracts
//! outside canister runtime state. Root registry storage, topology cascade DTOs,
//! and backup manifests remain separate boundary types.

pub mod artifacts;
pub mod discovery;
pub mod execution;
pub mod journal;
pub mod manifest;
pub mod persistence;
pub mod plan;
pub mod restore;
pub mod runner;
pub mod snapshot;
#[cfg(test)]
mod test_support;
pub mod timestamp;
pub mod topology;
