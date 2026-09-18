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
    /// A composed consumer rejected the sealed snapshot before commitment.
    #[error("composed memory admission rejected: {source}")]
    Admission {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Exactly one artifact owner may compose consumer admission.
    #[error("memory admission is already registered")]
    AdmissionAlreadyRegistered,

    /// Admission cannot change after Canic has selected its bootstrap policy.
    #[error("memory admission registration is sealed")]
    AdmissionRegistrationSealed,

    /// A poisoned registry cannot supply trustworthy admission authority.
    #[error("memory admission registry is poisoned")]
    AdmissionRegistryPoisoned,

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
}
