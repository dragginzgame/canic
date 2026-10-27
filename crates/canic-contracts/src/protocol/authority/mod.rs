//! Canonical native Wire bytes for current-release authority and receipt evidence.
//!
//! Hash owners retain their domain separation and exact authority projections.
//! This boundary admits the same maximum value budget as a full control-plane
//! response. Retained host evidence uses these same bytes; external IC protocols
//! and stable storage records keep their own encoding contracts.

/// Maximum current authority projection, matching the control-plane response allowance.
pub const LIMITS: ic_wire::Limits = crate::protocol::admission_control::RESPONSE_LIMITS;

/// Encode the complete canonical authority value before its owner hashes it.
pub fn encode<T: serde::Serialize>(value: T) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::native::encode(&value, LIMITS)
}

/// Read retained current-release authority evidence with the same resource allowance.
pub fn decode<'de, T: serde::Deserialize<'de>>(bytes: &'de [u8]) -> Result<T, ic_wire::Error> {
    ic_wire::native::decode(bytes, LIMITS)
}
