//! Controller-authenticated live cycle balance; no cached history or remote inspection.

pub const METHOD: &str = "canic_cycle_balance";
pub const METHOD_ID: u64 = 0x4341_4e49_0000_0100;
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 64,
    allocation: 512,
    nodes: 16,
    depth: 8,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 128,
    allocation: 4096,
    nodes: 64,
    depth: 16,
};
