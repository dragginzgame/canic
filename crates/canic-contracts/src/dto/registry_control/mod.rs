//! Native compare-and-commit registry mutation boundaries.
use crate::dto::{fleet_snapshot::FleetSubnetRootEntry, registry_discovery::RegistryVersion};
use serde::{Deserialize, Serialize};

/// FleetSubnetRootJoinRequest with native registry identities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct JoinRequest {
    pub expected_registry: RegistryVersion,
    pub entry: FleetSubnetRootEntry,
}

/// FleetSubnetRootJoinResponse with native registry identities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct JoinResponse {
    pub entry: FleetSubnetRootEntry,
    pub version: RegistryVersion,
}

/// FleetRegistryActivationRequest with native registry identities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ActivateRequest {
    pub expected_registry: RegistryVersion,
}

/// FleetRegistryActivationResponse with native registry identities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ActivateResponse {
    pub previous_version: RegistryVersion,
    pub version: RegistryVersion,
}

/// Exact Root synchronization intent; the canonical registry digest is unchanged.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct SynchronizeRequest {
    pub operation_id: [u8; 32],
    pub expected_registry: RegistryVersion,
    pub store_bootstrap: crate::dto::root_store::RootStoreBootstrapRequest,
}

/// Exact retained Root synchronization and Coordinator acknowledgement.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Synchronization {
    pub fleet_subnet_root: ic_platform::Principal,
    pub version: RegistryVersion,
    pub acknowledgement: crate::dto::coordinator_acknowledgements::RootAcknowledgement,
}

/// Complete atomic mirror activation, including the published Directory.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct MirrorActivation {
    pub fleet_subnet_root: ic_platform::Principal,
    pub previous_registry: RegistryVersion,
    pub version: RegistryVersion,
    pub directory: crate::dto::runtime_configuration::FleetDirectorySnapshot,
}

/// Exact synchronization operation evidence; observation never advances the mirror.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct SynchronizationStatus {
    pub synchronization: Synchronization,
    pub activation: Option<MirrorActivation>,
}
