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
    /// No successful host bootstrap supplied a diagnostic pool.
    #[error("host memory pool is not committed")]
    PoolUnavailable,

    /// A composed consumer rejected the sealed snapshot before commitment.
    #[error("composed memory admission rejected: {source}")]
    Admission {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// A declaration was rejected before or during `ic-memory` validation.
    #[error("memory declaration rejected for stable key '{stable_key}': {reason}")]
    InvalidDeclaration {
        stable_key: String,
        reason: &'static str,
    },

    /// The host supplied invalid namespace grants or physical exclusions.
    #[error(transparent)]
    Pool(#[from] ic_memory::MemoryAllocationPoolError),
}
