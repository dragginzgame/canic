use crate::{
    dto::{
        canister::{CanisterEntryView, CanisterSummaryView},
        topology::CanisterChildrenView,
    },
    model::memory::{CanisterEntry, CanisterSummary, children::CanisterChildren},
};

///
/// CanisterChildren
///

impl From<&CanisterChildren> for CanisterChildrenView {
    fn from(e: &CanisterChildren) -> Self {
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
