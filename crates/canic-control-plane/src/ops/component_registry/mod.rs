//! Module: ops::component_registry
//!
//! Responsibility: read and commit Component Registry authority and allocation reservations.
//! Does not own: Store, Fleet Registry, topology, admission, or lifecycle validation.
//! Boundary: converts stable records into read-only views before workflow use.

use crate::{
    storage::stable::component_registry::{
        ComponentRegistryPartitionRecord, RootComponentAllocationCommitError,
        RootComponentAllocationProgressRecord, RootComponentAllocationRecord,
        RootComponentCommitmentRecord, RootComponentCreationEffectRecord,
        RootComponentInstallEffectRecord, RootComponentMembershipRecord,
        RootComponentRegistryCommitError, RootComponentRegistryMetaRecord,
        RootComponentRegistryStore,
    },
    view::component_registry::{
        ComponentRegistryPartitionView, RootComponentAllocationProgressView,
        RootComponentAllocationView, RootComponentCommitmentView, RootComponentCreationEffectView,
        RootComponentInstallEffectView, RootComponentMembershipView, RootComponentRegistryView,
    },
};
use canic_core::{
    cdk::types::{Cycles, Principal},
    control_plane_support::{
        error::InternalError, model::replay::ReplayCostGuardSettlement,
        ops::component_runtime::ComponentRuntimeOps,
        policy::component_allocation::TopLevelComponentAllocationDecision,
    },
    dto::{
        component_registry::{
            ComponentDirectoryHead, ComponentDirectoryProvenance, ComponentLifecycleStatus,
            ComponentProvisioningOrigin, ComponentRegistryHead, ComponentRuntimeDirectoryAuthority,
        },
        fleet_registry::{FleetDirectorySnapshot, FleetRegistryVersion},
        root_store::RootStoreBootstrapRequest,
    },
    ids::{
        ComponentBinding, ComponentSpecId, FleetSubnetRootBinding, FleetSubnetRootReleaseSet,
        IntentId,
    },
};
use sha2::{Digest, Sha256};

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

