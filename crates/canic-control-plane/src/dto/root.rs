//! Module: dto::root
//!
//! Responsibility: carry Fleet Subnet Root operation-detail projections.
//! Does not own: profile-pruned status envelopes, durable state, advancement, or policy.
//! Boundary: the destination macro composes these durable details into its exact status union.

use candid::CandidType;
use canic_core::dto::{
    component_provisioning::RootComponentProvisioningStatusResponse,
    component_registry::{
        RootComponentAllocationResponse, RootComponentChildAllocationResponse,
        RootComponentDeletionResponse, RootComponentDrainingResponse,
        RootComponentSubtreeRemovalResponse,
    },
    fleet_activation::FleetActivationStatusResponse,
    fleet_registry::{
        FleetSubnetRootDeletionReadinessIntentRequest, FleetSubnetRootDeletionReadinessRequest,
        FleetSubnetRootRegistryMirrorActivationResponse, FleetSubnetRootRegistrySyncResponse,
        FleetSubnetRootRemovalPublicationResponse,
    },
    fleet_subnet_root::{
        FleetSubnetRootDeletionPreparationResponse, FleetSubnetRootDrainingResponse,
        FleetSubnetRootFinalInventoryResponse, FleetSubnetRootStoreBindingFinalizationResponse,
        FleetSubnetRootStoreDeletionResponse, FleetSubnetRootStoreReclamationResponse,
        FleetSubnetWasmStoreAdoptionResponse,
    },
    icp_refill::IcpRefillResponse,
    root_store::RootStoreBootstrapResponse,
};
use serde::Deserialize;

/// Root Component allocation detail projected through the operation lane.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct RootComponentOperationStatus {
    pub allocation: RootComponentAllocationResponse,
    pub complete: bool,
}

/// Root direct-child allocation detail projected through the operation lane.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct RootComponentChildOperationStatus {
    pub allocation: RootComponentChildAllocationResponse,
}

/// Root Component removal detail projected through the operation lane.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct RootComponentRemovalOperationStatus {
    pub draining: RootComponentDrainingResponse,
    pub deletion: Option<RootComponentDeletionResponse>,
}

/// Root-local removal progress across the existing durable high-level boundaries.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct RootRemovalOperationStatus {
    pub operation_id: [u8; 32],
    pub draining: FleetSubnetRootDrainingResponse,
    pub final_inventory: Option<FleetSubnetRootFinalInventoryResponse>,
    pub removal: Option<FleetSubnetRootRemovalPublicationResponse>,
    pub store_reclamation: Option<FleetSubnetRootStoreReclamationResponse>,
    pub store_binding_finalization: Option<FleetSubnetRootStoreBindingFinalizationResponse>,
    pub store_deletion: Option<FleetSubnetRootStoreDeletionResponse>,
    pub deletion_readiness_intent: Option<FleetSubnetRootDeletionReadinessIntentRequest>,
    pub deletion_readiness: Option<FleetSubnetRootDeletionReadinessRequest>,
    pub deletion_preparation: Option<FleetSubnetRootDeletionPreparationResponse>,
}

/// Root Registry synchronization plus its autonomously activated mirror.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct RootRegistrySynchronizationOperationStatus {
    pub synchronization: FleetSubnetRootRegistrySyncResponse,
    pub activation: Option<FleetSubnetRootRegistryMirrorActivationResponse>,
}

/// Root-owned durable operation detail selected by one operation ID.
#[derive(CandidType, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "the accepted Candid union carries each existing status DTO directly"
)]
pub enum RootOperationStatusResponse {
    AdoptStore(FleetSubnetWasmStoreAdoptionResponse),
    BootstrapStore(RootStoreBootstrapResponse),
    FleetActivation(FleetActivationStatusResponse),
    ProvisionChild(RootComponentChildOperationStatus),
    ProvisionComponent(RootComponentOperationStatus),
    ProvisionComponents(RootComponentProvisioningStatusResponse),
    RefillCycles(IcpRefillResponse),
    RemoveComponent(RootComponentRemovalOperationStatus),
    RemoveRoot(RootRemovalOperationStatus),
    RemoveSubtree(RootComponentSubtreeRemovalResponse),
    SynchronizeRegistry(RootRegistrySynchronizationOperationStatus),
}
