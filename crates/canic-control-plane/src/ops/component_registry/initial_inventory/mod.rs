//! Module: ops::component_registry::initial_inventory
//!
//! Responsibility: seal and monotonically advance the exact generation-one Component inventory.
//! Does not own: Component effects, Directory transport, Root runtime activation, or Registry bootstrap.
//! Boundary: compiles retained active membership into one activation-bound inventory receipt.

use super::{
    CompleteInitialInventory, ComponentRegistryOps, RootComponentInitialInventoryHashEntry,
    RootComponentInitialInventoryPlan, exact_active_partition, initial_inventory_record_to_view,
    map_allocation_commit_error, validate_partition_record,
};
use crate::{
    storage::stable::component_registry::{
        RootComponentAllocationProgressRecord, RootComponentAllocationRecord,
        RootComponentInitialInventoryRecord, RootComponentRegistryMetaRecord,
        RootComponentRegistryStore,
    },
    view::component_registry::RootComponentInitialInventoryView,
};
use canic_core::{
    control_plane_support::error::InternalError,
    dto::{component_registry::ComponentRegistryHead, fleet_registry::FleetRegistryVersion},
};
use sha2::{Digest, Sha256};

impl ComponentRegistryOps {
    pub(crate) fn registry_covers_preparation(
        prepared: &FleetRegistryVersion,
        current: &FleetRegistryVersion,
    ) -> bool {
        let authority_is_exact = prepared.authority == current.authority;
        let revision_is_covered = match prepared.revision.cmp(&current.revision) {
            std::cmp::Ordering::Less => true,
            std::cmp::Ordering::Equal => prepared.content_hash == current.content_hash,
            std::cmp::Ordering::Greater => false,
        };
        let hashes_are_present =
            prepared.content_hash != [0; 32] && current.content_hash != [0; 32];
        [authority_is_exact, revision_is_covered, hashes_are_present]
            .into_iter()
            .all(|valid| valid)
    }

    pub(crate) fn seal_initial_inventory(
        fleet_activation_operation_id: [u8; 32],
        sealed_at_ns: u64,
    ) -> Result<RootComponentInitialInventoryPlan, InternalError> {
        if sealed_at_ns == 0 {
            return Err(InternalError::invalid_input());
        }
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let inventory = complete_initial_inventory(&current)?;
        if let Some(existing) = current.initial_inventory {
            validate_initial_inventory_receipt(
                &existing,
                fleet_activation_operation_id,
                inventory.component_count,
                inventory.inventory_hash,
            )?;
            return Ok(RootComponentInitialInventoryPlan {
                receipt: initial_inventory_record_to_view(existing),
                operation_ids: inventory.operation_ids,
            });
        }

        let receipt = RootComponentInitialInventoryRecord {
            fleet_activation_operation_id,
            component_count: inventory.component_count,
            inventory_hash: inventory.inventory_hash,
            sealed_at_ns,
            directories_converged: false,
            root_runtime_activated: false,
        };
        let mut next = current.clone();
        next.initial_inventory = Some(receipt);
        RootComponentRegistryStore::replace_meta(&current, next)
            .map_err(map_allocation_commit_error)?;
        Ok(RootComponentInitialInventoryPlan {
            receipt: initial_inventory_record_to_view(receipt),
            operation_ids: inventory.operation_ids,
        })
    }

    pub(crate) fn validate_sealed_initial_inventory(
        fleet_activation_operation_id: [u8; 32],
    ) -> Result<RootComponentInitialInventoryPlan, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let receipt = current
            .initial_inventory
            .ok_or_else(InternalError::unavailable)?;
        let inventory = complete_initial_inventory(&current)?;
        validate_initial_inventory_receipt(
            &receipt,
            fleet_activation_operation_id,
            inventory.component_count,
            inventory.inventory_hash,
        )?;
        Ok(RootComponentInitialInventoryPlan {
            receipt: initial_inventory_record_to_view(receipt),
            operation_ids: inventory.operation_ids,
        })
    }

    pub(crate) fn initial_inventory(
        fleet_activation_operation_id: [u8; 32],
    ) -> Result<RootComponentInitialInventoryView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let receipt = current
            .initial_inventory
            .ok_or_else(InternalError::unavailable)?;
        if receipt.fleet_activation_operation_id != fleet_activation_operation_id {
            return Err(InternalError::conflict());
        }
        Ok(initial_inventory_record_to_view(receipt))
    }

    pub(crate) fn mark_initial_inventory_directories_converged(
        fleet_activation_operation_id: [u8; 32],
        expected_inventory_hash: [u8; 32],
    ) -> Result<RootComponentInitialInventoryView, InternalError> {
        update_initial_inventory_receipt(
            fleet_activation_operation_id,
            expected_inventory_hash,
            true,
            false,
        )
    }

    pub(crate) fn mark_initial_inventory_root_runtime_activated(
        fleet_activation_operation_id: [u8; 32],
        expected_inventory_hash: [u8; 32],
    ) -> Result<RootComponentInitialInventoryView, InternalError> {
        update_initial_inventory_receipt(
            fleet_activation_operation_id,
            expected_inventory_hash,
            true,
            true,
        )
    }
}

