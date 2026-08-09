//! Module: storage::stable::component_provisioning
//!
//! Responsibility: retain root-local aggregate Component Group provisioning authority.
//! Does not own: caller authentication, plan validation, Component effects, or publication.
//! Boundary: ops commits only a completely validated exact batch and reads it through typed keys.

use canic_core::{
    cdk::structures::{
        DefaultMemoryImpl, btreemap::BTreeMap as StableBtreeMap, cell::Cell, memory::VirtualMemory,
    },
    dto::{
        component_deployment::{ComponentDeploymentLimits, ComponentDeploymentPurpose},
        component_provisioning::{
            FleetSubnetRootProvisioningBatch, RootComponentActivationEvidence,
            RootComponentPublicationEvidence,
        },
        component_registry::ComponentRegistryHead,
        fleet_registry::FleetRegistryVersion,
    },
    eager_static,
    ids::{
        ComponentBinding, ComponentDeploymentConfigurationDigest, ComponentGroupDeploymentId,
        ComponentGroupMemberPath, ComponentGroupPlacementId, ComponentGroupSpecId,
        ComponentInstanceId, ComponentSpecId,
    },
    impl_storable_bounded,
    role_contract::allocation::memory::control_plane::{
        ROOT_COMPONENT_PROVISIONING_OPERATIONS_ID, ROOT_COMPONENT_PROVISIONING_PLACEMENTS_ID,
        ROOT_COMPONENT_PROVISIONING_STATE_ID,
    },
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

const ROOT_COMPONENT_PROVISIONING_OPERATION_MAX_BYTES: u32 = 8_650_000;
// CBOR encodes a 32-byte `[u8; 32]` key as a two-byte array header followed by
// at most two bytes per element.
const ROOT_COMPONENT_PROVISIONING_OPERATION_KEY_MAX_BYTES: u32 = 96;
// The placement owner adds one CBOR map header and the exact two field names to
// two maximum-width operation keys.
const ROOT_COMPONENT_PROVISIONING_PLACEMENT_RECORD_MAX_BYTES: u32 = 156;

struct RootComponentProvisioningOperations;
struct RootComponentProvisioningPlacements;
struct RootComponentProvisioningState;

eager_static! {
    static ROOT_COMPONENT_PROVISIONING_OPERATIONS: RefCell<
        StableBtreeMap<
            RootComponentOperationKey,
            RootComponentOperationRecord,
            VirtualMemory<DefaultMemoryImpl>,
        >,
    > = RefCell::new(StableBtreeMap::init(
        canic_core::ic_memory_key!(
            authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
            key = "canic.control_plane.root.component_provisioning.operations.v1",
            ty = RootComponentProvisioningOperations,
            id = ROOT_COMPONENT_PROVISIONING_OPERATIONS_ID
        ),
    ));
}

eager_static! {
    static ROOT_COMPONENT_PROVISIONING_PLACEMENTS: RefCell<
        StableBtreeMap<
            RootComponentProvisioningPlacementKey,
            RootComponentProvisioningPlacementRecord,
            VirtualMemory<DefaultMemoryImpl>,
        >,
    > = RefCell::new(StableBtreeMap::init(
        canic_core::ic_memory_key!(
            authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
            key = "canic.control_plane.root.component_provisioning.placements.v1",
            ty = RootComponentProvisioningPlacements,
            id = ROOT_COMPONENT_PROVISIONING_PLACEMENTS_ID
        ),
    ));
}

eager_static! {
    static ROOT_COMPONENT_PROVISIONING_STATE: RefCell<
        Cell<RootComponentProvisioningStateRecord, VirtualMemory<DefaultMemoryImpl>>,
    > = RefCell::new(Cell::init(
        canic_core::ic_memory_key!(
            authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
            key = "canic.control_plane.root.component_provisioning.state.v1",
            ty = RootComponentProvisioningState,
            id = ROOT_COMPONENT_PROVISIONING_STATE_ID
        ),
        RootComponentProvisioningStateRecord::default(),
    ));
}

/// Stable operation-map key separating batch and Directory-confirmation authority.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum RootComponentOperationKey {
    Provisioning([u8; 32]),
    DirectorySynchronization([u8; 32]),
}

impl_storable_bounded!(
    RootComponentOperationKey,
    ROOT_COMPONENT_PROVISIONING_OPERATION_KEY_MAX_BYTES,
    false
);

