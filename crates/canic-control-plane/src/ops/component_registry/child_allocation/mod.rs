//! Module: ops::component_registry::child_allocation
//!
//! Responsibility: resolve and reserve direct-child Component allocations under one active parent.
//! Does not own: canister effects, workflow ordering, child installation, activation, or retirement.
//! Boundary: commits one exact capacity-checked reservation through the existing Registry store.

use super::{
    ComponentParentRoleIdentity, ComponentRegistryOps, RootComponentChildInstallPlan,
    RootComponentCreationPlan, advance_child_install_phase, canister_is_in_subtree,
    child_allocation_record_to_view, child_creation_capacity, child_creation_charged_entry_bytes,
    child_install_capacity, child_install_charged_entry_bytes, child_reservation_partition,
    map_allocation_commit_error, validate_charged_child_record_size,
    validate_child_allocation_record, validate_child_creation_authority,
    validate_child_install_authority, validate_child_install_effect_record,
    validate_partition_record, validate_subtree_removal_record, validate_subtree_removal_root,
};
use crate::{
    storage::stable::component_registry::{
        ComponentRegistryParentRoleCountRecord, RootComponentChildAllocationProgressRecord,
        RootComponentChildAllocationRecord, RootComponentChildInstallEffectRecord,
        RootComponentCreationEffectRecord, RootComponentRegistryStore,
        RootComponentSubtreeRemovalProgressRecord,
    },
    view::component_registry::RootComponentChildAllocationView,
};
use canic_core::{
    cdk::types::Principal,
    control_plane_support::{
        error::InternalError, model::replay::ReplayCostGuardSettlement,
        policy::component_child_allocation::ComponentChildAllocationDecision,
    },
    dto::component_registry::{ComponentLifecycleStatus, ComponentRegistryHead},
    ids::{CanisterRole, ComponentInstanceId},
};

impl ComponentRegistryOps {
    pub(crate) fn child_allocation(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
    ) -> Result<Option<RootComponentChildAllocationView>, InternalError> {
        let Some(record) = RootComponentRegistryStore::child_allocation(component, operation_id)
        else {
            return Ok(None);
        };
        validate_child_allocation_record(&record)?;
        Ok(Some(child_allocation_record_to_view(record)))
    }

    /// Resolve one direct-child allocation through its domain-owned operation identity.
    pub(crate) fn child_allocation_by_operation(
        operation_id: [u8; 32],
    ) -> Result<Option<RootComponentChildAllocationView>, InternalError> {
        let mut matches = RootComponentRegistryStore::registry_components()
            .into_iter()
            .flat_map(RootComponentRegistryStore::child_allocations)
            .filter(|record| record.operation_id == operation_id);
        let Some(record) = matches.next() else {
            return Ok(None);
        };
        if matches.next().is_some() {
            return Err(InternalError::invariant());
        }
        validate_child_allocation_record(&record)?;
        Ok(Some(child_allocation_record_to_view(record)))
    }

