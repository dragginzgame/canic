//! Lossless component-removal transport conversions; no lifecycle effects.
use crate::dto::{component_registry as stored, component_removal_read as native};

impl From<crate::dto::root::RootComponentRemovalOperationStatus> for native::Status {
    fn from(value: crate::dto::root::RootComponentRemovalOperationStatus) -> Self {
        Self {
            draining: value.draining,
            deletion: value.deletion.map(Into::into),
        }
    }
}

impl From<native::Status> for crate::dto::root::RootComponentRemovalOperationStatus {
    fn from(value: native::Status) -> Self {
        Self {
            draining: value.draining,
            deletion: value.deletion.map(Into::into),
        }
    }
}

impl From<stored::RootComponentDeletionResponse> for native::Deletion {
    fn from(value: stored::RootComponentDeletionResponse) -> Self {
        Self {
            operation_id: value.operation_id,
            component: value.component,
            phase: value.phase.into(),
        }
    }
}

impl From<native::Deletion> for stored::RootComponentDeletionResponse {
    fn from(value: native::Deletion) -> Self {
        Self {
            operation_id: value.operation_id,
            component: value.component,
            phase: value.phase.into(),
        }
    }
}

impl From<stored::RootComponentQuiescenceStopIntent> for native::StopIntent {
    fn from(value: stored::RootComponentQuiescenceStopIntent) -> Self {
        Self {
            registry: value.registry,
            descendant_count: value.descendant_count,
            descendant_content_hash: value.descendant_content_hash,
            canister_id: value.canister_id.into(),
            controller: value.controller.into(),
            expected_module_hash: value.expected_module_hash,
            covered_fleet_registry_revision: value.covered_fleet_registry_revision,
            covered_fleet_registry_content_hash: value.covered_fleet_registry_content_hash,
            covered_authority_hash: value.covered_authority_hash,
            runtime_operation_id: value.runtime_operation_id,
            activation: value.activation,
            prepared_at_ns: value.prepared_at_ns,
        }
    }
}

impl From<native::StopIntent> for stored::RootComponentQuiescenceStopIntent {
    fn from(value: native::StopIntent) -> Self {
        Self {
            registry: value.registry,
            descendant_count: value.descendant_count,
            descendant_content_hash: value.descendant_content_hash,
            canister_id: value.canister_id.into(),
            controller: value.controller.into(),
            expected_module_hash: value.expected_module_hash,
            covered_fleet_registry_revision: value.covered_fleet_registry_revision,
            covered_fleet_registry_content_hash: value.covered_fleet_registry_content_hash,
            covered_authority_hash: value.covered_authority_hash,
            runtime_operation_id: value.runtime_operation_id,
            activation: value.activation,
            prepared_at_ns: value.prepared_at_ns,
        }
    }
}

impl From<stored::RootComponentQuiescentReceipt> for native::Quiescent {
    fn from(value: stored::RootComponentQuiescentReceipt) -> Self {
        Self {
            stop: value.stop.into(),
            observed_module_hash: value.observed_module_hash,
            quiesced_at_ns: value.quiesced_at_ns,
        }
    }
}

impl From<native::Quiescent> for stored::RootComponentQuiescentReceipt {
    fn from(value: native::Quiescent) -> Self {
        Self {
            stop: value.stop.into(),
            observed_module_hash: value.observed_module_hash,
            quiesced_at_ns: value.quiesced_at_ns,
        }
    }
}

impl From<stored::RootComponentDeletionIntent> for native::DeleteIntent {
    fn from(value: stored::RootComponentDeletionIntent) -> Self {
        Self {
            final_inventory: value.final_inventory,
            quiescence: value.quiescence.into(),
            prepared_at_ns: value.prepared_at_ns,
        }
    }
}

impl From<native::DeleteIntent> for stored::RootComponentDeletionIntent {
    fn from(value: native::DeleteIntent) -> Self {
        Self {
            final_inventory: value.final_inventory,
            quiescence: value.quiescence.into(),
            prepared_at_ns: value.prepared_at_ns,
        }
    }
}

impl From<stored::RootComponentDeletedReceipt> for native::Deleted {
    fn from(value: stored::RootComponentDeletedReceipt) -> Self {
        Self {
            deletion: value.deletion.into(),
            deleted_at_ns: value.deleted_at_ns,
        }
    }
}

impl From<native::Deleted> for stored::RootComponentDeletedReceipt {
    fn from(value: native::Deleted) -> Self {
        Self {
            deletion: value.deletion.into(),
            deleted_at_ns: value.deleted_at_ns,
        }
    }
}

impl From<stored::RootComponentMembershipRemovedReceipt> for native::MembershipRemoved {
    fn from(value: stored::RootComponentMembershipRemovedReceipt) -> Self {
        Self {
            deleted: value.deleted.into(),
            allocation_operation_id: value.allocation_operation_id,
            remaining_spec_committed_instances: value.remaining_spec_committed_instances,
            root_committed_component_instances: value.root_committed_component_instances,
            root_known_created_component_canisters: value.root_known_created_component_canisters,
            root_registry_encoded_bytes: value.root_registry_encoded_bytes,
            removed_at_ns: value.removed_at_ns,
            removal_hash: value.removal_hash,
        }
    }
}

impl From<native::MembershipRemoved> for stored::RootComponentMembershipRemovedReceipt {
    fn from(value: native::MembershipRemoved) -> Self {
        Self {
            deleted: value.deleted.into(),
            allocation_operation_id: value.allocation_operation_id,
            remaining_spec_committed_instances: value.remaining_spec_committed_instances,
            root_committed_component_instances: value.root_committed_component_instances,
            root_known_created_component_canisters: value.root_known_created_component_canisters,
            root_registry_encoded_bytes: value.root_registry_encoded_bytes,
            removed_at_ns: value.removed_at_ns,
            removal_hash: value.removal_hash,
        }
    }
}

impl From<stored::RootComponentDeletionPhase> for native::Phase {
    fn from(value: stored::RootComponentDeletionPhase) -> Self {
        match value {
            stored::RootComponentDeletionPhase::DeleteIntent(value) => {
                Self::DeleteIntent(Box::new(value.into()))
            }
            stored::RootComponentDeletionPhase::Deleted(value) => {
                Self::Deleted(Box::new(value.into()))
            }
            stored::RootComponentDeletionPhase::MembershipRemoved(value) => {
                Self::MembershipRemoved(Box::new(value.into()))
            }
        }
    }
}

impl From<native::Phase> for stored::RootComponentDeletionPhase {
    fn from(value: native::Phase) -> Self {
        match value {
            native::Phase::DeleteIntent(value) => Self::DeleteIntent((*value).into()),
            native::Phase::Deleted(value) => Self::Deleted((*value).into()),
            native::Phase::MembershipRemoved(value) => Self::MembershipRemoved((*value).into()),
        }
    }
}
