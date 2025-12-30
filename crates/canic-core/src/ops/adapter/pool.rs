use crate::{
    dto::pool::{CanisterPoolEntryView, CanisterPoolStatusView, CanisterPoolView},
    model::memory::pool::{
        CanisterPoolData, CanisterPoolHeader, CanisterPoolState, CanisterPoolStatus,
    },
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
pub fn canister_pool_entry_to_view(
    header: &CanisterPoolHeader,
    state: &CanisterPoolState,
) -> CanisterPoolEntryView {
    CanisterPoolEntryView {
        created_at: header.created_at,
        cycles: state.cycles.clone(),
        status: canister_pool_status_to_view(&state.status),
        role: state.role.clone(),
        parent: state.parent,
        module_hash: state.module_hash.clone(),
    }
}

#[must_use]
pub fn canister_pool_to_view(data: CanisterPoolData) -> CanisterPoolView {
    let view = data
        .entries
        .into_iter()
        .map(|(pid, entry)| {
            (
                pid,
                canister_pool_entry_to_view(&entry.header, &entry.state),
            )
        })
        .collect();

    CanisterPoolView(view)
}
