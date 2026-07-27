//! Module: ops::component_registry
//!
//! Responsibility: read and commit Component Registry authority and allocation reservations.
//! Does not own: Store, Fleet Registry, topology, admission, or lifecycle validation.
//! Boundary: converts stable records into read-only views before workflow use.

use crate::{
    storage::stable::component_registry::{
        RootComponentAllocationCommitError, RootComponentAllocationProgressRecord,
        RootComponentAllocationRecord, RootComponentCreationEffectRecord,
        RootComponentRegistryCommitError, RootComponentRegistryMetaRecord,
        RootComponentRegistryStore,
    },
    view::component_registry::{
        RootComponentAllocationProgressView, RootComponentAllocationView,
        RootComponentCreationEffectView, RootComponentRegistryView,
    },
};
use canic_core::{
    cdk::types::{Cycles, Principal},
    control_plane_support::{
        error::InternalError, model::replay::ReplayCostGuardSettlement,
        policy::component_allocation::TopLevelComponentAllocationDecision,
    },
    dto::{
        component_registry::ComponentProvisioningOrigin, fleet_registry::FleetRegistryVersion,
        root_store::RootStoreBootstrapRequest,
    },
    ids::{ComponentSpecId, FleetSubnetRootBinding, FleetSubnetRootReleaseSet, IntentId},
};

///
/// ComponentRegistryOps
///
/// Single-step root-local Component Registry meta storage operations.
///

pub struct ComponentRegistryOps;

///
/// ComponentSpecInstanceCounts
///
/// Root-local reserved and committed top-level instance counts for one Component Spec.
///

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ComponentSpecInstanceCounts {
    pub reserved: u32,
    pub committed: u32,
}

///
/// RootComponentCreationPlan
///
/// Exact artifact and root-owned settings selected before a creation effect is admitted.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentCreationPlan {
    pub wasm_store: Principal,
    pub payload_hash: [u8; 32],
    pub payload_size_bytes: u64,
    pub initial_cycles: Cycles,
    pub controller: Principal,
}

impl ComponentRegistryOps {
    pub(crate) fn current() -> Option<RootComponentRegistryView> {
        RootComponentRegistryStore::current().map(record_to_view)
    }

    pub(crate) fn prepare(
        root: FleetSubnetRootBinding,
        prepared_against_registry: FleetRegistryVersion,
        release_set: FleetSubnetRootReleaseSet,
        store_bootstrap: RootStoreBootstrapRequest,
    ) -> Result<RootComponentRegistryView, InternalError> {
        let record = RootComponentRegistryMetaRecord {
            root,
            prepared_against_registry,
            release_set,
            store_bootstrap,
            next_allocation_sequence: 1,
            reserved_component_instances: 0,
            committed_component_instances: 0,
            managed_descendants: 0,
            encoded_bytes: 0,
        };
        RootComponentRegistryStore::prepare(record.clone()).map_err(|error| match error {
            RootComponentRegistryCommitError::ConflictingState => InternalError::conflict(
                "root Component Registry is already prepared under different authority",
            ),
        })?;
        Ok(record_to_view(record))
    }

    pub(crate) fn allocation(operation_id: [u8; 32]) -> Option<RootComponentAllocationView> {
        RootComponentRegistryStore::allocation(operation_id).map(allocation_record_to_view)
    }

