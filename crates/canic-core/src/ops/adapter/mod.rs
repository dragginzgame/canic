use crate::{
    dto::{
        canister::{CanisterEntryView, CanisterSummaryView},
        placement::WorkerEntryView,
        topology::CanisterChildrenView,
    },
    model::memory::{
        CanisterEntry, CanisterSummary, children::CanisterChildrenData, scaling::WorkerEntry,
    },
};

///
/// CanisterChildrenData
///

impl From<&CanisterChildrenData> for CanisterChildrenView {
    fn from(e: &CanisterChildrenData) -> Self {
        Self {
            pid: e.pid,
            role: e.role.clone(),
            parent_pid: e.parent_pid,
            module_hash: e.module_hash.clone(),
            created_at: e.created_at,
        }
    }
}

///
/// CanisterEntry
///

impl From<&CanisterEntry> for CanisterEntryView {
    fn from(e: &CanisterEntry) -> Self {
        Self {
            pid: e.pid,
            role: e.role.clone(),
            parent_pid: e.parent_pid,
            module_hash: e.module_hash.clone(),
            created_at: e.created_at,
        }
    }
}

///
/// CanisterSummary
///

impl From<&CanisterSummary> for CanisterSummaryView {
    fn from(s: &CanisterSummary) -> Self {
        Self {
            pid: s.pid,
            role: s.role.clone(),
            parent_pid: s.parent_pid,
        }
    }
}

///
/// WorkerEntry
///

impl From<&WorkerEntry> for WorkerEntryView {
    fn from(w: &WorkerEntry) -> Self {
        Self {
            pool: w.pool.as_str().to_string(),
            canister_role: w.canister_role,
            created_at_secs: w.created_at_secs,
        }
    }
}
