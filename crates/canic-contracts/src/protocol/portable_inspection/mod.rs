//! Root-only receiver inspection, independent of caller-self Auth endpoints.
pub const METHOD: &str = "canic_portable_sessions_inspect";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_1110;
pub const MAX_PAGE: u16 = 32;
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 128,
    allocation: 4096,
    nodes: 256,
    depth: 8,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 131_072,
    allocation: 1_048_576,
    nodes: 65_536,
    depth: 16,
};

#[cfg(test)]
mod tests;

/// Root-only declared policy and current enrollment observation.
pub mod policy {
    pub const METHOD: &str = "canic_portable_policy_inspect";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1111;
    pub use super::{REQUEST_LIMITS, RESPONSE_LIMITS};
}
