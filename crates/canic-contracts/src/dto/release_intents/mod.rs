//! Module: dto::release_intents
//!
//! Responsibility: expose bounded controller-only accounting discovery.
//! Does not own: authentication, settlement, cleanup or effect admission.
//! Boundary: original local and receipt-backed identities remain distinct.

use candid::{CandidType, Principal};
use serde::Deserialize;

/// Canonical scan order: local accounting first, then receipt-backed reservations.

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
pub enum IntentReleaseKey {
    Local(u64),
    ReceiptBacked([u8; 32]),
}

/// Retained terminal evidence, without interpreting it as a new authorization.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct IntentReleaseEvidence {
    pub source_canister: Principal,
    pub schema_version: u32,
    pub fingerprint: [u8; 32],
}

/// Exact receipt-backed state, independent of retention deadlines.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum IntentReleaseReceiptState {
    Pending,
    Committed(IntentReleaseEvidence),
    RolledBack(IntentReleaseEvidence),
}

/// One canonical receipt-backed reservation, including application retention.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
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
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum IntentReleaseEntry {
    Local {
        intent_id: u64,
        record: crate::dto::release_receipts::ReplayReleaseIntent,
    },
    ReceiptBacked(IntentReleaseReceipt),
}

/// One bounded row with key-only lookahead; discovery never expires or settles work.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct IntentReleaseResponse {
    pub owner: Principal,
    pub entry: Option<IntentReleaseEntry>,
    pub next_after: Option<IntentReleaseKey>,
}