///
/// RootComponentInstallPlan
///
/// Exact module source and immutable target binding selected before installation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentInstallPlan {
    pub raw_module_hash: [u8; 32],
    pub chunk_hashes: Vec<Vec<u8>>,
    pub binding: ComponentBinding,
    pub maximum_registry_bytes: u64,
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
        let (reserved, committed) = RootComponentRegistryStore::allocation_counts(component_spec);
        Ok(ComponentSpecInstanceCounts {
            reserved: u32::try_from(reserved).map_err(|_| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "root Component reservation count exceeds u32",
                )
            })?,
            committed: u32::try_from(committed).map_err(|_| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "root committed Component count exceeds u32",
                )
            })?,
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
            RootComponentAllocationProgressRecord::InstallIntent {
                canister: existing, ..
            }
            | RootComponentAllocationProgressRecord::Installed {
                canister: existing, ..
            }
            | RootComponentAllocationProgressRecord::Verified {
                canister: existing, ..
            }
            | RootComponentAllocationProgressRecord::Committed {
                canister: existing, ..
            } if existing == &canister => return Ok(allocation_record_to_view(record)),
            RootComponentAllocationProgressRecord::Created { .. }
            | RootComponentAllocationProgressRecord::InstallIntent { .. }
            | RootComponentAllocationProgressRecord::Installed { .. }
            | RootComponentAllocationProgressRecord::Verified { .. }
            | RootComponentAllocationProgressRecord::Committed { .. } => {
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

    pub(crate) fn validate_install_capacity(
        operation_id: [u8; 32],
        plan: &RootComponentInstallPlan,
    ) -> Result<(), InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        if !matches!(
            record.progress,
            RootComponentAllocationProgressRecord::Created { .. }
        ) {
            return Err(InternalError::conflict(
                "Component allocation is not ready to cross its install-intent boundary",
            ));
        }

        let charged_entry_bytes = install_charged_entry_bytes(&record, plan)?;
        validate_install_capacity(&current, &record, charged_entry_bytes).map(|_| ())
    }

    pub(crate) fn begin_install(
        operation_id: [u8; 32],
        plan: RootComponentInstallPlan,
        cost_guard_settlement: ReplayCostGuardSettlement,
    ) -> Result<RootComponentAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        let (creation, canister) = match &record.progress {
            RootComponentAllocationProgressRecord::Created { effect, canister } => {
                (effect.clone(), *canister)
            }
            _ => {
                return Err(InternalError::conflict(
                    "Component allocation is not ready for installation",
                ));
            }
        };
        let charged_entry_bytes = install_charged_entry_bytes(&record, &plan)?;
        let next_encoded_bytes = validate_install_capacity(&current, &record, charged_entry_bytes)?;
        let mut next_record = record.clone();
        next_record.progress = RootComponentAllocationProgressRecord::InstallIntent {
            creation,
            canister,
            installation: RootComponentInstallEffectRecord {
                raw_module_hash: plan.raw_module_hash,
                chunk_hashes: plan.chunk_hashes,
                binding: plan.binding,
                cost_guard_settlement,
                charged_entry_bytes,
            },
        };
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

    pub(crate) fn renew_install_intent(
        operation_id: [u8; 32],
        plan: &RootComponentInstallPlan,
        cost_guard_settlement: ReplayCostGuardSettlement,
    ) -> Result<RootComponentAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        let (creation, canister, existing) = match &record.progress {
            RootComponentAllocationProgressRecord::InstallIntent {
                creation,
                canister,
                installation,
            } => (creation.clone(), *canister, installation),
            _ => {
                return Err(InternalError::conflict(
                    "Component allocation has no renewable install intent",
                ));
            }
        };
        validate_install_effect_record(existing, plan)?;
        let charged_entry_bytes = existing.charged_entry_bytes;
        let mut next_record = record.clone();
        next_record.progress = RootComponentAllocationProgressRecord::InstallIntent {
            creation,
            canister,
            installation: RootComponentInstallEffectRecord {
                raw_module_hash: plan.raw_module_hash,
                chunk_hashes: plan.chunk_hashes.clone(),
                binding: plan.binding.clone(),
                cost_guard_settlement,
                charged_entry_bytes,
            },
        };
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

    pub(crate) fn mark_installed(
        operation_id: [u8; 32],
    ) -> Result<RootComponentAllocationView, InternalError> {
        advance_install_phase(operation_id, false)
    }

    pub(crate) fn mark_verified(
        operation_id: [u8; 32],
    ) -> Result<RootComponentAllocationView, InternalError> {
        advance_install_phase(operation_id, true)
    }

    pub(crate) fn partition(
        component: canic_core::ids::ComponentInstanceId,
    ) -> Result<Option<ComponentRegistryPartitionView>, InternalError> {
        let Some(record) = RootComponentRegistryStore::partition(component) else {
            return Ok(None);
        };
        validate_partition_record(&record)?;
        Ok(Some(partition_record_to_view(record)))
    }

    pub(crate) fn prepared_partition(
        operation_id: [u8; 32],
    ) -> Result<ComponentRegistryPartitionView, InternalError> {
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        let RootComponentAllocationProgressRecord::Committed { commitment, .. } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component allocation has no committed Registry authority",
            ));
        };
        exact_committed_partition(&record, commitment).map(partition_record_to_view)
    }

    pub(crate) fn component_for_principal(
        canister: Principal,
    ) -> Option<canic_core::ids::ComponentInstanceId> {
        RootComponentRegistryStore::component_for_principal(canister)
    }

    pub(crate) fn commit_verified(
        operation_id: [u8; 32],
        directory_synchronized_at_ns: u64,
        maximum_component_registry_bytes: u64,
        fleet_directory: FleetDirectorySnapshot,
    ) -> Result<(RootComponentAllocationView, ComponentRegistryPartitionView), InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        if let RootComponentAllocationProgressRecord::Committed { commitment, .. } =
            &record.progress
        {
            let partition = exact_committed_partition(&record, commitment)?;
            validate_directory_authority_hash(&partition, &fleet_directory, commitment)?;
            return Ok((
                allocation_record_to_view(record),
                partition_record_to_view(partition),
            ));
        }
        if directory_synchronized_at_ns == 0 {
            return Err(InternalError::invalid_input(
                "Component Directory synchronization timestamp must be positive",
            ));
        }
        let RootComponentAllocationProgressRecord::Verified {
            creation,
            canister,
            installation,
        } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component allocation is not ready for Registry commitment",
            ));
        };

        let (next_record, partition) = committed_records(
            &record,
            creation,
            *canister,
            installation,
            directory_synchronized_at_ns,
            &fleet_directory,
        )?;
        if partition.encoded_bytes > installation.charged_entry_bytes {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component commitment exceeds its pre-install Registry byte reservation",
            ));
        }
        if partition.encoded_bytes > maximum_component_registry_bytes {
            return Err(InternalError::resource_exhausted(format!(
                "Component Registry commitment requires {} bytes, exceeding protected Component limit {maximum_component_registry_bytes}",
                partition.encoded_bytes
            )));
        }
        let encoded_bytes = current
            .encoded_bytes
            .checked_sub(installation.charged_entry_bytes)
            .and_then(|value| value.checked_add(partition.encoded_bytes))
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "root Component Registry byte accounting cannot commit its reserved partition",
                )
            })?;
        if encoded_bytes > current.root.limits.maximum_registry_bytes {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "pre-install Registry reservation exceeds the protected root limit at commitment",
            ));
        }

        let mut next_meta = current.clone();
        next_meta.reserved_component_instances = next_meta
            .reserved_component_instances
            .checked_sub(1)
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "root reserved Component count is zero at commitment",
                )
            })?;
        next_meta.committed_component_instances = next_meta
            .committed_component_instances
            .checked_add(1)
            .ok_or_else(|| {
                InternalError::resource_exhausted("committed Component instance count overflow")
            })?;
        next_meta.encoded_bytes = encoded_bytes;

        RootComponentRegistryStore::commit_component(
            &current,
            next_meta,
            &record,
            next_record.clone(),
            partition.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok((
            allocation_record_to_view(next_record),
            partition_record_to_view(partition),
        ))
    }

    pub(crate) fn mark_directory_prepared(
        operation_id: [u8; 32],
        expected_authority_hash: [u8; 32],
    ) -> Result<RootComponentAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        let RootComponentAllocationProgressRecord::Committed {
            creation,
            canister,
            installation,
            commitment,
        } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component allocation is not committed for Directory preparation",
            ));
        };
        if commitment.directory_authority_hash != expected_authority_hash {
            return Err(InternalError::conflict(
                "Component Directory authority differs from its committed root receipt",
            ));
        }
        if commitment.directory_prepared {
            return Ok(allocation_record_to_view(record));
        }
        let mut next_record = record.clone();
        next_record.progress = RootComponentAllocationProgressRecord::Committed {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
            commitment: RootComponentCommitmentRecord {
                registry: commitment.registry.clone(),
                prepared_registry_encoded_bytes: commitment.prepared_registry_encoded_bytes,
                directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
                directory_authority_hash: commitment.directory_authority_hash,
                directory_prepared: true,
                runtime_activated: commitment.runtime_activated,
                membership: commitment.membership.clone(),
            },
        };
        validate_charged_record_size(&next_record, installation.charged_entry_bytes)?;
        if RootComponentRegistryStore::allocation_entry_bytes(&next_record)
            != RootComponentRegistryStore::allocation_entry_bytes(&record)
        {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Directory receipt changed its precharged stable footprint",
            ));
        }
        RootComponentRegistryStore::replace_allocation(
            &current,
            current.clone(),
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(allocation_record_to_view(next_record))
    }

    pub(crate) fn mark_runtime_activated(
        operation_id: [u8; 32],
        expected_authority_hash: [u8; 32],
    ) -> Result<RootComponentAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        let RootComponentAllocationProgressRecord::Committed {
            creation,
            canister,
            installation,
            commitment,
        } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component allocation is not committed for runtime activation",
            ));
        };
        if commitment.directory_authority_hash != expected_authority_hash
            || !commitment.directory_prepared
        {
            return Err(InternalError::conflict(
                "Component runtime activation requires its exact prepared Directory authority",
            ));
        }
        if commitment.runtime_activated {
            return Ok(allocation_record_to_view(record));
        }
        let mut next_record = record.clone();
        next_record.progress = RootComponentAllocationProgressRecord::Committed {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
            commitment: RootComponentCommitmentRecord {
                registry: commitment.registry.clone(),
                prepared_registry_encoded_bytes: commitment.prepared_registry_encoded_bytes,
                directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
                directory_authority_hash: commitment.directory_authority_hash,
                directory_prepared: true,
                runtime_activated: true,
                membership: commitment.membership.clone(),
            },
        };
        validate_charged_record_size(&next_record, installation.charged_entry_bytes)?;
        if RootComponentRegistryStore::allocation_entry_bytes(&next_record)
            != RootComponentRegistryStore::allocation_entry_bytes(&record)
        {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component runtime activation receipt changed its precharged stable footprint",
            ));
        }
        RootComponentRegistryStore::replace_allocation(
            &current,
            current.clone(),
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(allocation_record_to_view(next_record))
    }

    pub(crate) fn activate_membership(
        operation_id: [u8; 32],
        directory_synchronized_at_ns: u64,
        maximum_component_registry_bytes: u64,
        fleet_directory: FleetDirectorySnapshot,
    ) -> Result<(RootComponentAllocationView, ComponentRegistryPartitionView), InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        let RootComponentAllocationProgressRecord::Committed {
            installation,
            commitment,
            ..
        } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component allocation is not committed for membership activation",
            ));
        };
        let prepared = exact_committed_partition(&record, commitment)?;
        if let Some(membership) = &commitment.membership {
            let active = exact_active_partition(&record, commitment, membership)?;
            validate_membership_directory_authority_hash(&active, &fleet_directory, membership)?;
            return Ok((
                allocation_record_to_view(record),
                partition_record_to_view(active),
            ));
        }
        if !commitment.directory_prepared || !commitment.runtime_activated {
            return Err(InternalError::conflict(
                "Component membership activation requires terminal Directory and runtime receipts",
            ));
        }
        if directory_synchronized_at_ns <= commitment.directory_synchronized_at_ns {
            return Err(InternalError::invalid_input(
                "active Component Directory synchronization must follow its prepared authority",
            ));
        }

        let (next_record, active) = active_membership_records(
            &record,
            commitment,
            directory_synchronized_at_ns,
            &fleet_directory,
        )?;
        if active.encoded_bytes > installation.charged_entry_bytes {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component membership exceeds its pre-install Registry byte reservation",
            ));
        }
        if active.encoded_bytes > maximum_component_registry_bytes {
            return Err(InternalError::resource_exhausted(format!(
                "active Component Registry requires {} bytes, exceeding protected Component limit {maximum_component_registry_bytes}",
                active.encoded_bytes
            )));
        }
        let encoded_bytes = current
            .encoded_bytes
            .checked_sub(prepared.encoded_bytes)
            .and_then(|value| value.checked_add(active.encoded_bytes))
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "root Component Registry byte accounting cannot activate membership",
                )
            })?;
        if encoded_bytes > current.root.limits.maximum_registry_bytes {
            return Err(InternalError::resource_exhausted(
                "active Component Registry exceeds the protected root byte limit",
            ));
        }
        let mut next_meta = current.clone();
        next_meta.encoded_bytes = encoded_bytes;
        RootComponentRegistryStore::replace_component_partition(
            &current,
            next_meta,
            &record,
            next_record.clone(),
            &prepared,
            active.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok((
            allocation_record_to_view(next_record),
            partition_record_to_view(active),
        ))
    }

    pub(crate) fn mark_membership_synchronized(
        operation_id: [u8; 32],
        expected_authority_hash: [u8; 32],
    ) -> Result<RootComponentAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
            InternalError::unavailable("Component allocation operation has not been reserved")
        })?;
        let RootComponentAllocationProgressRecord::Committed {
            creation,
            canister,
            installation,
            commitment,
        } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component allocation is not committed for membership synchronization",
            ));
        };
        let membership = commitment.membership.as_ref().ok_or_else(|| {
            InternalError::conflict("Component Registry membership has not been activated")
        })?;
        let _active = exact_active_partition(&record, commitment, membership)?;
        if membership.directory_authority_hash != expected_authority_hash {
            return Err(InternalError::conflict(
                "current Component Directory differs from its active membership authority",
            ));
        }
        if membership.directory_synchronized {
            return Ok(allocation_record_to_view(record));
        }

        let mut next_membership = membership.clone();
        next_membership.directory_synchronized = true;
        let mut next_record = record.clone();
        next_record.progress = RootComponentAllocationProgressRecord::Committed {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
            commitment: RootComponentCommitmentRecord {
                registry: commitment.registry.clone(),
                prepared_registry_encoded_bytes: commitment.prepared_registry_encoded_bytes,
                directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
                directory_authority_hash: commitment.directory_authority_hash,
                directory_prepared: commitment.directory_prepared,
                runtime_activated: commitment.runtime_activated,
                membership: Some(next_membership),
            },
        };
        validate_charged_record_size(&next_record, installation.charged_entry_bytes)?;
        if RootComponentRegistryStore::allocation_entry_bytes(&next_record)
            != RootComponentRegistryStore::allocation_entry_bytes(&record)
        {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component membership receipt changed its precharged stable footprint",
            ));
        }
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
            RootComponentAllocationProgressRecord::InstallIntent {
                creation,
                canister,
                installation,
            } => RootComponentAllocationProgressView::InstallIntent {
                creation: creation_effect_record_to_view(creation),
                canister,
                installation: install_effect_record_to_view(installation),
            },
            RootComponentAllocationProgressRecord::Installed {
                creation,
                canister,
                installation,
            } => RootComponentAllocationProgressView::Installed {
                creation: creation_effect_record_to_view(creation),
                canister,
                installation: install_effect_record_to_view(installation),
            },
            RootComponentAllocationProgressRecord::Verified {
                creation,
                canister,
                installation,
            } => RootComponentAllocationProgressView::Verified {
                creation: creation_effect_record_to_view(creation),
                canister,
                installation: install_effect_record_to_view(installation),
            },
            RootComponentAllocationProgressRecord::Committed {
                creation,
                canister,
                installation,
                commitment,
            } => RootComponentAllocationProgressView::Committed {
                creation: creation_effect_record_to_view(creation),
                canister,
                installation: install_effect_record_to_view(installation),
                commitment: commitment_record_to_view(commitment),
            },
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

