use crate::{dto::placement::WorkerEntryView, model::memory::scaling::WorkerEntry};

#[must_use]
pub fn worker_entry_from_view(view: WorkerEntryView) -> WorkerEntry {
    WorkerEntry {
        pool: view.pool,
        canister_role: view.canister_role,
        created_at_secs: view.created_at_secs,
    }
}

#[must_use]
pub fn worker_entry_to_view(entry: &WorkerEntry) -> WorkerEntryView {
    WorkerEntryView {
        pool: entry.pool.clone(),
        canister_role: entry.canister_role.clone(),
        created_at_secs: entry.created_at_secs,
    }
}
