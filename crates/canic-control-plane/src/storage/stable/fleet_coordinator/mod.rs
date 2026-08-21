//! Module: storage::stable::fleet_coordinator
//!
//! Responsibility: own the Fleet Coordinator's authoritative stable Registry record.
//! Does not own: Registry validation, endpoint authorization, or lifecycle orchestration.
//! Boundary: Coordinator ops may commit or export one complete validated record.

#[cfg(feature = "fleet-coordinator-canister")]
use std::cell::RefCell;

use candid::{CandidType, Principal};
#[cfg(feature = "fleet-coordinator-canister")]
use canic_core::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static, impl_storable_bounded,
    role_contract::allocation::memory::control_plane::FLEET_COORDINATOR_REGISTRY_ID,
};
use canic_core::{
    control_plane_support::config::{
        ComponentDeploymentConfiguration, ComponentGroupPlacementPolicy,
    },
    dto::{
        component_provisioning::{
            FleetComponentActivationRootProgress, FleetComponentProvisioningOperation,
            FleetComponentProvisioningPlan, RootComponentActivationEvidence,
            RootComponentActivationRequest, RootComponentDirectorySynchronizationRequest,
            RootComponentDirectorySynchronizationResponse, RootComponentProvisioningAdvanceRequest,
            RootComponentProvisioningStatusResponse, RootComponentPublicationRequest,
        },
        fleet_registry::{
            FleetRegistry, FleetRegistryActivationRequest, FleetRegistryActivationResponse,
            FleetRegistryVersion, FleetServiceBinding, FleetSubnetRootDeletionExecutionResponse,
            FleetSubnetRootDeletionReadinessIntentResponse,
            FleetSubnetRootDeletionReadinessResponse, FleetSubnetRootDeletionResponse,
            FleetSubnetRootDrainingPublicationRequest, FleetSubnetRootDrainingPublicationResponse,
            FleetSubnetRootDrainingReservationResponse, FleetSubnetRootEntry,
            FleetSubnetRootRemovalPublicationRequest, FleetSubnetRootRemovalPublicationResponse,
            FleetSubnetRootSnapshotAcknowledgement,
        },
    },
    ids::{
        AppId, ComponentDeploymentConfigurationDigest, ComponentGroupDeploymentId,
        ComponentGroupPlacementId, ComponentGroupSpecId, FleetCoordinatorRootFundingPolicy,
        FleetRegistryAuthority,
    },
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "fleet-coordinator-canister")]
// The record may contain one complete compiled deployment configuration, one
// Registry snapshot, the root-entry portion of that Registry again as
// immutable join receipts, the complete Component provisioning plan plus one
// compact acceptance and one terminal provisioning receipt per planned root,
// one compact placement record per committed Component Group deployment copy,
// one compact terminal receipt per completed scale-out placement range, at
// most one in-progress scale-out plan plus one compact acceptance per selected
// root,
// the complete service set again in at most one fresh and one scale-out
// publication receipt, one exact acknowledgement per current root, and at most
// one draining reservation, one draining receipt and one removal receipt per
// root.
const FLEET_COORDINATOR_STATE_MAX_BYTES: u32 = 33_554_432;

#[cfg(feature = "fleet-coordinator-canister")]
struct FleetCoordinatorRegistryState;

#[cfg(feature = "fleet-coordinator-canister")]
eager_static! {
    static FLEET_COORDINATOR_STATE:
        RefCell<Cell<FleetCoordinatorStateRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            canic_core::ic_memory_key!(
                authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
                key = "canic.control_plane.fleet_coordinator.registry.v1",
                ty = FleetCoordinatorRegistryState,
                id = FLEET_COORDINATOR_REGISTRY_ID
            ),
            FleetCoordinatorStateRecord::default(),
        ));
}

