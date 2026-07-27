//! Module: dto::fleet_registry
//!
//! Responsibility: carry canonical Fleet Registry snapshots and versions across boundaries.
//! Does not own: validation, canonical encoding, persistence, or lifecycle transitions.
//! Boundary: Coordinator and Fleet Subnet Root workflows validate these passive shapes.

use crate::dto::root_store::RootStoreBootstrapRequest;
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