fn complete_initial_inventory(
    current: &RootComponentRegistryMetaRecord,
) -> Result<CompleteInitialInventory, InternalError> {
    if current.reserved_component_instances != 0 {
        return Err(InternalError::unavailable());
    }

    let mut allocations = RootComponentRegistryStore::allocations();
    allocations.sort_by_key(|record| record.allocation_sequence);
    let component_count =
        u32::try_from(allocations.len()).map_err(|_| InternalError::invariant())?;
    if component_count != current.committed_component_instances
        || current.next_allocation_sequence != u64::from(component_count) + 1
    {
        return Err(InternalError::invariant());
    }
    let maximum_known_created = component_count
        .checked_add(current.managed_descendants)
        .ok_or_else(InternalError::invariant)?;
    if current.known_created_component_canisters < component_count
        || current.known_created_component_canisters > maximum_known_created
    {
        return Err(InternalError::invariant());
    }

    let partitions = RootComponentRegistryStore::partitions();
    if partitions.len() != allocations.len() {
        return Err(InternalError::invariant());
    }

    let mut entries = Vec::with_capacity(allocations.len());
    let mut operation_ids = Vec::with_capacity(allocations.len());
    let mut encoded_bytes = 0_u64;
    for (index, record) in allocations.iter().enumerate() {
        let (entry, partition_bytes) = initial_inventory_hash_entry(record, index)?;
        encoded_bytes = encoded_bytes
            .checked_add(partition_bytes)
            .ok_or_else(InternalError::resource_exhausted)?;
        operation_ids.push(record.operation_id);
        entries.push(entry);
    }
    if encoded_bytes != current.encoded_bytes {
        return Err(InternalError::invariant());
    }

    let inventory_hash = initial_inventory_hash(&entries)?;
    Ok(CompleteInitialInventory {
        component_count,
        inventory_hash,
        operation_ids,
    })
}

fn initial_inventory_hash_entry(
    record: &RootComponentAllocationRecord,
    index: usize,
) -> Result<(RootComponentInitialInventoryHashEntry, u64), InternalError> {
    if record.allocation_sequence != index as u64 + 1 {
        return Err(InternalError::invariant());
    }
    let RootComponentAllocationProgressRecord::Committed { commitment, .. } = &record.progress
    else {
        return Err(InternalError::unavailable());
    };
    let membership = commitment
        .membership
        .as_ref()
        .ok_or_else(InternalError::unavailable)?;
    if !commitment.directory_prepared
        || !commitment.runtime_activated
        || !membership.directory_synchronized
    {
        return Err(InternalError::unavailable());
    }
    let active = exact_active_partition(record, commitment, membership)?;
    validate_partition_record(&active)?;
    let partition_bytes = active.encoded_bytes;
    Ok((
        RootComponentInitialInventoryHashEntry {
            operation_id: record.operation_id,
            allocation_sequence: record.allocation_sequence,
            component: record.component,
            component_spec: record.component_spec.clone(),
            spec_hash: record.spec_hash,
            role: record.role.clone(),
            provisioning_origin: record.provisioning_origin.clone(),
            release_set: record.release_set,
            prepared_registry: commitment.registry.clone(),
            prepared_registry_encoded_bytes: commitment.prepared_registry_encoded_bytes,
            prepared_directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
            prepared_directory_authority_hash: commitment.directory_authority_hash,
            active_binding: active.binding.clone(),
            active_registry: ComponentRegistryHead {
                component: active.binding.component,
                revision: active.revision,
                content_hash: active.content_hash,
            },
            active_registry_encoded_bytes: active.encoded_bytes,
            active_directory_synchronized_at_ns: membership.directory_synchronized_at_ns,
            active_directory_authority_hash: membership.directory_authority_hash,
        },
        partition_bytes,
    ))
}

fn initial_inventory_hash(
    entries: &[RootComponentInitialInventoryHashEntry],
) -> Result<[u8; 32], InternalError> {
    const DOMAIN: &[u8] = b"canic.root-component-initial-inventory.v1";
    let payload = candid::encode_one(entries).map_err(|_error| InternalError::invariant())?;
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    Ok(hasher.finalize().into())
}

fn validate_initial_inventory_receipt(
    receipt: &RootComponentInitialInventoryRecord,
    fleet_activation_operation_id: [u8; 32],
    component_count: u32,
    inventory_hash: [u8; 32],
) -> Result<(), InternalError> {
    if receipt.fleet_activation_operation_id != fleet_activation_operation_id {
        return Err(InternalError::conflict());
    }
    if receipt.component_count != component_count
        || receipt.inventory_hash != inventory_hash
        || receipt.sealed_at_ns == 0
    {
        return Err(InternalError::invariant());
    }
    Ok(())
}

fn update_initial_inventory_receipt(
    fleet_activation_operation_id: [u8; 32],
    expected_inventory_hash: [u8; 32],
    directories_converged: bool,
    root_runtime_activated: bool,
) -> Result<RootComponentInitialInventoryView, InternalError> {
    let current = RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
    let mut receipt = current
        .initial_inventory
        .ok_or_else(InternalError::unavailable)?;
    if receipt.fleet_activation_operation_id != fleet_activation_operation_id
        || receipt.inventory_hash != expected_inventory_hash
    {
        return Err(InternalError::conflict());
    }
    if root_runtime_activated && !directories_converged {
        return Err(InternalError::invariant());
    }
    receipt.directories_converged |= directories_converged;
    receipt.root_runtime_activated |= root_runtime_activated;
    if receipt.root_runtime_activated && !receipt.directories_converged {
        return Err(InternalError::invariant());
    }
    if current.initial_inventory == Some(receipt) {
        return Ok(initial_inventory_record_to_view(receipt));
    }
    let mut next = current.clone();
    next.initial_inventory = Some(receipt);
    RootComponentRegistryStore::replace_meta(&current, next)
        .map_err(map_allocation_commit_error)?;
    Ok(initial_inventory_record_to_view(receipt))
}
