//! Bounded native fixture byte transport; content and stored grant encodings are unchanged.
/// Exact grant plus chunk index in the existing small ingress envelope.
pub const READ_REQUEST_LIMITS: ic_wire::Limits = crate::protocol::store_control::REQUEST_LIMITS;
/// One opaque management-sized chunk plus metadata and native framing.
pub const CHUNK_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 1_114_112,
    allocation: 1_114_112 * 9,
    nodes: 16_384,
    depth: 64,
};
/// Upload progress or typed Store/public failure.
pub const UPLOAD_RESPONSE_LIMITS: ic_wire::Limits = crate::protocol::store_control::RESPONSE_LIMITS;
/// Authenticated exact-grant read, preserving replicated update semantics.
pub mod read {
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1800;
}
/// Sequential opaque chunk upload under existing publication authority.
pub mod upload {
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1801;
}
#[cfg(test)]
mod tests;