fn install_effect_record_to_view(
    effect: RootComponentInstallEffectRecord,
) -> RootComponentInstallEffectView {
    RootComponentInstallEffectView {
        raw_module_hash: effect.raw_module_hash,
        chunk_hashes: effect.chunk_hashes,
        binding: effect.binding,
        cost_guard_settlement: effect.cost_guard_settlement,
        charged_entry_bytes: effect.charged_entry_bytes,
    }
}

fn commitment_record_to_view(
    commitment: RootComponentCommitmentRecord,
) -> RootComponentCommitmentView {
    RootComponentCommitmentView {
        registry: commitment.registry,
        prepared_registry_encoded_bytes: commitment.prepared_registry_encoded_bytes,
        directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
        directory_authority_hash: commitment.directory_authority_hash,
        directory_prepared: commitment.directory_prepared,
        runtime_activated: commitment.runtime_activated,
        membership: commitment.membership.map(membership_record_to_view),
    }
}

const fn membership_record_to_view(
    membership: RootComponentMembershipRecord,
) -> RootComponentMembershipView {
    RootComponentMembershipView {
        registry_encoded_bytes: membership.registry_encoded_bytes,
        directory_synchronized_at_ns: membership.directory_synchronized_at_ns,
        directory_authority_hash: membership.directory_authority_hash,
        directory_synchronized: membership.directory_synchronized,
    }
}

