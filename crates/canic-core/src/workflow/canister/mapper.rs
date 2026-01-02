use crate::{
    cdk::types::Principal,
    dto::{
        canister::{CanisterEntryView, CanisterSummaryView},
        cascade::{TopologyDirectChildView, TopologyPathNodeView},
    },
    ops::storage::registry::subnet::{CanisterEntrySnapshot, CanisterSummarySnapshot},
};

///
/// CanisterMapper
///

pub struct CanisterMapper;

impl CanisterMapper {
    #[must_use]
    pub fn entry_to_view(e: &CanisterEntrySnapshot) -> CanisterEntryView {
        CanisterEntryView {
            role: e.role.clone(),
            parent_pid: e.parent_pid,
            module_hash: e.module_hash.clone(),
            created_at: e.created_at,
        }
    }

    #[must_use]
    pub fn summary_to_view(s: &CanisterSummarySnapshot) -> CanisterSummaryView {
        CanisterSummaryView {
            role: s.role.clone(),
            parent_pid: s.parent_pid,
        }
    }

    #[must_use]
    pub fn summary_to_topology_node(
        pid: Principal,
        summary: &CanisterSummarySnapshot,
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
        summary: &CanisterSummarySnapshot,
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
    ) -> CanisterSummarySnapshot {
        CanisterSummarySnapshot {
            role: node.role.clone(),
            parent_pid: Some(parent_pid),
        }
    }
}