    pub(crate) fn component_spec_counts(
        component_spec: &ComponentSpecId,
    ) -> Result<ComponentSpecInstanceCounts, InternalError> {
        let reserved = RootComponentRegistryStore::allocation_count(component_spec);
        Ok(ComponentSpecInstanceCounts {
            reserved: u32::try_from(reserved).map_err(|_| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "root Component reservation count exceeds u32",
                )
            })?,
            committed: 0,
        })
    }

    pub(crate) fn reserve_allocation(
        decision: TopLevelComponentAllocationDecision,
        operation_id: [u8; 32],
        provisioning_origin: ComponentProvisioningOrigin,
    ) -> Result<RootComponentAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentAllocationRecord {
            operation_id,
            allocation_sequence: decision.allocation_sequence,
            component: decision.component,
            component_spec: decision.component_spec,
            spec_hash: decision.spec_hash,
            role: decision.role,
            provisioning_origin,
            release_set: current.release_set,
            progress: RootComponentAllocationProgressRecord::Reserved,
        };
        if let Some(existing) = RootComponentRegistryStore::allocation(operation_id) {
            return if existing == record {
                Ok(allocation_record_to_view(existing))
            } else {
                Err(InternalError::conflict(
                    "Component allocation operation is already bound to different intent",
                ))
            };
        }

        if current.next_allocation_sequence != record.allocation_sequence {
            return Err(InternalError::conflict(
                "Component allocation sequence changed before reservation commit",
            ));
        }
        let entry_bytes = RootComponentRegistryStore::allocation_entry_bytes(&record);
        let encoded_bytes = current
            .encoded_bytes
            .checked_add(entry_bytes)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        if encoded_bytes > current.root.limits.maximum_registry_bytes {
            return Err(InternalError::resource_exhausted(format!(
                "Component Registry reservation requires {encoded_bytes} bytes, exceeding protected limit {}",
                current.root.limits.maximum_registry_bytes
            )));
        }
        let mut next = current.clone();
        next.next_allocation_sequence =
            next.next_allocation_sequence
                .checked_add(1)
                .ok_or_else(|| {
                    InternalError::resource_exhausted("Component allocation sequence is exhausted")
                })?;
        next.reserved_component_instances = next
            .reserved_component_instances
            .checked_add(1)
            .ok_or_else(|| {
                InternalError::resource_exhausted("reserved Component instance count overflow")
            })?;
        next.encoded_bytes = encoded_bytes;

        RootComponentRegistryStore::reserve_allocation(&current, next, record.clone())
            .map_err(map_allocation_commit_error)?;
        Ok(allocation_record_to_view(record))
    }

    pub(crate) fn validate_creation_capacity(
        operation_id: [u8; 32],
        plan: &RootComponentCreationPlan,
    ) -> Result<(), InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        if !matches!(
            record.progress,
            RootComponentAllocationProgressRecord::Reserved
        ) {
            return Err(InternalError::conflict(
                "Component allocation has already crossed its creation-intent boundary",
            ));
        }

        let charged_entry_bytes = creation_charged_entry_bytes(&record, plan);
        validate_creation_capacity(&current, &record, charged_entry_bytes).map(|_| ())
    }

    pub(crate) fn begin_creation(
        operation_id: [u8; 32],
        plan: RootComponentCreationPlan,
        cost_guard_settlement: ReplayCostGuardSettlement,
    ) -> Result<RootComponentAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        if !matches!(
            record.progress,
            RootComponentAllocationProgressRecord::Reserved
        ) {
            return Err(InternalError::conflict(
                "Component allocation has already crossed its creation-intent boundary",
            ));
        }

        let charged_entry_bytes = creation_charged_entry_bytes(&record, &plan);
        let next_encoded_bytes =
            validate_creation_capacity(&current, &record, charged_entry_bytes)?;
        let mut next_record = record.clone();
        next_record.progress = RootComponentAllocationProgressRecord::CreationIntent(
            RootComponentCreationEffectRecord {
                wasm_store: plan.wasm_store,
                payload_hash: plan.payload_hash,
                payload_size_bytes: plan.payload_size_bytes,
                initial_cycles: plan.initial_cycles,
                controller: plan.controller,
                cost_guard_settlement,
                charged_entry_bytes,
            },
        );
        validate_charged_record_size(&next_record, charged_entry_bytes)?;

        let mut next_meta = current.clone();
        next_meta.encoded_bytes = next_encoded_bytes;
        RootComponentRegistryStore::replace_allocation(
            &current,
            next_meta,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(allocation_record_to_view(next_record))
    }

    pub(crate) fn mark_created(
        operation_id: [u8; 32],
        canister: Principal,
    ) -> Result<RootComponentAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        let effect = match &record.progress {
            RootComponentAllocationProgressRecord::CreationIntent(effect) => effect.clone(),
            RootComponentAllocationProgressRecord::Created {
                canister: existing, ..
            } if existing == &canister => return Ok(allocation_record_to_view(record)),
            RootComponentAllocationProgressRecord::Created { .. } => {
                return Err(InternalError::conflict(
                    "Component allocation is already bound to a different created Canister",
                ));
            }
            RootComponentAllocationProgressRecord::Reserved => {
                return Err(InternalError::conflict(
                    "Component allocation has no durable creation intent",
                ));
            }
        };
        let charged_entry_bytes = effect.charged_entry_bytes;
        let mut next_record = record.clone();
        next_record.progress = RootComponentAllocationProgressRecord::Created { effect, canister };
        validate_charged_record_size(&next_record, charged_entry_bytes)?;
        RootComponentRegistryStore::replace_allocation(
            &current,
            current.clone(),
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(allocation_record_to_view(next_record))
    }
}

