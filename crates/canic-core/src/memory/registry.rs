//! Module: memory::registry
//!
//! Responsibility: define memory registry bootstrap and validation errors.
//! Does not own: allocation policy, stable schemas, or Storage allocation.
//! Boundary: memory bootstrap maps `ic-storage` validation failures into this type.

use thiserror::Error as ThisError;

///
/// MemoryRegistryError
///
/// Canic-facing errors returned while bootstrapping or reading the
/// `ic-storage` allocation ledger.
/// Owned by memory registry and returned to lifecycle/bootstrap callers.
///

#[derive(Debug, ThisError)]
pub enum MemoryRegistryError {
    /// A composed consumer rejected the sealed snapshot before commitment.
    #[error("composed memory admission rejected: {source}")]
    Admission {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// A declaration was rejected before or during `ic-storage` validation.
    #[error("memory declaration rejected for stable key '{stable_key}': {reason}")]
    InvalidDeclaration {
        stable_key: String,
        reason: &'static str,
    },

    /// The host supplied invalid namespace grants.
    #[error(transparent)]
    Grants(#[from] ic_storage::AdmissionError),
}