    pub(crate) fn parent_role_instances(
        component: ComponentInstanceId,
        parent_canister_id: Principal,
        child_role: &CanisterRole,
    ) -> Result<u32, InternalError> {
        let Some(record) = RootComponentRegistryStore::parent_role_count(
            component,
            parent_canister_id,
            child_role,
        ) else {
            return Ok(0);
        };
        let expected_identity =
            ComponentParentRoleIdentity::new(component, parent_canister_id, child_role);
        if ComponentParentRoleIdentity::from_count(&record) != expected_identity
            || record.instances == 0
        {
            return Err(InternalError::invariant());
        }
        Ok(record.instances)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one synchronous transaction keeps every child-reservation capacity mutation together"
    )]
    pub(crate) fn reserve_child_allocation(
        decision: ComponentChildAllocationDecision,
        operation_id: [u8; 32],
        application_init_args: Option<Vec<u8>>,
        reserved_against_registry: ComponentRegistryHead,
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let partition = RootComponentRegistryStore::partition(decision.component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        let record = RootComponentChildAllocationRecord {
            operation_id,
            component: decision.component,
            parent_canister_id: decision.parent_canister_id,
            parent_role: decision.parent_role,
            child_role: decision.child_role,
            child_kind: decision.child_kind,
            maximum_instances_per_parent: decision.maximum_instances_per_parent,
            maximum_descendants: decision.maximum_descendants,
            maximum_registry_bytes: decision.maximum_registry_bytes,
            application_init_args,
            reserved_against_registry,
            release_set: current.release_set,
            progress: RootComponentChildAllocationProgressRecord::Reserved,
        };
        if let Some(existing) =
            RootComponentRegistryStore::child_allocation(record.component, operation_id)
        {
            return if existing.has_same_reservation(&record) {
                Ok(child_allocation_record_to_view(existing))
            } else {
                Err(InternalError::conflict())
            };
        }
        let spec_authority_matches = partition.binding.component_spec == decision.component_spec
            && partition.binding.spec_hash == decision.spec_hash;
        let partition_is_active = partition.release_set == current.release_set
            && partition.status == ComponentLifecycleStatus::Active;
        let expected_registry = ComponentRegistryHead {
            component: decision.component,
            revision: partition.revision,
            content_hash: partition.content_hash,
        };
        if !spec_authority_matches
            || !partition_is_active
            || record.reserved_against_registry != expected_registry
        {
            return Err(InternalError::conflict());
        }
        let traversal_limit = partition
            .committed_descendants
            .checked_add(1)
            .ok_or_else(InternalError::resource_exhausted)?;
        for removal in RootComponentRegistryStore::subtree_removals(record.component) {
            validate_subtree_removal_record(&removal)?;
            validate_subtree_removal_root(&removal, &current.root)?;
            if matches!(
                removal.progress,
                RootComponentSubtreeRemovalProgressRecord::Completed(_)
            ) {
                continue;
            }
            if canister_is_in_subtree(
                &partition,
                record.parent_canister_id,
                removal.target.canister_id,
                traversal_limit,
            )? {
                return Err(InternalError::conflict());
            }
        }

        let current_count = RootComponentRegistryStore::parent_role_count(
            record.component,
            record.parent_canister_id,
            &record.child_role,
        );
        let next_count = ComponentRegistryParentRoleCountRecord {
            component: record.component,
            parent_canister_id: record.parent_canister_id,
            child_role: record.child_role.clone(),
            instances: current_count
                .as_ref()
                .map_or(0, |count| count.instances)
                .checked_add(1)
                .ok_or_else(InternalError::resource_exhausted)?,
        };
        if next_count.instances > decision.maximum_instances_per_parent {
            return Err(InternalError::resource_exhausted());
        }
        let (next_partition, registry_delta) =
            child_reservation_partition(&partition, &record, current_count.as_ref(), &next_count)?;
        let component_descendants = next_partition
            .reserved_descendants
            .checked_add(next_partition.committed_descendants)
            .ok_or_else(InternalError::resource_exhausted)?;
        if component_descendants > decision.maximum_descendants {
            return Err(InternalError::resource_exhausted());
        }
        if next_partition.encoded_bytes > decision.maximum_registry_bytes {
            return Err(InternalError::resource_exhausted());
        }
        let mut next_meta = current.clone();
        next_meta.managed_descendants = next_meta
            .managed_descendants
            .checked_add(1)
            .ok_or_else(InternalError::resource_exhausted)?;
        next_meta.encoded_bytes = next_meta
            .encoded_bytes
            .checked_add(registry_delta)
            .ok_or_else(InternalError::resource_exhausted)?;
        if next_meta.encoded_bytes > next_meta.root.limits.maximum_registry_bytes {
            return Err(InternalError::resource_exhausted());
        }

        RootComponentRegistryStore::reserve_child_allocation(
            &current,
            next_meta,
            &partition,
            next_partition,
            record.clone(),
            current_count.as_ref(),
            next_count,
        )
        .map_err(map_allocation_commit_error)?;
        Ok(child_allocation_record_to_view(record))
    }

    pub(crate) fn validate_child_creation_capacity(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        plan: &RootComponentCreationPlan,
    ) -> Result<(), InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        validate_child_creation_authority(&current, &partition, &record, plan)?;
        if !matches!(
            record.progress,
            RootComponentChildAllocationProgressRecord::Reserved
        ) {
            return Err(InternalError::conflict());
        }
        let charged_entry_bytes = child_creation_charged_entry_bytes(&record, plan);
        child_creation_capacity(&current, &partition, &record, charged_entry_bytes).map(|_| ())
    }

    pub(crate) fn begin_child_creation(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        plan: RootComponentCreationPlan,
        cost_guard_settlement: ReplayCostGuardSettlement,
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        validate_child_creation_authority(&current, &partition, &record, &plan)?;
        if !matches!(
            record.progress,
            RootComponentChildAllocationProgressRecord::Reserved
        ) {
            return Err(InternalError::conflict());
        }

        let charged_entry_bytes = child_creation_charged_entry_bytes(&record, &plan);
        let (next_partition, registry_delta) =
            child_creation_capacity(&current, &partition, &record, charged_entry_bytes)?;
        let mut next_record = record.clone();
        next_record.progress = RootComponentChildAllocationProgressRecord::CreationIntent(
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
        validate_charged_child_record_size(&next_record, charged_entry_bytes)?;
        let mut next_meta = current.clone();
        next_meta.encoded_bytes = next_meta
            .encoded_bytes
            .checked_add(registry_delta)
            .ok_or_else(InternalError::resource_exhausted)?;

        RootComponentRegistryStore::replace_child_allocation(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(child_allocation_record_to_view(next_record))
    }

    pub(crate) fn mark_child_created(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        canister: Principal,
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        let effect = match &record.progress {
            RootComponentChildAllocationProgressRecord::CreationIntent(effect) => effect.clone(),
            RootComponentChildAllocationProgressRecord::Created {
                canister: existing, ..
            } if existing == &canister => return Ok(child_allocation_record_to_view(record)),
            RootComponentChildAllocationProgressRecord::InstallIntent {
                canister: existing,
                ..
            }
            | RootComponentChildAllocationProgressRecord::Installed {
                canister: existing, ..
            }
            | RootComponentChildAllocationProgressRecord::Verified {
                canister: existing, ..
            }
            | RootComponentChildAllocationProgressRecord::Committed {
                canister: existing, ..
            } if existing == &canister => return Ok(child_allocation_record_to_view(record)),
            RootComponentChildAllocationProgressRecord::Created { .. }
            | RootComponentChildAllocationProgressRecord::InstallIntent { .. }
            | RootComponentChildAllocationProgressRecord::Installed { .. }
            | RootComponentChildAllocationProgressRecord::Verified { .. }
            | RootComponentChildAllocationProgressRecord::Committed { .. }
            | RootComponentChildAllocationProgressRecord::Reserved => {
                return Err(InternalError::conflict());
            }
        };
        let protected_principals = [
            Principal::anonymous(),
            current.root.fleet_subnet_root,
            current.root.authority.binding.coordinator,
            partition.binding.canister_id,
            record.parent_canister_id,
        ];
        if protected_principals.contains(&canister)
            || RootComponentRegistryStore::component_for_principal(canister).is_some()
        {
            return Err(InternalError::conflict());
        }

        let charged_entry_bytes = effect.charged_entry_bytes;
        let mut next_record = record.clone();
        next_record.progress =
            RootComponentChildAllocationProgressRecord::Created { effect, canister };
        validate_charged_child_record_size(&next_record, charged_entry_bytes)?;
        let mut next_meta = current.clone();
        next_meta.known_created_component_canisters = next_meta
            .known_created_component_canisters
            .checked_add(1)
            .ok_or_else(InternalError::invariant)?;
        let allocated_component_canisters = current
            .reserved_component_instances
            .checked_add(current.committed_component_instances)
            .and_then(|count| count.checked_add(current.managed_descendants))
            .ok_or_else(InternalError::invariant)?;
        if next_meta.known_created_component_canisters > allocated_component_canisters {
            return Err(InternalError::invariant());
        }

        RootComponentRegistryStore::replace_child_allocation(
            &current,
            next_meta,
            &partition,
            partition.clone(),
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(child_allocation_record_to_view(next_record))
    }

    pub(crate) fn validate_child_install_capacity(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        plan: &RootComponentChildInstallPlan,
    ) -> Result<(), InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        validate_child_install_authority(&current, &partition, &record, plan)?;
        if !matches!(
            record.progress,
            RootComponentChildAllocationProgressRecord::Created { .. }
        ) {
            return Err(InternalError::conflict());
        }
        let charged_entry_bytes = child_install_charged_entry_bytes(&record, plan)?;
        child_install_capacity(&current, &partition, &record, charged_entry_bytes).map(|_| ())
    }

    pub(crate) fn begin_child_install(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        plan: RootComponentChildInstallPlan,
        cost_guard_settlement: ReplayCostGuardSettlement,
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        validate_child_install_authority(&current, &partition, &record, &plan)?;
        let (creation, canister) = match &record.progress {
            RootComponentChildAllocationProgressRecord::Created { effect, canister } => {
                (effect.clone(), *canister)
            }
            _ => return Err(InternalError::conflict()),
        };
        let charged_entry_bytes = child_install_charged_entry_bytes(&record, &plan)?;
        let (next_partition, registry_delta) =
            child_install_capacity(&current, &partition, &record, charged_entry_bytes)?;
        let mut next_record = record.clone();
        next_record.progress = RootComponentChildAllocationProgressRecord::InstallIntent {
            creation,
            canister,
            installation: RootComponentChildInstallEffectRecord {
                raw_module_hash: plan.raw_module_hash,
                protocol_profile_digest: plan.protocol_profile_digest,
                chunk_hashes: plan.chunk_hashes,
                binding: plan.binding,
                cost_guard_settlement,
                charged_entry_bytes,
            },
        };
        validate_charged_child_record_size(&next_record, charged_entry_bytes)?;
        let mut next_meta = current.clone();
        next_meta.encoded_bytes = next_meta
            .encoded_bytes
            .checked_add(registry_delta)
            .ok_or_else(InternalError::resource_exhausted)?;

        RootComponentRegistryStore::replace_child_allocation(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(child_allocation_record_to_view(next_record))
    }

    pub(crate) fn renew_child_install_intent(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        plan: &RootComponentChildInstallPlan,
        cost_guard_settlement: ReplayCostGuardSettlement,
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        validate_child_install_authority(&current, &partition, &record, plan)?;
        let (creation, canister, existing) = match &record.progress {
            RootComponentChildAllocationProgressRecord::InstallIntent {
                creation,
                canister,
                installation,
            } => (creation.clone(), *canister, installation),
            _ => return Err(InternalError::conflict()),
        };
        validate_child_install_effect_record(existing, plan)?;
        let charged_entry_bytes = existing.charged_entry_bytes;
        let mut next_record = record.clone();
        next_record.progress = RootComponentChildAllocationProgressRecord::InstallIntent {
            creation,
            canister,
            installation: RootComponentChildInstallEffectRecord {
                raw_module_hash: plan.raw_module_hash,
                protocol_profile_digest: plan.protocol_profile_digest,
                chunk_hashes: plan.chunk_hashes.clone(),
                binding: plan.binding.clone(),
                cost_guard_settlement,
                charged_entry_bytes,
            },
        };
        validate_charged_child_record_size(&next_record, charged_entry_bytes)?;
        RootComponentRegistryStore::replace_child_allocation(
            &current,
            current.clone(),
            &partition,
            partition.clone(),
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(child_allocation_record_to_view(next_record))
    }

    pub(crate) fn mark_child_installed(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        advance_child_install_phase(component, operation_id, false)
    }

    pub(crate) fn mark_child_verified(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        advance_child_install_phase(component, operation_id, true)
    }
}
