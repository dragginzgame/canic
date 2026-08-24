//! Module: storage::stable::fleet_admission
//!
//! Responsibility: persist the sole Coordinator-owned Fleet-admission policy and replay state.
//! Does not own: mutation decisions, transport authorization, participant distribution, or status.
//! Boundary: ops converts complete model state to and from this memory-ID-64 record.

use candid::Principal;
#[cfg(feature = "fleet-coordinator-canister")]
use canic_core::{
    cdk::structures::{
        DefaultMemoryImpl, btreemap::BTreeMap as StableBtreeMap, memory::VirtualMemory,
    },
    eager_static,
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
const FLEET_ADMISSION_RECORD_KEY: u8 = 0;

#[cfg(feature = "fleet-coordinator-canister")]
eager_static! {
    static FLEET_ADMISSION: RefCell<
        StableBtreeMap<u8, FleetAdmissionAuthorityRecord, VirtualMemory<DefaultMemoryImpl>>,
    > = RefCell::new(StableBtreeMap::init(canic_core::ic_memory_key!(
        authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
        key = "canic.control_plane.fleet_admission.v1",
        ty = FleetAdmissionAuthorityStore,
        id = FLEET_COORDINATOR_ADMISSION_ID,
    )));
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
        FLEET_ADMISSION.with_borrow(|store| store.get(&FLEET_ADMISSION_RECORD_KEY))
    }

    pub(crate) fn initialize(record: FleetAdmissionAuthorityRecord) -> bool {
        FLEET_ADMISSION.with_borrow_mut(|store| {
            if store.get(&FLEET_ADMISSION_RECORD_KEY).is_some() {
                return false;
            }
            store.insert(FLEET_ADMISSION_RECORD_KEY, record);
            true
        })
    }

    pub(crate) fn replace(record: FleetAdmissionAuthorityRecord) -> bool {
        FLEET_ADMISSION.with_borrow_mut(|store| {
            if store.get(&FLEET_ADMISSION_RECORD_KEY).is_none() {
                return false;
            }
            store.insert(FLEET_ADMISSION_RECORD_KEY, record);
            true
        })
    }

    pub(crate) fn compare_and_replace(
        expected: &FleetAdmissionAuthorityRecord,
        next: FleetAdmissionAuthorityRecord,
    ) -> bool {
        FLEET_ADMISSION.with_borrow_mut(|store| {
            let Some(current) = store.get(&FLEET_ADMISSION_RECORD_KEY) else {
                return false;
            };
            if &current == expected {
                store.insert(FLEET_ADMISSION_RECORD_KEY, next);
                true
            } else {
                current == next
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::{
        cdk::structures::storable::Storable,
        ids::{
            AppId, CanonicalNetworkId, FleetAdmissionRule, FleetBinding, FleetId, FleetKey,
            SubnetId,
        },
        shared_support::fleet_admission_policy::compile_installed_fleet_admission_policy,
    };

    #[test]
    fn maximum_b3_current_plus_last_authority_fits_memory_id_64() {
        let fleet = FleetBinding {
            fleet: FleetKey {
                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                fleet_id: FleetId::from_generated_bytes([u8::MAX; 32]),
            },
            app: AppId::from("a".repeat(40)),
        };
        let fleet_principals = (1..=256).map(principal).collect::<Vec<_>>();
        let rules = (0..32)
            .map(|index| FleetAdmissionRule {
                selector: FleetAdmissionSelector::ComponentSpec(
                    format!("s{index:02}").parse().expect("Component Spec ID"),
                ),
                principals: fleet_principals[(index * 4)..(index * 4 + 4)].to_vec(),
            })
            .collect::<Vec<_>>();
        let active = compile_installed_fleet_admission_policy(
            fleet.clone(),
            u64::MAX - 1,
            fleet_principals.clone(),
            rules.clone(),
        )
        .expect("maximum active policy");
        let successor = compile_installed_fleet_admission_policy(
            fleet.clone(),
            u64::MAX,
            fleet_principals,
            rules,
        )
        .expect("maximum successor policy");
        let authority = FleetCoordinatorBinding {
            fleet,
            coordinator_subnet: SubnetId::from_principal(principal(300)),
            coordinator: principal(301),
        };
        let roots = (1..=4_096)
            .map(|index| FleetAdmissionCoordinatorRootProgressRecord {
                fleet_subnet_root: principal(index),
                placement_subnet: SubnetId::from_principal(principal(index + 4_096)),
                phase: FleetAdmissionCoordinatorRootPhaseRecord::Open,
                participant_catalog_digest: Some([0xf7; 32]),
                participant_count: Some(1),
                last_receipt_hash: Some([0xf9; 32]),
            })
            .collect::<Vec<_>>();
        let request = FleetAdmissionMutationRequestRecord {
            authority,
            expected_generation: u64::MAX - 1,
            expected_policy_digest: active.policy_digest,
            action: FleetAdmissionMutationActionRecord::Add,
            selector: FleetAdmissionSelector::Fleet,
            principal: principal(302),
            operation_id: [0xfe; 32],
            successor_policy_digest: successor.policy_digest,
            participant_catalog_digest: [0xf6; 32],
            participant_count: 4_096,
        };
        let record = FleetAdmissionAuthorityRecord {
            schema_version: 1,
            active_policy: active,
            current_transition: Some(FleetAdmissionTransitionRecord {
                request: request.clone(),
                request_hash: [0xfd; 32],
                successor,
                phase: FleetAdmissionCoordinatorTransitionPhaseRecord::Opening,
                roots: roots.clone(),
            }),
            last_result: Some(FleetAdmissionRetainedResultRecord {
                request,
                request_hash: [0xfc; 32],
                response: FleetAdmissionMutationResponseRecord {
                    outcome: FleetAdmissionMutationOutcomeRecord::Converged,
                    operation_id: [0xfe; 32],
                    generation: u64::MAX - 1,
                    policy_digest: [0xfb; 32],
                },
                roots,
            }),
        };

        let bytes = record.to_bytes();
        eprintln!(
            "maximum Coordinator admission current-plus-last encoded bytes: {}",
            bytes.len()
        );
        assert!(bytes.len() <= MAX_FLEET_ADMISSION_AUTHORITY_RECORD_BYTES as usize);
    }

    fn principal(index: u16) -> Principal {
        Principal::from_slice(&index.to_be_bytes())
    }
}
