//! Native fixture byte boundary; canonical descriptors, grants and content digests stay unchanged.
use serde::{Deserialize, Serialize};
/// Read one chunk under the exact retained grant revision.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ReadRequest {
    pub grant: crate::dto::runtime_reporting::FixtureGrant,
    pub index: u32,
}
/// Upload one opaque independently verified content chunk.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct UploadRequest {
    pub content_id: [u8; 32],
    pub index: u32,
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
}
/// Opaque bytes, decoded as a byte string rather than an unbounded element sequence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ChunkResponse {
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
}
