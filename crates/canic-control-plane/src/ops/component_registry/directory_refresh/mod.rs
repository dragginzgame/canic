//! Module: ops::component_registry::directory_refresh
//!
//! Responsibility: select, plan, retain, and replay top-level Component Directory refreshes.
//! Does not own: Directory transport, workflow ordering, Fleet publication, or runtime mutation.
//! Boundary: commits one exact pre-journalled Component Registry head through the existing store.

use super::{
    ComponentAllocationPartitionAuthority, ComponentRegistryOps, component_partition_content_hash,
    component_partition_head, map_allocation_commit_error, partition_record_to_view,
    validate_partition_record,
};
use crate::{
    storage::stable::component_registry::{
        ComponentRegistryPartitionRecord, RootComponentAllocationProgressRecord,
        RootComponentRegistryStore,
    },
    view::{
        component_directory_synchronization::{
            RootComponentDirectorySynchronizationIntentView,
            RootComponentDirectorySynchronizationTargetView,
        },
        component_registry::{
            ComponentRegistryPartitionView, RootComponentDirectoryRefreshPlanView,
        },
    },
};
use canic_core::{
    control_plane_support::{error::InternalError, ops::component_runtime::ComponentRuntimeOps},
    dto::{
        component_provisioning::ComponentGroupDirectory,
        component_registry::{
            ComponentDirectoryHead, ComponentDirectoryProvenance, ComponentLifecycleStatus,
            ComponentRegistryHead, ComponentRuntimeDirectoryAuthority,
        },
        fleet_registry::FleetDirectorySnapshot,
    },
    ids::ComponentInstanceId,
};
use std::collections::{BTreeMap, BTreeSet};