/// Stable operation-map value sharing one grouped control-plane allocation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootComponentOperationRecord {
    Provisioning(Box<RootComponentProvisioningRecord>),
    DirectorySynchronization(Box<RootComponentDirectorySynchronizationRecord>),
}

impl RootComponentOperationRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "RootComponentOperationRecord";
}

impl_storable_bounded!(
    RootComponentOperationRecord,
    ROOT_COMPONENT_PROVISIONING_OPERATION_MAX_BYTES,
    false
);

/// Stable placement-index key.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RootComponentProvisioningPlacementKey {
    pub deployment: ComponentGroupDeploymentId,
    pub ordinal: u32,
}

impl From<&ComponentGroupPlacementId> for RootComponentProvisioningPlacementKey {
    fn from(placement: &ComponentGroupPlacementId) -> Self {
        Self {
            deployment: placement.deployment.clone(),
            ordinal: placement.ordinal,
        }
    }
}

impl_storable_bounded!(RootComponentProvisioningPlacementKey, 256, false);

/// Durable aggregate root operation and immutable accepted batch.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentProvisioningRecord {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub fleet_registry: FleetRegistryVersion,
    pub configuration_digest: ComponentDeploymentConfigurationDigest,
    pub batch: FleetSubnetRootProvisioningBatch,
    pub runtime_mode: RootComponentProvisioningRuntimeModeRecord,
    pub state: RootComponentProvisioningStateRecordPhase,
}

/// Protected root runtime state observed before one batch was accepted.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootComponentProvisioningRuntimeModeRecord {
    FreshRoot,
    ActiveRoot,
}

/// Durable root-local scale-out Directory synchronization authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentDirectorySynchronizationRecord {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub source_fleet_registry: FleetRegistryVersion,
    pub published_fleet_registry: FleetRegistryVersion,
    pub fleet_subnet_root: candid::Principal,
    pub fleet_directory_content_hash: [u8; 32],
    pub targets: Vec<RootComponentDirectorySynchronizationTargetRecord>,
    pub state: RootComponentDirectorySynchronizationStateRecord,
}

/// Immutable existing service member selected before the root mirror advances.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentDirectorySynchronizationTargetRecord {
    pub component: ComponentInstanceId,
    pub canister_id: candid::Principal,
    pub allocation_operation_id: [u8; 32],
    pub source_registry: ComponentRegistryHead,
}

/// Monotonic root-local progress over affected existing Components.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootComponentDirectorySynchronizationStateRecord {
    Planned {
        planned_at_ns: u64,
    },
    Synchronizing {
        planned_at_ns: u64,
        synchronized_component_count: u32,
        in_flight: Option<Box<RootComponentDirectorySynchronizationIntentRecord>>,
    },
    Synchronized {
        planned_at_ns: u64,
        synchronized_at_ns: u64,
        receipt_content_hash: [u8; 32],
    },
}

/// Durable pre-call intent for one exact active Component Directory replacement.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentDirectorySynchronizationIntentRecord {
    pub component_index: u32,
    pub component: ComponentInstanceId,
    pub canister_id: candid::Principal,
    pub allocation_operation_id: [u8; 32],
    pub previous_registry: ComponentRegistryHead,
    pub registry: ComponentRegistryHead,
    pub directory_synchronized_at_ns: u64,
    pub directory_authority_hash: [u8; 32],
    pub started_at_ns: u64,
}

