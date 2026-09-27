//! Durable reservation and effect evidence owned by the existing pool singleton.

use canic_core::dto::pool_import::{
    PoolImportReservation, PoolImportRootReceipt, PoolImportSourceReceipt,
};
use serde::{Deserialize, Serialize};

/// Exact fresh-install hold retained until registered Root capacity is published.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PoolImportBootstrapRecord {
    pub review_sha256: [u8; 32],
    pub install_id: [u8; 32],
    pub operator: candid::Principal,
    pub store: candid::Principal,
    pub sources: Vec<candid::Principal>,
}

/// Root-issued effect awaiting exact post-effect management evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PoolImportResetProgressRecord {
    AwaitingHandoff,
    ControllersIssued {
        before_total_cycles: u128,
        before_canister_version: u64,
        sender_canister_version: u64,
    },
    ControllersConfirmed {
        retained_total_cycles: u128,
        canister_version: u64,
    },
    StopIssued {
        before_total_cycles: u128,
        before_canister_version: u64,
    },
    Stopped {
        retained_total_cycles: u128,
        canister_version: u64,
    },
    UninstallIssued {
        before_total_cycles: u128,
        before_canister_version: u64,
        sender_canister_version: u64,
    },
    Ready(PoolImportSourceReceipt),
}

/// Reservation remains exclusive until the host retains local publication evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PoolImportPhaseRecord {
    Reserved,
    Ready,
    Released { publication_sha256: [u8; 32] },
}

/// Complete bounded same-release import; released receipts survive workload assignment.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PoolImportRecord {
    pub reserved_at_ns: u64,
    pub reservation: PoolImportReservation,
    pub progress: Vec<PoolImportResetProgressRecord>,
    pub phase: PoolImportPhaseRecord,
    pub paid_calls: u32,
    pub reserved_debit_cycles: u128,
    pub last_root_cycles: u128,
    #[serde(deserialize_with = "required_receipt")]
    pub root_receipt: Option<PoolImportRootReceipt>,
}

/// Missing current-contract import state must not silently remove an allocation fence.
pub(super) fn required_record<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<PoolImportRecord>, D::Error> {
    Option::<PoolImportRecord>::deserialize(deserializer)
}

fn required_receipt<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<PoolImportRootReceipt>, D::Error> {
    Option::<PoolImportRootReceipt>::deserialize(deserializer)
}

/// Absence of a required current-contract hold field must not silently reopen allocation.
pub(super) fn required_bootstrap<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<PoolImportBootstrapRecord>, D::Error> {
    Option::<PoolImportBootstrapRecord>::deserialize(deserializer)
}