///
/// FleetCoordinatorRegistryRecord
///
/// Complete protected topology and current canonical Fleet Registry.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetCoordinatorRegistryRecord {
    pub configured_app: AppId,
    pub authority: FleetRegistryAuthority,
    pub component_deployment_configuration: ComponentDeploymentConfiguration,
    pub root_funding: Option<FleetCoordinatorRootFundingPolicy>,
    pub registry: FleetRegistry,
    pub root_join_receipts: Vec<FleetSubnetRootJoinReceiptRecord>,
    pub root_snapshot_acknowledgements: Vec<FleetSubnetRootSnapshotAcknowledgement>,
    pub registry_activation_receipt: Option<FleetRegistryActivationReceiptRecord>,
    pub component_provisioning: Option<FleetComponentProvisioningRecord>,
    pub component_group_deployments: Vec<FleetComponentGroupDeploymentRecord>,
    pub component_scale_out_receipts: Vec<FleetComponentScaleOutReceiptRecord>,
    pub component_scale_out: Option<FleetComponentProvisioningRecord>,
    pub service_publication_receipts: Vec<FleetServicePublicationReceiptRecord>,
    pub root_draining_reservations: Vec<FleetSubnetRootDrainingReservationRecord>,
    pub root_draining_publication_receipts: Vec<FleetSubnetRootDrainingPublicationReceiptRecord>,
    pub root_removal_publication_receipts: Vec<FleetSubnetRootRemovalPublicationReceiptRecord>,
    pub root_deletion_readiness_intents: Vec<FleetSubnetRootDeletionReadinessIntentResponse>,
    pub root_deletion_readiness_receipts: Vec<FleetSubnetRootDeletionReadinessResponse>,
    pub root_deletion_execution_intents: Vec<FleetSubnetRootDeletionExecutionResponse>,
    pub root_deletion_receipts: Vec<FleetSubnetRootDeletionResponse>,
}

#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
impl FleetCoordinatorRegistryRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "FleetCoordinatorRegistryRecord";
}

///
/// FleetSubnetRootJoinReceiptRecord
///
/// Persisted exact response authority for one root's original `Joining` commit.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootJoinReceiptRecord {
    pub entry: FleetSubnetRootEntry,
    pub version: FleetRegistryVersion,
}

/// Persisted exact response authority for the initial all-`Active` commit.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetRegistryActivationReceiptRecord {
    pub request: FleetRegistryActivationRequest,
    pub response: FleetRegistryActivationResponse,
}

/// Complete Coordinator-owned plan retained before the first root effect.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetComponentProvisioningRecord {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub plan: FleetComponentProvisioningPlan,
    pub state: FleetComponentProvisioningStateRecord,
}

///
/// FleetComponentGroupDeploymentRecord
///
/// Coordinator-owned bounded placement authority for one configured deployment.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetComponentGroupDeploymentRecord {
    pub deployment: ComponentGroupDeploymentId,
    pub component_group: ComponentGroupSpecId,
    pub configuration_digest: ComponentDeploymentConfigurationDigest,
    pub initial_placements: u32,
    pub maximum_placements: u32,
    pub placement_policy: ComponentGroupPlacementPolicy,
    pub next_placement_ordinal: u32,
    pub placements: Vec<FleetComponentGroupPlacementRecord>,
}

///
/// FleetComponentGroupPlacementRecord
///
/// Exact root and terminal receipt authority for one committed group placement.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetComponentGroupPlacementRecord {
    pub placement: ComponentGroupPlacementId,
    pub fleet_subnet_root: Principal,
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub root_receipt_content_hash: [u8; 32],
}

///
/// FleetComponentScaleOutReceiptRecord
///
/// Compact terminal replay and committed-placement authority for one retired scale-out journal.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetComponentScaleOutReceiptRecord {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub fleet_registry: FleetRegistryVersion,
    pub configuration_digest: ComponentDeploymentConfigurationDigest,
    pub operation: FleetComponentProvisioningOperation,
    pub directory_confirmation_root_count: u32,
    pub root_batch_count: u32,
    pub component_count: u32,
    pub planned_at_ns: u64,
    pub roots_accepted_at_ns: u64,
    pub components_provisioned_at_ns: u64,
    pub published_fleet_registry: FleetRegistryVersion,
    pub service_topology_published_at_ns: u64,
    pub directories_confirmed_at_ns: u64,
    pub runtimes_activated_at_ns: u64,
    pub placements: Vec<FleetComponentGroupPlacementRecord>,
    pub receipt_content_hash: [u8; 32],
}

