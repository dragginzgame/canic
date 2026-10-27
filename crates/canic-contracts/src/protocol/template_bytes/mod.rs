//! Native template publication retains its exact manifest, hashes and bounded opaque chunk.
pub const METHOD_ID: u64 = 0x4341_4e49_0000_1a00;
pub const REQUEST_LIMITS: ic_wire::Limits = crate::protocol::fixture_bytes::CHUNK_LIMITS;
pub const RESPONSE_LIMITS: ic_wire::Limits = crate::protocol::store_publication::RESPONSE_LIMITS;
#[cfg(test)]
mod tests;
