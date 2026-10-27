//! Demo-owned method identities and string payload budgets.
//! Generated clients consume these bounds from the native schema.

// Preserve the framework's 16 KiB update ingress ceiling for the demo's strings.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_384,
    allocation: 65_536,
    nodes: 64,
    depth: 8,
};
// Reports include the input and its placement/debug description.
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 2_097_152,
    allocation: 8_388_608,
    nodes: 64,
    depth: 8,
};

pub const OVERVIEW: u64 = 0x4445_4d4f_0000_0200;

pub const DESCRIBE: u64 = 0x4445_4d4f_0000_0201;
