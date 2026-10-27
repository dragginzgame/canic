//! Native receipt fixture protocol; no lifecycle or persistence ownership.

/// One receipt reservation input; durable intent state remains owned by Canic.
#[derive(serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub(super) struct BeginRequest {
    pub(super) operation_seed: u8,
    pub(super) payload_seed: u8,
    pub(super) resource_seed: u8,
    pub(super) quantity: u64,
    pub(super) replay_deadline_ns: u64,
}

/// Terminal evidence for the exact retained operation and expected revision.
#[derive(serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub(super) struct SettleRequest {
    pub(super) operation_seed: u8,
    pub(super) payload_seed: u8,
    pub(super) expected_revision: u64,
    pub(super) decision: crate::ReceiptDecisionView,
    pub(super) source_canister: ic_platform::Principal,
    pub(super) evidence_seed: u8,
}

pub(super) const BEGIN: u64 = 0x5445_5354_0000_0a00;
pub(super) const LOAD: u64 = 0x5445_5354_0000_0a01;
pub(super) const SETTLE: u64 = 0x5445_5354_0000_0a02;
pub(super) const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_384,
    allocation: 65_536,
    nodes: 4096,
    depth: 32,
};
