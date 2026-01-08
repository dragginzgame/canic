use crate::{
    dto::canister::{CanisterChildView, CanisterSummaryView},
    ops::storage::{
        children::{ChildSnapshot, ChildrenSnapshot},
        registry::subnet::SubnetRegistrySnapshot,
    },
    workflow::prelude::*,
};

///
/// ChildrenMapper
///

pub struct ChildrenMapper;

impl ChildrenMapper {
    #[must_use]
    pub fn child_snapshot_to_view(child: ChildSnapshot) -> CanisterSummaryView {
        CanisterSummaryView {
            role: child.role,
            parent_pid: child.parent_pid,
        }
    }

    #[must_use]
    pub fn child_snapshot_to_child_view(child: ChildSnapshot) -> CanisterChildView {
        CanisterChildView {
            pid: child.pid,
            role: child.role,
            parent_pid: child.parent_pid,
        }
    }

    #[must_use]
    pub fn snapshot_to_views(snapshot: ChildrenSnapshot) -> Vec<CanisterSummaryView> {
        snapshot
            .entries
            .into_iter()
            .map(Self::child_snapshot_to_view)
            .collect()
    }

    #[must_use]
    pub fn from_registry_snapshot(
        snapshot: &SubnetRegistrySnapshot,
        parent: Principal,
    ) -> ChildrenSnapshot {
        // Projection only; canonical child derivation is in SubnetRegistry*::children.
        let entries = snapshot
            .entries
            .iter()
            .filter(|&(_, entry)| entry.parent_pid == Some(parent))
            .map(|(pid, entry)| ChildSnapshot {
                pid: *pid,
                role: entry.role.clone(),
                parent_pid: entry.parent_pid,
            })
            .collect();

        ChildrenSnapshot { entries }
    }
}