/// Monotonic durable Coordinator provisioning state implemented in this slice.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetComponentProvisioningStateRecord {
    Planned {
        planned_at_ns: u64,
    },
    AcceptingRoots {
        planned_at_ns: u64,
        acceptances: Vec<FleetComponentProvisioningRootAcceptanceRecord>,
        in_flight: Option<FleetComponentProvisioningRootAcceptanceIntentRecord>,
    },
    RootsAccepted {
        planned_at_ns: u64,
        acceptances: Vec<FleetComponentProvisioningRootAcceptanceRecord>,
        roots_accepted_at_ns: u64,
    },
    ProvisioningRoots {
        planned_at_ns: u64,
        acceptances: Vec<FleetComponentProvisioningRootAcceptanceRecord>,
        roots_accepted_at_ns: u64,
        provisions: Vec<FleetComponentProvisioningRootProvisionRecord>,
        current: Option<Box<FleetComponentProvisioningRootProvisionRecord>>,
        in_flight: Option<FleetComponentProvisioningRootProvisionIntentRecord>,
    },
    ComponentsProvisioned {
        planned_at_ns: u64,
        acceptances: Vec<FleetComponentProvisioningRootAcceptanceRecord>,
        roots_accepted_at_ns: u64,
        provisions: Vec<FleetComponentProvisioningRootProvisionRecord>,
        components_provisioned_at_ns: u64,
    },
    ServiceTopologyPublished {
        planned_at_ns: u64,
        acceptances: Vec<FleetComponentProvisioningRootAcceptanceRecord>,
        roots_accepted_at_ns: u64,
        provisions: Vec<FleetComponentProvisioningRootProvisionRecord>,
        components_provisioned_at_ns: u64,
        published_fleet_registry: FleetRegistryVersion,
        service_topology_published_at_ns: u64,
    },
    ConfirmingDirectories {
        planned_at_ns: u64,
        acceptances: Vec<FleetComponentProvisioningRootAcceptanceRecord>,
        roots_accepted_at_ns: u64,
        provisions: Vec<FleetComponentProvisioningRootProvisionRecord>,
        components_provisioned_at_ns: u64,
        published_fleet_registry: FleetRegistryVersion,
        service_topology_published_at_ns: u64,
        confirmations: Vec<FleetComponentDirectoryConfirmationRecord>,
        current: Option<Box<FleetComponentDirectoryConfirmationRecord>>,
        in_flight: Option<Box<FleetComponentDirectoryConfirmationIntentRecord>>,
    },
    DirectoriesConfirmed {
        planned_at_ns: u64,
        acceptances: Vec<FleetComponentProvisioningRootAcceptanceRecord>,
        roots_accepted_at_ns: u64,
        provisions: Vec<FleetComponentProvisioningRootProvisionRecord>,
        components_provisioned_at_ns: u64,
        published_fleet_registry: FleetRegistryVersion,
        service_topology_published_at_ns: u64,
        confirmations: Vec<FleetComponentDirectoryConfirmationRecord>,
        directories_confirmed_at_ns: u64,
    },
    ActivatingRuntimes {
        planned_at_ns: u64,
        acceptances: Vec<FleetComponentProvisioningRootAcceptanceRecord>,
        roots_accepted_at_ns: u64,
        provisions: Vec<FleetComponentProvisioningRootProvisionRecord>,
        components_provisioned_at_ns: u64,
        published_fleet_registry: FleetRegistryVersion,
        service_topology_published_at_ns: u64,
        confirmations: Vec<FleetComponentDirectoryConfirmationRecord>,
        directories_confirmed_at_ns: u64,
        activations: Vec<FleetComponentRuntimeActivationRecord>,
        current: Option<Box<FleetComponentRuntimeActivationRecord>>,
        in_flight: Option<FleetComponentRuntimeActivationIntentRecord>,
    },
    RuntimesActivated {
        planned_at_ns: u64,
        acceptances: Vec<FleetComponentProvisioningRootAcceptanceRecord>,
        roots_accepted_at_ns: u64,
        provisions: Vec<FleetComponentProvisioningRootProvisionRecord>,
        components_provisioned_at_ns: u64,
        published_fleet_registry: FleetRegistryVersion,
        service_topology_published_at_ns: u64,
        confirmations: Vec<FleetComponentDirectoryConfirmationRecord>,
        directories_confirmed_at_ns: u64,
        activations: Vec<FleetComponentRuntimeActivationRecord>,
        runtimes_activated_at_ns: u64,
    },
}

