//! Module: dto::fleet_registry
//!
//! Responsibility: carry canonical Fleet Registry snapshots and versions across boundaries.
//! Does not own: validation, canonical encoding, persistence, or lifecycle transitions.
//! Boundary: Coordinator and Fleet Subnet Root workflows validate these passive shapes.

use crate::dto::{
    fleet_subnet_root::{FleetSubnetRootDrainingResponse, FleetSubnetRootFinalInventoryResponse},
    root_store::RootStoreBootstrapRequest,
};
use crate::ids::{
    CanisterRole, ComponentSpecAdmission, ComponentSpecId, ComponentTopologyDigest,
    FleetRegistryAuthority, FleetSubnetRootLimits, FleetSubnetRootReleaseSet, SubnetId,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

///
/// FleetSubnetRootStatus
///
/// Lifecycle state of one Fleet Subnet Root in the Fleet Registry snapshot.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetSubnetRootStatus {
    Joining,
    Active,
    Draining,
    Removed,
}

///
/// FleetComponentSpecEntry
///
/// Fleet-wide immutable Component Spec declaration projected into the Registry.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetComponentSpecEntry {
    pub component_spec: ComponentSpecId,
    pub spec_hash: [u8; 32],
    pub component_role: CanisterRole,
    pub maximum_fleet_instances: u32,
}

///
/// FleetSubnetRootEntry
///
/// One Fleet Subnet Root's immutable placement and admission facts plus lifecycle state.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootEntry {
    pub placement_subnet: SubnetId,
    pub fleet_subnet_root: Principal,
    pub component_admissions: Vec<ComponentSpecAdmission>,
    pub component_topology_digest: ComponentTopologyDigest,
    pub active_release_set: FleetSubnetRootReleaseSet,
    pub limits: FleetSubnetRootLimits,
    pub status: FleetSubnetRootStatus,
}

///
/// FleetRegistry
///
/// Complete canonical Fleet Registry snapshot distributed by one Coordinator.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetRegistry {
    pub authority: FleetRegistryAuthority,
    pub revision: u64,
    pub component_specs: Vec<FleetComponentSpecEntry>,
    pub fleet_subnet_roots: Vec<FleetSubnetRootEntry>,
}

///
/// FleetRegistryManifest
///
/// Compact current-head evidence for one complete canonical Registry snapshot.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetRegistryManifest {
    pub authority: FleetRegistryAuthority,
    pub revision: u64,
    pub byte_length: u64,
    pub content_hash: [u8; 32],
}

///
/// FleetRegistryVersion
///
/// Compact immutable identity used by mirrors, acknowledgements, and journals.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetRegistryVersion {
    pub authority: FleetRegistryAuthority,
    pub revision: u64,
    pub content_hash: [u8; 32],
}

///
/// FleetSubnetRootJoinRequest
///
/// Controller command that compare-and-commits one exact root as Registry `Joining`.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootJoinRequest {
    pub expected_registry: FleetRegistryVersion,
    pub entry: FleetSubnetRootEntry,
}

///
/// FleetSubnetRootJoinResponse
///
/// Durable response receipt for one exact root's original `Joining` commit.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootJoinResponse {
    pub entry: FleetSubnetRootEntry,
    pub version: FleetRegistryVersion,
}

///
/// FleetRegistryActivationRequest
///
/// Controller compare-and-commit command for the complete acknowledged `Joining` root set.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetRegistryActivationRequest {
    pub expected_registry: FleetRegistryVersion,
}

///
/// FleetRegistryActivationResponse
///
/// Durable response authority for one atomic all-`Active` Registry transition.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetRegistryActivationResponse {
    pub previous_version: FleetRegistryVersion,
    pub version: FleetRegistryVersion,
}

///
/// FleetSubnetRootDrainingPublicationRequest
///
/// Controller command publishing one root's exact local draining fence to the Coordinator.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDrainingPublicationRequest {
    pub expected_registry: FleetRegistryVersion,
    pub root_draining: FleetSubnetRootDrainingResponse,
}

///
/// FleetSubnetRootDrainingPublicationResponse
///
/// Durable response authority for one root's canonical `Active -> Draining` transition.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDrainingPublicationResponse {
    pub root_draining: FleetSubnetRootDrainingResponse,
    pub previous_version: FleetRegistryVersion,
    pub version: FleetRegistryVersion,
}

///
/// FleetSubnetRootRemovalPublicationRequest
///
/// Root-authenticated command publishing one exact terminal inventory to the Coordinator.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootRemovalPublicationRequest {
    pub expected_registry: FleetRegistryVersion,
    pub final_inventory: FleetSubnetRootFinalInventoryResponse,
}

///
/// FleetSubnetRootRemovalPublicationResponse
///
/// Durable response authority for one root's canonical `Draining -> Removed` transition.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootRemovalPublicationResponse {
    pub final_inventory: FleetSubnetRootFinalInventoryResponse,
    pub previous_version: FleetRegistryVersion,
    pub version: FleetRegistryVersion,
}

/// Root-authenticated command freezing its pre-transfer physical-deletion readiness authority.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDeletionReadinessIntentRequest {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub final_inventory_hash: [u8; 32],
    pub store_deletion_hash: [u8; 32],
    pub observed_cycles_before_reclamation: u128,
    pub maximum_cycles_to_retain: u128,
    pub observed_reserved_cycles: u128,
    pub observed_idle_cycles_burned_per_day: u128,
    pub observed_freezing_threshold_seconds: u128,
    pub prepared_at_ns: u64,
}

/// Coordinator receipt proving root-deletion readiness intent is durable before cycle transfer.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDeletionReadinessIntentResponse {
    pub request: FleetSubnetRootDeletionReadinessIntentRequest,
    pub coordinator: Principal,
    pub recorded_at_ns: u64,
    pub intent_hash: [u8; 32],
}

