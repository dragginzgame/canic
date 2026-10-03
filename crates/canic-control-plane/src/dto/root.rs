//! Module: dto::root
//!
//! Responsibility: carry Fleet Subnet Root operation-detail projections.
//! Does not own: profile-pruned status envelopes, durable state, advancement, or policy.
//! Boundary: the destination macro composes these durable details into its exact status union.

use candid::CandidType;
use canic_core::{
    cdk::types::{Cycles, Principal},
    dto::{
        component_provisioning::RootComponentProvisioningStatusResponse,
        component_registry::{
            RootComponentAllocationResponse, RootComponentChildAllocationResponse,
            RootComponentDeletionResponse, RootComponentDrainingResponse,
            RootComponentSubtreeRemovalResponse,
        },
        fleet_activation::FleetActivationStatusResponse,
        fleet_funding::{
            FleetFundingPolicyRotationRootReceipt, FleetRootFundingRequest,
            FleetRootFundingResponse,
        },
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
        icp_refill::{IcpRefillResponse, IcpRefillTrigger},
        root_store::RootStoreBootstrapResponse,
    },
    ids::{FleetFundingProfile, FleetSubnetRootFundingPolicy, FleetSubnetRootIcpRefillPolicy},
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

/// Latest Root-local ICP refill outcome with its durable manual/automatic owner.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct RootIcpRefillStatusResponse {
    pub trigger: IcpRefillTrigger,
    pub amount_e8s: u64,
    pub fee_e8s: u64,
    pub budget_window_start_secs: u64,
    pub resumable: bool,
    pub response: IcpRefillResponse,
}

/// One exact retained ICP-refill effect and its source account for release review.
/// Retry exhaustion is evidence, never a settlement receipt.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct RootIcpRefillReleaseEvidence {
    pub transfer_uncertain: bool,
    pub record_id: u64,
    pub trigger: IcpRefillTrigger,
    pub policy_hash: [u8; 32],
    pub source_canister: Principal,
    pub source_subaccount: Option<[u8; 32]>,
    pub target_canister: Principal,
    pub ledger_canister_id: Principal,
    pub cmc_canister_id: Principal,
    pub cmc_to_account_owner: Principal,
    pub cmc_to_account_subaccount: Option<[u8; 32]>,
    pub amount_e8s: u64,
    pub fee_e8s: u64,
    pub budget_window_start_secs: u64,
    pub budget_reserved: bool,
    pub memo: Vec<u8>,
    pub created_at_time_ns: u64,
    pub notify_attempts: u32,
    pub response: IcpRefillResponse,
    pub refund_block_index: Option<u64>,
    pub transaction_too_old_min_block_index: Option<u64>,
}

/// Controller-only funding census; does not seal producers or authorize reset.
/// Pages include all retained refill outcomes and historical source accounts.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct RootFundingReleaseResponse {
    pub fleet_subnet_root: Principal,
    pub policy_generation: u64,
    pub policy_hash: [u8; 32],
    pub icp_refill_policy: Option<FleetSubnetRootIcpRefillPolicy>,
    pub current_request: Option<FleetRootFundingRequest>,
    pub accepted_grant: Option<canic_core::dto::fleet_funding::FleetRootFundingAcceptanceReceipt>,
    pub rotation_current:
        Option<canic_core::dto::fleet_funding::FleetFundingPolicyRotationRootPrepareRequest>,
    pub icp_refills: Vec<RootIcpRefillReleaseEvidence>,
    pub next_after: Option<u64>,
}

/// Controller-only Root operating-funding and emergency-refill projection.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct RootFundingStatusResponse {
    pub fleet_subnet_root: Principal,
    pub lifecycle_status: canic_core::dto::fleet_registry::FleetSubnetRootStatus,
    pub funding_eligible: bool,
    pub cycles_funding_enabled: bool,
    pub current_cycles: Cycles,
    pub policy_generation: u64,
    pub funding_profile: FleetFundingProfile,
    pub policy_hash: [u8; 32],
    pub root_policy: FleetSubnetRootFundingPolicy,
    pub current_operation: Option<FleetRootFundingRequest>,
    pub last_result: Option<FleetRootFundingResponse>,
    pub historical_automatic_grants: u64,
    pub historical_automatic_cycles: Cycles,
    pub automatic_grants: u32,
    pub automatic_cycles: Cycles,
    pub rotation_current: Option<FleetFundingPolicyRotationRootReceipt>,
    pub rotation_last: Option<FleetFundingPolicyRotationRootReceipt>,
    pub icp_refill_policy: Option<FleetSubnetRootIcpRefillPolicy>,
    pub icp_window_start_secs: Option<u64>,
    pub icp_window_reserved_e8s: u64,
    pub automatic_icp_refills: u32,
    pub automatic_icp_refill_e8s: u64,
    pub latest_icp_refill: Option<RootIcpRefillStatusResponse>,
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

/// Retained pool obligations, independent of admission for new import or maintenance work.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct RootPoolReleaseResponse {
    pub root: Principal,
    pub bootstrap: Option<RootPoolBootstrapReleaseEvidence>,
    pub capacity_import: Option<canic_core::dto::pool_import::PoolImportStatus>,
    pub creation: Option<canic_core::dto::pool::CanisterPoolCreation>,
    pub handoff: Option<canic_core::dto::pool::CanisterPoolHandoff>,
}

/// Keep all supplied identities and the initialization hold until custody is accounted for.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct RootPoolBootstrapReleaseEvidence {
    pub review_sha256: [u8; 32],
    pub install_id: [u8; 32],
    pub operator: Principal,
    pub store: Principal,
    pub sources: Vec<Principal>,
}
