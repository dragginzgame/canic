use crate::{
    cdk::types::Principal,
    dto::{
        canister::{CanisterEntryView, CanisterSummaryView},
        snapshot::{TopologyDirectChildView, TopologyPathNodeView},
    },
    storage::canister::{CanisterEntry, CanisterSummary},
};

///
/// CanisterMapper
///

pub struct CanisterMapper;

impl CanisterMapper {
    #[must_use]
    pub fn entry_to_view(e: &CanisterEntry) -> CanisterEntryView {
        CanisterEntryView {
            role: e.role.clone(),
            parent_pid: e.parent_pid,
            module_hash: e.module_hash.clone(),
            created_at: e.created_at,
        }
    }

    #[must_use]
    pub fn summary_to_view(s: &CanisterSummary) -> CanisterSummaryView {
        CanisterSummaryView {
            role: s.role.clone(),
            parent_pid: s.parent_pid,
        }
    }

    #[must_use]
    pub fn summary_to_topology_node(
        pid: Principal,
        summary: &CanisterSummary,
    ) -> TopologyPathNodeView {
        TopologyPathNodeView {
            pid,
            role: summary.role.clone(),
            parent_pid: summary.parent_pid,
        }
    }

    #[must_use]
    pub fn summary_to_topology_child(
        pid: Principal,
        summary: &CanisterSummary,
    ) -> TopologyDirectChildView {
        TopologyDirectChildView {
            pid,
            role: summary.role.clone(),
        }
    }

    #[must_use]
    pub fn summary_from_topology_child(
        node: &TopologyDirectChildView,
        parent_pid: Principal,
    ) -> CanisterSummary {
        CanisterSummary {
            role: node.role.clone(),
            parent_pid: Some(parent_pid),
        }
    }
}
