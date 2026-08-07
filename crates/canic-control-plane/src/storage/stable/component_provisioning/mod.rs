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
            FleetSubnetRootProvisioningBatch, RootComponentPublicationEvidence,
        },
        fleet_registry::FleetRegistryVersion,
    },
    eager_static,
    ids::{
        ComponentBinding, ComponentDeploymentConfigurationDigest, ComponentGroupDeploymentId,
        ComponentGroupMemberPath, ComponentGroupPlacementId, ComponentGroupSpecId, ComponentSpecId,
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
const ROOT_COMPONENT_PROVISIONING_OPERATION_KEY_MAX_BYTES: u32 = 66;
// The placement owner adds one CBOR map header and the exact two field names to
// two maximum-width operation keys.
const ROOT_COMPONENT_PROVISIONING_PLACEMENT_RECORD_MAX_BYTES: u32 = 156;

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

impl_storable_bounded!(
    RootComponentProvisioningOperationKey,
    ROOT_COMPONENT_PROVISIONING_OPERATION_KEY_MAX_BYTES,
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

    pub(crate) fn replace_operation(
        current: &RootComponentProvisioningRecord,
        next: RootComponentProvisioningRecord,
    ) -> Result<(), RootComponentProvisioningCommitError> {
        let key = RootComponentProvisioningOperationKey(current.operation_id);
        if next.operation_id != current.operation_id
            || ROOT_COMPONENT_PROVISIONING_OPERATIONS
                .with_borrow(|operations| operations.get(&key))
                .as_ref()
                != Some(current)
        {
            return Err(RootComponentProvisioningCommitError::OperationChanged);
        }
        ROOT_COMPONENT_PROVISIONING_OPERATIONS.with_borrow_mut(|operations| {
            operations.insert(key, next);
        });
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
