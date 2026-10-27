//! Lossless operation boundary conversions without allocation or removal policy.
use crate::dto::{component_control as wire, component_registry as stored};
impl From<stored::RootComponentSubtreeRemovalRequest> for wire::SubtreeRequest {
    fn from(value: stored::RootComponentSubtreeRemovalRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            component: value.component,
            target_canister_id: value.target_canister_id.into(),
            expected_registry: value.expected_registry,
        }
    }
}
impl From<wire::SubtreeRequest> for stored::RootComponentSubtreeRemovalRequest {
    fn from(value: wire::SubtreeRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            component: value.component,
            target_canister_id: value.target_canister_id.into(),
            expected_registry: value.expected_registry,
        }
    }
}
impl From<stored::RootPeerComponentAllocationRequest> for wire::PeerRequest {
    fn from(value: stored::RootPeerComponentAllocationRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            component_spec: value.component_spec,
            requester: value.requester.into(),
        }
    }
}
impl From<wire::PeerRequest> for stored::RootPeerComponentAllocationRequest {
    fn from(value: wire::PeerRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            component_spec: value.component_spec,
            requester: value.requester.into(),
        }
    }
}
impl From<stored::PeerComponentRequester> for wire::PeerRequester {
    fn from(value: stored::PeerComponentRequester) -> Self {
        match value {
            stored::PeerComponentRequester::SameRoot => Self::SameRoot,
            stored::PeerComponentRequester::FleetService {
                service,
                expected_registry,
            } => Self::FleetService {
                service,
                expected_registry: Box::new((*expected_registry).into()),
            },
        }
    }
}
impl From<wire::PeerRequester> for stored::PeerComponentRequester {
    fn from(value: wire::PeerRequester) -> Self {
        match value {
            wire::PeerRequester::SameRoot => Self::SameRoot,
            wire::PeerRequester::FleetService {
                service,
                expected_registry,
            } => Self::FleetService {
                service,
                expected_registry: Box::new((*expected_registry).into()),
            },
        }
    }
}

impl From<stored::RootComponentRegistryPreparationRequest> for wire::PreparationRequest {
    fn from(value: stored::RootComponentRegistryPreparationRequest) -> Self {
        Self {
            store_bootstrap: value.store_bootstrap,
            expected_fleet_registry: value.expected_fleet_registry.into(),
        }
    }
}

impl From<wire::PreparationRequest> for stored::RootComponentRegistryPreparationRequest {
    fn from(value: wire::PreparationRequest) -> Self {
        Self {
            store_bootstrap: value.store_bootstrap,
            expected_fleet_registry: value.expected_fleet_registry.into(),
        }
    }
}

impl From<stored::RootComponentRegistryStatusResponse> for wire::RegistryStatus {
    fn from(value: stored::RootComponentRegistryStatusResponse) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            prepared_against_registry: value.prepared_against_registry.into(),
            release_set: value.release_set,
            component_topology_digest: value.component_topology_digest,
            next_allocation_sequence: value.next_allocation_sequence,
            reserved_component_instances: value.reserved_component_instances,
            committed_component_instances: value.committed_component_instances,
            managed_descendants: value.managed_descendants,
            known_created_component_canisters: value.known_created_component_canisters,
            encoded_bytes: value.encoded_bytes,
            initial_inventory: value.initial_inventory,
        }
    }
}

impl From<wire::RegistryStatus> for stored::RootComponentRegistryStatusResponse {
    fn from(value: wire::RegistryStatus) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            prepared_against_registry: value.prepared_against_registry.into(),
            release_set: value.release_set,
            component_topology_digest: value.component_topology_digest,
            next_allocation_sequence: value.next_allocation_sequence,
            reserved_component_instances: value.reserved_component_instances,
            committed_component_instances: value.committed_component_instances,
            managed_descendants: value.managed_descendants,
            known_created_component_canisters: value.known_created_component_canisters,
            encoded_bytes: value.encoded_bytes,
            initial_inventory: value.initial_inventory,
        }
    }
}
