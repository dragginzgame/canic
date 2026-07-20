//! Host-side deployment backup and restore primitives for Canic.
//!
//! This crate intentionally keeps backup-run provenance and restore contracts
//! outside canister runtime state. Root registry storage, topology cascade DTOs,
//! and backup manifests remain separate boundary types.

pub mod artifacts;
pub mod discovery;
pub mod execution;
mod hash;
pub mod journal;
pub mod manifest;
#[cfg(test)]
pub(crate) mod operational_readiness;
pub mod persistence;
pub mod plan;
pub mod registry;
pub mod restore;
pub mod runner;
mod serialization;
#[cfg(test)]
mod test_support;
pub mod timestamp;
pub mod topology;
