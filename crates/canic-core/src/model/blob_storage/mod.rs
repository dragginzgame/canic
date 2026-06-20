//! Module: model::blob_storage
//!
//! Responsibility: define pure blob-storage state models shared across layers.
//! Does not own: stable storage access, endpoint authorization, or gateway calls.
//! Boundary: consumed by blob-storage ops, storage, workflow, and views.

mod hash;

pub use hash::{BLOB_ROOT_HASH_BYTE_LENGTH, BlobRootHash, BlobRootHashError};
