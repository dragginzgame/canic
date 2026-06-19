//! Module: memory::registry
//!
//! Responsibility: define memory registry bootstrap and validation errors.
//! Does not own: allocation policy, stable schemas, or memory-manager storage.
//! Boundary: memory bootstrap maps `ic-memory` validation failures into this type.

use thiserror::Error as ThisError;

///
/// MemoryRegistryError
///
/// Canic-facing errors returned while bootstrapping or reading the
/// `ic-memory` allocation ledger.
/// Owned by memory registry and returned to lifecycle/bootstrap callers.
///

#[derive(Debug, ThisError)]
pub enum MemoryRegistryError {
    /// A declaration was rejected before or during `ic-memory` validation.
    #[error("memory declaration rejected for stable key '{stable_key}': {reason}")]
    InvalidDeclaration {
        stable_key: String,
        reason: &'static str,
    },

    /// The stable key namespace and memory ID range do not match.
    #[error(
        "memory stable key '{stable_key}' with id {id} violates namespace/range authority: {reason}"
    )]
    RangeAuthorityViolation {
        /// Stable key being registered.
        stable_key: String,
        /// Stable-memory ID being registered.
        id: u8,
        /// Human-readable reason for the rejection.
        reason: &'static str,
    },

    /// The persisted ABI ledger cannot be validated.
    #[error("memory layout ledger is corrupt: {reason}")]
    LedgerCorrupt {
        /// Human-readable corruption reason.
        reason: &'static str,
    },
}
