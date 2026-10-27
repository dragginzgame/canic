//! Lossless physical pool boundary conversion; no allocation, pagination or retry policy.
use crate::dto::{pool as stored, pool_inventory as wire};
impl From<stored::CanisterPoolStatusRequest> for wire::Request {
    fn from(value: stored::CanisterPoolStatusRequest) -> Self {
        Self {
            start_after: value.start_after.map(Into::into),
            limit: value.limit,
        }
    }
}
impl From<wire::Request> for stored::CanisterPoolStatusRequest {
    fn from(value: wire::Request) -> Self {
        Self {
            start_after: value.start_after.map(Into::into),
            limit: value.limit,
        }
    }
}
impl From<stored::CanisterPoolResponse> for wire::Page {
    fn from(value: stored::CanisterPoolResponse) -> Self {
        Self {
            config: value.config,
            tracked: value.tracked,
            store: value.store,
            store_deletion_pending: value.store_deletion_pending,
            pooled: value.pooled,
            workload: value.workload,
            surplus: value.surplus,
            ready: value.ready,
            pending_reset: value.pending_reset,
            claimed: value.claimed,
            recycling: value.recycling,
            handing_off: value.handing_off,
            failed: value.failed,
            completed_handoffs: value.completed_handoffs,
            pending_creation: value.pending_creation.map(Into::into),
            pending_handoff: value.pending_handoff.map(Into::into),
            entries: value.entries.into_iter().map(Into::into).collect(),
            next_start_after: value.next_start_after.map(Into::into),
        }
    }
}
impl From<wire::Page> for stored::CanisterPoolResponse {
    fn from(value: wire::Page) -> Self {
        Self {
            config: value.config,
            tracked: value.tracked,
            store: value.store,
            store_deletion_pending: value.store_deletion_pending,
            pooled: value.pooled,
            workload: value.workload,
            surplus: value.surplus,
            ready: value.ready,
            pending_reset: value.pending_reset,
            claimed: value.claimed,
            recycling: value.recycling,
            handing_off: value.handing_off,
            failed: value.failed,
            completed_handoffs: value.completed_handoffs,
            pending_creation: value.pending_creation.map(Into::into),
            pending_handoff: value.pending_handoff.map(Into::into),
            entries: value.entries.into_iter().map(Into::into).collect(),
            next_start_after: value.next_start_after.map(Into::into),
        }
    }
}
impl From<stored::CanisterPoolAsset> for wire::Asset {
    fn from(value: stored::CanisterPoolAsset) -> Self {
        Self {
            canister_id: value.canister_id.into(),
            creation_receipt: value.creation_receipt.map(Into::into),
            cycles: value.cycles,
            origin: value.origin,
            status: value.status.into(),
            added_at_ns: value.added_at_ns,
            updated_at_ns: value.updated_at_ns,
        }
    }
}
impl From<wire::Asset> for stored::CanisterPoolAsset {
    fn from(value: wire::Asset) -> Self {
        Self {
            canister_id: value.canister_id.into(),
            creation_receipt: value.creation_receipt.map(Into::into),
            cycles: value.cycles,
            origin: value.origin,
            status: value.status.into(),
            added_at_ns: value.added_at_ns,
            updated_at_ns: value.updated_at_ns,
        }
    }
}
impl From<stored::CanisterPoolCreationReceipt> for wire::CreationReceipt {
    fn from(value: stored::CanisterPoolCreationReceipt) -> Self {
        Self {
            block_index: value.block_index,
            operation_id: value.operation_id,
            cycles_ledger: value.cycles_ledger.into(),
            ledger_amount: value.ledger_amount,
            ledger_fee: value.ledger_fee,
            readiness_floor: value.readiness_floor,
            creation_execution_margin: value.creation_execution_margin,
            management_creation_fee: value.management_creation_fee,
            first_observed_cycles: value.first_observed_cycles,
        }
    }
}
impl From<wire::CreationReceipt> for stored::CanisterPoolCreationReceipt {
    fn from(value: wire::CreationReceipt) -> Self {
        Self {
            block_index: value.block_index,
            operation_id: value.operation_id,
            cycles_ledger: value.cycles_ledger.into(),
            ledger_amount: value.ledger_amount,
            ledger_fee: value.ledger_fee,
            readiness_floor: value.readiness_floor,
            creation_execution_margin: value.creation_execution_margin,
            management_creation_fee: value.management_creation_fee,
            first_observed_cycles: value.first_observed_cycles,
        }
    }
}
impl From<stored::CanisterPoolAssetStatus> for wire::AssetStatus {
    fn from(value: stored::CanisterPoolAssetStatus) -> Self {
        match value {
            stored::CanisterPoolAssetStatus::Store => Self::Store,
            stored::CanisterPoolAssetStatus::StoreDeletionPending { operation_id } => {
                Self::StoreDeletionPending { operation_id }
            }
            stored::CanisterPoolAssetStatus::PendingReset => Self::PendingReset,
            stored::CanisterPoolAssetStatus::Ready => Self::Ready,
            stored::CanisterPoolAssetStatus::Claimed { claim } => Self::Claimed { claim },
            stored::CanisterPoolAssetStatus::Workload { claim } => Self::Workload { claim },
            stored::CanisterPoolAssetStatus::Recycling { claim, reset } => {
                Self::Recycling { claim, reset }
            }
            stored::CanisterPoolAssetStatus::HandingOff { recipient } => Self::HandingOff {
                recipient: recipient.into(),
            },
            stored::CanisterPoolAssetStatus::Failed { reason } => Self::Failed { reason },
        }
    }
}
impl From<wire::AssetStatus> for stored::CanisterPoolAssetStatus {
    fn from(value: wire::AssetStatus) -> Self {
        match value {
            wire::AssetStatus::Store => Self::Store,
            wire::AssetStatus::StoreDeletionPending { operation_id } => {
                Self::StoreDeletionPending { operation_id }
            }
            wire::AssetStatus::PendingReset => Self::PendingReset,
            wire::AssetStatus::Ready => Self::Ready,
            wire::AssetStatus::Claimed { claim } => Self::Claimed { claim },
            wire::AssetStatus::Workload { claim } => Self::Workload { claim },
            wire::AssetStatus::Recycling { claim, reset } => Self::Recycling { claim, reset },
            wire::AssetStatus::HandingOff { recipient } => Self::HandingOff {
                recipient: recipient.into(),
            },
            wire::AssetStatus::Failed { reason } => Self::Failed { reason },
        }
    }
}
