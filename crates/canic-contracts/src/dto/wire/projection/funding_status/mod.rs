//! Passive funding status transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::{
    cycles::Cycles,
    dto::{
        fleet_coordinator::CoordinatorFundingWindowStatusResponse as RemoteFundingWindowStatus,
        fleet_funding::{FleetRootFundingRequest, FleetRootFundingResponse},
        fleet_registry::{FleetRegistry, FleetSubnetRootStatus},
        root::RootIcpRefillStatusResponse as RemoteRootIcpRefillStatus,
    },
    ids::{
        FleetCoordinatorRootFundingPolicy, FleetFundingProfile, FleetSubnetRootFundingPolicy,
        FleetSubnetRootIcpRefillPolicy,
    },
};
use candid::{CandidType, Principal, Reserved};
use serde::{Deserialize, Serialize};

/// Bounded RemoteCoordinatorStatusResponse wire projection for funding status.

#[derive(CandidType, Deserialize)]
pub enum RemoteCoordinatorStatusResponse {
    Funding(Box<RemoteCoordinatorFundingStatus>),
    Registry(Box<FleetRegistry>),
}

/// Bounded RemoteCoordinatorRootFundingStatus wire projection for funding status.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct RemoteCoordinatorRootFundingStatus {
    pub fleet_subnet_root: Principal,
    pub lifecycle_status: FleetSubnetRootStatus,
    pub policy_hash: [u8; 32],
    pub policy: FleetSubnetRootFundingPolicy,
    pub window: RemoteFundingWindowStatus,
    pub historical_automatic_grants: u64,
    pub historical_automatic_cycles: Cycles,
    pub automatic_grants: u32,
    pub automatic_cycles: Cycles,
    pub last_successful_grant_at_ns: Option<u64>,
    pub current_operation: Option<FleetRootFundingRequest>,
    pub last_result: Option<FleetRootFundingResponse>,
}

/// Bounded RemoteFundingPolicyRotationPhase wire projection for funding status.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum RemoteFundingPolicyRotationPhase {
    ActivatingRoots {
        activated_root_count: u32,
        expected_root_count: u32,
        successor_registry: Box<crate::dto::fleet_registry::FleetRegistryVersion>,
    },
    Completed(Reserved),
    PreparingRoots {
        prepared_root_count: u32,
        expected_root_count: u32,
    },
    Staging {
        staged_root_count: u32,
        expected_root_count: u32,
    },
}

/// Bounded RemoteFundingPolicyRotationStatus wire projection for funding status.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct RemoteFundingPolicyRotationStatus {
    pub operation_id: [u8; 32],
    pub plan_digest: [u8; 32],
    pub predecessor_generation: u64,
    pub successor_generation: u64,
    pub phase: RemoteFundingPolicyRotationPhase,
}

/// Bounded RemoteCoordinatorFundingStatus wire projection for funding status.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct RemoteCoordinatorFundingStatus {
    pub coordinator: Principal,
    pub current_cycles: Cycles,
    pub policy_generation: u64,
    pub funding_enabled: bool,
    pub funding_profile: Option<FleetFundingProfile>,
    pub policy: Option<FleetCoordinatorRootFundingPolicy>,
    pub fleet_window: Option<RemoteFundingWindowStatus>,
    pub historical_automatic_grants: u64,
    pub historical_automatic_cycles: Cycles,
    pub automatic_grants: u32,
    pub automatic_cycles: Cycles,
    pub rotation_checkpoint_count: u32,
    pub rotation_checkpoint_root_count: u32,
    pub rotation_checkpoint_root_capacity_remaining: u32,
    pub rotation: Option<RemoteFundingPolicyRotationStatus>,
    pub roots: Vec<RemoteCoordinatorRootFundingStatus>,
}

/// Bounded RemoteRootStatusResponse wire projection for funding status.
#[derive(CandidType, Deserialize)]
pub enum RemoteRootStatusResponse {
    Funding(RemoteRootFundingStatus),
}

/// Bounded RemoteRootFundingStatus wire projection for funding status.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct RemoteRootFundingStatus {
    pub fleet_subnet_root: Principal,
    pub lifecycle_status: FleetSubnetRootStatus,
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
    pub rotation_current: Option<Reserved>,
    pub rotation_last: Option<Reserved>,
    pub icp_refill_policy: Option<FleetSubnetRootIcpRefillPolicy>,
    pub icp_window_start_secs: Option<u64>,
    pub icp_window_reserved_e8s: u64,
    pub automatic_icp_refills: u32,
    pub automatic_icp_refill_e8s: u64,
    pub latest_icp_refill: Option<RemoteRootIcpRefillStatus>,
}
