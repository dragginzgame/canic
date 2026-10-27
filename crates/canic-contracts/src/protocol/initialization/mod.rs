//! Native Wire installation values for Canic-managed lifecycle entrypoints.
//!
//! The exact compiled artifact owns the tuple shape. Management `install_code`
//! still uses its external Candid contract; its opaque `arg` contains these bytes.
//! Callers retain the original encoded bytes across uncertain install retries.

/// Default managed installation envelope, including nested application bytes.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 1_048_576,
    nodes: 65_536,
    depth: 64,
};

/// Encode the artifact's exact current initialization value or argument tuple.
pub fn encode<T: serde::Serialize>(value: T) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::native::encode(&value, LIMITS)
}

/// Decode the complete current installation value under the shared allowance.
pub fn decode<'de, T: serde::Deserialize<'de>>(bytes: &'de [u8]) -> Result<T, ic_wire::Error> {
    ic_wire::native::decode(bytes, LIMITS)
}
