//! Root-mediated external management history with native request/reply framing.
pub const METHOD: &str = "canic_root_inspect_history";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_2b00;
pub use crate::protocol::admission_control::REQUEST_LIMITS;
/// Bound the raw upstream reply and its native envelope independently.
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 2_097_152,
    allocation: 8_388_608,
    nodes: 2_097_152,
    depth: 64,
};

#[cfg(test)]
mod tests;
