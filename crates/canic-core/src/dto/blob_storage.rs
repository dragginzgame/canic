use crate::dto::prelude::*;

///
/// CreateCertificateResult
///
/// Passive DTO returned by the blob-storage create-certificate endpoint.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CreateCertificateResult {
    pub method: String,
    pub blob_hash: String,
}

///
/// BlobStorageLocalCounters
///
/// Passive DTO for host-owned blob-storage status wrappers.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlobStorageLocalCounters {
    pub stored_blobs: u64,
    pub pending_deletions: u64,
    pub gateway_principals: u64,
}

impl BlobStorageLocalCounters {
    #[must_use]
    pub const fn new(stored_blobs: u64, pending_deletions: u64, gateway_principals: u64) -> Self {
        Self {
            stored_blobs,
            pending_deletions,
            gateway_principals,
        }
    }
}
