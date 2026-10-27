//! Module: dto::fleet_coordinator
//!
//! Responsibility: carry the protected fresh-install input for one Fleet Coordinator.
//! Does not own: validation, stable state, Registry compilation, or lifecycle effects.
//! Boundary: the Coordinator lifecycle adapter passes this passive payload to workflow.

use crate::{
    cycles::Cycles,
    dto::{
        fleet_funding::{
            FleetFundingPolicyRotationReceipt, FleetRootFundingRequest, FleetRootFundingResponse,
        },
        fleet_registry::{
            FleetRegistryVersion, FleetSubnetRootDeletionExecutionResponse,
            FleetSubnetRootDeletionReadinessIntentResponse,
            FleetSubnetRootDeletionReadinessResponse, FleetSubnetRootDeletionResponse,
            FleetSubnetRootDrainingPublicationResponse, FleetSubnetRootDrainingReservationResponse,
            FleetSubnetRootRemovalPublicationResponse,
        },
    },
    ids::{FleetCoordinatorRootFundingPolicy, FleetFundingProfile, FleetSubnetRootFundingPolicy},
};
use serde::Deserialize;

/// Current spent and reserved cycles in one exact epoch-anchored funding window.
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct CoordinatorFundingWindowStatusResponse {
    pub window_start_secs: u64,
    pub spent_cycles: Cycles,
    pub reserved_cycles: Cycles,
}

/// Controller-only funding usage and operation state for one registered Root.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct CoordinatorRootFundingStatusResponse {
    pub fleet_subnet_root: ic_platform::principal::ExternalPrincipal,
    pub lifecycle_status: crate::dto::fleet_registry::FleetSubnetRootStatus,
    pub policy_hash: [u8; 32],
    pub policy: FleetSubnetRootFundingPolicy,
    pub window: CoordinatorFundingWindowStatusResponse,
    pub historical_automatic_grants: u64,
    pub historical_automatic_cycles: Cycles,
    pub automatic_grants: u32,
    pub automatic_cycles: Cycles,
    pub last_successful_grant_at_ns: Option<u64>,
    pub current_operation: Option<FleetRootFundingRequest>,
    pub last_result: Option<FleetRootFundingResponse>,
}

/// Controller-only Coordinator treasury policy, headroom and per-Root usage.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct CoordinatorFundingStatusResponse {
    pub coordinator: ic_platform::principal::ExternalPrincipal,
    pub current_cycles: Cycles,
    pub policy_generation: u64,
    pub funding_enabled: bool,
    pub funding_profile: Option<FleetFundingProfile>,
    pub policy: Option<FleetCoordinatorRootFundingPolicy>,
    pub fleet_window: Option<CoordinatorFundingWindowStatusResponse>,
    pub historical_automatic_grants: u64,
    pub historical_automatic_cycles: Cycles,
    pub automatic_grants: u32,
    pub automatic_cycles: Cycles,
    pub rotation_checkpoint_count: u32,
    pub rotation_checkpoint_root_count: u32,
    pub rotation_checkpoint_root_capacity_remaining: u32,
    pub rotation: Option<FleetFundingPolicyRotationStatusResponse>,
    pub roots: Vec<CoordinatorRootFundingStatusResponse>,
}

/// Protected durable phase of one Coordinator-owned policy rotation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub enum FleetFundingPolicyRotationStatusPhase {
    Staging {
        staged_root_count: u32,
        expected_root_count: u32,
    },
    PreparingRoots {
        prepared_root_count: u32,
        expected_root_count: u32,
    },
    ActivatingRoots {
        activated_root_count: u32,
        expected_root_count: u32,
        successor_registry: Box<FleetRegistryVersion>,
    },
    Completed(Box<FleetFundingPolicyRotationReceipt>),
}

/// Controller-only status of one exact current or terminal rotation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct FleetFundingPolicyRotationStatusResponse {
    pub operation_id: [u8; 32],
    pub plan_digest: [u8; 32],
    pub predecessor_generation: u64,
    pub successor_generation: u64,
    pub phase: FleetFundingPolicyRotationStatusPhase,
}

/// Coordinator-owned progress across the existing durable root-removal boundaries.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct CoordinatorRootRemovalOperationStatus {
    pub operation_id: [u8; 32],
    pub reservation: FleetSubnetRootDrainingReservationResponse,
    pub draining: Option<FleetSubnetRootDrainingPublicationResponse>,
    pub removal: Option<FleetSubnetRootRemovalPublicationResponse>,
    pub readiness_intent: Option<FleetSubnetRootDeletionReadinessIntentResponse>,
    pub readiness: Option<FleetSubnetRootDeletionReadinessResponse>,
    pub execution: Option<FleetSubnetRootDeletionExecutionResponse>,
    pub completion: Option<FleetSubnetRootDeletionResponse>,
}
