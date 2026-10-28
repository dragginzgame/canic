//! Receiver-pulled portable issuer enrollment, independent of user admission.

pub const METHOD: &str = "canic_root_portable_enrollment";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_1106;
pub const MAX_ENTRIES: usize = 32;
pub const MAX_RETIRED_KEYS: usize = 256;
pub const MAX_FRESHNESS_NS: u64 = 3_600_000_000_000;
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 128,
    // Native admission charges the fixed array and every scalar visit. The
    // request remains exactly one 32-byte challenge; it has no variable fields.
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
