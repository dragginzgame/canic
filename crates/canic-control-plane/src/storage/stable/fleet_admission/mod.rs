//! Module: storage::stable::fleet_admission
//!
//! Responsibility: persist the sole Coordinator-owned Fleet-admission policy and replay state.
//! Does not own: mutation decisions, transport authorization, participant distribution, or status.
//! Boundary: ops converts complete model state to and from this memory-ID-64 record.

use candid::Principal;
#[cfg(feature = "fleet-coordinator-canister")]
use canic_core::cdk::bounded_cell::BoundedCell;
#[cfg(feature = "fleet-coordinator-canister")]
use canic_core::{
    cdk::structures::{DefaultMemoryImpl, memory::RuntimeMemory},
    role_contract::allocation::memory::control_plane::FLEET_COORDINATOR_ADMISSION_ID,
};
use canic_core::{
    ids::{FleetAdmissionPolicy, FleetAdmissionSelector, FleetCoordinatorBinding, SubnetId},
    impl_storable_bounded,
    shared_support::fleet_admission_authority::MAX_FLEET_ADMISSION_AUTHORITY_RECORD_BYTES,
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "fleet-coordinator-canister")]
use std::cell::RefCell;

#[cfg(feature = "fleet-coordinator-canister")]
std::thread_local! {
    static FLEET_ADMISSION: RefCell<
        BoundedCell<Option<FleetAdmissionAuthorityRecord>, RuntimeMemory<DefaultMemoryImpl>>,
    > = RefCell::new(BoundedCell::init(canic_core::ic_memory_key!(
        authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
        key = "canic.control_plane.fleet_admission.v1",
        ty = FleetAdmissionAuthorityStore,
        id = FLEET_COORDINATOR_ADMISSION_ID,
    ), None));
}

/// Stable closed mutation action.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetAdmissionMutationActionRecord {
    Add,
    Remove,
}

/// Stable semantic outcome of one accepted request.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetAdmissionMutationOutcomeRecord {
    Planned,
    Converged,
    CatalogChanged,
    AlreadyPresent,
    AlreadyAbsent,
}

/// Complete authority-bearing request retained for exact replay.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetAdmissionMutationRequestRecord {
    pub authority: FleetCoordinatorBinding,
    pub expected_generation: u64,
    pub expected_policy_digest: [u8; 32],
    pub action: FleetAdmissionMutationActionRecord,
    pub selector: FleetAdmissionSelector,
    pub principal: Principal,
    pub operation_id: [u8; 32],
    pub successor_policy_digest: [u8; 32],
    pub participant_catalog_digest: [u8; 32],
    pub participant_count: u32,
}

/// Stable exact response retained for retry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetAdmissionMutationResponseRecord {
    pub outcome: FleetAdmissionMutationOutcomeRecord,
    pub operation_id: [u8; 32],
    pub generation: u64,
    pub policy_digest: [u8; 32],
}

/// Stable Fleet-level convergence phase.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetAdmissionCoordinatorTransitionPhaseRecord {
    Planned,
    Preparing,
    Releasing,
    PerimeterFenced,
    Activating,
    Opening,
}

/// Stable monotonic aggregate phase for one Root.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetAdmissionCoordinatorRootPhaseRecord {
    Pending,
    Reserved,
    Prepared,
    Activated,
    Open,
    Released,
}

/// Stable minimal Coordinator-owned Root progress row.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetAdmissionCoordinatorRootProgressRecord {
    pub fleet_subnet_root: Principal,
    pub placement_subnet: SubnetId,
    pub phase: FleetAdmissionCoordinatorRootPhaseRecord,
    pub participant_catalog_digest: Option<[u8; 32]>,
    pub participant_count: Option<u32>,
    pub last_receipt_hash: Option<[u8; 32]>,
}

