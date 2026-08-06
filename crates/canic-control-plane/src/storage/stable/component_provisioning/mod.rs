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
        component_provisioning::FleetSubnetRootProvisioningBatch,
        fleet_registry::FleetRegistryVersion,
    },
    eager_static,
    ids::{
        ComponentDeploymentConfigurationDigest, ComponentGroupDeploymentId,
        ComponentGroupPlacementId,
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

struct RootComponentProvisioningOperations;
struct RootComponentProvisioningPlacements;
struct RootComponentProvisioningState;

eager_static! {
    static ROOT_COMPONENT_PROVISIONING_OPERATIONS: RefCell<
        StableBtreeMap<
            RootComponentProvisioningOperationKey,
            RootComponentProvisioningRecord,
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

/// Stable operation-map key.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RootComponentProvisioningOperationKey(pub [u8; 32]);

impl_storable_bounded!(RootComponentProvisioningOperationKey, 64, false);

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
    pub state: RootComponentProvisioningStateRecordPhase,
}

impl RootComponentProvisioningRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "RootComponentProvisioningRecord";
}

impl_storable_bounded!(
    RootComponentProvisioningRecord,
    ROOT_COMPONENT_PROVISIONING_OPERATION_MAX_BYTES,
    false
);

/// Durable root-local aggregate phase.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RootComponentProvisioningStateRecordPhase {
    Accepted {
        placement_count: u32,
        component_count: u32,
        accepted_at_ns: u64,
        receipt_content_hash: [u8; 32],
    },
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

impl_storable_bounded!(RootComponentProvisioningPlacementRecord, 128, false);

/// Compact aggregate state for exact capacity and active-operation fencing.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootComponentProvisioningStateRecord {
    pub tracked_group_placements: u32,
    pub active_operation_id: Option<[u8; 32]>,
}

impl RootComponentProvisioningStateRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "RootComponentProvisioningStateRecord";
}

impl_storable_bounded!(RootComponentProvisioningStateRecord, 128, false);

/// Complete typed snapshot used by backup qualification and focused tests.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RootComponentProvisioningData {
    pub state: RootComponentProvisioningStateRecord,
    pub operations: Vec<RootComponentProvisioningRecord>,
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
    PlacementConflict,
    PlacementCountOverflow,
}

/// Narrow stable-storage owner for root aggregate Component provisioning.
pub struct RootComponentProvisioningStore;

impl RootComponentProvisioningStore {
    pub(crate) fn accept(
        record: RootComponentProvisioningRecord,
    ) -> Result<RootComponentProvisioningCommitOutcome, RootComponentProvisioningCommitError> {
        let operation_key = RootComponentProvisioningOperationKey(record.operation_id);
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
            assert!(operations.insert(operation_key, record.clone()).is_none());
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
            });
        });
        Ok(RootComponentProvisioningCommitOutcome::Committed)
    }

    #[must_use]
    pub(crate) fn operation(operation_id: [u8; 32]) -> Option<RootComponentProvisioningRecord> {
        ROOT_COMPONENT_PROVISIONING_OPERATIONS.with_borrow(|operations| {
            operations.get(&RootComponentProvisioningOperationKey(operation_id))
        })
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
            operations: ROOT_COMPONENT_PROVISIONING_OPERATIONS
                .with_borrow(|operations| operations.iter().map(|entry| entry.value()).collect()),
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
                    RootComponentProvisioningOperationKey(record.operation_id),
                    record,
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
