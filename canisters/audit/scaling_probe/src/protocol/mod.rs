//! Native identity and budgets for this audit-only endpoint.

pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_384,
    allocation: 65_536,
    nodes: 4096,
    depth: 32,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 2_097_152,
    allocation: 8_388_608,
    nodes: 131_072,
    depth: 32,
};
pub const PLAN: u64 = 0x4155445400000015;