fn partition_record_to_view(
    record: ComponentRegistryPartitionRecord,
) -> ComponentRegistryPartitionView {
    ComponentRegistryPartitionView {
        binding: record.binding,
        provisioning_origin: record.provisioning_origin,
        release_set: record.release_set,
        status: record.status,
        revision: record.revision,
        content_hash: record.content_hash,
        directory_synchronized_at_ns: record.directory_synchronized_at_ns,
        encoded_bytes: record.encoded_bytes,
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

fn install_charged_entry_bytes(
    record: &RootComponentAllocationRecord,
    plan: &RootComponentInstallPlan,
) -> Result<u64, InternalError> {
    let (creation, canister) = match &record.progress {
        RootComponentAllocationProgressRecord::Created { effect, canister } => {
            (effect.clone(), *canister)
        }
        _ => {
            return Err(InternalError::conflict(
                "Component allocation is not ready for installation",
            ));
        }
    };
    let mut maximum = record.clone();
    let installation = RootComponentInstallEffectRecord {
        raw_module_hash: plan.raw_module_hash,
        chunk_hashes: plan.chunk_hashes.clone(),
        binding: plan.binding.clone(),
        cost_guard_settlement: ReplayCostGuardSettlement {
            quota_intent_id: IntentId(u64::MAX),
            reservation_intent_id: IntentId(u64::MAX),
        },
        charged_entry_bytes: u64::MAX,
    };
    let registry = ComponentRegistryHead {
        component: record.component,
        revision: 1,
        content_hash: component_partition_content_hash(
            &plan.binding,
            &record.provisioning_origin,
            record.release_set,
            ComponentLifecycleStatus::Prepared,
            1,
        )?,
    };
    maximum.progress = RootComponentAllocationProgressRecord::Committed {
        creation,
        canister,
        installation,
        commitment: RootComponentCommitmentRecord {
            registry,
            prepared_registry_encoded_bytes: u64::MAX,
            directory_synchronized_at_ns: u64::MAX,
            directory_authority_hash: [u8::MAX; 32],
            directory_prepared: true,
            runtime_activated: true,
            membership: Some(RootComponentMembershipRecord {
                registry_encoded_bytes: u64::MAX,
                directory_synchronized_at_ns: u64::MAX,
                directory_authority_hash: [u8::MAX; 32],
                directory_synchronized: true,
            }),
        },
    };
    let partition = ComponentRegistryPartitionRecord {
        binding: plan.binding.clone(),
        provisioning_origin: record.provisioning_origin.clone(),
        release_set: record.release_set,
        status: ComponentLifecycleStatus::Active,
        revision: u64::MAX,
        content_hash: [u8::MAX; 32],
        directory_synchronized_at_ns: u64::MAX,
        encoded_bytes: u64::MAX,
    };
    let charged = RootComponentRegistryStore::allocation_entry_bytes(&maximum)
        .checked_add(RootComponentRegistryStore::partition_entry_bytes(
            &partition,
        ))
        .and_then(|value| {
            value.checked_add(RootComponentRegistryStore::principal_index_entry_bytes(
                plan.binding.canister_id,
                record.component,
            ))
        })
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))?;
    if charged > plan.maximum_registry_bytes {
        return Err(InternalError::resource_exhausted(format!(
            "Component Registry commitment requires {charged} bytes, exceeding protected Component limit {}",
            plan.maximum_registry_bytes
        )));
    }
    Ok(charged)
}

fn committed_records(
    record: &RootComponentAllocationRecord,
    creation: &RootComponentCreationEffectRecord,
    canister: Principal,
    installation: &RootComponentInstallEffectRecord,
    directory_synchronized_at_ns: u64,
    fleet_directory: &FleetDirectorySnapshot,
) -> Result<
    (
        RootComponentAllocationRecord,
        ComponentRegistryPartitionRecord,
    ),
    InternalError,
> {
    let revision = 1;
    let content_hash = component_partition_content_hash(
        &installation.binding,
        &record.provisioning_origin,
        record.release_set,
        ComponentLifecycleStatus::Prepared,
        revision,
    )?;
    let registry = ComponentRegistryHead {
        component: record.component,
        revision,
        content_hash,
    };
    let directory = ComponentDirectoryHead {
        provenance: ComponentDirectoryProvenance {
            component: installation.binding.clone(),
            source_fleet_subnet_root: installation.binding.fleet_subnet_root,
            component_registry_revision: registry.revision,
            component_registry_content_hash: registry.content_hash,
            synchronized_at_ns: directory_synchronized_at_ns,
        },
        descendant_count: 0,
    };
    let directory_authority_hash =
        ComponentRuntimeOps::directory_authority_hash(&ComponentRuntimeDirectoryAuthority {
            fleet: fleet_directory.clone(),
            component: directory,
        })?;
    let mut next_record = record.clone();
    next_record.progress = RootComponentAllocationProgressRecord::Committed {
        creation: creation.clone(),
        canister,
        installation: installation.clone(),
        commitment: RootComponentCommitmentRecord {
            registry,
            prepared_registry_encoded_bytes: 0,
            directory_synchronized_at_ns,
            directory_authority_hash,
            directory_prepared: false,
            runtime_activated: false,
            membership: None,
        },
    };
    let mut partition = ComponentRegistryPartitionRecord {
        binding: installation.binding.clone(),
        provisioning_origin: record.provisioning_origin.clone(),
        release_set: record.release_set,
        status: ComponentLifecycleStatus::Prepared,
        revision,
        content_hash,
        directory_synchronized_at_ns,
        encoded_bytes: 0,
    };
    let index_bytes = RootComponentRegistryStore::principal_index_entry_bytes(
        installation.binding.canister_id,
        record.component,
    );
    for _ in 0..8 {
        let operation_bytes = RootComponentRegistryStore::allocation_entry_bytes(&next_record);
        let encoded_bytes = operation_bytes
            .checked_add(RootComponentRegistryStore::partition_entry_bytes(
                &partition,
            ))
            .and_then(|value| value.checked_add(index_bytes))
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let RootComponentAllocationProgressRecord::Committed { commitment, .. } =
            &mut next_record.progress
        else {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
                "new Component commitment changed phase during byte accounting",
            ));
        };
        if partition.encoded_bytes == encoded_bytes
            && commitment.prepared_registry_encoded_bytes == encoded_bytes
        {
            return Ok((next_record, partition));
        }
        partition.encoded_bytes = encoded_bytes;
        commitment.prepared_registry_encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component Registry partition byte accounting did not converge",
    ))
}

