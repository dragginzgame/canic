//! Module: ops::component_registry::child_activation
//!
//! Responsibility: commit and converge one verified direct child into active membership.
//! Does not own: runtime calls, Directory transport, workflow ordering, allocation, or retirement.
//! Boundary: persists exact child Registry, activation, membership, and synchronization transitions.

use super::{
    ComponentDirectoryAuthorityInput, ComponentRegistryOps, child_allocation_record_to_view,
    committed_child_records, exact_active_child_partition, exact_committed_child_partition,
    map_allocation_commit_error, partition_record_to_view, persist_child_membership_activation,
    validate_charged_child_record_size, validate_child_directory_authority_hash,
    validate_child_membership_directory_authority_hash, validate_child_record,
    validate_partition_record,
};
use crate::{
    storage::stable::component_registry::{
        RootComponentChildAllocationProgressRecord, RootComponentRegistryStore,
    },
    view::component_registry::{ComponentRegistryPartitionView, RootComponentChildAllocationView},
};
use canic_core::{
    control_plane_support::error::InternalError,
    dto::{
        component_provisioning::ComponentGroupDirectory,
        component_registry::ComponentLifecycleStatus, fleet_registry::FleetDirectorySnapshot,
    },
    ids::ComponentInstanceId,
};

impl ComponentRegistryOps {
    pub(crate) fn commit_verified_child(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        directory_synchronized_at_ns: u64,
        fleet_directory: FleetDirectorySnapshot,
        component_group: Option<&ComponentGroupDirectory>,
    ) -> Result<
        (
            RootComponentChildAllocationView,
            ComponentRegistryPartitionView,
        ),
        InternalError,
    > {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        if let RootComponentChildAllocationProgressRecord::Committed { commitment, .. } =
            &record.progress
        {
            let committed = exact_committed_child_partition(&record, commitment)?;
            validate_child_directory_authority_hash(
                &committed,
                &fleet_directory,
                component_group,
                commitment,
            )?;
            return Ok((
                child_allocation_record_to_view(record),
                partition_record_to_view(committed),
            ));
        }
        if directory_synchronized_at_ns <= partition.directory_synchronized_at_ns {
            return Err(InternalError::invalid_input());
        }
        let RootComponentChildAllocationProgressRecord::Verified {
            creation,
            canister,
            installation,
        } = &record.progress
        else {
            return Err(InternalError::conflict());
        };

        let (next_record, next_partition, child, traversal) = committed_child_records(
            &record,
            creation,
            *canister,
            installation,
            &partition,
            ComponentDirectoryAuthorityInput {
                synchronized_at_ns: directory_synchronized_at_ns,
                fleet: &fleet_directory,
                component_group,
            },
        )?;
        let actual_terminal_bytes =
            RootComponentRegistryStore::child_allocation_entry_bytes(&next_record)
                .checked_add(RootComponentRegistryStore::child_entry_bytes(&child))
                .and_then(|value| {
                    value.checked_add(RootComponentRegistryStore::child_traversal_entry_bytes(
                        &traversal,
                    ))
                })
                .and_then(|value| {
                    value.checked_add(RootComponentRegistryStore::principal_index_entry_bytes(
                        child.canister_id,
                        component,
                    ))
                })
                .ok_or_else(InternalError::resource_exhausted)?;
        if actual_terminal_bytes > installation.charged_entry_bytes {
            return Err(InternalError::invariant());
        }
        if next_partition.encoded_bytes > record.maximum_registry_bytes {
            return Err(InternalError::invariant());
        }
        let registry_reduction = partition
            .encoded_bytes
            .checked_sub(next_partition.encoded_bytes)
            .ok_or_else(InternalError::invariant)?;
        let mut next_meta = current.clone();
        next_meta.encoded_bytes = next_meta
            .encoded_bytes
            .checked_sub(registry_reduction)
            .ok_or_else(InternalError::invariant)?;

        RootComponentRegistryStore::commit_child(
            &current,
            next_meta,
            &partition,
            next_partition.clone(),
            &record,
            next_record.clone(),
            child,
            traversal,
        )
        .map_err(map_allocation_commit_error)?;
        Ok((
            child_allocation_record_to_view(next_record),
            partition_record_to_view(next_partition),
        ))
    }

