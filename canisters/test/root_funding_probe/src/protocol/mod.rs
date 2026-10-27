//! Native funding proof contracts; foreign ledger/CMC and installation remain Candid.

/// Exact target and grant authority for preparation.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PrepareRequest {
    pub root: ic_platform::Principal,
    pub operation_id: [u8; 32],
    pub amount: u128,
}
/// Native projection of the retained external transfer authority.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct EmergencyTransfer {
    pub operation_id: [u8; 32],
    pub from_subaccount: Option<[u8; 32]>,
    pub to_owner: ic_platform::Principal,
    pub to_subaccount: [u8; 32],
    pub amount_e8s: u64,
    pub fee_e8s: u64,
    pub memo: Vec<u8>,
    pub created_at_time_ns: u64,
}
/// One retained emergency operation; observation does not reissue it.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub(super) struct EmergencyOperation {
    pub(super) transfer: EmergencyTransfer,
    pub(super) max_call_cost: u128,
    pub(super) phase: crate::EmergencyRefillPhase,
}
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16_384,
    allocation: 65_536,
    nodes: 4096,
    depth: 32,
};
pub const PREPARE: u64 = 0x5445535400000d00;
pub const HANDLE_REQUEST: u64 = 0x5445535400000d01;
pub const REQUEST: u64 = 0x5445535400000d02;
pub const EMERGENCY_REFILL: u64 = 0x5445535400000d03;
pub const EMERGENCY_REFILL_TRANSFER_THEN_TRAP: u64 = 0x5445535400000d04;
pub const EMERGENCY_REFILL_NOTIFY_THEN_TRAP: u64 = 0x5445535400000d05;
pub const EMERGENCY_OPERATION: u64 = 0x5445535400000d06;
pub const ISSUE: u64 = 0x5445535400000d07;
pub const ISSUE_ROOT_ACCEPT_THEN_TRAP: u64 = 0x5445535400000d08;
pub const ISSUE_THEN_TRAP: u64 = 0x5445535400000d09;
pub const INTENT: u64 = 0x5445535400000d0a;
pub const RECEIPT: u64 = 0x5445535400000d0b;
pub const ACCEPTANCE_OBSERVATION: u64 = 0x5445535400000d0c;
pub const ACCEPT: u64 = 0x5445535400000d0d;
pub const ACCEPT_THEN_TRAP: u64 = 0x5445535400000d0e;
