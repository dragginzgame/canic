//! Root-mediated external management status, carried by native Wire.
pub const METHOD: &str = "canic_root_inspect_canister";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_2d00;
pub use crate::protocol::admission_control::REQUEST_LIMITS;
/// Full frame ceiling matches the existing Host inspection reply budget.
/// Allocation/nodes bound the complete record separately from each numeric field.
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 4 * 1024 * 1024,
    allocation: 32 * 1024 * 1024,
    nodes: 1_048_576,
    depth: 64,
};

#[cfg(test)]
mod tests;
