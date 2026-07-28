//! Module: view::component_registry
//!
//! Responsibility: model read-only Component Registry authority and allocation reservations.
//! Does not own: persisted records, validation, allocation, or lifecycle mutation.
//! Boundary: Component Registry ops construct these values for workflow consumption.

use canic_core::{
    cdk::types::{Cycles, Principal},
    control_plane_support::model::replay::ReplayCostGuardSettlement,
    dto::{
        component_registry::{
            ComponentLifecycleStatus, ComponentProvisioningOrigin, ComponentRegistryHead,
        },
        fleet_registry::FleetRegistryVersion,
        root_store::RootStoreBootstrapRequest,
    },
    ids::{
        CanisterRole, ComponentBinding, ComponentInstanceId, ComponentSpecId,
        FleetSubnetRootBinding, FleetSubnetRootReleaseSet,
    },
};

///
/// RootComponentRegistryView
///
/// Read-only durable preparation authority and current allocation counters.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentRegistryView {
    pub root: FleetSubnetRootBinding,
    pub prepared_against_registry: FleetRegistryVersion,
    pub release_set: FleetSubnetRootReleaseSet,
    pub store_bootstrap: RootStoreBootstrapRequest,
    pub next_allocation_sequence: u64,
    pub reserved_component_instances: u32,
    pub committed_component_instances: u32,
    pub managed_descendants: u32,
    pub encoded_bytes: u64,
}

///
/// RootComponentAllocationView
///
/// Read-only exact top-level Component identity and capacity reservation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentAllocationView {
    pub operation_id: [u8; 32],
    pub allocation_sequence: u64,
    pub component: ComponentInstanceId,
    pub component_spec: ComponentSpecId,
    pub spec_hash: [u8; 32],
    pub role: CanisterRole,
    pub provisioning_origin: ComponentProvisioningOrigin,
    pub release_set: FleetSubnetRootReleaseSet,
    pub progress: RootComponentAllocationProgressView,
}

///
/// RootComponentAllocationProgressView
///
/// Read-only paid-effect state for one top-level Component allocation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RootComponentAllocationProgressView {
    Reserved,
    CreationIntent(RootComponentCreationEffectView),
    Created {
        effect: RootComponentCreationEffectView,
        canister: Principal,
    },
    InstallIntent {
        creation: RootComponentCreationEffectView,
        canister: Principal,
        installation: RootComponentInstallEffectView,
    },
    Installed {
        creation: RootComponentCreationEffectView,
        canister: Principal,
        installation: RootComponentInstallEffectView,
    },
    Verified {
        creation: RootComponentCreationEffectView,
        canister: Principal,
        installation: RootComponentInstallEffectView,
    },
    Committed {
        creation: RootComponentCreationEffectView,
        canister: Principal,
        installation: RootComponentInstallEffectView,
        commitment: RootComponentCommitmentView,
    },
}

///
/// RootComponentCreationEffectView
///
/// Read-only exact Store artifact, creation settings and cost settlement.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentCreationEffectView {
    pub wasm_store: Principal,
    pub payload_hash: [u8; 32],
    pub payload_size_bytes: u64,
    pub initial_cycles: Cycles,
    pub controller: Principal,
    pub cost_guard_settlement: ReplayCostGuardSettlement,
    pub charged_entry_bytes: u64,
}

///
/// RootComponentInstallEffectView
///
/// Read-only exact raw artifact, install source, target identity and cost settlement.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentInstallEffectView {
    pub raw_module_hash: [u8; 32],
    pub chunk_hashes: Vec<Vec<u8>>,
    pub binding: ComponentBinding,
    pub cost_guard_settlement: ReplayCostGuardSettlement,
    pub charged_entry_bytes: u64,
}

///
/// RootComponentCommitmentView
///
/// Read-only link from one allocation receipt to its Registry and Directory authority.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentCommitmentView {
    pub registry: ComponentRegistryHead,
    pub directory_synchronized_at_ns: u64,
}

///
/// ComponentRegistryPartitionView
///
/// Read-only normalized authority for one committed Component tree.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentRegistryPartitionView {
    pub binding: ComponentBinding,
    pub provisioning_origin: ComponentProvisioningOrigin,
    pub release_set: FleetSubnetRootReleaseSet,
    pub status: ComponentLifecycleStatus,
    pub revision: u64,
    pub content_hash: [u8; 32],
    pub directory_synchronized_at_ns: u64,
    pub encoded_bytes: u64,
}
