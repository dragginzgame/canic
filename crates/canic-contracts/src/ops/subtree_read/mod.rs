//! Lossless subtree transport conversions; no state mutation or lifecycle advancement.
use crate::dto::{component_registry as stored, subtree_read as native};

impl From<stored::RootComponentSubtreeRemovalResponse> for native::Status {
    fn from(value: stored::RootComponentSubtreeRemovalResponse) -> Self {
        Self {
            operation_id: value.operation_id,
            component: value.component,
            target_canister_id: value.target_canister_id.into(),
            target_parent_canister_id: value.target_parent_canister_id.into(),
            target_role: value.target_role,
            target_status: value.target_status,
            reserved_against_registry: value.reserved_against_registry,
            maximum_completed_leaves: value.maximum_completed_leaves,
            completed_leaves: value.completed_leaves,
            traversal_steps: value.traversal_steps,
            phase: value.phase.into(),
        }
    }
}

impl From<native::Status> for stored::RootComponentSubtreeRemovalResponse {
    fn from(value: native::Status) -> Self {
        Self {
            operation_id: value.operation_id,
            component: value.component,
            target_canister_id: value.target_canister_id.into(),
            target_parent_canister_id: value.target_parent_canister_id.into(),
            target_role: value.target_role,
            target_status: value.target_status,
            reserved_against_registry: value.reserved_against_registry,
            maximum_completed_leaves: value.maximum_completed_leaves,
            completed_leaves: value.completed_leaves,
            traversal_steps: value.traversal_steps,
            phase: value.phase.into(),
        }
    }
}

impl From<stored::RootComponentSubtreeRemovalNode> for native::Node {
    fn from(value: stored::RootComponentSubtreeRemovalNode) -> Self {
        Self {
            canister_id: value.canister_id.into(),
            parent_canister_id: value.parent_canister_id.into(),
            role: value.role,
            kind: value.kind,
            installed_artifact_hash: value.installed_artifact_hash,
            status: value.status,
        }
    }
}

impl From<native::Node> for stored::RootComponentSubtreeRemovalNode {
    fn from(value: native::Node) -> Self {
        Self {
            canister_id: value.canister_id.into(),
            parent_canister_id: value.parent_canister_id.into(),
            role: value.role,
            kind: value.kind,
            installed_artifact_hash: value.installed_artifact_hash,
            status: value.status,
        }
    }
}

impl From<stored::RootComponentSubtreeRemovalStopIntent> for native::StopIntent {
    fn from(value: stored::RootComponentSubtreeRemovalStopIntent) -> Self {
        Self {
            leaf: value.leaf.into(),
            controller: value.controller.into(),
        }
    }
}

impl From<native::StopIntent> for stored::RootComponentSubtreeRemovalStopIntent {
    fn from(value: native::StopIntent) -> Self {
        Self {
            leaf: value.leaf.into(),
            controller: value.controller.into(),
        }
    }
}

impl From<stored::RootComponentSubtreeRemovalStoppedReceipt> for native::Stopped {
    fn from(value: stored::RootComponentSubtreeRemovalStoppedReceipt) -> Self {
        Self {
            stop: value.stop.into(),
            observed_module_hash: value.observed_module_hash,
        }
    }
}

impl From<native::Stopped> for stored::RootComponentSubtreeRemovalStoppedReceipt {
    fn from(value: native::Stopped) -> Self {
        Self {
            stop: value.stop.into(),
            observed_module_hash: value.observed_module_hash,
        }
    }
}

impl From<stored::RootComponentSubtreeRemovalDeleteIntent> for native::DeleteIntent {
    fn from(value: stored::RootComponentSubtreeRemovalDeleteIntent) -> Self {
        Self {
            stopped: value.stopped.into(),
        }
    }
}

impl From<native::DeleteIntent> for stored::RootComponentSubtreeRemovalDeleteIntent {
    fn from(value: native::DeleteIntent) -> Self {
        Self {
            stopped: value.stopped.into(),
        }
    }
}