fn active_membership_records(
    record: &RootComponentAllocationRecord,
    commitment: &RootComponentCommitmentRecord,
    directory_synchronized_at_ns: u64,
    fleet_directory: &FleetDirectorySnapshot,
) -> Result<
    (
        RootComponentAllocationRecord,
        ComponentRegistryPartitionRecord,
    ),
    InternalError,
> {
    let RootComponentAllocationProgressRecord::Committed {
        creation,
        canister,
        installation,
        ..
    } = &record.progress
    else {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "membership activation requires a committed Component allocation",
        ));
    };
    let revision =
        commitment.registry.revision.checked_add(1).ok_or_else(|| {
            InternalError::resource_exhausted("Component Registry revision overflow")
        })?;
    let content_hash = component_partition_content_hash(
        &installation.binding,
        &record.provisioning_origin,
        record.release_set,
        ComponentLifecycleStatus::Active,
        revision,
    )?;
    let directory_authority_hash = component_directory_authority_hash(
        &installation.binding,
        revision,
        content_hash,
        directory_synchronized_at_ns,
        fleet_directory,
    )?;
    let mut next_record = record.clone();
    let mut active = ComponentRegistryPartitionRecord {
        binding: installation.binding.clone(),
        provisioning_origin: record.provisioning_origin.clone(),
        release_set: record.release_set,
        status: ComponentLifecycleStatus::Active,
        revision,
        content_hash,
        directory_synchronized_at_ns,
        encoded_bytes: 0,
    };
    next_record.progress = RootComponentAllocationProgressRecord::Committed {
        creation: creation.clone(),
        canister: *canister,
        installation: installation.clone(),
        commitment: RootComponentCommitmentRecord {
            registry: commitment.registry.clone(),
            prepared_registry_encoded_bytes: commitment.prepared_registry_encoded_bytes,
            directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
            directory_authority_hash: commitment.directory_authority_hash,
            directory_prepared: commitment.directory_prepared,
            runtime_activated: commitment.runtime_activated,
            membership: Some(RootComponentMembershipRecord {
                registry_encoded_bytes: 0,
                directory_synchronized_at_ns,
                directory_authority_hash,
                directory_synchronized: false,
            }),
        },
    };
    let index_bytes = RootComponentRegistryStore::principal_index_entry_bytes(
        installation.binding.canister_id,
        record.component,
    );
    for _ in 0..8 {
        let encoded_bytes = RootComponentRegistryStore::allocation_entry_bytes(&next_record)
            .checked_add(RootComponentRegistryStore::partition_entry_bytes(&active))
            .and_then(|value| value.checked_add(index_bytes))
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let RootComponentAllocationProgressRecord::Committed { commitment, .. } =
            &mut next_record.progress
        else {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
                "active Component commitment changed phase during byte accounting",
            ));
        };
        let membership = commitment.membership.as_mut().ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
                "active Component commitment lost membership during byte accounting",
            )
        })?;
        if active.encoded_bytes == encoded_bytes
            && membership.registry_encoded_bytes == encoded_bytes
        {
            return Ok((next_record, active));
        }
        active.encoded_bytes = encoded_bytes;
        membership.registry_encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "active Component Registry byte accounting did not converge",
    ))
}

fn component_directory_authority_hash(
    binding: &ComponentBinding,
    revision: u64,
    content_hash: [u8; 32],
    synchronized_at_ns: u64,
    fleet_directory: &FleetDirectorySnapshot,
) -> Result<[u8; 32], InternalError> {
    ComponentRuntimeOps::directory_authority_hash(&ComponentRuntimeDirectoryAuthority {
        fleet: fleet_directory.clone(),
        component: ComponentDirectoryHead {
            provenance: ComponentDirectoryProvenance {
                component: binding.clone(),
                source_fleet_subnet_root: binding.fleet_subnet_root,
                component_registry_revision: revision,
                component_registry_content_hash: content_hash,
                synchronized_at_ns,
            },
            descendant_count: 0,
        },
    })
}

fn exact_committed_partition(
    record: &RootComponentAllocationRecord,
    commitment: &RootComponentCommitmentRecord,
) -> Result<ComponentRegistryPartitionRecord, InternalError> {
    let RootComponentAllocationProgressRecord::Committed { installation, .. } = &record.progress
    else {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "Component partition validation requires a committed allocation",
        ));
    };
    let current = RootComponentRegistryStore::partition(record.component).ok_or_else(|| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "committed Component allocation has no Registry partition",
        )
    })?;
    let prepared = ComponentRegistryPartitionRecord {
        binding: installation.binding.clone(),
        provisioning_origin: record.provisioning_origin.clone(),
        release_set: record.release_set,
        status: ComponentLifecycleStatus::Prepared,
        revision: commitment.registry.revision,
        content_hash: commitment.registry.content_hash,
        directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
        encoded_bytes: commitment.prepared_registry_encoded_bytes,
    };
    if prepared.binding.component != record.component
        || commitment.registry.component != record.component
        || RootComponentRegistryStore::component_for_principal(prepared.binding.canister_id)
            != Some(record.component)
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "committed Component allocation differs from its prepared Registry receipt",
        ));
    }
    validate_partition_record(&prepared)?;
    match &commitment.membership {
        None if current == prepared => {}
        Some(membership) => {
            let _active = validate_active_partition(record, commitment, membership, &current)?;
        }
        None => {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "current Component partition differs from its prepared Registry receipt",
            ));
        }
    }
    Ok(prepared)
}

fn exact_active_partition(
    record: &RootComponentAllocationRecord,
    commitment: &RootComponentCommitmentRecord,
    membership: &RootComponentMembershipRecord,
) -> Result<ComponentRegistryPartitionRecord, InternalError> {
    let current = RootComponentRegistryStore::partition(record.component).ok_or_else(|| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "active Component allocation has no Registry partition",
        )
    })?;
    validate_active_partition(record, commitment, membership, &current)
}

fn validate_active_partition(
    record: &RootComponentAllocationRecord,
    commitment: &RootComponentCommitmentRecord,
    membership: &RootComponentMembershipRecord,
    current: &ComponentRegistryPartitionRecord,
) -> Result<ComponentRegistryPartitionRecord, InternalError> {
    let expected_revision =
        commitment.registry.revision.checked_add(1).ok_or_else(|| {
            InternalError::resource_exhausted("Component Registry revision overflow")
        })?;
    let registry_encoded_bytes_match = membership.registry_encoded_bytes == current.encoded_bytes;
    if !commitment.directory_prepared
        || !commitment.runtime_activated
        || !registry_encoded_bytes_match
        || membership.directory_synchronized_at_ns <= commitment.directory_synchronized_at_ns
        || membership.directory_synchronized_at_ns != current.directory_synchronized_at_ns
        || membership.directory_authority_hash == [0; 32]
        || current.binding.component != record.component
        || current.status != ComponentLifecycleStatus::Active
        || current.revision != expected_revision
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "active Component partition differs from its immutable membership receipt",
        ));
    }
    validate_partition_record(current)?;
    Ok(current.clone())
}

