use crate::{
    cdk::candid::Principal,
    dto::{
        canister::{CanisterEntryView, CanisterSummaryView},
        directory::DirectoryView,
        placement::WorkerEntryView,
        state::AppModeView,
    },
    ids::CanisterRole,
    model::memory::{CanisterEntry, CanisterSummary, scaling::WorkerEntry, state::AppMode},
};

///
/// AppMode
///

impl From<AppMode> for AppModeView {
    fn from(m: AppMode) -> Self {
        match m {
            AppMode::Enabled => Self::Enabled,
            AppMode::Readonly => Self::Readonly,
            AppMode::Disabled => Self::Disabled,
        }
    }
}

///
/// CanisterEntry
///

impl From<&CanisterEntry> for CanisterEntryView {
    fn from(e: &CanisterEntry) -> Self {
        Self {
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
            role: s.role.clone(),
            parent_pid: s.parent_pid,
        }
    }
}

///
/// DirectoryView
///

impl From<Vec<(CanisterRole, Principal)>> for DirectoryView {
    fn from(v: Vec<(CanisterRole, Principal)>) -> Self {
        DirectoryView(v)
    }
}

///
/// WorkerEntry
///

impl From<&WorkerEntry> for WorkerEntryView {
    fn from(w: &WorkerEntry) -> Self {
        Self {
            pool: w.pool.clone(),
            canister_role: w.canister_role.clone(),
            created_at_secs: w.created_at_secs,
        }
    }
}