/// Durable root-local aggregate phase.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootComponentProvisioningStateRecordPhase {
    Accepted {
        placement_count: u32,
        component_count: u32,
        reservation_cursor: RootComponentProvisioningReservationCursorRecord,
        claim_cursor: RootComponentProvisioningClaimCursorRecord,
        install_cursor: RootComponentProvisioningInstallCursorRecord,
        registry_cursor: RootComponentProvisioningRegistryCursorRecord,
        accepted_at_ns: u64,
        receipt_content_hash: [u8; 32],
    },
    Provisioned {
        placement_count: u32,
        component_count: u32,
        result: RootComponentProvisioningResultRecord,
        accepted_at_ns: u64,
        provisioned_at_ns: u64,
        receipt_content_hash: [u8; 32],
    },
    Publishing {
        placement_count: u32,
        component_count: u32,
        result: RootComponentProvisioningResultRecord,
        publication: RootComponentPublicationEvidence,
        published_component_count: u32,
        in_flight: Option<RootComponentPublicationIntentRecord>,
        accepted_at_ns: u64,
        provisioned_at_ns: u64,
        publication_started_at_ns: u64,
        provisioned_receipt_content_hash: [u8; 32],
    },
    Published {
        placement_count: u32,
        component_count: u32,
        result: RootComponentProvisioningResultRecord,
        publication: RootComponentPublicationEvidence,
        accepted_at_ns: u64,
        provisioned_at_ns: u64,
        published_at_ns: u64,
        receipt_content_hash: [u8; 32],
    },
    Activating {
        placement_count: u32,
        component_count: u32,
        result: RootComponentProvisioningResultRecord,
        publication: RootComponentPublicationEvidence,
        activated_component_count: u32,
        accepted_at_ns: u64,
        provisioned_at_ns: u64,
        published_at_ns: u64,
        activation_started_at_ns: u64,
        published_receipt_content_hash: [u8; 32],
    },
    RuntimesActive {
        placement_count: u32,
        component_count: u32,
        result: RootComponentProvisioningResultRecord,
        publication: RootComponentPublicationEvidence,
        activation: RootComponentActivationEvidence,
        accepted_at_ns: u64,
        provisioned_at_ns: u64,
        published_at_ns: u64,
        activation_started_at_ns: u64,
        runtimes_activated_at_ns: u64,
        published_receipt_content_hash: [u8; 32],
        receipt_content_hash: [u8; 32],
    },
}

/// Durable pre-call intent for one exact prepared Component Directory delivery.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentPublicationIntentRecord {
    pub component_index: u32,
    pub canister_id: candid::Principal,
    pub directory_authority_hash: [u8; 32],
    pub started_at_ns: u64,
}

/// Persisted Component occurrence in one terminal root provisioning result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootProvisionedGroupMemberRecord {
    pub member_path: ComponentGroupMemberPath,
    pub component_spec: ComponentSpecId,
    pub purpose: ComponentDeploymentPurpose,
    pub limits: ComponentDeploymentLimits,
    pub binding: ComponentBinding,
    pub component_registry_revision: u64,
    pub component_registry_content_hash: [u8; 32],
}

/// Persisted terminal result for one exact group placement.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootProvisionedGroupPlacementRecord {
    pub group_placement: ComponentGroupPlacementId,
    pub component_group: ComponentGroupSpecId,
    pub members: Vec<RootProvisionedGroupMemberRecord>,
}

/// Persisted complete group-partitioned provisioning result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentProvisioningResultRecord {
    pub placements: Vec<RootProvisionedGroupPlacementRecord>,
}

/// Canonical O(1) cursor over the accepted placement/member sequence.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentProvisioningReservationCursorRecord {
    pub placement_index: u32,
    pub member_index: u32,
    pub reserved_component_count: u32,
    pub content_hash: [u8; 32],
}

/// Canonical O(1) cursor over prepaid-Canister claims for accepted members.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentProvisioningClaimCursorRecord {
    pub placement_index: u32,
    pub member_index: u32,
    pub claimed_component_count: u32,
    pub content_hash: [u8; 32],
}

/// Canonical O(1) cursor over Store-backed installs for claimed members.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentProvisioningInstallCursorRecord {
    pub placement_index: u32,
    pub member_index: u32,
    pub installed_component_count: u32,
    pub content_hash: [u8; 32],
}

/// Canonical O(1) cursor over Component Registry commitments for installed members.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentProvisioningRegistryCursorRecord {
    pub placement_index: u32,
    pub member_index: u32,
    pub registry_committed_component_count: u32,
    pub content_hash: [u8; 32],
}

/// Placement reservation proving one ID belongs to one exact operation and plan.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentProvisioningPlacementRecord {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
}

impl RootComponentProvisioningPlacementRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "RootComponentProvisioningPlacementRecord";
}

impl_storable_bounded!(
    RootComponentProvisioningPlacementRecord,
    ROOT_COMPONENT_PROVISIONING_PLACEMENT_RECORD_MAX_BYTES,
    false
);

/// Compact aggregate state for exact capacity and active-operation fencing.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentProvisioningStateRecord {
    pub tracked_group_placements: u32,
    pub active_operation_id: Option<[u8; 32]>,
    pub active_directory_synchronization_operation_id: Option<[u8; 32]>,
}

impl RootComponentProvisioningStateRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "RootComponentProvisioningStateRecord";
}

impl_storable_bounded!(RootComponentProvisioningStateRecord, 256, false);

/// Complete typed snapshot used by backup qualification and focused tests.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RootComponentProvisioningData {
    pub state: RootComponentProvisioningStateRecord,
    pub operations: Vec<RootComponentProvisioningRecord>,
    pub directory_synchronizations: Vec<RootComponentDirectorySynchronizationRecord>,
    pub placements: Vec<(
        RootComponentProvisioningPlacementKey,
        RootComponentProvisioningPlacementRecord,
    )>,
}

impl RootComponentProvisioningData {
    pub const STATE_CONTRACT_NAME: &'static str = "RootComponentProvisioningData";
}

/// Exact stable-store acceptance result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootComponentProvisioningCommitOutcome {
    Committed,
    Existing,
}

/// Stable-store conflicts detected before mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootComponentProvisioningCommitError {
    ActiveOperationConflict,
    ConflictingOperation,
    OperationChanged,
    PlacementConflict,
    PlacementCountOverflow,
}

/// Narrow stable-storage owner for root aggregate Component provisioning.
pub struct RootComponentProvisioningStore;

impl RootComponentProvisioningStore {
    pub(crate) fn accept(
        record: RootComponentProvisioningRecord,
    ) -> Result<RootComponentProvisioningCommitOutcome, RootComponentProvisioningCommitError> {
        let operation_key = RootComponentOperationKey::Provisioning(record.operation_id);
        if let Some(existing) = Self::operation(record.operation_id) {
            return if existing == record {
                Ok(RootComponentProvisioningCommitOutcome::Existing)
            } else {
                Err(RootComponentProvisioningCommitError::ConflictingOperation)
            };
        }

        let state = Self::state();
        if state
            .active_operation_id
            .is_some_and(|operation| operation != record.operation_id)
        {
            return Err(RootComponentProvisioningCommitError::ActiveOperationConflict);
        }
        let placement_count = u32::try_from(record.batch.placements.len())
            .map_err(|_| RootComponentProvisioningCommitError::PlacementCountOverflow)?;
        let tracked_group_placements = state
            .tracked_group_placements
            .checked_add(placement_count)
            .ok_or(RootComponentProvisioningCommitError::PlacementCountOverflow)?;
        let placement_keys = record
            .batch
            .placements
            .iter()
            .map(|placement| {
                RootComponentProvisioningPlacementKey::from(&placement.group_placement)
            })
            .collect::<Vec<_>>();
        if placement_keys
            .iter()
            .any(|placement| Self::placement(placement).is_some())
        {
            return Err(RootComponentProvisioningCommitError::PlacementConflict);
        }

        ROOT_COMPONENT_PROVISIONING_OPERATIONS.with_borrow_mut(|operations| {
            assert!(
                operations
                    .insert(
                        operation_key,
                        RootComponentOperationRecord::Provisioning(Box::new(record.clone())),
                    )
                    .is_none()
            );
        });
        let placement_record = RootComponentProvisioningPlacementRecord {
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
        };
        ROOT_COMPONENT_PROVISIONING_PLACEMENTS.with_borrow_mut(|placements| {
            for placement in placement_keys {
                assert!(placements.insert(placement, placement_record).is_none());
            }
        });
        ROOT_COMPONENT_PROVISIONING_STATE.with_borrow_mut(|cell| {
            cell.set(RootComponentProvisioningStateRecord {
                tracked_group_placements,
                active_operation_id: Some(record.operation_id),
                active_directory_synchronization_operation_id: state
                    .active_directory_synchronization_operation_id,
            });
        });
        Ok(RootComponentProvisioningCommitOutcome::Committed)
    }

    #[must_use]
    pub(crate) fn operation(operation_id: [u8; 32]) -> Option<RootComponentProvisioningRecord> {
        ROOT_COMPONENT_PROVISIONING_OPERATIONS.with_borrow(|operations| {
            operations
                .get(&RootComponentOperationKey::Provisioning(operation_id))
                .and_then(|record| match record {
                    RootComponentOperationRecord::Provisioning(record) => Some(*record),
                    RootComponentOperationRecord::DirectorySynchronization(_) => None,
                })
        })
    }