fn record_to_view(record: RootComponentRegistryMetaRecord) -> RootComponentRegistryView {
    RootComponentRegistryView {
        root: record.root,
        prepared_against_registry: record.prepared_against_registry,
        release_set: record.release_set,
        store_bootstrap: record.store_bootstrap,
        next_allocation_sequence: record.next_allocation_sequence,
        reserved_component_instances: record.reserved_component_instances,
        committed_component_instances: record.committed_component_instances,
        managed_descendants: record.managed_descendants,
        encoded_bytes: record.encoded_bytes,
    }
}

fn allocation_record_to_view(record: RootComponentAllocationRecord) -> RootComponentAllocationView {
    RootComponentAllocationView {
        operation_id: record.operation_id,
        allocation_sequence: record.allocation_sequence,
        component: record.component,
        component_spec: record.component_spec,
        spec_hash: record.spec_hash,
        role: record.role,
        provisioning_origin: record.provisioning_origin,
        release_set: record.release_set,
        progress: match record.progress {
            RootComponentAllocationProgressRecord::Reserved => {
                RootComponentAllocationProgressView::Reserved
            }
            RootComponentAllocationProgressRecord::CreationIntent(effect) => {
                RootComponentAllocationProgressView::CreationIntent(creation_effect_record_to_view(
                    effect,
                ))
            }
            RootComponentAllocationProgressRecord::Created { effect, canister } => {
                RootComponentAllocationProgressView::Created {
                    effect: creation_effect_record_to_view(effect),
                    canister,
                }
            }
        },
    }
}

const fn creation_effect_record_to_view(
    effect: RootComponentCreationEffectRecord,
) -> RootComponentCreationEffectView {
    RootComponentCreationEffectView {
        wasm_store: effect.wasm_store,
        payload_hash: effect.payload_hash,
        payload_size_bytes: effect.payload_size_bytes,
        initial_cycles: effect.initial_cycles,
        controller: effect.controller,
        cost_guard_settlement: effect.cost_guard_settlement,
        charged_entry_bytes: effect.charged_entry_bytes,
    }
}

fn creation_charged_entry_bytes(
    record: &RootComponentAllocationRecord,
    plan: &RootComponentCreationPlan,
) -> u64 {
    let mut maximum = record.clone();
    maximum.progress = RootComponentAllocationProgressRecord::Created {
        effect: RootComponentCreationEffectRecord {
            wasm_store: plan.wasm_store,
            payload_hash: plan.payload_hash,
            payload_size_bytes: u64::MAX,
            initial_cycles: Cycles::new(u128::MAX),
            controller: plan.controller,
            cost_guard_settlement: ReplayCostGuardSettlement {
                quota_intent_id: IntentId(u64::MAX),
                reservation_intent_id: IntentId(u64::MAX),
            },
            charged_entry_bytes: u64::MAX,
        },
        canister: Principal::from_slice(&[u8::MAX; 29]),
    };
    RootComponentRegistryStore::allocation_entry_bytes(&maximum)
}

fn validate_creation_capacity(
    current: &RootComponentRegistryMetaRecord,
    record: &RootComponentAllocationRecord,
    charged_entry_bytes: u64,
) -> Result<u64, InternalError> {
    if charged_entry_bytes > RootComponentRegistryStore::allocation_record_max_bytes() + 128 {
        return Err(InternalError::resource_exhausted(
            "Component creation evidence exceeds its stable record bound",
        ));
    }
    let current_entry_bytes = RootComponentRegistryStore::allocation_entry_bytes(record);
    let without_current = current
        .encoded_bytes
        .checked_sub(current_entry_bytes)
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Registry encoded-byte accounting is below its reserved record",
            )
        })?;
    let next_encoded_bytes = without_current
        .checked_add(charged_entry_bytes)
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))?;
    if next_encoded_bytes > current.root.limits.maximum_registry_bytes {
        return Err(InternalError::resource_exhausted(format!(
            "Component creation evidence requires {next_encoded_bytes} bytes, exceeding protected limit {}",
            current.root.limits.maximum_registry_bytes
        )));
    }
    Ok(next_encoded_bytes)
}