impl From<stored::RootComponentSubtreeRemovalDeletedReceipt> for native::Deleted {
    fn from(value: stored::RootComponentSubtreeRemovalDeletedReceipt) -> Self {
        Self {
            deletion: value.deletion.into(),
        }
    }
}

impl From<native::Deleted> for stored::RootComponentSubtreeRemovalDeletedReceipt {
    fn from(value: native::Deleted) -> Self {
        Self {
            deletion: value.deletion.into(),
        }
    }
}

impl From<stored::RootComponentSubtreeRemovalMembershipRemovedReceipt>
    for native::MembershipRemoved
{
    fn from(value: stored::RootComponentSubtreeRemovalMembershipRemovedReceipt) -> Self {
        Self {
            deleted: value.deleted.into(),
            removed_from_registry: value.removed_from_registry,
            previous_descendant_content_hash: value.previous_descendant_content_hash,
            previous_committed_descendants: value.previous_committed_descendants,
            registry: value.registry,
            descendant_content_hash: value.descendant_content_hash,
            registry_encoded_bytes: value.registry_encoded_bytes,
            reserved_descendants: value.reserved_descendants,
            committed_descendants: value.committed_descendants,
            directory_synchronized_at_ns: value.directory_synchronized_at_ns,
            directory_authority_hash: value.directory_authority_hash,
            parent_role_instances: value.parent_role_instances,
            root_managed_descendants: value.root_managed_descendants,
            root_known_created_component_canisters: value.root_known_created_component_canisters,
        }
    }
}

impl From<native::MembershipRemoved>
    for stored::RootComponentSubtreeRemovalMembershipRemovedReceipt
{
    fn from(value: native::MembershipRemoved) -> Self {
        Self {
            deleted: value.deleted.into(),
            removed_from_registry: value.removed_from_registry,
            previous_descendant_content_hash: value.previous_descendant_content_hash,
            previous_committed_descendants: value.previous_committed_descendants,
            registry: value.registry,
            descendant_content_hash: value.descendant_content_hash,
            registry_encoded_bytes: value.registry_encoded_bytes,
            reserved_descendants: value.reserved_descendants,
            committed_descendants: value.committed_descendants,
            directory_synchronized_at_ns: value.directory_synchronized_at_ns,
            directory_authority_hash: value.directory_authority_hash,
            parent_role_instances: value.parent_role_instances,
            root_managed_descendants: value.root_managed_descendants,
            root_known_created_component_canisters: value.root_known_created_component_canisters,
        }
    }
}

impl From<stored::RootComponentSubtreeRemovalDirectoryConvergenceEvidence>
    for native::DirectoryConvergence
{
    fn from(value: stored::RootComponentSubtreeRemovalDirectoryConvergenceEvidence) -> Self {
        Self {
            operation_id: value.operation_id,
            canister_id: value.canister_id.into(),
            activation: value.activation,
        }
    }
}

impl From<native::DirectoryConvergence>
    for stored::RootComponentSubtreeRemovalDirectoryConvergenceEvidence
{
    fn from(value: native::DirectoryConvergence) -> Self {
        Self {
            operation_id: value.operation_id,
            canister_id: value.canister_id.into(),
            activation: value.activation,
        }
    }
}

impl From<stored::RootComponentSubtreeRemovalDirectorySynchronizedReceipt>
    for native::DirectorySynchronized
{
    fn from(value: stored::RootComponentSubtreeRemovalDirectorySynchronizedReceipt) -> Self {
        Self {
            membership_removed: value.membership_removed.into(),
            covered_fleet_registry_revision: value.covered_fleet_registry_revision,
            covered_fleet_registry_content_hash: value.covered_fleet_registry_content_hash,
            covered_component_registry: value.covered_component_registry,
            covered_authority_hash: value.covered_authority_hash,
            owning_component: value.owning_component.map(Into::into),
            parent: value.parent.map(Into::into),
        }
    }
}

