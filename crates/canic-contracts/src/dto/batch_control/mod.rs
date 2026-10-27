//! Native provisioning authority; stored plan bytes and hashes retain their canonical owner.
use crate::{
    dto::{
        component_provisioning::{
            ComponentGroupPlacementPlan, FleetComponentProvisioningOperation,
        },
        fleet_snapshot::FleetSubnetRootBinding,
        registry_discovery::RegistryVersion,
    },
    ids::{ComponentDeploymentConfigurationDigest, FleetBinding, FleetSubnetRootReleaseSet},
};
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Native FleetComponentProvisioningPlan boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Plan {
    pub fleet: FleetBinding,
    pub fleet_registry: RegistryVersion,
    pub configuration_digest: ComponentDeploymentConfigurationDigest,
    pub operation: FleetComponentProvisioningOperation,
    pub directory_confirmation_roots: Vec<Principal>,
    pub batches: Vec<Batch>,
}

/// Native FleetComponentProvisioningPrepareRequest boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PrepareRequest {
    pub operation_id: [u8; 32],
    pub plan: Plan,
}

/// Native FleetSubnetRootProvisioningBatch boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Batch {
    pub root: FleetSubnetRootBinding,
    pub active_release_set: FleetSubnetRootReleaseSet,
    pub placements: Vec<ComponentGroupPlacementPlan>,
}

/// Native RootComponentProvisioningAcceptanceRequest boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct RootRequest {
    pub fleet_registry: RegistryVersion,
    pub configuration_digest: ComponentDeploymentConfigurationDigest,
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub batch: Batch,
}

/// Native RootComponentDirectorySynchronizationRequest boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct SynchronizeRequest {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub source_fleet_registry: RegistryVersion,
    pub published_fleet_registry: RegistryVersion,
    pub expected_synchronized_component_count: u32,
}

/// Native RootComponentDirectorySynchronizationResponse boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Synchronization {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub source_fleet_registry: RegistryVersion,
    pub published_fleet_registry: RegistryVersion,
    pub fleet_subnet_root: Principal,
    pub affected_component_count: u32,
    pub synchronized_component_count: u32,
    pub fleet_directory_content_hash: [u8; 32],
    pub complete: bool,
    pub synchronized_at_ns: Option<u64>,
    pub receipt_content_hash: [u8; 32],
}
