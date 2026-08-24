//! Module: model::fleet_admission_authority
//!
//! Responsibility: own the Coordinator's canonical Fleet-admission mutation identities and state.
//! Does not own: transport DTOs, hashing, stable encoding, caller authorization, or distribution.
//! Boundary: pure policy returns complete replacement state; ops validates hashes and persists it.

use crate::ids::{FleetAdmissionPolicy, FleetAdmissionSelector, FleetCoordinatorBinding, SubnetId};
use candid::Principal;

/// Current product schema for the sole Coordinator admission authority.
pub const FLEET_ADMISSION_AUTHORITY_SCHEMA_VERSION: u16 = 1;
/// Maximum encoded authority record admitted to memory ID 64.
pub const MAX_FLEET_ADMISSION_AUTHORITY_RECORD_BYTES: u32 = 8 * 1024 * 1024;
/// Maximum immutable admission publications retained by the canonical Registry history.
pub const MAX_FLEET_ADMISSION_PUBLICATIONS: usize = 4_096;
/// Maximum Principals returned by one protected inspection page.
pub const MAX_FLEET_ADMISSION_STATUS_PAGE: u64 = 128;

/// Closed mutation action used by the model and canonical request hashing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetAdmissionMutationActionModel {
    Add,
    Remove,
}

impl FleetAdmissionMutationActionModel {
    /// Return the frozen canonical hash discriminator.
    #[must_use]
    pub const fn hash_byte(self) -> u8 {
        match self {
            Self::Add => 0,
            Self::Remove => 1,
        }
    }
}

/// Model-owned semantic result of one accepted controller request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetAdmissionMutationOutcomeModel {
    Planned,
    Converged,
    CatalogChanged,
    AlreadyPresent,
    AlreadyAbsent,
}

/// Model-owned immutable identity of one Root participant catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionRootCatalogAuthorityModel {
    pub fleet_subnet_root: Principal,
    pub participant_catalog_digest: [u8; 32],
    pub participant_count: u32,
}

/// Complete authority-bearing mutation input after DTO conversion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionMutationRequestModel {
    pub authority: FleetCoordinatorBinding,
    pub expected_generation: u64,
    pub expected_policy_digest: [u8; 32],
    pub action: FleetAdmissionMutationActionModel,
    pub selector: FleetAdmissionSelector,
    pub principal: Principal,
    pub operation_id: [u8; 32],
    pub successor_policy_digest: [u8; 32],
    pub participant_catalog_digest: [u8; 32],
    pub participant_count: u32,
}

/// Immutable no-effect inputs that derive one operator mutation identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionMutationOperationInput {
    pub expected_generation: u64,
    pub expected_policy_digest: [u8; 32],
    pub action: FleetAdmissionMutationActionModel,
    pub selector: FleetAdmissionSelector,
    pub principal: Principal,
    pub successor_policy_digest: [u8; 32],
    pub participant_catalog_digest: [u8; 32],
    pub participant_count: u32,
}

/// Exact accepted mutation response retained for retry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionMutationResponseModel {
    pub outcome: FleetAdmissionMutationOutcomeModel,
    pub operation_id: [u8; 32],
    pub generation: u64,
    pub policy_digest: [u8; 32],
}

/// Fleet-level phase owned only by the Coordinator transition journal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetAdmissionCoordinatorTransitionPhaseModel {
    Planned,
    Preparing,
    Releasing,
    PerimeterFenced,
    Activating,
    Opening,
}

/// Monotonic aggregate progress retained for one exact registered Root.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetAdmissionCoordinatorRootPhaseModel {
    Pending,
    Reserved,
    Prepared,
    Activated,
    Open,
    Released,
}

/// Minimal Root identity and aggregate replay evidence retained by the Coordinator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionCoordinatorRootProgressModel {
    pub fleet_subnet_root: Principal,
    pub placement_subnet: SubnetId,
    pub phase: FleetAdmissionCoordinatorRootPhaseModel,
    pub participant_catalog_digest: Option<[u8; 32]>,
    pub participant_count: Option<u32>,
    pub last_receipt_hash: Option<[u8; 32]>,
}

/// One effective mutation durably planned before participant effects begin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionTransitionModel {
    pub request: FleetAdmissionMutationRequestModel,
    pub request_hash: [u8; 32],
    pub successor: FleetAdmissionPolicy,
    pub phase: FleetAdmissionCoordinatorTransitionPhaseModel,
    pub roots: Vec<FleetAdmissionCoordinatorRootProgressModel>,
}

/// One terminal idempotent or converged mutation retained for exact replay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionRetainedResultModel {
    pub request: FleetAdmissionMutationRequestModel,
    pub request_hash: [u8; 32],
    pub response: FleetAdmissionMutationResponseModel,
    pub roots: Vec<FleetAdmissionCoordinatorRootProgressModel>,
}

/// Sole Coordinator-owned Fleet-admission policy and bounded replay state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAdmissionAuthorityState {
    pub schema_version: u16,
    pub active_policy: FleetAdmissionPolicy,
    pub current_transition: Option<FleetAdmissionTransitionModel>,
    pub last_result: Option<FleetAdmissionRetainedResultModel>,
}
