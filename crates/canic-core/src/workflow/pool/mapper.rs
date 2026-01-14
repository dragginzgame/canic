use crate::{
    dto::pool::{CanisterPoolEntryView, CanisterPoolStatusView, CanisterPoolView},
    ops::storage::pool::{PoolData, PoolRecord, PoolStatus},
    workflow::prelude::*,
};

///
/// PoolMapper
///

pub struct PoolMapper;

impl PoolMapper {
    #[must_use]
    pub fn entry_data_to_view(pid: Principal, record: PoolRecord) -> CanisterPoolEntryView {
        CanisterPoolEntryView {
            pid,
            created_at: record.header.created_at,
            cycles: record.state.cycles,
            status: Self::status_to_view(&record.state.status),
            role: record.state.role,
            parent: record.state.parent,
            module_hash: record.state.module_hash,
        }
    }

    #[must_use]
    fn status_to_view(status: &PoolStatus) -> CanisterPoolStatusView {
        match status {
            PoolStatus::PendingReset => CanisterPoolStatusView::PendingReset,
            PoolStatus::Ready => CanisterPoolStatusView::Ready,
            PoolStatus::Failed { reason } => CanisterPoolStatusView::Failed {
                reason: reason.clone(),
            },
        }
    }

    #[must_use]
    pub fn data_to_view(data: PoolData) -> CanisterPoolView {
        CanisterPoolView {
            entries: data
                .entries
                .into_iter()
                .map(|(pid, record)| Self::entry_data_to_view(pid, record))
                .collect(),
        }
    }
}
