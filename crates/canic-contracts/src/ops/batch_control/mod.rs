//! Lossless transport conversion without rehashing plans or changing operation authority.
use crate::dto::{batch_control as wire, component_provisioning as stored};

impl From<stored::FleetComponentProvisioningPlan> for wire::Plan {
    fn from(value: stored::FleetComponentProvisioningPlan) -> Self {
        Self {
            fleet: value.fleet,
            fleet_registry: value.fleet_registry.into(),
            configuration_digest: value.configuration_digest,
            operation: value.operation,
            directory_confirmation_roots: value
                .directory_confirmation_roots
                .into_iter()
                .map(Into::into)
                .collect(),
            batches: value.batches.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<wire::Plan> for stored::FleetComponentProvisioningPlan {
    fn from(value: wire::Plan) -> Self {
        Self {
            fleet: value.fleet,
            fleet_registry: value.fleet_registry.into(),
            configuration_digest: value.configuration_digest,
            operation: value.operation,
            directory_confirmation_roots: value
                .directory_confirmation_roots
                .into_iter()
                .map(Into::into)
                .collect(),
            batches: value.batches.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<stored::FleetComponentProvisioningPrepareRequest> for wire::PrepareRequest {
    fn from(value: stored::FleetComponentProvisioningPrepareRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            plan: value.plan.into(),
        }
    }
}

impl From<wire::PrepareRequest> for stored::FleetComponentProvisioningPrepareRequest {
    fn from(value: wire::PrepareRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            plan: value.plan.into(),
        }
    }
}

impl From<stored::FleetSubnetRootProvisioningBatch> for wire::Batch {
    fn from(value: stored::FleetSubnetRootProvisioningBatch) -> Self {
        Self {
            root: value.root.into(),
            active_release_set: value.active_release_set,
            placements: value.placements,
        }
    }
}

impl From<wire::Batch> for stored::FleetSubnetRootProvisioningBatch {
    fn from(value: wire::Batch) -> Self {
        Self {
            root: value.root.into(),
            active_release_set: value.active_release_set,
            placements: value.placements,
        }
    }
}

impl From<stored::RootComponentProvisioningAcceptanceRequest> for wire::RootRequest {
    fn from(value: stored::RootComponentProvisioningAcceptanceRequest) -> Self {
        Self {
            fleet_registry: value.fleet_registry.into(),
            configuration_digest: value.configuration_digest,
            operation_id: value.operation_id,
            plan_hash: value.plan_hash,
            batch: value.batch.into(),
        }
    }
}

impl From<wire::RootRequest> for stored::RootComponentProvisioningAcceptanceRequest {
    fn from(value: wire::RootRequest) -> Self {
        Self {
            fleet_registry: value.fleet_registry.into(),
            configuration_digest: value.configuration_digest,
            operation_id: value.operation_id,
            plan_hash: value.plan_hash,
            batch: value.batch.into(),
        }
    }
}

impl From<stored::RootComponentDirectorySynchronizationRequest> for wire::SynchronizeRequest {
    fn from(value: stored::RootComponentDirectorySynchronizationRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            plan_hash: value.plan_hash,
            source_fleet_registry: value.source_fleet_registry.into(),
            published_fleet_registry: value.published_fleet_registry.into(),
            expected_synchronized_component_count: value.expected_synchronized_component_count,
        }
    }
}

impl From<wire::SynchronizeRequest> for stored::RootComponentDirectorySynchronizationRequest {
    fn from(value: wire::SynchronizeRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            plan_hash: value.plan_hash,
            source_fleet_registry: value.source_fleet_registry.into(),
            published_fleet_registry: value.published_fleet_registry.into(),
            expected_synchronized_component_count: value.expected_synchronized_component_count,
        }
    }
}

impl From<stored::RootComponentDirectorySynchronizationResponse> for wire::Synchronization {
    fn from(value: stored::RootComponentDirectorySynchronizationResponse) -> Self {
        Self {
            operation_id: value.operation_id,
            plan_hash: value.plan_hash,
            source_fleet_registry: value.source_fleet_registry.into(),
            published_fleet_registry: value.published_fleet_registry.into(),
            fleet_subnet_root: value.fleet_subnet_root.into(),
            affected_component_count: value.affected_component_count,
            synchronized_component_count: value.synchronized_component_count,
            fleet_directory_content_hash: value.fleet_directory_content_hash,
            complete: value.complete,
            synchronized_at_ns: value.synchronized_at_ns,
            receipt_content_hash: value.receipt_content_hash,
        }
    }
}

impl From<wire::Synchronization> for stored::RootComponentDirectorySynchronizationResponse {
    fn from(value: wire::Synchronization) -> Self {
        Self {
            operation_id: value.operation_id,
            plan_hash: value.plan_hash,
            source_fleet_registry: value.source_fleet_registry.into(),
            published_fleet_registry: value.published_fleet_registry.into(),
            fleet_subnet_root: value.fleet_subnet_root.into(),
            affected_component_count: value.affected_component_count,
            synchronized_component_count: value.synchronized_component_count,
            fleet_directory_content_hash: value.fleet_directory_content_hash,
            complete: value.complete,
            synchronized_at_ns: value.synchronized_at_ns,
            receipt_content_hash: value.receipt_content_hash,
        }
    }
}
