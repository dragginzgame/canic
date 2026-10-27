//! Native controls for the managed child allocation fixture.

/// Exact allocation observation; heap state and installation remain separately owned.
#[derive(serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ChildAllocation {
    pub request_id: [u8; 32],
    pub parent: ic_platform::Principal,
    pub canister_role: canic::ids::CanisterRole,
    pub extra_arg: Option<Vec<u8>>,
    pub child: ic_platform::Principal,
    pub settled: bool,
    pub acknowledged: bool,
}

pub const ALLOCATIONS: u64 = 0x5445_5354_0000_0b00;
pub const IS_SETTLED: u64 = 0x5445_5354_0000_0b01;
pub const SETTLE: u64 = 0x5445_5354_0000_0b02;
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 4_194_304,
    allocation: 16_777_216,
    nodes: 262_144,
    depth: 64,
};