fn validate_membership_directory_authority_hash(
    partition: &ComponentRegistryPartitionRecord,
    fleet_directory: &FleetDirectorySnapshot,
    membership: &RootComponentMembershipRecord,
) -> Result<(), InternalError> {
    let authority = ComponentRuntimeDirectoryAuthority {
        fleet: fleet_directory.clone(),
        component: ComponentDirectoryHead {
            provenance: ComponentDirectoryProvenance {
                component: partition.binding.clone(),
                source_fleet_subnet_root: partition.binding.fleet_subnet_root,
                component_registry_revision: partition.revision,
                component_registry_content_hash: partition.content_hash,
                synchronized_at_ns: partition.directory_synchronized_at_ns,
            },
            descendant_count: 0,
        },
    };
    if ComponentRuntimeOps::directory_authority_hash(&authority)?
        != membership.directory_authority_hash
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "active Component Directory authority differs from its membership receipt",
        ));
    }
    Ok(())
}

fn validate_directory_authority_hash(
    partition: &ComponentRegistryPartitionRecord,
    fleet_directory: &FleetDirectorySnapshot,
    commitment: &RootComponentCommitmentRecord,
) -> Result<(), InternalError> {
    let authority = ComponentRuntimeDirectoryAuthority {
        fleet: fleet_directory.clone(),
        component: ComponentDirectoryHead {
            provenance: ComponentDirectoryProvenance {
                component: partition.binding.clone(),
                source_fleet_subnet_root: partition.binding.fleet_subnet_root,
                component_registry_revision: partition.revision,
                component_registry_content_hash: partition.content_hash,
                synchronized_at_ns: partition.directory_synchronized_at_ns,
            },
            descendant_count: 0,
        },
    };
    if ComponentRuntimeOps::directory_authority_hash(&authority)?
        != commitment.directory_authority_hash
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "committed Component Directory authority hash differs from current Registry authority",
        ));
    }
    Ok(())
}

fn validate_partition_record(
    partition: &ComponentRegistryPartitionRecord,
) -> Result<(), InternalError> {
    if partition.revision == 0
        || partition.directory_synchronized_at_ns == 0
        || partition.content_hash
            != component_partition_content_hash(
                &partition.binding,
                &partition.provisioning_origin,
                partition.release_set,
                partition.status,
                partition.revision,
            )?
        || RootComponentRegistryStore::component_for_principal(partition.binding.canister_id)
            != Some(partition.binding.component)
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component Registry partition has invalid head, Directory time or principal index",
        ));
    }
    Ok(())
}

fn component_partition_content_hash(
    binding: &ComponentBinding,
    provisioning_origin: &ComponentProvisioningOrigin,
    release_set: FleetSubnetRootReleaseSet,
    status: ComponentLifecycleStatus,
    revision: u64,
) -> Result<[u8; 32], InternalError> {
    const DOMAIN: &[u8] = b"canic.component-registry.partition.v1";
    let payload = candid::encode_one((
        binding.clone(),
        provisioning_origin.clone(),
        release_set,
        status,
        revision,
    ))
    .map_err(|error| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            format!("Component Registry hash input cannot be encoded: {error}"),
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    Ok(hasher.finalize().into())
}

fn validate_install_capacity(
    current: &RootComponentRegistryMetaRecord,
    record: &RootComponentAllocationRecord,
    charged_entry_bytes: u64,
) -> Result<u64, InternalError> {
    let current_reserved_bytes = match &record.progress {
        RootComponentAllocationProgressRecord::Created { effect, .. } => effect.charged_entry_bytes,
        _ => {
            return Err(InternalError::conflict(
                "Component allocation is not ready to reserve install capacity",
            ));
        }
    };
    let without_current = current
        .encoded_bytes
        .checked_sub(current_reserved_bytes)
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Registry encoded-byte accounting is below its creation reservation",
            )
        })?;
    let next_encoded_bytes = without_current
        .checked_add(charged_entry_bytes)
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))?;
    if next_encoded_bytes > current.root.limits.maximum_registry_bytes {
        return Err(InternalError::resource_exhausted(format!(
            "Component installation evidence requires {next_encoded_bytes} bytes, exceeding protected limit {}",
            current.root.limits.maximum_registry_bytes
        )));
    }
    Ok(next_encoded_bytes)
}

fn validate_install_effect_record(
    effect: &RootComponentInstallEffectRecord,
    plan: &RootComponentInstallPlan,
) -> Result<(), InternalError> {
    if effect.raw_module_hash != plan.raw_module_hash
        || effect.chunk_hashes != plan.chunk_hashes
        || effect.binding != plan.binding
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "durable Component install intent differs from verified module or binding authority",
        ));
    }
    Ok(())
}

fn advance_install_phase(
    operation_id: [u8; 32],
    verified: bool,
) -> Result<RootComponentAllocationView, InternalError> {
    let current = RootComponentRegistryStore::current().ok_or_else(|| {
        InternalError::unavailable("root Component Registry authority has not been prepared")
    })?;
    let record = RootComponentRegistryStore::allocation(operation_id).ok_or_else(|| {
        InternalError::unavailable("Component allocation operation has not been reserved")
    })?;
    let next_progress = match (&record.progress, verified) {
        (
            RootComponentAllocationProgressRecord::InstallIntent {
                creation,
                canister,
                installation,
            },
            false,
        ) => RootComponentAllocationProgressRecord::Installed {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
        },
        (RootComponentAllocationProgressRecord::Installed { .. }, false)
        | (
            RootComponentAllocationProgressRecord::Verified { .. }
            | RootComponentAllocationProgressRecord::Committed { .. },
            _,
        ) => {
            return Ok(allocation_record_to_view(record));
        }
        (
            RootComponentAllocationProgressRecord::Installed {
                creation,
                canister,
                installation,
            },
            true,
        ) => RootComponentAllocationProgressRecord::Verified {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
        },
        _ => {
            return Err(InternalError::conflict(if verified {
                "Component allocation has not recorded successful installation"
            } else {
                "Component allocation has no durable install intent"
            }));
        }
    };
    let charged_entry_bytes = match &next_progress {
        RootComponentAllocationProgressRecord::Installed { installation, .. }
        | RootComponentAllocationProgressRecord::Verified { installation, .. } => {
            installation.charged_entry_bytes
        }
        _ => unreachable!(),
    };
    let mut next_record = record.clone();
    next_record.progress = next_progress;
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
            "Component allocation record exceeds its pre-effect Registry byte charge",
        ));
    }
    Ok(())
}

