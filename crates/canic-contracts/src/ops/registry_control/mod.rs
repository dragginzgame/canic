//! Lossless registry mutation projection; canonical registry hashes remain unchanged.
use crate::dto::{fleet_registry as stored, registry_control as wire};
impl From<stored::FleetSubnetRootJoinRequest> for wire::JoinRequest {
    fn from(value: stored::FleetSubnetRootJoinRequest) -> Self {
        Self {
            expected_registry: value.expected_registry.into(),
            entry: value.entry.into(),
        }
    }
}
impl From<wire::JoinRequest> for stored::FleetSubnetRootJoinRequest {
    fn from(value: wire::JoinRequest) -> Self {
        Self {
            expected_registry: value.expected_registry.into(),
            entry: value.entry.into(),
        }
    }
}
impl From<stored::FleetSubnetRootJoinResponse> for wire::JoinResponse {
    fn from(value: stored::FleetSubnetRootJoinResponse) -> Self {
        Self {
            entry: value.entry.into(),
            version: value.version.into(),
        }
    }
}
impl From<wire::JoinResponse> for stored::FleetSubnetRootJoinResponse {
    fn from(value: wire::JoinResponse) -> Self {
        Self {
            entry: value.entry.into(),
            version: value.version.into(),
        }
    }
}
impl From<stored::FleetRegistryActivationRequest> for wire::ActivateRequest {
    fn from(value: stored::FleetRegistryActivationRequest) -> Self {
        Self {
            expected_registry: value.expected_registry.into(),
        }
    }
}
impl From<wire::ActivateRequest> for stored::FleetRegistryActivationRequest {
    fn from(value: wire::ActivateRequest) -> Self {
        Self {
            expected_registry: value.expected_registry.into(),
        }
    }
}
impl From<stored::FleetRegistryActivationResponse> for wire::ActivateResponse {
    fn from(value: stored::FleetRegistryActivationResponse) -> Self {
        Self {
            previous_version: value.previous_version.into(),
            version: value.version.into(),
        }
    }
}
impl From<wire::ActivateResponse> for stored::FleetRegistryActivationResponse {
    fn from(value: wire::ActivateResponse) -> Self {
        Self {
            previous_version: value.previous_version.into(),
            version: value.version.into(),
        }
    }
}

impl From<stored::FleetSubnetRootRegistrySyncRequest> for wire::SynchronizeRequest {
    fn from(value: stored::FleetSubnetRootRegistrySyncRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            expected_registry: value.expected_registry.into(),
            store_bootstrap: value.store_bootstrap,
        }
    }
}
impl From<wire::SynchronizeRequest> for stored::FleetSubnetRootRegistrySyncRequest {
    fn from(value: wire::SynchronizeRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            expected_registry: value.expected_registry.into(),
            store_bootstrap: value.store_bootstrap,
        }
    }
}

impl From<crate::dto::fleet_registry::FleetSubnetRootRegistrySyncResponse>
    for wire::Synchronization
{
    fn from(value: crate::dto::fleet_registry::FleetSubnetRootRegistrySyncResponse) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            version: value.version.into(),
            acknowledgement: value.acknowledgement.into(),
        }
    }
}

impl From<wire::Synchronization>
    for crate::dto::fleet_registry::FleetSubnetRootRegistrySyncResponse
{
    fn from(value: wire::Synchronization) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            version: value.version.into(),
            acknowledgement: value.acknowledgement.into(),
        }
    }
}

impl From<crate::dto::fleet_registry::FleetSubnetRootRegistryMirrorActivationResponse>
    for wire::MirrorActivation
{
    fn from(
        value: crate::dto::fleet_registry::FleetSubnetRootRegistryMirrorActivationResponse,
    ) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            previous_registry: value.previous_registry.into(),
            version: value.version.into(),
            directory: value.directory.into(),
        }
    }
}

impl From<wire::MirrorActivation>
    for crate::dto::fleet_registry::FleetSubnetRootRegistryMirrorActivationResponse
{
    fn from(value: wire::MirrorActivation) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            previous_registry: value.previous_registry.into(),
            version: value.version.into(),
            directory: value.directory.into(),
        }
    }
}

impl From<crate::dto::root::RootRegistrySynchronizationOperationStatus>
    for wire::SynchronizationStatus
{
    fn from(value: crate::dto::root::RootRegistrySynchronizationOperationStatus) -> Self {
        Self {
            synchronization: value.synchronization.into(),
            activation: value.activation.map(Into::into),
        }
    }
}

impl From<wire::SynchronizationStatus>
    for crate::dto::root::RootRegistrySynchronizationOperationStatus
{
    fn from(value: wire::SynchronizationStatus) -> Self {
        Self {
            synchronization: value.synchronization.into(),
            activation: value.activation.map(Into::into),
        }
    }
}
