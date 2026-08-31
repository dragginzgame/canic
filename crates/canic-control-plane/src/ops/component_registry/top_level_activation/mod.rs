//! Module: ops::component_registry::top_level_activation
//!
//! Responsibility: commit and converge one verified top-level Component into active membership.
//! Does not own: runtime calls, Directory transport, workflow ordering, allocation, or retirement.
//! Boundary: persists exact Registry, activation, membership, and synchronization transitions.

use super::{
    ComponentRegistryOps, active_membership_records, allocation_record_to_view, committed_records,
    exact_active_partition, exact_committed_partition, map_allocation_commit_error,
    partition_record_to_view, validate_charged_record_size, validate_directory_authority_hash,
    validate_membership_directory_authority_hash,
};
use crate::{
    storage::stable::component_registry::{
        RootComponentAllocationProgressRecord, RootComponentCommitmentRecord,
        RootComponentRegistryStore,
    },
    view::component_registry::{ComponentRegistryPartitionView, RootComponentAllocationView},
};
use canic_core::{
    control_plane_support::error::InternalError,
    dto::{
        component_provisioning::ComponentGroupDirectory,
        component_registry::ComponentProvisioningOrigin, fleet_registry::FleetDirectorySnapshot,
    },
};

impl ComponentRegistryOps {
    pub(crate) fn commit_verified(
        operation_id: [u8; 32],
        directory_synchronized_at_ns: u64,
        maximum_component_registry_bytes: u64,
        fleet_directory: FleetDirectorySnapshot,
    ) -> Result<(RootComponentAllocationView, ComponentRegistryPartitionView), InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::allocation(operation_id)
            .ok_or_else(InternalError::unavailable)?;
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
            return Err(InternalError::invalid_input());
        }
        let RootComponentAllocationProgressRecord::Verified {
            creation,
            canister,
            installation,
        } = &record.progress
        else {
            return Err(InternalError::conflict());
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
            return Err(InternalError::invariant());
        }
        if partition.encoded_bytes > maximum_component_registry_bytes {
            return Err(InternalError::resource_exhausted());
        }
        let encoded_bytes = current
            .encoded_bytes
            .checked_sub(installation.charged_entry_bytes)
            .and_then(|value| value.checked_add(partition.encoded_bytes))
            .ok_or_else(InternalError::invariant)?;
        if encoded_bytes > current.root.limits.maximum_registry_bytes {
            return Err(InternalError::invariant());
        }

        let mut next_meta = current.clone();
        next_meta.reserved_component_instances = next_meta
            .reserved_component_instances
            .checked_sub(1)
            .ok_or_else(InternalError::invariant)?;
        next_meta.committed_component_instances = next_meta
            .committed_component_instances
            .checked_add(1)
            .ok_or_else(InternalError::resource_exhausted)?;
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
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::allocation(operation_id)
            .ok_or_else(InternalError::unavailable)?;
        let RootComponentAllocationProgressRecord::Committed {
            creation,
            canister,
            installation,
            commitment,
        } = &record.progress
        else {
            return Err(InternalError::conflict());
        };
        if commitment.directory_authority_hash != expected_authority_hash {
            return Err(InternalError::conflict());
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
            return Err(InternalError::invariant());
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

    pub(crate) fn record_group_directory_prepared(
        operation_id: [u8; 32],
        previous_authority_hash: [u8; 32],
        expected_authority_hash: [u8; 32],
    ) -> Result<RootComponentAllocationView, InternalError> {
        if expected_authority_hash == [0; 32] {
            return Err(InternalError::invalid_input());
        }
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::allocation(operation_id)
            .ok_or_else(InternalError::unavailable)?;
        if !matches!(
            &record.provisioning_origin,
            ComponentProvisioningOrigin::ComponentGroup { .. }
        ) {
            return Err(InternalError::conflict());
        }
        let RootComponentAllocationProgressRecord::Committed {
            creation,
            canister,
            installation,
            commitment,
        } = &record.progress
        else {
            return Err(InternalError::conflict());
        };
        let replay_is_exact = [
            commitment.directory_authority_hash == expected_authority_hash,
            commitment.directory_prepared,
            !commitment.runtime_activated,
            commitment.membership.is_none(),
        ]
        .into_iter()
        .all(|exact| exact);
        if replay_is_exact {
            return Ok(allocation_record_to_view(record));
        }
        let previous_hash_is_valid = [
            previous_authority_hash != [0; 32],
            previous_authority_hash != expected_authority_hash,
        ]
        .into_iter()
        .all(|valid| valid);
        if !previous_hash_is_valid {
            return Err(InternalError::invalid_input());
        }
        let transition_is_open = [
            commitment.directory_authority_hash == previous_authority_hash,
            !commitment.directory_prepared,
            !commitment.runtime_activated,
            commitment.membership.is_none(),
        ]
        .into_iter()
        .all(|open| open);
        if !transition_is_open {
            return Err(InternalError::conflict());
        }
        let mut next_commitment = commitment.clone();
        next_commitment.directory_authority_hash = expected_authority_hash;
        next_commitment.directory_prepared = true;
        let mut next_record = record.clone();
        next_record.progress = RootComponentAllocationProgressRecord::Committed {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
            commitment: next_commitment,
        };
        validate_charged_record_size(&next_record, installation.charged_entry_bytes)?;
        // The install intent already charged the maximum terminal record. Hash byte values can
        // change the encoded record length, so the durable byte ledger retains that frozen charge.
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
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::allocation(operation_id)
            .ok_or_else(InternalError::unavailable)?;
        let RootComponentAllocationProgressRecord::Committed {
            creation,
            canister,
            installation,
            commitment,
        } = &record.progress
        else {
            return Err(InternalError::conflict());
        };
        if commitment.directory_authority_hash != expected_authority_hash
            || !commitment.directory_prepared
        {
            return Err(InternalError::conflict());
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
            return Err(InternalError::invariant());
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
        Self::activate_membership_with_group_directory(
            operation_id,
            directory_synchronized_at_ns,
            maximum_component_registry_bytes,
            fleet_directory,
            None,
        )
    }

    pub(crate) fn activate_group_membership(
        operation_id: [u8; 32],
        directory_synchronized_at_ns: u64,
        maximum_component_registry_bytes: u64,
        fleet_directory: FleetDirectorySnapshot,
        component_group: &ComponentGroupDirectory,
    ) -> Result<(RootComponentAllocationView, ComponentRegistryPartitionView), InternalError> {
        Self::activate_membership_with_group_directory(
            operation_id,
            directory_synchronized_at_ns,
            maximum_component_registry_bytes,
            fleet_directory,
            Some(component_group),
        )
    }

    fn activate_membership_with_group_directory(
        operation_id: [u8; 32],
        directory_synchronized_at_ns: u64,
        maximum_component_registry_bytes: u64,
        fleet_directory: FleetDirectorySnapshot,
        component_group: Option<&ComponentGroupDirectory>,
    ) -> Result<(RootComponentAllocationView, ComponentRegistryPartitionView), InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::allocation(operation_id)
            .ok_or_else(InternalError::unavailable)?;
        let RootComponentAllocationProgressRecord::Committed {
            installation,
            commitment,
            ..
        } = &record.progress
        else {
            return Err(InternalError::conflict());
        };
        let allocation_is_grouped = matches!(
            &record.provisioning_origin,
            ComponentProvisioningOrigin::ComponentGroup { .. }
        );
        if allocation_is_grouped != component_group.is_some() {
            return Err(InternalError::conflict());
        }
        let prepared = exact_committed_partition(&record, commitment)?;
        if let Some(membership) = &commitment.membership {
            let active = exact_active_partition(&record, commitment, membership)?;
            validate_membership_directory_authority_hash(
                &active,
                &fleet_directory,
                component_group,
                membership,
            )?;
            return Ok((
                allocation_record_to_view(record),
                partition_record_to_view(active),
            ));
        }
        if !commitment.directory_prepared || !commitment.runtime_activated {
            return Err(InternalError::conflict());
        }
        if directory_synchronized_at_ns <= commitment.directory_synchronized_at_ns {
            return Err(InternalError::invalid_input());
        }

        let (next_record, active) = active_membership_records(
            &record,
            commitment,
            directory_synchronized_at_ns,
            &fleet_directory,
            component_group,
        )?;
        if active.encoded_bytes > installation.charged_entry_bytes {
            return Err(InternalError::invariant());
        }
        if active.encoded_bytes > maximum_component_registry_bytes {
            return Err(InternalError::resource_exhausted());
        }
        let encoded_bytes = current
            .encoded_bytes
            .checked_sub(prepared.encoded_bytes)
            .and_then(|value| value.checked_add(active.encoded_bytes))
            .ok_or_else(InternalError::invariant)?;
        if encoded_bytes > current.root.limits.maximum_registry_bytes {
            return Err(InternalError::resource_exhausted());
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
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::allocation(operation_id)
            .ok_or_else(InternalError::unavailable)?;
        let RootComponentAllocationProgressRecord::Committed {
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
        let _active = exact_active_partition(&record, commitment, membership)?;
        if membership.directory_authority_hash != expected_authority_hash {
            return Err(InternalError::conflict());
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
            return Err(InternalError::invariant());
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
