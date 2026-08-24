//! Module: model::fleet_admission_root
//!
//! Responsibility: own one Root's durable admission-distribution journal invariants.
//! Does not own: DTO conversion, hashing, stable access, calls, timers, or Fleet policy choice.
//! Boundary: ops supplies exact digests and workflow commits only validated replacements.

use crate::ids::{
    FleetAdmissionPolicy, FleetCoordinatorBinding, FleetSubnetRootBinding, ManagedCanisterBinding,
};

/// Current schema for the sole Root admission-distribution journal.
pub const FLEET_ADMISSION_ROOT_SCHEMA_VERSION: u16 = 1;
/// Maximum encoded Root journal admitted to memory ID 65.
pub const MAX_FLEET_ADMISSION_ROOT_RECORD_BYTES: u32 = 8 * 1024 * 1024;
/// Maximum managed admission participants retained by one Root.
pub const MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS: usize = 4_096;
/// Maximum target progress rows returned by protected status.
pub const MAX_FLEET_ADMISSION_ROOT_STATUS_PAGE: u64 = 32;

/// Root aggregate convergence phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetAdmissionRootPhaseModel {
    Preparing,
    PerimeterFenced,
    Activating,
    Opening,
}

/// Monotonic target phase retained in the Root journal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetAdmissionRootParticipantPhaseModel {
    Pending,
    Prepared,
    Activated,
    Open,
}

/// One exact target binding and derived successor evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionRootParticipantModel {
    pub target: ManagedCanisterBinding,
    pub projection_digest: [u8; 32],
    pub phase: FleetAdmissionRootParticipantPhaseModel,
    pub last_receipt_hash: Option<[u8; 32]>,
}

/// Complete initial Coordinator-authored Root transition identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionRootPrepareRequestModel {
    pub authority: FleetCoordinatorBinding,
    pub root: FleetSubnetRootBinding,
    pub operation_id: [u8; 32],
    pub expected_generation: u64,
    pub expected_policy_digest: [u8; 32],
    pub successor: FleetAdmissionPolicy,
    pub request_hash: [u8; 32],
}

/// One Root-owned in-flight subtree transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionRootTransitionModel {
    pub request: FleetAdmissionRootPrepareRequestModel,
    pub phase: FleetAdmissionRootPhaseModel,
    pub participant_catalog_digest: [u8; 32],
    pub participants: Vec<FleetAdmissionRootParticipantModel>,
    pub fence_request_hash: Option<[u8; 32]>,
    pub prepare_receipt_hash: Option<[u8; 32]>,
    pub activate_request_hash: Option<[u8; 32]>,
    pub activate_receipt_hash: Option<[u8; 32]>,
    pub open_request_hash: Option<[u8; 32]>,
}

/// Pre-effect catalog reservation released after a stale plan is detected.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionRootReleasedReservationModel {
    pub request: FleetAdmissionRootPrepareRequestModel,
    pub participant_catalog_digest: [u8; 32],
    pub participant_count: u32,
    pub release_request_hash: [u8; 32],
    pub receipt_hash: [u8; 32],
}

/// Terminal operation retained with complete participant history for exact replay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionRootRetainedResultModel {
    pub request: FleetAdmissionRootPrepareRequestModel,
    pub participant_catalog_digest: [u8; 32],
    pub participants: Vec<FleetAdmissionRootParticipantModel>,
    pub fence_request_hash: [u8; 32],
    pub prepare_receipt_hash: [u8; 32],
    pub activate_request_hash: [u8; 32],
    pub activate_receipt_hash: [u8; 32],
    pub open_request_hash: [u8; 32],
    pub receipt_hash: [u8; 32],
}

/// Sole Root-owned policy-distribution state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionRootState {
    pub schema_version: u16,
    pub active_policy: FleetAdmissionPolicy,
    pub current_transition: Option<FleetAdmissionRootTransitionModel>,
    pub last_result: Option<FleetAdmissionRootRetainedResultModel>,
    pub last_release: Option<FleetAdmissionRootReleasedReservationModel>,
}
