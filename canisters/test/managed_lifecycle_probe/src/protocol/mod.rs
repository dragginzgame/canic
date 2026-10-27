//! Native contracts for the managed admission and initialization probes.

pub const PUBLIC: u64 = 0x5445_5354_0000_0400;
pub const GUARD: u64 = 0x5445_5354_0000_0401;
pub const OWNED: u64 = 0x5445_5354_0000_0402;
pub const WORKFLOW_RUNS: u64 = 0x5445_5354_0000_0403;
pub const PARITY: u64 = 0x5445_5354_0000_0404;
pub const INITIALIZATION: u64 = 0x5445_5354_0000_0405;
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_384,
    allocation: 65_536,
    nodes: 128,
    depth: 16,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 262_144,
    nodes: 4096,
    depth: 32,
};
/// The retained initialization payload can occupy the entire 64 KiB input budget.
/// Include framing and result/vector overhead in its observation budget.
pub const INITIALIZATION_RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 131_072,
    allocation: 524_288,
    nodes: 131_072,
    depth: 32,
};
