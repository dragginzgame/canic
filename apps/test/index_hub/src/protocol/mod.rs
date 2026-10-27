//! Native contracts for indexed placement and child-startup qualification.

pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_384,
    allocation: 65_536,
    nodes: 128,
    depth: 16,
};
/// Admit a complete operation identity and its accompanying scalar fields.
pub const OPERATION_REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    nodes: 256,
    ..REQUEST_LIMITS
};
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 262_144,
    nodes: 4096,
    depth: 32,
};

/// Exact placement observation with native principals at the endpoint boundary.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum IndexStatus {
    Pending {
        owner_pid: ic_platform::Principal,
        created_at: u64,
        provisional_pid: Option<ic_platform::Principal>,
    },
    Bound {
        instance_pid: ic_platform::Principal,
        bound_at: u64,
    },
}

/// Caller-retained child identity and bounded initialization work.
#[derive(serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct StartupRequest {
    pub operation_id: [u8; 32],
    pub rounds: u32,
}

pub const RESOLVE_ITEM: u64 = 0x5445_5354_0000_0610;
pub const CREATE_STARTUP_CHILD: u64 = 0x5445_5354_0000_0611;
