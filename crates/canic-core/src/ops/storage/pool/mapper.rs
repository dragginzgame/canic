//! Module: ops::storage::pool::mapper
//!
//! Responsibility: convert pool storage records into pool response views.
//! Does not own: pool mutation, scheduling workflow, or DTO definitions.
//! Boundary: storage ops conversion layer for stable pool records.

use crate::{
    domain::pool::CanisterPoolStatus,
    dto::pool::{CanisterPoolEntry, CanisterPoolResponse},
    ops::{
        prelude::*,
        storage::pool::{CanisterPoolData, PoolRecord, PoolStatus},
    },
};

///
/// CanisterPoolEntryMapper
///
/// Storage-ops mapper for pool records and pool entry response views.
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
/// Storage-ops mapper for full pool snapshots and response views.
///

pub struct CanisterPoolResponseMapper;

impl CanisterPoolResponseMapper {
    #[must_use]
    pub fn data_to_view(data: CanisterPoolData) -> CanisterPoolResponse {
        CanisterPoolResponse {
            entries: data
                .entries
                .into_iter()
                .map(|entry| CanisterPoolEntryMapper::record_to_view(entry.pid, entry.record))
                .collect(),
        }
    }
}