    pub(crate) fn replace_operation(
        current: &RootComponentProvisioningRecord,
        next: RootComponentProvisioningRecord,
    ) -> Result<(), RootComponentProvisioningCommitError> {
        let key = RootComponentOperationKey::Provisioning(current.operation_id);
        let expected = RootComponentOperationRecord::Provisioning(Box::new(current.clone()));
        if next.operation_id != current.operation_id
            || ROOT_COMPONENT_PROVISIONING_OPERATIONS
                .with_borrow(|operations| operations.get(&key))
                .as_ref()
                != Some(&expected)
        {
            return Err(RootComponentProvisioningCommitError::OperationChanged);
        }
        ROOT_COMPONENT_PROVISIONING_OPERATIONS.with_borrow_mut(|operations| {
            operations.insert(
                key,
                RootComponentOperationRecord::Provisioning(Box::new(next)),
            );
        });
        Ok(())
    }

    pub(crate) fn complete_operation(
        current: &RootComponentProvisioningRecord,
        next: RootComponentProvisioningRecord,
    ) -> Result<(), RootComponentProvisioningCommitError> {
        let key = RootComponentOperationKey::Provisioning(current.operation_id);
        let expected = RootComponentOperationRecord::Provisioning(Box::new(current.clone()));
        let stored =
            ROOT_COMPONENT_PROVISIONING_OPERATIONS.with_borrow(|operations| operations.get(&key));
        let state = Self::state();
        if next.operation_id != current.operation_id
            || stored.as_ref() != Some(&expected)
            || state.active_operation_id != Some(current.operation_id)
        {
            return Err(RootComponentProvisioningCommitError::OperationChanged);
        }
        ROOT_COMPONENT_PROVISIONING_OPERATIONS.with_borrow_mut(|operations| {
            operations.insert(
                key,
                RootComponentOperationRecord::Provisioning(Box::new(next)),
            );
        });
        ROOT_COMPONENT_PROVISIONING_STATE.with_borrow_mut(|cell| {
            cell.set(RootComponentProvisioningStateRecord {
                tracked_group_placements: state.tracked_group_placements,
                active_operation_id: None,
                active_directory_synchronization_operation_id: state
                    .active_directory_synchronization_operation_id,
            });
        });
        Ok(())
    }

    pub(crate) fn accept_directory_synchronization(
        record: RootComponentDirectorySynchronizationRecord,
    ) -> Result<RootComponentProvisioningCommitOutcome, RootComponentProvisioningCommitError> {
        let key = RootComponentOperationKey::DirectorySynchronization(record.operation_id);
        if let Some(existing) = Self::directory_synchronization(record.operation_id) {
            return if existing == record {
                Ok(RootComponentProvisioningCommitOutcome::Existing)
            } else {
                Err(RootComponentProvisioningCommitError::ConflictingOperation)
            };
        }
        let state = Self::state();
        if state
            .active_directory_synchronization_operation_id
            .is_some_and(|operation| operation != record.operation_id)
        {
            return Err(RootComponentProvisioningCommitError::ActiveOperationConflict);
        }
        ROOT_COMPONENT_PROVISIONING_OPERATIONS.with_borrow_mut(|operations| {
            assert!(
                operations
                    .insert(
                        key,
                        RootComponentOperationRecord::DirectorySynchronization(Box::new(
                            record.clone(),
                        )),
                    )
                    .is_none()
            );
        });
        ROOT_COMPONENT_PROVISIONING_STATE.with_borrow_mut(|cell| {
            cell.set(RootComponentProvisioningStateRecord {
                active_directory_synchronization_operation_id: Some(record.operation_id),
                ..state
            });
        });
        Ok(RootComponentProvisioningCommitOutcome::Committed)
    }

    #[must_use]
    pub(crate) fn directory_synchronization(
        operation_id: [u8; 32],
    ) -> Option<RootComponentDirectorySynchronizationRecord> {
        ROOT_COMPONENT_PROVISIONING_OPERATIONS.with_borrow(|operations| {
            operations
                .get(&RootComponentOperationKey::DirectorySynchronization(
                    operation_id,
                ))
                .and_then(|record| match record {
                    RootComponentOperationRecord::DirectorySynchronization(record) => Some(*record),
                    RootComponentOperationRecord::Provisioning(_) => None,
                })
        })
    }