impl ComponentRegistryOps {
    /// Resolve exact active top-level members selected by a Fleet-service Directory barrier.
    pub(crate) fn directory_synchronization_targets(
        components: &[ComponentInstanceId],
    ) -> Result<Vec<RootComponentDirectorySynchronizationTargetView>, InternalError> {
        if components.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(InternalError::invalid_input());
        }
        let selected = components.iter().copied().collect::<BTreeSet<_>>();
        let mut allocations = BTreeMap::new();
        for allocation in RootComponentRegistryStore::allocations() {
            if selected.contains(&allocation.component)
                && allocations
                    .insert(allocation.component, allocation)
                    .is_some()
            {
                return Err(InternalError::invariant());
            }
        }
        components
            .iter()
            .map(|component| {
                let partition = RootComponentRegistryStore::partition(*component)
                    .ok_or_else(InternalError::unavailable)?;
                validate_partition_record(&partition)?;
                if partition.status != ComponentLifecycleStatus::Active {
                    return Err(InternalError::conflict());
                }
                let allocation = allocations
                    .remove(component)
                    .ok_or_else(InternalError::invariant)?;
                if ComponentAllocationPartitionAuthority::from_committed_allocation(&allocation)
                    != Some(ComponentAllocationPartitionAuthority::from_partition(
                        &partition,
                    ))
                {
                    return Err(InternalError::invariant());
                }
                let RootComponentAllocationProgressRecord::Committed { commitment, .. } =
                    &allocation.progress
                else {
                    unreachable!("authority comparison accepted only a committed allocation");
                };
                let membership_is_terminal = commitment.runtime_activated
                    && commitment
                        .membership
                        .as_ref()
                        .is_some_and(|membership| membership.directory_synchronized);
                if !membership_is_terminal {
                    return Err(InternalError::conflict());
                }
                Ok(RootComponentDirectorySynchronizationTargetView {
                    component: *component,
                    canister_id: partition.binding.canister_id,
                    allocation_operation_id: allocation.operation_id,
                    source_registry: component_partition_head(&partition),
                })
            })
            .collect()
    }

    /// Derive one exact next Component head under the published Fleet Directory.
    pub(crate) fn prepare_directory_refresh(
        target: &RootComponentDirectorySynchronizationTargetView,
        fleet_directory: FleetDirectorySnapshot,
        component_group: Option<ComponentGroupDirectory>,
        directory_synchronized_at_ns: u64,
    ) -> Result<RootComponentDirectoryRefreshPlanView, InternalError> {
        let partition = RootComponentRegistryStore::partition(target.component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        let current_head = component_partition_head(&partition);
        let baseline_is_covered = current_head.component == target.source_registry.component
            && current_head.revision >= target.source_registry.revision;
        if partition.status != ComponentLifecycleStatus::Active
            || partition.binding.canister_id != target.canister_id
            || !baseline_is_covered
        {
            return Err(InternalError::conflict());
        }
        if directory_synchronized_at_ns <= partition.directory_synchronized_at_ns {
            return Err(InternalError::invalid_input());
        }
        let revision = partition
            .revision
            .checked_add(1)
            .ok_or_else(InternalError::resource_exhausted)?;
        let content_hash = component_partition_content_hash(
            &partition.binding,
            partition.protocol_profile_digest,
            &partition.provisioning_origin,
            partition.release_set,
            partition.status,
            revision,
            partition.descendant_content_hash,
            partition.committed_descendants,
        )?;
        let registry = ComponentRegistryHead {
            component: target.component,
            revision,
            content_hash,
        };
        let authority = ComponentRuntimeDirectoryAuthority {
            fleet: fleet_directory,
            component: ComponentDirectoryHead {
                provenance: ComponentDirectoryProvenance {
                    component: partition.binding.clone(),
                    source_fleet_subnet_root: partition.binding.fleet_subnet_root,
                    component_registry_revision: revision,
                    component_registry_content_hash: content_hash,
                    synchronized_at_ns: directory_synchronized_at_ns,
                },
                descendant_count: partition.committed_descendants,
            },
            component_group,
        };
        let directory_authority_hash = ComponentRuntimeOps::directory_authority_hash(&authority)?;
        Ok(RootComponentDirectoryRefreshPlanView {
            allocation_operation_id: target.allocation_operation_id,
            previous_registry: current_head,
            registry,
            directory_synchronized_at_ns,
            directory_authority_hash,
            authority,
        })
    }

    /// Commit the exact pre-journalled Component head before its runtime call.
    pub(crate) fn commit_directory_refresh(
        plan: &RootComponentDirectoryRefreshPlanView,
        maximum_component_registry_bytes: u64,
    ) -> Result<ComponentRegistryPartitionView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let partition = RootComponentRegistryStore::partition(plan.registry.component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        if component_partition_head(&partition) == plan.registry {
            return Ok(partition_record_to_view(partition));
        }
        if component_partition_head(&partition) != plan.previous_registry
            || partition.status != ComponentLifecycleStatus::Active
        {
            return Err(InternalError::conflict());
        }
        let allocation = RootComponentRegistryStore::allocation(plan.allocation_operation_id)
            .ok_or_else(InternalError::invariant)?;
        if allocation.component != plan.registry.component {
            return Err(InternalError::conflict());
        }
        let mut next_partition = partition.clone();
        next_partition.revision = plan.registry.revision;
        next_partition.content_hash = plan.registry.content_hash;
        next_partition.directory_synchronized_at_ns = plan.directory_synchronized_at_ns;
        converge_directory_refresh_bytes(&partition, &mut next_partition)?;
        if next_partition.encoded_bytes > maximum_component_registry_bytes {
            return Err(InternalError::resource_exhausted());
        }
        let previous_entry_bytes = RootComponentRegistryStore::partition_entry_bytes(&partition);
        let next_entry_bytes = RootComponentRegistryStore::partition_entry_bytes(&next_partition);
        let mut next_meta = current.clone();
        next_meta.encoded_bytes = replace_encoded_bytes(
            current.encoded_bytes,
            previous_entry_bytes,
            next_entry_bytes,
        )?;
        if next_meta.encoded_bytes > current.root.limits.maximum_registry_bytes {
            return Err(InternalError::resource_exhausted());
        }
        RootComponentRegistryStore::replace_component_partition(
            &current,
            next_meta,
            &allocation,
            allocation.clone(),
            &partition,
            next_partition.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(partition_record_to_view(next_partition))
    }

    /// Reconstruct one previously journalled refresh before or after its partition commit.
    pub(crate) fn directory_refresh_plan_for_intent(
        intent: &RootComponentDirectorySynchronizationIntentView,
        fleet_directory: FleetDirectorySnapshot,
        component_group: Option<ComponentGroupDirectory>,
    ) -> Result<RootComponentDirectoryRefreshPlanView, InternalError> {
        let partition = RootComponentRegistryStore::partition(intent.component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        let current = component_partition_head(&partition);
        if current == intent.previous_registry {
            let target = RootComponentDirectorySynchronizationTargetView {
                component: intent.component,
                canister_id: intent.canister_id,
                allocation_operation_id: intent.allocation_operation_id,
                source_registry: intent.previous_registry.clone(),
            };
            let plan = Self::prepare_directory_refresh(
                &target,
                fleet_directory,
                component_group,
                intent.directory_synchronized_at_ns,
            )?;
            validate_refresh_plan_against_intent(&plan, intent)?;
            return Ok(plan);
        }
        if current != intent.registry
            || partition.binding.canister_id != intent.canister_id
            || partition.directory_synchronized_at_ns != intent.directory_synchronized_at_ns
        {
            return Err(InternalError::conflict());
        }
        let authority = ComponentRuntimeDirectoryAuthority {
            fleet: fleet_directory,
            component: ComponentDirectoryHead {
                provenance: ComponentDirectoryProvenance {
                    component: partition.binding.clone(),
                    source_fleet_subnet_root: partition.binding.fleet_subnet_root,
                    component_registry_revision: partition.revision,
                    component_registry_content_hash: partition.content_hash,
                    synchronized_at_ns: partition.directory_synchronized_at_ns,
                },
                descendant_count: partition.committed_descendants,
            },
            component_group,
        };
        let plan = RootComponentDirectoryRefreshPlanView {
            allocation_operation_id: intent.allocation_operation_id,
            previous_registry: intent.previous_registry.clone(),
            registry: intent.registry.clone(),
            directory_synchronized_at_ns: intent.directory_synchronized_at_ns,
            directory_authority_hash: ComponentRuntimeOps::directory_authority_hash(&authority)?,
            authority,
        };
        validate_refresh_plan_against_intent(&plan, intent)?;
        Ok(plan)
    }
}

fn converge_directory_refresh_bytes(
    current: &ComponentRegistryPartitionRecord,
    next: &mut ComponentRegistryPartitionRecord,
) -> Result<(), InternalError> {
    let current_entry_bytes = RootComponentRegistryStore::partition_entry_bytes(current);
    for _ in 0..8 {
        let next_entry_bytes = RootComponentRegistryStore::partition_entry_bytes(next);
        let encoded_bytes =
            replace_encoded_bytes(current.encoded_bytes, current_entry_bytes, next_entry_bytes)?;
        if next.encoded_bytes == encoded_bytes {
            return Ok(());
        }
        next.encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant())
}

fn validate_refresh_plan_against_intent(
    plan: &RootComponentDirectoryRefreshPlanView,
    intent: &RootComponentDirectorySynchronizationIntentView,
) -> Result<(), InternalError> {
    let exact = [
        plan.allocation_operation_id == intent.allocation_operation_id,
        plan.previous_registry == intent.previous_registry,
        plan.registry == intent.registry,
        plan.directory_synchronized_at_ns == intent.directory_synchronized_at_ns,
        plan.directory_authority_hash == intent.directory_authority_hash,
    ]
    .into_iter()
    .all(|matches| matches);
    if !exact {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn replace_encoded_bytes(
    current: u64,
    previous_entry: u64,
    next_entry: u64,
) -> Result<u64, InternalError> {
    current
        .checked_sub(previous_entry)
        .and_then(|remaining| remaining.checked_add(next_entry))
        .ok_or_else(InternalError::invariant)
}