impl From<native::DirectorySynchronized>
    for stored::RootComponentSubtreeRemovalDirectorySynchronizedReceipt
{
    fn from(value: native::DirectorySynchronized) -> Self {
        Self {
            membership_removed: value.membership_removed.into(),
            covered_fleet_registry_revision: value.covered_fleet_registry_revision,
            covered_fleet_registry_content_hash: value.covered_fleet_registry_content_hash,
            covered_component_registry: value.covered_component_registry,
            covered_authority_hash: value.covered_authority_hash,
            owning_component: value.owning_component.map(Into::into),
            parent: value.parent.map(Into::into),
        }
    }
}

impl From<stored::RootComponentSubtreeRemovalCompletedReceipt> for native::Completed {
    fn from(value: stored::RootComponentSubtreeRemovalCompletedReceipt) -> Self {
        Self {
            registry: value.registry,
            directory_authority_hash: value.directory_authority_hash,
        }
    }
}

impl From<native::Completed> for stored::RootComponentSubtreeRemovalCompletedReceipt {
    fn from(value: native::Completed) -> Self {
        Self {
            registry: value.registry,
            directory_authority_hash: value.directory_authority_hash,
        }
    }
}

impl From<stored::RootComponentSubtreeRemovalPhase> for native::Phase {
    fn from(value: stored::RootComponentSubtreeRemovalPhase) -> Self {
        match value {
            stored::RootComponentSubtreeRemovalPhase::Fenced => Self::Fenced,
            stored::RootComponentSubtreeRemovalPhase::Traversing(value) => {
                Self::Traversing(Box::new(value.into()))
            }
            stored::RootComponentSubtreeRemovalPhase::LeafSelected(value) => {
                Self::LeafSelected(Box::new(value.into()))
            }
            stored::RootComponentSubtreeRemovalPhase::StopIntent(value) => {
                Self::StopIntent(Box::new(value.into()))
            }
            stored::RootComponentSubtreeRemovalPhase::Stopped(value) => {
                Self::Stopped(Box::new(value.into()))
            }
            stored::RootComponentSubtreeRemovalPhase::DeleteIntent(value) => {
                Self::DeleteIntent(Box::new(value.into()))
            }
            stored::RootComponentSubtreeRemovalPhase::Deleted(value) => {
                Self::Deleted(Box::new(value.into()))
            }
            stored::RootComponentSubtreeRemovalPhase::MembershipRemoved(value) => {
                Self::MembershipRemoved(Box::new(value.into()))
            }
            stored::RootComponentSubtreeRemovalPhase::DirectorySynchronized(value) => {
                Self::DirectorySynchronized(Box::new(value.into()))
            }
            stored::RootComponentSubtreeRemovalPhase::Completed(value) => {
                Self::Completed(Box::new(value.into()))
            }
        }
    }
}

impl From<native::Phase> for stored::RootComponentSubtreeRemovalPhase {
    fn from(value: native::Phase) -> Self {
        match value {
            native::Phase::Fenced => Self::Fenced,
            native::Phase::Traversing(value) => Self::Traversing((*value).into()),
            native::Phase::LeafSelected(value) => Self::LeafSelected((*value).into()),
            native::Phase::StopIntent(value) => Self::StopIntent((*value).into()),
            native::Phase::Stopped(value) => Self::Stopped((*value).into()),
            native::Phase::DeleteIntent(value) => Self::DeleteIntent((*value).into()),
            native::Phase::Deleted(value) => Self::Deleted((*value).into()),
            native::Phase::MembershipRemoved(value) => Self::MembershipRemoved((*value).into()),
            native::Phase::DirectorySynchronized(value) => {
                Self::DirectorySynchronized((*value).into())
            }
            native::Phase::Completed(value) => Self::Completed((*value).into()),
        }
    }
}