fn validate_charged_record_size(
    record: &RootComponentAllocationRecord,
    charged_entry_bytes: u64,
) -> Result<(), InternalError> {
    let entry_bytes = RootComponentRegistryStore::allocation_entry_bytes(record);
    if entry_bytes > charged_entry_bytes {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component creation record exceeds its pre-effect Registry byte charge",
        ));
    }
    Ok(())
}

fn map_allocation_commit_error(error: RootComponentAllocationCommitError) -> InternalError {
    match error {
        RootComponentAllocationCommitError::ComponentIdentityConflict => InternalError::conflict(
            "derived Component identity is already reserved by another operation",
        ),
        RootComponentAllocationCommitError::ConflictingOperation => InternalError::conflict(
            "Component allocation operation is already bound to different intent",
        ),
        RootComponentAllocationCommitError::ConflictingState => InternalError::conflict(
            "Component Registry authority changed before allocation mutation",
        ),
        RootComponentAllocationCommitError::MissingOperation => {
            InternalError::unavailable("Component allocation operation has not been reserved")
        }
        RootComponentAllocationCommitError::Uninitialized => {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::stable::component_registry::RootComponentRegistryData;
    use canic_core::{
        cdk::types::Cycles,
        control_plane_support::policy::component_allocation::TopLevelComponentAllocationDecision,
        dto::{
            component_registry::ComponentProvisioningOrigin, fleet_registry::FleetRegistryVersion,
            root_store::RootStoreBootstrapRequest,
        },
        ids::{
            AppId, CanisterRole, CanonicalNetworkId, ComponentInstanceId, ComponentSpecAdmission,
            ComponentTopologyDigest, CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding,
            FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetRootLimits, ReleaseBuildId,
            ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
        },
    };

    #[test]
    fn preparation_is_exact_idempotent_and_conflict_closed() {
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
        let root = root_binding();
        let version = FleetRegistryVersion {
            authority: root.authority.clone(),
            revision: 4,
            content_hash: [5; 32],
        };
        let release_set = FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [8; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
        };
        let store_bootstrap = RootStoreBootstrapRequest {
            manifest_payload_size_bytes: 128,
        };

        let prepared = ComponentRegistryOps::prepare(
            root.clone(),
            version.clone(),
            release_set,
            store_bootstrap.clone(),
        )
        .expect("prepare");
        let repeated =
            ComponentRegistryOps::prepare(root.clone(), version, release_set, store_bootstrap)
                .expect("exact retry");

        assert_eq!(prepared, repeated);
        assert_eq!(prepared.next_allocation_sequence, 1);
        assert_eq!(prepared.reserved_component_instances, 0);
        assert_eq!(prepared.committed_component_instances, 0);
        assert_eq!(prepared.managed_descendants, 0);
        assert_eq!(prepared.encoded_bytes, 0);

        let mut conflicting = root;
        conflicting.limits.maximum_component_instances += 1;
        assert!(
            ComponentRegistryOps::prepare(
                conflicting,
                repeated.prepared_against_registry,
                release_set,
                repeated.store_bootstrap,
            )
            .is_err()
        );
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    #[test]
    fn allocation_reservation_is_exact_idempotent_and_charges_registry_capacity() {
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
        let root = root_binding();
        let release_set = FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [8; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
        };
        let version = FleetRegistryVersion {
            authority: root.authority.clone(),
            revision: 4,
            content_hash: [5; 32],
        };
        ComponentRegistryOps::prepare(
            root,
            version,
            release_set,
            RootStoreBootstrapRequest {
                manifest_payload_size_bytes: 128,
            },
        )
        .expect("prepare");
        let decision = TopLevelComponentAllocationDecision {
            allocation_sequence: 1,
            component: ComponentInstanceId::from_generated_bytes([10; 32]),
            component_spec: "projects".parse().expect("Component Spec"),
            spec_hash: [6; 32],
            role: CanisterRole::new("project_hub"),
        };
        let origin = ComponentProvisioningOrigin::FleetAdministrator {
            caller: candid::Principal::from_slice(&[11; 29]),
        };

        let reserved =
            ComponentRegistryOps::reserve_allocation(decision.clone(), [12; 32], origin.clone())
                .expect("reserve");
        let interrupted_snapshot = RootComponentRegistryStore::export();
        RootComponentRegistryStore::import(interrupted_snapshot);
        let repeated = ComponentRegistryOps::reserve_allocation(decision, [12; 32], origin)
            .expect("exact retry");

        assert_eq!(reserved, repeated);
        assert_eq!(reserved.allocation_sequence, 1);
        let status = ComponentRegistryOps::current().expect("Registry status");
        assert_eq!(status.next_allocation_sequence, 2);
        assert_eq!(status.reserved_component_instances, 1);
        assert_eq!(status.committed_component_instances, 0);
        assert!(status.encoded_bytes > 0);
        assert_eq!(
            ComponentRegistryOps::component_spec_counts(&reserved.component_spec)
                .expect("Spec counts"),
            ComponentSpecInstanceCounts {
                reserved: 1,
                committed: 0,
            }
        );

        let conflicting = TopLevelComponentAllocationDecision {
            allocation_sequence: 2,
            component: ComponentInstanceId::from_generated_bytes([13; 32]),
            component_spec: "projects".parse().expect("Component Spec"),
            spec_hash: [6; 32],
            role: CanisterRole::new("project_hub"),
        };
        assert!(
            ComponentRegistryOps::reserve_allocation(
                conflicting,
                [12; 32],
                ComponentProvisioningOrigin::FleetAdministrator {
                    caller: candid::Principal::from_slice(&[11; 29]),
                },
            )
            .is_err()
        );
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    #[test]
    fn creation_intent_reserves_terminal_bytes_and_created_retry_preserves_principal() {
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
        let root = root_binding();
        let version = FleetRegistryVersion {
            authority: root.authority.clone(),
            revision: 4,
            content_hash: [5; 32],
        };
        ComponentRegistryOps::prepare(
            root,
            version,
            FleetSubnetRootReleaseSet {
                release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                    [8; 32],
                )),
                manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
            },
            RootStoreBootstrapRequest {
                manifest_payload_size_bytes: 128,
            },
        )
        .expect("prepare");
        ComponentRegistryOps::reserve_allocation(
            TopLevelComponentAllocationDecision {
                allocation_sequence: 1,
                component: ComponentInstanceId::from_generated_bytes([10; 32]),
                component_spec: "projects".parse().expect("Component Spec"),
                spec_hash: [6; 32],
                role: CanisterRole::new("project_hub"),
            },
            [12; 32],
            ComponentProvisioningOrigin::FleetAdministrator {
                caller: candid::Principal::from_slice(&[11; 29]),
            },
        )
        .expect("reserve");
        let reserved_bytes = ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes;
        let plan = RootComponentCreationPlan {
            wasm_store: candid::Principal::from_slice(&[13; 29]),
            payload_hash: [14; 32],
            payload_size_bytes: 4_096,
            initial_cycles: Cycles::new(5_000_000_000_000),
            controller: candid::Principal::from_slice(&[15; 29]),
        };

        assert_creation_capacity_is_reserved_before_effect(&plan, reserved_bytes);

        ComponentRegistryOps::validate_creation_capacity([12; 32], &plan)
            .expect("creation capacity");
        let intent = ComponentRegistryOps::begin_creation(
            [12; 32],
            plan.clone(),
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(16),
                reservation_intent_id: IntentId(17),
            },
        )
        .expect("creation intent");
        let intent_bytes = ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes;
        assert!(intent_bytes > reserved_bytes);
        assert!(matches!(
            intent.progress,
            RootComponentAllocationProgressView::CreationIntent(_)
        ));

        let interrupted = RootComponentRegistryStore::export();
        RootComponentRegistryStore::import(interrupted);
        let canister = candid::Principal::from_slice(&[18; 29]);
        let created =
            ComponentRegistryOps::mark_created([12; 32], canister).expect("record created");
        let repeated =
            ComponentRegistryOps::mark_created([12; 32], canister).expect("exact created retry");

        assert_eq!(created, repeated);
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .encoded_bytes,
            intent_bytes,
            "the intent must reserve terminal record capacity before the effect"
        );
        assert!(matches!(
            created.progress,
            RootComponentAllocationProgressView::Created {
                canister: created_canister,
                ..
            } if created_canister == canister
        ));
        assert!(
            ComponentRegistryOps::mark_created([12; 32], candid::Principal::from_slice(&[19; 29]),)
                .is_err()
        );
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    fn assert_creation_capacity_is_reserved_before_effect(
        plan: &RootComponentCreationPlan,
        reserved_bytes: u64,
    ) {
        let before_creation = RootComponentRegistryStore::export();
        let mut exhausted = before_creation.clone();
        exhausted
            .current
            .as_mut()
            .expect("Registry meta")
            .root
            .limits
            .maximum_registry_bytes = reserved_bytes;
        RootComponentRegistryStore::import(exhausted);

        let capacity_error = ComponentRegistryOps::validate_creation_capacity([12; 32], plan)
            .expect_err("terminal creation evidence must fit before the paid effect");
        assert!(capacity_error.is_public_resource_exhausted());
        assert!(matches!(
            ComponentRegistryOps::allocation([12; 32])
                .expect("reserved allocation")
                .progress,
            RootComponentAllocationProgressView::Reserved
        ));
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .encoded_bytes,
            reserved_bytes
        );
        RootComponentRegistryStore::import(before_creation);
    }

    #[test]
    fn allocation_reservation_fails_before_mutation_when_registry_capacity_is_exhausted() {
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
        let mut root = root_binding();
        root.limits.maximum_registry_bytes = 1;
        let version = FleetRegistryVersion {
            authority: root.authority.clone(),
            revision: 4,
            content_hash: [5; 32],
        };
        ComponentRegistryOps::prepare(
            root,
            version,
            FleetSubnetRootReleaseSet {
                release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                    [8; 32],
                )),
                manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
            },
            RootStoreBootstrapRequest {
                manifest_payload_size_bytes: 128,
            },
        )
        .expect("prepare");

        let error = ComponentRegistryOps::reserve_allocation(
            TopLevelComponentAllocationDecision {
                allocation_sequence: 1,
                component: ComponentInstanceId::from_generated_bytes([10; 32]),
                component_spec: "projects".parse().expect("Component Spec"),
                spec_hash: [6; 32],
                role: CanisterRole::new("project_hub"),
            },
            [12; 32],
            ComponentProvisioningOrigin::FleetAdministrator {
                caller: candid::Principal::from_slice(&[11; 29]),
            },
        )
        .expect_err("Registry byte capacity must reject reservation");
        assert!(error.is_public_resource_exhausted());
        assert!(ComponentRegistryOps::allocation([12; 32]).is_none());

        let status = ComponentRegistryOps::current().expect("Registry status");
        assert_eq!(status.next_allocation_sequence, 1);
        assert_eq!(status.reserved_component_instances, 0);
        assert_eq!(status.encoded_bytes, 0);
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    fn root_binding() -> FleetSubnetRootBinding {
        let coordinator_subnet = SubnetId::from_principal(candid::Principal::from_slice(&[2; 29]));
        FleetSubnetRootBinding {
            authority: FleetRegistryAuthority {
                binding: FleetCoordinatorBinding {
                    fleet: FleetBinding {
                        fleet: FleetKey {
                            canonical_network_id: CanonicalNetworkId::public_ic(),
                            fleet_id: FleetId::from_generated_bytes([1; 32]),
                        },
                        app: AppId::from("toko"),
                    },
                    coordinator_subnet,
                    coordinator: candid::Principal::from_slice(&[3; 29]),
                },
                epoch: 1,
            },
            placement_subnet: SubnetId::from_principal(candid::Principal::from_slice(&[4; 29])),
            fleet_subnet_root: candid::Principal::from_slice(&[5; 29]),
            component_admissions: vec![ComponentSpecAdmission {
                component_spec: "projects".parse().expect("Component Spec"),
                spec_hash: [6; 32],
                maximum_root_instances: 10,
            }],
            component_topology_digest: ComponentTopologyDigest::from_bytes([7; 32]),
            limits: FleetSubnetRootLimits {
                maximum_component_instances: 10,
                maximum_managed_canisters: 20_000,
                maximum_registry_bytes: 16_777_216,
                maximum_wasm_store_bytes: 268_435_456,
                cycles_funding: CyclesFundingBudget {
                    window_secs: 3_600,
                    maximum_cycles: Cycles::new(1_000_000_000_000),
                },
            },
        }
    }
}