/// Durable pre-call intent for one exact canonical root batch.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetComponentProvisioningRootAcceptanceIntentRecord {
    pub root_index: u32,
    pub fleet_subnet_root: Principal,
    pub started_at_ns: u64,
}

/// Authenticated root acceptance retained with Coordinator observation time.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetComponentProvisioningRootAcceptanceRecord {
    pub started_at_ns: u64,
    pub response: RootComponentProvisioningStatusResponse,
    pub recorded_at_ns: u64,
}

/// Durable pre-call intent for one exact root-local provisioning cursor.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetComponentProvisioningRootProvisionIntentRecord {
    pub root_index: u32,
    pub fleet_subnet_root: Principal,
    pub request: RootComponentProvisioningAdvanceRequest,
    pub started_at_ns: u64,
}

/// Latest authenticated root provisioning response retained with observation time.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetComponentProvisioningRootProvisionRecord {
    pub started_at_ns: u64,
    pub response: RootComponentProvisioningStatusResponse,
    pub recorded_at_ns: u64,
}

/// Durable pre-call intent for one exact root Directory publication cursor.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetComponentDirectoryConfirmationIntentRecord {
    FreshPublication {
        root_index: u32,
        fleet_subnet_root: Principal,
        request: RootComponentPublicationRequest,
        started_at_ns: u64,
    },
    ScaleOutSynchronization {
        root_index: u32,
        fleet_subnet_root: Principal,
        request: RootComponentDirectorySynchronizationRequest,
        started_at_ns: u64,
    },
    ScaleOutPublication {
        root_index: u32,
        fleet_subnet_root: Principal,
        request: RootComponentPublicationRequest,
        started_at_ns: u64,
    },
}

/// Latest authenticated root Directory publication response and observation time.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetComponentDirectoryConfirmationRecord {
    FreshPublication {
        started_at_ns: u64,
        response: Box<RootComponentProvisioningStatusResponse>,
        recorded_at_ns: u64,
    },
    ScaleOut {
        started_at_ns: u64,
        synchronization: Box<RootComponentDirectorySynchronizationResponse>,
        publication: Option<Box<RootComponentProvisioningStatusResponse>>,
        recorded_at_ns: u64,
    },
}

/// Durable pre-call intent for one exact root runtime-activation cursor.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetComponentRuntimeActivationIntentRecord {
    pub root_index: u32,
    pub fleet_subnet_root: Principal,
    pub request: RootComponentActivationRequest,
    pub started_at_ns: u64,
}

/// Compact authenticated root runtime-activation progress retained by the Coordinator.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetComponentRuntimeActivationRecord {
    pub started_at_ns: u64,
    pub progress: FleetComponentActivationRootProgress,
    pub activation: Option<RootComponentActivationEvidence>,
    pub activation_started_at_ns: Option<u64>,
    pub runtimes_activated_at_ns: Option<u64>,
    pub receipt_content_hash: [u8; 32],
    pub recorded_at_ns: u64,
}

/// Persisted exact authority and response for one Fleet-service publication.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetServicePublicationReceiptRecord {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub configuration_digest: ComponentDeploymentConfigurationDigest,
    pub root_receipt_content_hashes: Vec<[u8; 32]>,
    pub services: Vec<FleetServiceBinding>,
    pub previous_version: FleetRegistryVersion,
    pub version: FleetRegistryVersion,
}

/// Persisted exact Coordinator reservation for one root's Fleet-wide draining decision.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDrainingReservationRecord {
    pub response: FleetSubnetRootDrainingReservationResponse,
}

/// Persisted exact request and response for one root's published `Draining` transition.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDrainingPublicationReceiptRecord {
    pub request: FleetSubnetRootDrainingPublicationRequest,
    pub response: FleetSubnetRootDrainingPublicationResponse,
}

