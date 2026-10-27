//! Native receipt fixture protocol; no lifecycle or persistence ownership.

/// One receipt reservation input; durable intent state remains owned by Canic.
#[derive(serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct BeginRequest {
    pub operation_seed: u8,
    pub payload_seed: u8,
    pub resource_seed: u8,
    pub quantity: u64,
    pub replay_deadline_ns: u64,
}

/// Terminal evidence for the exact retained operation and expected revision.
#[derive(serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct SettleRequest {
    pub operation_seed: u8,
    pub payload_seed: u8,
    pub expected_revision: u64,
    pub decision: crate::ReceiptDecisionView,
    pub source_canister: ic_platform::Principal,
    pub evidence_seed: u8,
}

pub const BEGIN: u64 = 0x5445_5354_0000_0a00;
pub const LOAD: u64 = 0x5445_5354_0000_0a01;
pub const SETTLE: u64 = 0x5445_5354_0000_0a02;
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_384,
    allocation: 65_536,
    nodes: 4096,
    depth: 32,
};
