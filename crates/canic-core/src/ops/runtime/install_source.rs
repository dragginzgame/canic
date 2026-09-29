//! Module: ops::runtime::install_source
//!
//! Responsibility: describe approved Store-backed chunk sources for install workflows.
//! Does not own: source resolution, publication, storage, or install execution.
//! Boundary: the control plane supplies exact source metadata to installation workflows.

use crate::cdk::types::Principal;

///
/// ApprovedModuleSource
///
/// Approved install source metadata and payload for one canister role.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovedModuleSource {
    source_canister: Principal,
    source_label: String,
    module_hash: Vec<u8>,
    chunk_hashes: Vec<Vec<u8>>,
    payload_size_bytes: u64,
}

impl ApprovedModuleSource {
    /// Construct one chunk-store-backed module source.
    #[must_use]
    pub const fn chunked(
        source_canister: Principal,
        source_label: String,
        module_hash: Vec<u8>,
        chunk_hashes: Vec<Vec<u8>>,
        payload_size_bytes: u64,
    ) -> Self {
        Self {
            source_canister,
            source_label,
            module_hash,
            chunk_hashes,
            payload_size_bytes,
        }
    }

    /// Return the Store canister that owns the approved chunk set.
    #[must_use]
    pub const fn source_canister(&self) -> &Principal {
        &self.source_canister
    }

    /// Return the logical source label used for logs and status output.
    #[must_use]
    pub fn source_label(&self) -> &str {
        &self.source_label
    }

    /// Return the installable wasm module hash.
    #[must_use]
    pub fn module_hash(&self) -> &[u8] {
        &self.module_hash
    }

    /// Return the raw payload size in bytes.
    #[must_use]
    pub const fn payload_size_bytes(&self) -> u64 {
        self.payload_size_bytes
    }

    /// Return the approved chunk hashes in deterministic install order.
    #[must_use]
    pub fn chunk_hashes(&self) -> &[Vec<u8>] {
        &self.chunk_hashes
    }

    /// Return the approved chunk count.
    #[must_use]
    pub const fn chunk_count(&self) -> usize {
        self.chunk_hashes.len()
    }
}
