use crate::{
    dto::pool::{CanisterPoolEntryView, CanisterPoolStatusView, CanisterPoolView},
    ops::storage::pool::{PoolEntrySnapshot, PoolSnapshot},
    storage::memory::pool::PoolStatus,
};

///
/// PoolMapper
///

pub struct PoolMapper;

impl PoolMapper {
    #[must_use]
    pub fn entry_snapshot_to_view(entry: PoolEntrySnapshot) -> CanisterPoolEntryView {
        CanisterPoolEntryView {
            pid: entry.pid,
            created_at: entry.created_at,
            cycles: entry.cycles,
            status: Self::status_to_view(&entry.status),
            role: entry.role,
            parent: entry.parent,
            module_hash: entry.module_hash,
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
    pub fn snapshot_to_view(snapshot: PoolSnapshot) -> CanisterPoolView {
        CanisterPoolView {
            entries: snapshot
                .entries
                .into_iter()
                .map(Self::entry_snapshot_to_view)
                .collect(),
        }
    }
}
