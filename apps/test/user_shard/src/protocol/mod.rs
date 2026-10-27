//! Native admission probe contract for this installed application fixture.

pub const ADMISSION: u64 = 0x5445_5354_0000_0300;
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

pub const GENERATION: u64 = 0x5445_5354_0000_0501;
pub const SET_ROW: u64 = 0x5445_5354_0000_0502;
pub const ROWS: u64 = 0x5445_5354_0000_0503;
pub const RECOVERY_BALANCE: u64 = 0x5445_5354_0000_0504;
pub const TOPUP_REQUEST: u64 = 0x5445_5354_0000_0505;
pub const TOPUP_COLLISION: u64 = 0x5445_5354_0000_0506;
pub const TOPUP_DEMAND: u64 = 0x5445_5354_0000_0507;
pub const TOPUP_EXHAUSTION: u64 = 0x5445_5354_0000_0508;

/// One retained parent request, optionally discarding only its reply.
#[derive(serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TopupRequest {
    pub cycles: u128,
    pub discard_reply: bool,
}

/// Attempt to reuse another actor's parent-funding operation.
#[derive(serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TopupCollision {
    pub cycles: u128,
    pub operation_id: [u8; 32],
}
