//! Module: storage::stable::root_admission
//!
//! Responsibility: persist one Root's bounded admission-distribution journal.
//! Does not own: participant discovery, transition decisions, calls, timers, or status.
//! Boundary: Root admission ops convert complete model state to and from memory ID 65.

use canic_core::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static,
    ids::{
        FleetAdmissionPolicy, FleetCoordinatorBinding, FleetSubnetRootBinding,
        ManagedCanisterBinding,
    },
    impl_storable_bounded,
    role_contract::allocation::memory::control_plane::ROOT_ADMISSION_ID,
    shared_support::fleet_admission_root::MAX_FLEET_ADMISSION_ROOT_RECORD_BYTES,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

struct RootAdmissionState;

eager_static! {
    static ROOT_ADMISSION_STATE:
        RefCell<Cell<RootAdmissionStateRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            canic_core::ic_memory_key!(
                authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
                key = "canic.control_plane.root.admission.v1",
                ty = RootAdmissionState,
                id = ROOT_ADMISSION_ID
            ),
            RootAdmissionStateRecord::default(),
        ));
}

/// Stable Root aggregate convergence phase.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootAdmissionPhaseRecord {
    Preparing,
    PerimeterFenced,
    Activating,
    Opening,
}

/// Stable monotonic target phase.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootAdmissionParticipantPhaseRecord {
    Pending,
    Prepared,
    Activated,
    Open,
}

/// Stable target identity and successor evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootAdmissionParticipantRecord {
    pub target: ManagedCanisterBinding,
    pub projection_digest: [u8; 32],
    pub phase: RootAdmissionParticipantPhaseRecord,
    pub last_receipt_hash: Option<[u8; 32]>,
}

/// Stable complete initial Root transition request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootAdmissionPrepareRequestRecord {
    pub authority: FleetCoordinatorBinding,
    pub root: FleetSubnetRootBinding,
    pub operation_id: [u8; 32],
    pub expected_generation: u64,
    pub expected_policy_digest: [u8; 32],
    pub successor: FleetAdmissionPolicy,
    pub request_hash: [u8; 32],
}

/// Stable current Root convergence operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootAdmissionTransitionRecord {
    pub request: RootAdmissionPrepareRequestRecord,
    pub phase: RootAdmissionPhaseRecord,
    pub participant_catalog_digest: [u8; 32],
    pub participants: Vec<RootAdmissionParticipantRecord>,
    pub fence_request_hash: Option<[u8; 32]>,
    pub prepare_receipt_hash: Option<[u8; 32]>,
    pub activate_request_hash: Option<[u8; 32]>,
    pub activate_receipt_hash: Option<[u8; 32]>,
    pub open_request_hash: Option<[u8; 32]>,
}

/// Stable pre-effect catalog reservation released after stale-plan detection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootAdmissionReleasedReservationRecord {
    pub request: RootAdmissionPrepareRequestRecord,
    pub participant_catalog_digest: [u8; 32],
    pub participant_count: u32,
    pub release_request_hash: [u8; 32],
    pub receipt_hash: [u8; 32],
}

/// Stable last terminal operation and complete target history.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootAdmissionRetainedResultRecord {
    pub request: RootAdmissionPrepareRequestRecord,
    pub participant_catalog_digest: [u8; 32],
    pub participants: Vec<RootAdmissionParticipantRecord>,
    pub fence_request_hash: [u8; 32],
    pub prepare_receipt_hash: [u8; 32],
    pub activate_request_hash: [u8; 32],
    pub activate_receipt_hash: [u8; 32],
    pub open_request_hash: [u8; 32],
    pub receipt_hash: [u8; 32],
}

/// Complete memory-ID-65 Root admission journal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootAdmissionRecord {
    pub schema_version: u16,
    pub active_policy: FleetAdmissionPolicy,
    pub current_transition: Option<RootAdmissionTransitionRecord>,
    pub last_result: Option<RootAdmissionRetainedResultRecord>,
    pub last_release: Option<RootAdmissionReleasedReservationRecord>,
}

impl RootAdmissionRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "RootAdmissionRecord";
}

/// Optional fresh-install wrapper for the lazily initialized Root journal.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootAdmissionStateRecord {
    pub current: Option<RootAdmissionRecord>,
}

impl_storable_bounded!(
    RootAdmissionStateRecord,
    MAX_FLEET_ADMISSION_ROOT_RECORD_BYTES,
    false
);