/// One current planned successor before participant effects.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetAdmissionTransitionRecord {
    pub request: FleetAdmissionMutationRequestRecord,
    pub request_hash: [u8; 32],
    pub successor: FleetAdmissionPolicy,
    pub phase: FleetAdmissionCoordinatorTransitionPhaseRecord,
    pub roots: Vec<FleetAdmissionCoordinatorRootProgressRecord>,
}

/// One bounded terminal result retained for exact replay.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetAdmissionRetainedResultRecord {
    pub request: FleetAdmissionMutationRequestRecord,
    pub request_hash: [u8; 32],
    pub response: FleetAdmissionMutationResponseRecord,
    pub roots: Vec<FleetAdmissionCoordinatorRootProgressRecord>,
}

/// Canonical schema-1 Coordinator Fleet-admission authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetAdmissionAuthorityRecord {
    pub schema_version: u16,
    pub active_policy: FleetAdmissionPolicy,
    pub current_transition: Option<FleetAdmissionTransitionRecord>,
    pub last_result: Option<FleetAdmissionRetainedResultRecord>,
}

impl FleetAdmissionAuthorityRecord {
    #[cfg_attr(
        all(
            feature = "fleet-coordinator-canister",
            not(feature = "root-control-plane"),
            not(feature = "wasm-store-canister")
        ),
        expect(
            dead_code,
            reason = "Coordinator-only artifacts do not materialize host state-contract descriptors"
        )
    )]
    pub const STATE_CONTRACT_NAME: &'static str = "FleetAdmissionAuthorityRecord";
}

impl_storable_bounded!(
    FleetAdmissionAuthorityRecord,
    MAX_FLEET_ADMISSION_AUTHORITY_RECORD_BYTES,
    false
);

/// Test/audit snapshot of the optional Coordinator admission record.
#[cfg_attr(
    all(
        feature = "fleet-coordinator-canister",
        not(feature = "root-control-plane"),
        not(feature = "wasm-store-canister")
    ),
    expect(
        dead_code,
        reason = "Coordinator-only artifacts do not materialize host state-contract descriptors"
    )
)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FleetAdmissionAuthorityData {
    pub record: Option<FleetAdmissionAuthorityRecord>,
}

impl FleetAdmissionAuthorityData {
    #[cfg_attr(
        all(
            feature = "fleet-coordinator-canister",
            not(feature = "root-control-plane"),
            not(feature = "wasm-store-canister")
        ),
        expect(
            dead_code,
            reason = "Coordinator-only artifacts do not materialize host state-contract descriptors"
        )
    )]
    pub const STATE_CONTRACT_NAME: &'static str = "FleetAdmissionAuthorityData";
}

/// Single-record stable owner for memory ID 64.
#[cfg(feature = "fleet-coordinator-canister")]
pub struct FleetAdmissionAuthorityStore;

#[cfg(feature = "fleet-coordinator-canister")]
impl FleetAdmissionAuthorityStore {
    #[must_use]
    pub(crate) fn get() -> Option<FleetAdmissionAuthorityRecord> {
        FLEET_ADMISSION.with_borrow(|store| store.get().clone())
    }

    pub(crate) fn initialize(record: FleetAdmissionAuthorityRecord) -> bool {
        FLEET_ADMISSION.with_borrow_mut(|store| {
            if store.get().is_some() {
                return false;
            }
            store.set(Some(record));
            true
        })
    }

    pub(crate) fn replace(record: FleetAdmissionAuthorityRecord) -> bool {
        FLEET_ADMISSION.with_borrow_mut(|store| {
            if store.get().is_none() {
                return false;
            }
            store.set(Some(record));
            true
        })
    }

    pub(crate) fn compare_and_replace(
        expected: &FleetAdmissionAuthorityRecord,
        next: FleetAdmissionAuthorityRecord,
    ) -> bool {
        FLEET_ADMISSION.with_borrow_mut(|store| {
            let Some(current) = store.get().clone() else {
                return false;
            };
            if &current == expected {
                store.set(Some(next));
                true
            } else {
                current == next
            }
        })
    }
}

#[cfg(test)]
mod tests;