/// Persisted exact request and response for one root's published `Removed` transition.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootRemovalPublicationReceiptRecord {
    pub request: FleetSubnetRootRemovalPublicationRequest,
    pub response: FleetSubnetRootRemovalPublicationResponse,
}

///
/// FleetCoordinatorStateRecord
///
/// Stable optional state wrapper used before fresh Coordinator initialization.
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg(feature = "fleet-coordinator-canister")]
pub struct FleetCoordinatorStateRecord {
    pub current: Option<FleetCoordinatorRegistryRecord>,
}

#[cfg(feature = "fleet-coordinator-canister")]
impl_storable_bounded!(
    FleetCoordinatorStateRecord,
    FLEET_COORDINATOR_STATE_MAX_BYTES,
    false
);

///
/// FleetCoordinatorRegistryData
///
/// Canonical export snapshot for the Fleet Coordinator Registry allocation.
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FleetCoordinatorRegistryData {
    pub current: Option<FleetCoordinatorRegistryRecord>,
}

#[cfg(any(feature = "root-control-plane", feature = "wasm-store-canister"))]
impl FleetCoordinatorRegistryData {
    pub const STATE_CONTRACT_NAME: &'static str = "FleetCoordinatorRegistryData";
}

///
/// FleetCoordinatorCommitOutcome
///
/// Result of committing fresh genesis state to the single Coordinator cell.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(feature = "fleet-coordinator-canister")]
pub enum FleetCoordinatorCommitOutcome {
    Committed,
    Existing,
}

///
/// FleetCoordinatorCommitError
///
/// Stable-store rejection when fresh genesis conflicts with existing state.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(feature = "fleet-coordinator-canister")]
pub enum FleetCoordinatorCommitError {
    ConflictingState,
    Uninitialized,
}

///
/// FleetCoordinatorRegistryStore
///
/// Narrow stable-storage owner used only by Coordinator ops.
///

#[cfg(feature = "fleet-coordinator-canister")]
pub struct FleetCoordinatorRegistryStore;

#[cfg(feature = "fleet-coordinator-canister")]
impl FleetCoordinatorRegistryStore {
    pub(crate) fn commit_genesis(
        record: FleetCoordinatorRegistryRecord,
    ) -> Result<FleetCoordinatorCommitOutcome, FleetCoordinatorCommitError> {
        FLEET_COORDINATOR_STATE.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            match state.current.as_ref() {
                None => {
                    state.current = Some(record);
                    cell.set(state);
                    Ok(FleetCoordinatorCommitOutcome::Committed)
                }
                Some(existing) if existing == &record => {
                    Ok(FleetCoordinatorCommitOutcome::Existing)
                }
                Some(_) => Err(FleetCoordinatorCommitError::ConflictingState),
            }
        })
    }

    pub(crate) fn commit_transition(
        expected: &FleetCoordinatorRegistryRecord,
        next: FleetCoordinatorRegistryRecord,
    ) -> Result<FleetCoordinatorCommitOutcome, FleetCoordinatorCommitError> {
        FLEET_COORDINATOR_STATE.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            match state.current.as_ref() {
                None => Err(FleetCoordinatorCommitError::Uninitialized),
                Some(existing) if existing == &next => Ok(FleetCoordinatorCommitOutcome::Existing),
                Some(existing) if existing != expected => {
                    Err(FleetCoordinatorCommitError::ConflictingState)
                }
                Some(_) => {
                    state.current = Some(next);
                    cell.set(state);
                    Ok(FleetCoordinatorCommitOutcome::Committed)
                }
            }
        })
    }

    #[must_use]
    pub(crate) fn export() -> FleetCoordinatorRegistryData {
        FLEET_COORDINATOR_STATE.with_borrow(|cell| FleetCoordinatorRegistryData {
            current: cell.get().current.clone(),
        })
    }

    #[cfg(test)]
    pub(crate) fn import(data: FleetCoordinatorRegistryData) {
        FLEET_COORDINATOR_STATE.with_borrow_mut(|cell| {
            cell.set(FleetCoordinatorStateRecord {
                current: data.current,
            });
        });
    }
}
