use crate::{
    dto::canister::{CanisterEntryView, CanisterSummaryView},
    model::memory::{CanisterEntry, CanisterSummary},
};

#[must_use]
pub fn canister_entry_into_view(e: &CanisterEntry) -> CanisterEntryView {
    CanisterEntryView {
        role: e.role.clone(),
        parent_pid: e.parent_pid,
        module_hash: e.module_hash.clone(),
        created_at: e.created_at,
    }
}

#[must_use]
pub fn canister_summary_into_view(s: &CanisterSummary) -> CanisterSummaryView {
    CanisterSummaryView {
        role: s.role.clone(),
        parent_pid: s.parent_pid,
    }
}
