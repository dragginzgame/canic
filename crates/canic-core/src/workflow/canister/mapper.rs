use crate::{
    dto::{canister::CanisterEntryView, cascade::TopologyPathNodeView},
    ops::storage::registry::subnet::{CanisterEntrySnapshot, CanisterSummarySnapshot},
    workflow::prelude::*,
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
}