    pub(crate) fn replace_directory_synchronization(
        current: &RootComponentDirectorySynchronizationRecord,
        next: RootComponentDirectorySynchronizationRecord,
        complete: bool,
    ) -> Result<(), RootComponentProvisioningCommitError> {
        let key = RootComponentOperationKey::DirectorySynchronization(current.operation_id);
        let expected =
            RootComponentOperationRecord::DirectorySynchronization(Box::new(current.clone()));
        let stored =
            ROOT_COMPONENT_PROVISIONING_OPERATIONS.with_borrow(|operations| operations.get(&key));
        let state = Self::state();
        if next.operation_id != current.operation_id
            || stored.as_ref() != Some(&expected)
            || state.active_directory_synchronization_operation_id != Some(current.operation_id)
        {
            return Err(RootComponentProvisioningCommitError::OperationChanged);
        }
        ROOT_COMPONENT_PROVISIONING_OPERATIONS.with_borrow_mut(|operations| {
            operations.insert(
                key,
                RootComponentOperationRecord::DirectorySynchronization(Box::new(next)),
            );
        });
        if complete {
            ROOT_COMPONENT_PROVISIONING_STATE.with_borrow_mut(|cell| {
                cell.set(RootComponentProvisioningStateRecord {
                    active_directory_synchronization_operation_id: None,
                    ..state
                });
            });
        }
        Ok(())
    }

    #[must_use]
    pub(crate) fn placement(
        key: &RootComponentProvisioningPlacementKey,
    ) -> Option<RootComponentProvisioningPlacementRecord> {
        ROOT_COMPONENT_PROVISIONING_PLACEMENTS.with_borrow(|placements| placements.get(key))
    }

    #[must_use]
    pub(crate) fn state() -> RootComponentProvisioningStateRecord {
        ROOT_COMPONENT_PROVISIONING_STATE.with_borrow(|cell| *cell.get())
    }

    #[must_use]
    pub(crate) fn placement_count() -> u64 {
        ROOT_COMPONENT_PROVISIONING_PLACEMENTS.with_borrow(StableBtreeMap::len)
    }

    #[must_use]
    #[cfg(test)]
    pub(crate) fn export() -> RootComponentProvisioningData {
        RootComponentProvisioningData {
            state: Self::state(),
            operations: ROOT_COMPONENT_PROVISIONING_OPERATIONS.with_borrow(|operations| {
                operations
                    .iter()
                    .filter_map(|entry| match entry.value() {
                        RootComponentOperationRecord::Provisioning(record) => Some(*record),
                        RootComponentOperationRecord::DirectorySynchronization(_) => None,
                    })
                    .collect()
            }),
            directory_synchronizations: ROOT_COMPONENT_PROVISIONING_OPERATIONS.with_borrow(
                |operations| {
                    operations
                        .iter()
                        .filter_map(|entry| match entry.value() {
                            RootComponentOperationRecord::DirectorySynchronization(record) => {
                                Some(*record)
                            }
                            RootComponentOperationRecord::Provisioning(_) => None,
                        })
                        .collect()
                },
            ),
            placements: ROOT_COMPONENT_PROVISIONING_PLACEMENTS.with_borrow(|placements| {
                placements
                    .iter()
                    .map(|entry| (entry.key().clone(), entry.value()))
                    .collect()
            }),
        }
    }

    #[cfg(test)]
    pub(crate) fn import(data: RootComponentProvisioningData) {
        ROOT_COMPONENT_PROVISIONING_OPERATIONS.with_borrow_mut(StableBtreeMap::clear_new);
        ROOT_COMPONENT_PROVISIONING_PLACEMENTS.with_borrow_mut(StableBtreeMap::clear_new);
        ROOT_COMPONENT_PROVISIONING_OPERATIONS.with_borrow_mut(|operations| {
            for record in data.operations {
                operations.insert(
                    RootComponentOperationKey::Provisioning(record.operation_id),
                    RootComponentOperationRecord::Provisioning(Box::new(record)),
                );
            }
            for record in data.directory_synchronizations {
                operations.insert(
                    RootComponentOperationKey::DirectorySynchronization(record.operation_id),
                    RootComponentOperationRecord::DirectorySynchronization(Box::new(record)),
                );
            }
        });
        ROOT_COMPONENT_PROVISIONING_PLACEMENTS.with_borrow_mut(|placements| {
            for (key, record) in data.placements {
                placements.insert(key, record);
            }
        });
        ROOT_COMPONENT_PROVISIONING_STATE.with_borrow_mut(|cell| cell.set(data.state));
    }
}
