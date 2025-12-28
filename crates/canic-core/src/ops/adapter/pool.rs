use crate::{
    dto::pool::{CanisterPoolEntryView, CanisterPoolStatusView, CanisterPoolView},
    model::memory::pool::{CanisterPoolData, CanisterPoolEntry, CanisterPoolStatus},
};

#[must_use]
fn canister_pool_status_to_view(status: &CanisterPoolStatus) -> CanisterPoolStatusView {
    match status {
        CanisterPoolStatus::PendingReset => CanisterPoolStatusView::PendingReset,
        CanisterPoolStatus::Ready => CanisterPoolStatusView::Ready,
        CanisterPoolStatus::Failed { reason } => CanisterPoolStatusView::Failed {
            reason: reason.clone(),
        },
    }
}

#[must_use]
fn canister_pool_entry_to_view(entry: &CanisterPoolEntry) -> CanisterPoolEntryView {
    CanisterPoolEntryView {
        created_at: entry.created_at,
        cycles: entry.cycles.clone(),
        status: canister_pool_status_to_view(&entry.status),
        role: entry.role.clone(),
        parent: entry.parent,
        module_hash: entry.module_hash.clone(),
    }
}

#[must_use]
pub fn canister_pool_to_view(data: CanisterPoolData) -> CanisterPoolView {
    data.into_iter()
        .map(|(pid, entry)| (pid, canister_pool_entry_to_view(&entry)))
        .collect()
}
