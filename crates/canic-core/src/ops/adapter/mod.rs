use crate::{
    dto::{
        canister::{CanisterEntryView, CanisterSummaryView},
        directory::{AppDirectoryView, SubnetDirectoryView},
        placement::WorkerEntryView,
        state::AppModeView,
    },
    model::memory::{
        CanisterEntry, CanisterSummary,
        directory::{AppDirectoryData, SubnetDirectoryData},
        scaling::WorkerEntry,
        state::AppMode,
    },
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
/// AppDirectory
///

impl From<AppDirectoryView> for AppDirectoryData {
    fn from(view: AppDirectoryView) -> Self {
        view.0
    }
}

///
/// SubnetDirector
///

impl From<SubnetDirectoryView> for SubnetDirectoryData {
    fn from(view: SubnetDirectoryView) -> Self {
        view.0
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