/// Root-authenticated command recording its converged post-transfer cycle balance.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDeletionReadinessRequest {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub expected_intent_hash: [u8; 32],
    pub observed_cycles_after_reclamation: u128,
    pub cycles_reclaimed_at_ns: u64,
}

/// Coordinator receipt proving one removed root is ready for an external executor.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDeletionReadinessResponse {
    pub request: FleetSubnetRootDeletionReadinessRequest,
    pub coordinator: Principal,
    pub final_inventory_hash: [u8; 32],
    pub store_deletion_hash: [u8; 32],
    pub observed_cycles_before_reclamation: u128,
    pub maximum_cycles_to_retain: u128,
    pub observed_reserved_cycles: u128,
    pub observed_idle_cycles_burned_per_day: u128,
    pub observed_freezing_threshold_seconds: u128,
    pub prepared_at_ns: u64,
    pub recorded_at_ns: u64,
    pub readiness_hash: [u8; 32],
}

/// Controller command freezing independently observed root authority before stop/delete.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDeletionExecutionRequest {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub expected_readiness_hash: [u8; 32],
    pub observed_module_hash: [u8; 32],
    pub observed_controllers: Vec<Principal>,
    pub observed_cycles_after_reclamation: u128,
    pub observed_reserved_cycles: u128,
    pub observed_idle_cycles_burned_per_day: u128,
    pub observed_freezing_threshold_seconds: u128,
}

/// Durable Coordinator intent binding one authenticated external root-deletion executor.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDeletionExecutionResponse {
    pub request: FleetSubnetRootDeletionExecutionRequest,
    pub executor: Principal,
    pub prepared_at_ns: u64,
    pub execution_hash: [u8; 32],
}

/// Controller request confirming typed root absence under one durable execution intent.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDeletionCompletionRequest {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub expected_execution_hash: [u8; 32],
    pub observed_absent_at_ns: u64,
}

/// Read-only lookup key for one durable root-deletion execution intent or receipt.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDeletionStatusRequest {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
}

/// Terminal Coordinator receipt for externally observed Fleet Subnet Root absence.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDeletionResponse {
    pub operation_id: [u8; 32],
    pub fleet_subnet_root: Principal,
    pub coordinator: Principal,
    pub executor: Principal,
    pub readiness_hash: [u8; 32],
    pub execution_hash: [u8; 32],
    pub observed_module_hash: [u8; 32],
    pub observed_controllers: Vec<Principal>,
    pub observed_cycles_after_reclamation: u128,
    pub observed_absent_at_ns: u64,
    pub completed_at_ns: u64,
    pub deletion_hash: [u8; 32],
}

///
/// FleetRegistrySnapshotResponse
///
/// Complete current Coordinator snapshot supplied only to one registered root.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetRegistrySnapshotResponse {
    pub registry: FleetRegistry,
    pub manifest: FleetRegistryManifest,
    pub version: FleetRegistryVersion,
}

///
/// FleetSubnetRootSnapshotAcknowledgementRequest
///
/// Root-authenticated acknowledgement of one exact durably staged snapshot.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootSnapshotAcknowledgementRequest {
    pub version: FleetRegistryVersion,
}

///
/// FleetSubnetRootSnapshotAcknowledgement
///
/// Durable Coordinator receipt proving which root acknowledged which version.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootSnapshotAcknowledgement {
    pub fleet_subnet_root: Principal,
    pub version: FleetRegistryVersion,
}

/// Controller command asking a Prepared root to synchronize and acknowledge its Registry.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootRegistrySyncRequest {
    pub expected_registry: FleetRegistryVersion,
    pub store_bootstrap: RootStoreBootstrapRequest,
}

/// Exact root-local candidate and Coordinator acknowledgement evidence.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootRegistrySyncResponse {
    pub fleet_subnet_root: Principal,
    pub version: FleetRegistryVersion,
    pub acknowledgement: FleetSubnetRootSnapshotAcknowledgement,
}

///
/// FleetDirectoryProvenance
///
/// Exact Registry authority and root that published one local Fleet Directory projection.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetDirectoryProvenance {
    pub registry: FleetRegistryVersion,
    pub source_fleet_subnet_root: Principal,
}

///
/// FleetSubnetRootDirectoryEntry
///
/// One root placement and lifecycle status projected from the complete Fleet Registry.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootDirectoryEntry {
    pub placement_subnet: SubnetId,
    pub fleet_subnet_root: Principal,
    pub status: FleetSubnetRootStatus,
}

///
/// FleetDirectorySnapshot
///
/// Root-local read-only discovery projection derived from one exact published Registry.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetDirectorySnapshot {
    pub provenance: FleetDirectoryProvenance,
    pub fleet_subnet_roots: Vec<FleetSubnetRootDirectoryEntry>,
}

///
/// FleetSubnetRootRegistryMirrorActivationRequest
///
/// Controller command that atomically activates a newer complete Registry mirror and Directory.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootRegistryMirrorActivationRequest {
    pub previous_registry: FleetRegistryVersion,
    pub expected_registry: FleetRegistryVersion,
    pub expected_directory: FleetDirectorySnapshot,
    pub store_bootstrap: RootStoreBootstrapRequest,
}

///
/// FleetSubnetRootRegistryMirrorActivationResponse
///
/// Exact durable evidence for one root's current Registry mirror and Fleet Directory.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetSubnetRootRegistryMirrorActivationResponse {
    pub fleet_subnet_root: Principal,
    pub previous_registry: FleetRegistryVersion,
    pub version: FleetRegistryVersion,
    pub directory: FleetDirectorySnapshot,
}
