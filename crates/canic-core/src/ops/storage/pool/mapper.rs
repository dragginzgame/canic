use crate::{
    dto::pool::{CanisterPoolEntry, CanisterPoolResponse, CanisterPoolStatus},
    ops::prelude::*,
    ops::storage::pool::{PoolRecord, PoolStatus, PoolStoreRecord},
};

///
/// CanisterPoolEntryMapper
///

pub struct CanisterPoolEntryMapper;

impl CanisterPoolEntryMapper {
    #[must_use]
    pub fn record_to_view(pid: Principal, record: PoolRecord) -> CanisterPoolEntry {
        CanisterPoolEntry {
            pid,
            created_at: record.header.created_at,
            cycles: record.state.cycles,
            status: match &record.state.status {
                PoolStatus::PendingReset => CanisterPoolStatus::PendingReset,
                PoolStatus::Ready => CanisterPoolStatus::Ready,
                PoolStatus::Failed { reason } => CanisterPoolStatus::Failed {
                    reason: reason.clone(),
                },
            },
            role: record.state.role,
            parent: record.state.parent,
            module_hash: record.state.module_hash,
        }
    }
}

///
/// CanisterPoolResponseMapper
///

pub struct CanisterPoolResponseMapper;

impl CanisterPoolResponseMapper {
    #[must_use]
    pub fn record_to_view(data: PoolStoreRecord) -> CanisterPoolResponse {
        CanisterPoolResponse {
            entries: data
                .entries
                .into_iter()
                .map(|(pid, record)| CanisterPoolEntryMapper::record_to_view(pid, record))
                .collect(),
        }
    }
}