    pub(crate) fn mark_child_directory_prepared(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_authority_hash: [u8; 32],
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        let RootComponentChildAllocationProgressRecord::Committed {
            creation,
            canister,
            installation,
            commitment,
        } = &record.progress
        else {
            return Err(InternalError::conflict());
        };
        let _committed = exact_committed_child_partition(&record, commitment)?;
        if commitment.directory_authority_hash != expected_authority_hash {
            return Err(InternalError::conflict());
        }
        if commitment.directory_prepared {
            return Ok(child_allocation_record_to_view(record));
        }

        let mut next_commitment = commitment.clone();
        next_commitment.directory_prepared = true;
        let mut next_record = record.clone();
        next_record.progress = RootComponentChildAllocationProgressRecord::Committed {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
            commitment: next_commitment,
        };
        validate_charged_child_record_size(&next_record, installation.charged_entry_bytes)?;
        if RootComponentRegistryStore::child_allocation_entry_bytes(&next_record)
            != RootComponentRegistryStore::child_allocation_entry_bytes(&record)
        {
            return Err(InternalError::invariant());
        }
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

    pub(crate) fn mark_child_runtime_activated(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_authority_hash: [u8; 32],
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        let RootComponentChildAllocationProgressRecord::Committed {
            creation,
            canister,
            installation,
            commitment,
        } = &record.progress
        else {
            return Err(InternalError::conflict());
        };
        let _committed = exact_committed_child_partition(&record, commitment)?;
        if commitment.directory_authority_hash != expected_authority_hash
            || !commitment.directory_prepared
        {
            return Err(InternalError::conflict());
        }
        if commitment.runtime_activated {
            return Ok(child_allocation_record_to_view(record));
        }

        let mut next_commitment = commitment.clone();
        next_commitment.runtime_activated = true;
        let mut next_record = record.clone();
        next_record.progress = RootComponentChildAllocationProgressRecord::Committed {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
            commitment: next_commitment,
        };
        validate_charged_child_record_size(&next_record, installation.charged_entry_bytes)?;
        if RootComponentRegistryStore::child_allocation_entry_bytes(&next_record)
            != RootComponentRegistryStore::child_allocation_entry_bytes(&record)
        {
            return Err(InternalError::invariant());
        }
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

    pub(crate) fn activate_child_membership(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        directory_synchronized_at_ns: u64,
        fleet_directory: FleetDirectorySnapshot,
        component_group: Option<&ComponentGroupDirectory>,
    ) -> Result<
        (
            RootComponentChildAllocationView,
            ComponentRegistryPartitionView,
        ),
        InternalError,
    > {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        let RootComponentChildAllocationProgressRecord::Committed {
            canister,
            commitment,
            ..
        } = &record.progress
        else {
            return Err(InternalError::conflict());
        };
        let _committed = exact_committed_child_partition(&record, commitment)?;
        if let Some(membership) = &commitment.membership {
            let active = exact_active_child_partition(&record, commitment, membership)?;
            validate_child_membership_directory_authority_hash(
                &active,
                &fleet_directory,
                component_group,
                membership,
            )?;
            return Ok((
                child_allocation_record_to_view(record),
                partition_record_to_view(active),
            ));
        }
        if !commitment.directory_prepared || !commitment.runtime_activated {
            return Err(InternalError::conflict());
        }
        if directory_synchronized_at_ns <= partition.directory_synchronized_at_ns {
            return Err(InternalError::invalid_input());
        }
        let child = RootComponentRegistryStore::child(component, *canister)
            .ok_or_else(InternalError::invariant)?;
        validate_child_record(&partition, &child)?;
        if child.status != ComponentLifecycleStatus::Prepared {
            return Err(InternalError::conflict());
        }

        persist_child_membership_activation(
            &current,
            &partition,
            &record,
            &child,
            directory_synchronized_at_ns,
            &fleet_directory,
            component_group,
        )
    }

    pub(crate) fn mark_child_membership_synchronized(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_authority_hash: [u8; 32],
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        let RootComponentChildAllocationProgressRecord::Committed {
            creation,
            canister,
            installation,
            commitment,
        } = &record.progress
        else {
            return Err(InternalError::conflict());
        };
        let membership = commitment
            .membership
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        let _active = exact_active_child_partition(&record, commitment, membership)?;
        if membership.directory_authority_hash != expected_authority_hash {
            return Err(InternalError::conflict());
        }
        if membership.directory_synchronized {
            return Ok(child_allocation_record_to_view(record));
        }

        let mut next_membership = membership.clone();
        next_membership.directory_synchronized = true;
        let mut next_commitment = commitment.clone();
        next_commitment.membership = Some(next_membership);
        let mut next_record = record.clone();
        next_record.progress = RootComponentChildAllocationProgressRecord::Committed {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
            commitment: next_commitment,
        };
        validate_charged_child_record_size(&next_record, installation.charged_entry_bytes)?;
        if RootComponentRegistryStore::child_allocation_entry_bytes(&next_record)
            != RootComponentRegistryStore::child_allocation_entry_bytes(&record)
        {
            return Err(InternalError::invariant());
        }
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
}