/// Canonical snapshot projection for memory ID 65.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RootAdmissionData {
    pub current: Option<RootAdmissionRecord>,
}

impl RootAdmissionData {
    pub const STATE_CONTRACT_NAME: &'static str = "RootAdmissionData";
}

/// Compare-and-commit outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootAdmissionCommitOutcome {
    Committed,
    Existing,
}

/// Compare-and-commit rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootAdmissionCommitError {
    ConflictingState,
    Uninitialized,
}

/// Narrow stable owner for the sole Root admission journal.
pub struct RootAdmissionStore;

impl RootAdmissionStore {
    #[must_use]
    pub(crate) fn export() -> RootAdmissionData {
        ROOT_ADMISSION_STATE.with_borrow(|cell| RootAdmissionData {
            current: cell.get().current.clone(),
        })
    }

    #[cfg(test)]
    pub(crate) fn import(data: RootAdmissionData) {
        ROOT_ADMISSION_STATE.with_borrow_mut(|cell| {
            cell.set(RootAdmissionStateRecord {
                current: data.current,
            });
        });
    }

    pub(crate) fn commit_genesis(
        record: RootAdmissionRecord,
    ) -> Result<RootAdmissionCommitOutcome, RootAdmissionCommitError> {
        ROOT_ADMISSION_STATE.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            match state.current.as_ref() {
                None => {
                    state.current = Some(record);
                    cell.set(state);
                    Ok(RootAdmissionCommitOutcome::Committed)
                }
                Some(existing) if existing == &record => Ok(RootAdmissionCommitOutcome::Existing),
                Some(_) => Err(RootAdmissionCommitError::ConflictingState),
            }
        })
    }

    pub(crate) fn commit_transition(
        expected: &RootAdmissionRecord,
        next: RootAdmissionRecord,
    ) -> Result<RootAdmissionCommitOutcome, RootAdmissionCommitError> {
        ROOT_ADMISSION_STATE.with_borrow_mut(|cell| {
            let mut state = cell.get().clone();
            match state.current.as_ref() {
                None => Err(RootAdmissionCommitError::Uninitialized),
                Some(existing) if existing == &next => Ok(RootAdmissionCommitOutcome::Existing),
                Some(existing) if existing != expected => {
                    Err(RootAdmissionCommitError::ConflictingState)
                }
                Some(_) => {
                    state.current = Some(next);
                    cell.set(state);
                    Ok(RootAdmissionCommitOutcome::Committed)
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::{
        cdk::{structures::storable::Storable, types::Cycles},
        ids::{
            AppId, CanisterRole, CanonicalNetworkId, ComponentBinding, ComponentInstanceId,
            ComponentSpecAdmission, ComponentSpecId, ComponentTopologyDigest, CyclesFundingBudget,
            FleetAdmissionRule, FleetAdmissionSelector, FleetBinding, FleetId, FleetKey,
            FleetRegistryAuthority, FleetSubnetCanisterPoolConfig, FleetSubnetRootLimits, SubnetId,
        },
        shared_support::fleet_admission_policy::compile_installed_fleet_admission_policy,
    };

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one production-codec fixture materializes every maximum Root journal field"
    )]
    fn maximum_current_plus_last_root_journal_fits_memory_id_65() {
        let fleet = FleetBinding {
            fleet: FleetKey {
                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                fleet_id: FleetId::from_generated_bytes([0xe1; 32]),
            },
            app: AppId::from("a".repeat(40)),
        };
        let authority = FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: fleet.clone(),
                coordinator_subnet: SubnetId::from_principal(principal(1)),
                coordinator: principal(2),
            },
            epoch: u64::MAX,
        };
        let component_spec = ComponentSpecId::try_from("s".repeat(40)).expect("Component Spec ID");
        let root = FleetSubnetRootBinding {
            authority: authority.clone(),
            placement_subnet: SubnetId::from_principal(principal(3)),
            fleet_subnet_root: principal(4),
            component_admissions: vec![ComponentSpecAdmission {
                component_spec: component_spec.clone(),
                spec_hash: [0xe2; 32],
                maximum_root_instances: 4_096,
            }],
            component_topology_digest: ComponentTopologyDigest::from_bytes([0xe3; 32]),
            limits: FleetSubnetRootLimits {
                maximum_component_instances: 4_096,
                maximum_registry_bytes: u64::MAX,
                maximum_wasm_store_bytes: u64::MAX,
                canister_pool: FleetSubnetCanisterPoolConfig {
                    minimum_size: u32::MAX,
                    maximum_size: u32::MAX,
                    canister_cycles: Cycles::new(u128::MAX),
                },
                cycles_funding: CyclesFundingBudget {
                    window_secs: u64::MAX,
                    maximum_cycles: Cycles::new(u128::MAX),
                },
                maximum_group_placements: u32::MAX,
            },
            funding: crate::test_support::fleet_subnet_root_funding_authority(),
        };
        let fleet_principals = (0..256)
            .map(|index| principal(index + 10))
            .collect::<Vec<_>>();
        let rules = (0..32)
            .map(|index| FleetAdmissionRule {
                selector: FleetAdmissionSelector::ComponentSpec(
                    format!("r{index:02}").parse().expect("Component Spec ID"),
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
        let successor =
            compile_installed_fleet_admission_policy(fleet, u64::MAX, fleet_principals, rules)
                .expect("maximum successor policy");
        let participants = (0..4_096)
            .map(|index| RootAdmissionParticipantRecord {
                target: ManagedCanisterBinding::Component(ComponentBinding {
                    authority: authority.clone(),
                    component: ComponentInstanceId::from_generated_bytes(
                        u32::try_from(index)
                            .expect("bounded index")
                            .to_be_bytes()
                            .repeat(8)
                            .try_into()
                            .expect("32-byte Component ID"),
                    ),
                    component_spec: component_spec.clone(),
                    spec_hash: [0xe2; 32],
                    role: CanisterRole::from("r".repeat(40)),
                    placement_subnet: root.placement_subnet,
                    fleet_subnet_root: root.fleet_subnet_root,
                    canister_id: principal(index + 1_000),
                }),
                projection_digest: [0xe4; 32],
                phase: RootAdmissionParticipantPhaseRecord::Activated,
                last_receipt_hash: Some([0xe5; 32]),
            })
            .collect::<Vec<_>>();
        let request = RootAdmissionPrepareRequestRecord {
            authority: authority.binding,
            root,
            operation_id: [0xe6; 32],
            expected_generation: active.generation,
            expected_policy_digest: active.policy_digest,
            successor,
            request_hash: [0xe7; 32],
        };
        let mut terminal_participants = participants.clone();
        for participant in &mut terminal_participants {
            participant.phase = RootAdmissionParticipantPhaseRecord::Open;
        }
        let state = RootAdmissionStateRecord {
            current: Some(RootAdmissionRecord {
                schema_version: 1,
                active_policy: active,
                current_transition: Some(RootAdmissionTransitionRecord {
                    request: request.clone(),
                    phase: RootAdmissionPhaseRecord::Opening,
                    participant_catalog_digest: [0xe8; 32],
                    participants,
                    fence_request_hash: Some([0xe9; 32]),
                    prepare_receipt_hash: Some([0xea; 32]),
                    activate_request_hash: Some([0xeb; 32]),
                    activate_receipt_hash: Some([0xec; 32]),
                    open_request_hash: Some([0xed; 32]),
                }),
                last_result: Some(RootAdmissionRetainedResultRecord {
                    request,
                    participant_catalog_digest: [0xee; 32],
                    participants: terminal_participants,
                    fence_request_hash: [0xef; 32],
                    prepare_receipt_hash: [0xf0; 32],
                    activate_request_hash: [0xf1; 32],
                    activate_receipt_hash: [0xf2; 32],
                    open_request_hash: [0xf3; 32],
                    receipt_hash: [0xf4; 32],
                }),
                last_release: None,
            }),
        };

        let bytes = state.to_bytes();
        eprintln!(
            "maximum Root admission current-plus-last encoded bytes: {}",
            bytes.len()
        );
        assert!(bytes.len() <= MAX_FLEET_ADMISSION_ROOT_RECORD_BYTES as usize);
    }

    fn principal(index: usize) -> candid::Principal {
        let mut bytes = [0_u8; 29];
        bytes[..8].copy_from_slice(
            &u64::try_from(index)
                .expect("bounded fixture index")
                .to_be_bytes(),
        );
        bytes[8..].fill(u8::try_from(index % 251).expect("bounded fixture byte"));
        candid::Principal::from_slice(&bytes)
    }
}