fn map_allocation_commit_error(error: RootComponentAllocationCommitError) -> InternalError {
    match error {
        RootComponentAllocationCommitError::ComponentIdentityConflict => InternalError::conflict(
            "derived Component identity is already reserved by another operation",
        ),
        RootComponentAllocationCommitError::ComponentPrincipalConflict => InternalError::conflict(
            "Component Canister principal is already indexed by another Registry partition",
        ),
        RootComponentAllocationCommitError::ConflictingPartition => InternalError::conflict(
            "Component Registry partition is already committed under different authority",
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
            component_registry::ComponentProvisioningOrigin,
            fleet_registry::{
                FleetDirectoryProvenance, FleetDirectorySnapshot, FleetRegistryVersion,
                FleetSubnetRootDirectoryEntry, FleetSubnetRootStatus,
            },
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
            plan,
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

    #[test]
    fn install_intent_reserves_terminal_bytes_and_advances_idempotently() {
        let (root, created, canister) = prepared_created_allocation();
        let created_bytes = ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes;
        let plan = RootComponentInstallPlan {
            raw_module_hash: [20; 32],
            chunk_hashes: vec![vec![21; 32], vec![22; 32]],
            binding: ComponentBinding {
                authority: root.authority.clone(),
                component: created.component,
                component_spec: created.component_spec.clone(),
                spec_hash: created.spec_hash,
                role: created.role,
                placement_subnet: root.placement_subnet,
                fleet_subnet_root: root.fleet_subnet_root,
                canister_id: canister,
            },
            maximum_registry_bytes: 16_777_216,
        };

        let mut component_exhausted = plan.clone();
        component_exhausted.maximum_registry_bytes = 1;
        let capacity_error =
            ComponentRegistryOps::validate_install_capacity([12; 32], &component_exhausted)
                .expect_err("terminal Component partition must fit before installation");
        assert!(capacity_error.is_public_resource_exhausted());
        assert!(matches!(
            ComponentRegistryOps::allocation([12; 32])
                .expect("created allocation")
                .progress,
            RootComponentAllocationProgressView::Created { .. }
        ));

        let intent_bytes = advance_install_to_verified(&plan, created_bytes);

        let directory = fleet_directory(&root);
        let (committed, partition) = ComponentRegistryOps::commit_verified(
            [12; 32],
            31,
            plan.maximum_registry_bytes,
            directory.clone(),
        )
        .expect("commit verified Component");
        let interrupted = RootComponentRegistryStore::export();
        RootComponentRegistryStore::import(interrupted);
        let repeated = ComponentRegistryOps::commit_verified(
            [12; 32],
            32,
            plan.maximum_registry_bytes,
            directory,
        )
        .expect("exact commitment retry");
        assert_eq!(repeated, (committed.clone(), partition.clone()));
        assert!(matches!(
            committed.progress,
            RootComponentAllocationProgressView::Committed { .. }
        ));
        assert_eq!(partition.binding, plan.binding);
        assert_eq!(partition.status, ComponentLifecycleStatus::Prepared);
        assert_eq!(partition.revision, 1);
        assert_ne!(partition.content_hash, [0; 32]);
        assert_eq!(partition.directory_synchronized_at_ns, 31);
        assert_eq!(
            ComponentRegistryOps::component_for_principal(canister),
            Some(committed.component)
        );
        assert_eq!(
            ComponentRegistryOps::partition(committed.component)
                .expect("valid partition")
                .expect("committed partition"),
            partition
        );
        let status = ComponentRegistryOps::current().expect("Registry status");
        assert_eq!(status.reserved_component_instances, 0);
        assert_eq!(status.committed_component_instances, 1);
        assert_eq!(status.managed_descendants, 0);
        assert_eq!(status.encoded_bytes, partition.encoded_bytes);
        assert!(status.encoded_bytes <= intent_bytes);
        assert_eq!(
            ComponentRegistryOps::component_spec_counts(&committed.component_spec)
                .expect("Spec counts"),
            ComponentSpecInstanceCounts {
                reserved: 0,
                committed: 1,
            }
        );
        assert_directory_preparation_receipt(
            &committed,
            &partition,
            fleet_directory(&root),
            plan.maximum_registry_bytes,
        );
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    fn assert_directory_preparation_receipt(
        committed: &RootComponentAllocationView,
        prepared_partition: &ComponentRegistryPartitionView,
        directory: FleetDirectorySnapshot,
        maximum_component_registry_bytes: u64,
    ) {
        let RootComponentAllocationProgressView::Committed { commitment, .. } = &committed.progress
        else {
            panic!("committed allocation progress");
        };
        assert_ne!(commitment.directory_authority_hash, [0; 32]);
        assert_eq!(
            commitment.prepared_registry_encoded_bytes,
            prepared_partition.encoded_bytes
        );
        assert!(!commitment.directory_prepared);
        assert!(!commitment.runtime_activated);
        assert!(
            ComponentRegistryOps::mark_runtime_activated(
                [12; 32],
                commitment.directory_authority_hash,
            )
            .is_err()
        );
        let prepared = ComponentRegistryOps::mark_directory_prepared(
            [12; 32],
            commitment.directory_authority_hash,
        )
        .expect("mark Directory prepared");
        let prepared_again = ComponentRegistryOps::mark_directory_prepared(
            [12; 32],
            commitment.directory_authority_hash,
        )
        .expect("retry Directory receipt");
        assert_eq!(prepared_again, prepared);
        assert!(matches!(
            &prepared.progress,
            RootComponentAllocationProgressView::Committed {
                commitment: RootComponentCommitmentView {
                    directory_prepared: true,
                    runtime_activated: false,
                    ..
                },
                ..
            }
        ));
        let activated = ComponentRegistryOps::mark_runtime_activated(
            [12; 32],
            commitment.directory_authority_hash,
        )
        .expect("mark runtime activated");
        let activated_again = ComponentRegistryOps::mark_runtime_activated(
            [12; 32],
            commitment.directory_authority_hash,
        )
        .expect("retry runtime activation receipt");
        assert_eq!(activated_again, activated);
        assert!(matches!(
            &activated.progress,
            RootComponentAllocationProgressView::Committed {
                commitment: RootComponentCommitmentView {
                    directory_prepared: true,
                    runtime_activated: true,
                    membership: None,
                    ..
                },
                ..
            }
        ));
        assert_membership_receipt(
            &activated,
            prepared_partition,
            directory,
            maximum_component_registry_bytes,
        );
    }

    fn assert_membership_receipt(
        activated: &RootComponentAllocationView,
        prepared_partition: &ComponentRegistryPartitionView,
        directory: FleetDirectorySnapshot,
        maximum_component_registry_bytes: u64,
    ) {
        assert!(committed_membership(activated).is_none());
        let (membership_activated, active_partition) = ComponentRegistryOps::activate_membership(
            [12; 32],
            33,
            maximum_component_registry_bytes,
            directory.clone(),
        )
        .expect("activate Registry membership");
        let repeated_membership = ComponentRegistryOps::activate_membership(
            [12; 32],
            34,
            maximum_component_registry_bytes,
            directory.clone(),
        )
        .expect("repeat Registry membership activation");
        assert_eq!(
            repeated_membership,
            (membership_activated.clone(), active_partition.clone())
        );
        assert_eq!(active_partition.status, ComponentLifecycleStatus::Active);
        assert_eq!(active_partition.revision, 2);
        assert_eq!(active_partition.directory_synchronized_at_ns, 33);
        assert_ne!(
            active_partition.content_hash,
            prepared_partition.content_hash
        );
        assert_eq!(
            ComponentRegistryOps::prepared_partition([12; 32])
                .expect("reconstruct prepared partition"),
            *prepared_partition
        );
        assert_eq!(
            ComponentRegistryOps::commit_verified(
                [12; 32],
                35,
                maximum_component_registry_bytes,
                directory,
            )
            .expect("commit retry after membership activation")
            .1,
            *prepared_partition
        );

        let membership_synchronized = ComponentRegistryOps::mark_membership_synchronized(
            [12; 32],
            committed_membership(&membership_activated)
                .expect("membership receipt")
                .directory_authority_hash,
        )
        .expect("mark membership Directory synchronized");
        let synchronized_again = ComponentRegistryOps::mark_membership_synchronized(
            [12; 32],
            committed_membership(&membership_activated)
                .expect("membership receipt")
                .directory_authority_hash,
        )
        .expect("repeat membership Directory receipt");
        assert_eq!(synchronized_again, membership_synchronized);
        assert!(
            committed_membership(&membership_synchronized)
                .expect("membership receipt")
                .directory_synchronized
        );
    }

    fn committed_membership(
        allocation: &RootComponentAllocationView,
    ) -> Option<&RootComponentMembershipView> {
        let RootComponentAllocationProgressView::Committed { commitment, .. } =
            &allocation.progress
        else {
            panic!("committed allocation progress");
        };
        commitment.membership.as_ref()
    }

    fn fleet_directory(root: &FleetSubnetRootBinding) -> FleetDirectorySnapshot {
        FleetDirectorySnapshot {
            provenance: FleetDirectoryProvenance {
                registry: FleetRegistryVersion {
                    authority: root.authority.clone(),
                    revision: 4,
                    content_hash: [5; 32],
                },
                source_fleet_subnet_root: root.fleet_subnet_root,
            },
            fleet_subnet_roots: vec![FleetSubnetRootDirectoryEntry {
                placement_subnet: root.placement_subnet,
                fleet_subnet_root: root.fleet_subnet_root,
                status: FleetSubnetRootStatus::Active,
            }],
        }
    }

    fn advance_install_to_verified(plan: &RootComponentInstallPlan, created_bytes: u64) -> u64 {
        ComponentRegistryOps::validate_install_capacity([12; 32], plan).expect("install capacity");
        let intent = ComponentRegistryOps::begin_install(
            [12; 32],
            plan.clone(),
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(23),
                reservation_intent_id: IntentId(24),
            },
        )
        .expect("install intent");
        let intent_bytes = ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes;
        assert!(intent_bytes > created_bytes);
        assert!(matches!(
            intent.progress,
            RootComponentAllocationProgressView::InstallIntent { .. }
        ));

        let mut conflicting = plan.clone();
        conflicting.raw_module_hash = [25; 32];
        assert!(
            ComponentRegistryOps::renew_install_intent(
                [12; 32],
                &conflicting,
                ReplayCostGuardSettlement {
                    quota_intent_id: IntentId(26),
                    reservation_intent_id: IntentId(27),
                },
            )
            .is_err()
        );

        let interrupted = RootComponentRegistryStore::export();
        RootComponentRegistryStore::import(interrupted);
        let renewed = ComponentRegistryOps::renew_install_intent(
            [12; 32],
            plan,
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(28),
                reservation_intent_id: IntentId(29),
            },
        )
        .expect("renew exact install intent");
        let RootComponentAllocationProgressView::InstallIntent { installation, .. } =
            &renewed.progress
        else {
            panic!("renewed install intent");
        };
        assert_eq!(installation.raw_module_hash, plan.raw_module_hash);
        assert_eq!(installation.binding, plan.binding);
        assert_eq!(
            installation.cost_guard_settlement.quota_intent_id,
            IntentId(28)
        );
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .encoded_bytes,
            intent_bytes
        );

        let installed = ComponentRegistryOps::mark_installed([12; 32]).expect("mark installed");
        let installed_retry =
            ComponentRegistryOps::mark_installed([12; 32]).expect("installed retry");
        assert_eq!(installed, installed_retry);
        assert!(matches!(
            installed.progress,
            RootComponentAllocationProgressView::Installed { .. }
        ));

        let verified = ComponentRegistryOps::mark_verified([12; 32]).expect("mark verified");
        let verified_retry = ComponentRegistryOps::mark_verified([12; 32]).expect("verified retry");
        assert_eq!(verified, verified_retry);
        assert!(matches!(
            verified.progress,
            RootComponentAllocationProgressView::Verified { .. }
        ));
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .encoded_bytes,
            intent_bytes,
            "the install intent must reserve terminal record capacity before the effect"
        );
        intent_bytes
    }

    fn prepared_created_allocation() -> (
        FleetSubnetRootBinding,
        RootComponentAllocationView,
        candid::Principal,
    ) {
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
        let root = root_binding();
        ComponentRegistryOps::prepare(
            root.clone(),
            FleetRegistryVersion {
                authority: root.authority.clone(),
                revision: 4,
                content_hash: [5; 32],
            },
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
        ComponentRegistryOps::begin_creation(
            [12; 32],
            RootComponentCreationPlan {
                wasm_store: candid::Principal::from_slice(&[13; 29]),
                payload_hash: [14; 32],
                payload_size_bytes: 4_096,
                initial_cycles: Cycles::new(5_000_000_000_000),
                controller: root.fleet_subnet_root,
            },
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(16),
                reservation_intent_id: IntentId(17),
            },
        )
        .expect("creation intent");
        let canister = candid::Principal::from_slice(&[18; 29]);
        let created = ComponentRegistryOps::mark_created([12; 32], canister)
            .expect("record created allocation");
        (root, created, canister)
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
