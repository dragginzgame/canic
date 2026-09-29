//! Passive authority and progress contracts for reviewed Root capacity import.

use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

/// Initialization hold for supplied capacity awaiting reviewed clearing and publication.
/// The host review binds this declaration; only its exact import may release allocation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PoolImportBootstrap {
    pub review_sha256: [u8; 32],
    pub operator: Principal,
    pub sources: Vec<Principal>,
}

/// Exact source identity and destructive disposition frozen before controller handoff.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PoolImportSource {
    pub canister_id: Principal,
    /// Actual source custody before any optional operator handoff.
    pub controllers: Vec<Principal>,
    #[serde(deserialize_with = "crate::cdk::serialize::required_option")]
    pub module_sha256: Option<[u8; 32]>,
    pub canister_version: u64,
    pub stopped: bool,
    pub disposition_sha256: [u8; 32],
    pub observed_cycles: u128,
    pub observed_reserved_cycles: u128,
    pub minimum_ready_cycles: u128,
    pub maximum_debit_cycles: u128,
}

/// Exact current-Root reservation; sequence prevents replay after a later import.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PoolImportReservation {
    pub sequence: u64,
    pub plan_sha256: [u8; 32],
    pub root_authority_sha256: [u8; 32],
    pub root: Principal,
    pub operator: Principal,
    pub subnet: Principal,
    pub transitional_controllers: Vec<Principal>,
    pub final_controllers: Vec<Principal>,
    pub sources: Vec<PoolImportSource>,
    pub observed_root_cycles: u128,
    pub observed_root_reserved_cycles: u128,
    pub minimum_root_cycles: u128,
    pub maximum_root_debit_cycles: u128,
    pub maximum_paid_calls: u32,
}

/// Identifies one exact retained reservation without allowing changed bounds.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PoolImportIdentity {
    pub sequence: u64,
    pub plan_sha256: [u8; 32],
}

/// Exact protected evidence retained before releasing imported assets to allocation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PoolImportSourceReceipt {
    pub root_sender_canister_version: u64,
    pub canister_id: Principal,
    pub canister_version: u64,
    pub before_uninstall_canister_version: u64,
    pub retained_cycles: u128,
    pub retained_reserved_cycles: u128,
    pub observed_debit_cycles: u128,
}

/// Protected progress for a source; issued work is reconciled, never blindly repeated.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PoolImportSourceProgress {
    AwaitingHandoff,
    ControllersIssued,
    ControllersConfirmed,
    StopIssued,
    Stopped,
    UninstallIssued,
    Ready(PoolImportSourceReceipt),
}

/// Root retains Ready evidence until the host confirms durable local publication.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PoolImportPhase {
    Reserved,
    Ready,
    Released { publication_sha256: [u8; 32] },
}

/// Controller-visible evidence for one bounded import, including original debit limits.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PoolImportStatus {
    pub reserved_at_ns: u64,
    pub reservation: PoolImportReservation,
    pub progress: Vec<PoolImportSourceProgress>,
    pub phase: PoolImportPhase,
    pub paid_calls: u32,
    pub reserved_debit_cycles: u128,
    pub last_root_cycles: u128,
    #[serde(deserialize_with = "crate::cdk::serialize::required_option")]
    pub root_receipt: Option<PoolImportRootReceipt>,
}

/// Protected current placement, authority and next import number used during host review.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PoolImportContext {
    /// Effect-free upper quote for any supported import call on this Root.
    pub maximum_call_debit_cycles: u128,
    pub bootstrap: Option<PoolImportBootstrap>,
    pub binding: crate::ids::FleetSubnetRootBinding,
    pub root_authority_sha256: [u8; 32],
    pub next_sequence: u64,
    pub active_import: Option<PoolImportIdentity>,
}

/// Root balance boundary observed after all source effects and paid reconciliation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PoolImportRootReceipt {
    pub retained_cycles: u128,
    pub retained_reserved_cycles: u128,
    pub observed_debit_cycles: u128,
}

/// Authenticated operator actions for one reviewed import into the current Root.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PoolImportCommand {
    Advance {
        identity: PoolImportIdentity,
        canister_id: Principal,
    },
    Release {
        identity: PoolImportIdentity,
        publication_sha256: [u8; 32],
    },
    Reserve(Box<PoolImportReservation>),
    Settle(PoolImportIdentity),
}
