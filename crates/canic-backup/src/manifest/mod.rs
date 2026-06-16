//! Module: manifest
//!
//! Responsibility: define, validate, and summarize backup manifests.
//! Does not own: discovery, snapshot capture, restore execution, or storage IO.
//! Boundary: exposes backup manifest contracts to backup and restore flows.

mod error;
mod summary;
#[cfg(test)]
mod tests;
mod types;
mod validation;

pub use error::ManifestValidationError;
pub use summary::manifest_validation_summary;
pub use types::*;
