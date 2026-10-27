//! Module: dto::release_intents
//!
//! Responsibility: expose bounded controller-only accounting discovery.
//! Does not own: authentication, settlement, cleanup or effect admission.
//! Boundary: original local and receipt-backed identities remain distinct.

use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Canonical scan order: local accounting first, then receipt-backed reservations.

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum IntentReleaseKey {
    Local(u64),
    ReceiptBacked([u8; 32]),
}

/// Retained terminal evidence, without interpreting it as a new authorization.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct IntentReleaseEvidence {
    pub source_canister: Principal,
    pub schema_version: u32,
    pub fingerprint: [u8; 32],
}

/// Exact receipt-backed state, independent of retention deadlines.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum IntentReleaseReceiptState {
    Pending,
    Committed(IntentReleaseEvidence),
    RolledBack(IntentReleaseEvidence),
}

/// One canonical receipt-backed reservation, including application retention.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct IntentReleaseReceipt {
    pub operation_id: [u8; 32],
    pub payload_hash_schema_version: u32,
    pub payload_hash: [u8; 32],
    pub resource_key: String,
    pub quantity: u64,
    pub state: IntentReleaseReceiptState,
    pub revision: u64,
    pub created_at_ns: u64,
    pub updated_at_ns: u64,
    pub application_replay_deadline_ns: Option<u64>,
}

/// One original accounting row; local records need not have a linked replay receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum IntentReleaseEntry {
    Local {
        intent_id: u64,
        record: crate::dto::release_receipts::ReplayReleaseIntent,
    },
    ReceiptBacked(IntentReleaseReceipt),
}

/// One bounded row with key-only lookahead; discovery never expires or settles work.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct IntentReleaseResponse {
    pub owner: Principal,
    pub entry: Option<IntentReleaseEntry>,
    pub next_after: Option<IntentReleaseKey>,
}
