//! Module: model
//!
//! Responsibility: define pure runtime state models shared across layers.
//! Does not own: stable storage access, orchestration, or platform side effects.
//! Boundary: model types are consumed by ops, workflow, storage, and views.

#[cfg(feature = "blob-storage")]
pub mod blob_storage;
pub mod intent;
pub mod replay;
