use crate::{
    dto::{
        canister::{CanisterEntryView, CanisterSummaryView},
        snapshot::TopologyNodeView,
    },
    model::memory::{CanisterEntry, CanisterSummary},
};

#[must_use]
pub fn canister_entry_to_view(e: &CanisterEntry) -> CanisterEntryView {
    CanisterEntryView {
        role: e.role.clone(),
        parent_pid: e.parent_pid,
        module_hash: e.module_hash.clone(),
        created_at: e.created_at,
    }
}

#[must_use]
pub fn canister_summary_to_view(s: &CanisterSummary) -> CanisterSummaryView {
    CanisterSummaryView {
        role: s.role.clone(),
        parent_pid: s.parent_pid,
    }
}

#[must_use]
pub fn canister_summary_from_topology_node(node: &TopologyNodeView) -> CanisterSummary {
    CanisterSummary {
        role: node.role.clone(),
        parent_pid: node.parent_pid,
    }
}
