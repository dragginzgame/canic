//! Module: ops::component_registry::component_retirement
//!
//! Responsibility: retain and advance one top-level Component from draining through membership removal.
//! Does not own: runtime effects, workflow ordering, descendant traversal, or Root retirement.
//! Boundary: commits exact quiescence, inventory, deletion, and removal evidence through the existing Registry store.

use super::{
    ComponentRegistryOps, child_allocation_is_terminal, committed_component_allocation,
    component_directory_authority_hash, component_draining_record_to_view,
    component_draining_state, component_draining_subtree_operation_id,
    component_final_inventory_hash, component_final_inventory_record_to_view,
    component_has_terminal_quiescence, component_membership_removal_records,
    component_partition_content_hash, component_partition_head, component_quiescence_intent_state,
    component_quiescence_terminal_entry_bytes, empty_component_descendant_content_hash,
    ensure_component_deletion_inventory, ensure_component_deletion_operation,
    ensure_component_final_inventory_candidate, ensure_component_final_inventory_fleet_authority,
    ensure_component_final_inventory_indexes_are_empty, ensure_component_final_inventory_time,
    ensure_component_lifecycle_history_is_terminal, first_registered_child,
    map_allocation_commit_error, require_ordinary_component_lifecycle,
    subtree_directory_convergence_record, subtree_removal_record_to_view,
    terminal_component_quiesced_at_ns, terminal_component_quiescence,
    validate_child_allocation_record, validate_component_draining_record,
    validate_partition_record, validate_removed_component_authority,
    validate_subtree_removal_progress, validate_subtree_removal_record,
    validate_subtree_removal_root,
};
use crate::{
    storage::stable::component_registry::{
        RootComponentDeletedReceiptRecord, RootComponentDeletionIntentRecord,
        RootComponentDeletionProgressRecord, RootComponentDrainingRecord,
        RootComponentFinalInventoryRecord, RootComponentMembershipRemovalCommit,
        RootComponentQuiescenceProgressRecord, RootComponentQuiescenceStopIntentRecord,
        RootComponentQuiescentReceiptRecord, RootComponentRegistryStore,
        RootComponentSubtreeRemovalProgressRecord,
    },
    view::component_registry::{
        RootComponentDrainingAdvanceView, RootComponentDrainingView,
        RootComponentFinalInventoryView,
    },
};
use canic_core::{
    control_plane_support::error::InternalError,
    dto::{
        component_registry::{
            ComponentLifecycleStatus, ComponentRegistryHead,
            ComponentRuntimeDirectoryConvergenceEvidence,
        },
        fleet_registry::FleetDirectorySnapshot,
    },
    ids::{ComponentInstanceId, ManagedCanisterBinding},
};

impl ComponentRegistryOps {
    pub(crate) fn component_draining(
        component: ComponentInstanceId,
    ) -> Result<Option<RootComponentDrainingView>, InternalError> {
        let Some(record) = RootComponentRegistryStore::component_draining(component) else {
            return Ok(None);
        };
        match RootComponentRegistryStore::partition(component) {
            Some(partition) => {
                validate_partition_record(&partition)?;
                validate_component_draining_record(&partition, &record)?;
            }
            None => {
                validate_removed_component_authority(&record)?;
            }
        }
        Ok(Some(component_draining_record_to_view(record)))
    }

    /// Resolve one top-level Component draining operation through its durable identity.
    pub(crate) fn component_draining_by_operation(
        operation_id: [u8; 32],
    ) -> Result<Option<RootComponentDrainingView>, InternalError> {
        let mut matches = RootComponentRegistryStore::component_drainings()
            .into_iter()
            .filter(|record| record.operation_id == operation_id);
        let Some(record) = matches.next() else {
            return Ok(None);
        };
        if matches.next().is_some() {
            return Err(InternalError::invariant());
        }
        match RootComponentRegistryStore::partition(record.component) {
            Some(partition) => {
                validate_partition_record(&partition)?;
                validate_component_draining_record(&partition, &record)?;
            }
            None => validate_removed_component_authority(&record)?,
        }
        Ok(Some(component_draining_record_to_view(record)))
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one synchronous transition validates, derives and atomically charges the complete draining fence"
    )]
    pub(crate) fn begin_component_draining(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_registry: ComponentRegistryHead,
        started_at_ns: u64,
        maximum_component_registry_bytes: u64,
        fleet_directory: FleetDirectorySnapshot,
    ) -> Result<RootComponentDrainingView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        require_ordinary_component_lifecycle(&partition)?;
        if let Some(existing) = RootComponentRegistryStore::component_draining(component) {
            validate_component_draining_record(&partition, &existing)?;
            return if existing.operation_id == operation_id
                && existing.previous_registry == expected_registry
            {
                Ok(component_draining_record_to_view(existing))
            } else {
                Err(InternalError::conflict())
            };
        }
        let current_registry = component_partition_head(&partition);
        if operation_id == [0; 32]
            || partition.status != ComponentLifecycleStatus::Active
            || expected_registry != current_registry
        {
            return Err(InternalError::conflict());
        }
        if started_at_ns <= partition.directory_synchronized_at_ns {
            return Err(InternalError::invalid_input());
        }
        if partition.reserved_descendants != 0 {
            return Err(InternalError::unavailable());
        }
        for allocation in RootComponentRegistryStore::child_allocations(component) {
            validate_child_allocation_record(&allocation)?;
            if !child_allocation_is_terminal(&allocation) {
                return Err(InternalError::unavailable());
            }
        }
        for removal in RootComponentRegistryStore::subtree_removals(component) {
            validate_subtree_removal_record(&removal)?;
            validate_subtree_removal_root(&removal, &current.root)?;
            validate_subtree_removal_progress(&partition, &removal)?;
            if !matches!(
                removal.progress,
                RootComponentSubtreeRemovalProgressRecord::Completed(_)
            ) {
                return Err(InternalError::unavailable());
            }
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
            ComponentLifecycleStatus::Draining,
            revision,
            partition.descendant_content_hash,
            partition.committed_descendants,
        )?;
        let mut next_partition = partition.clone();
        next_partition.status = ComponentLifecycleStatus::Draining;
        next_partition.revision = revision;
        next_partition.content_hash = content_hash;
        next_partition.directory_synchronized_at_ns = started_at_ns;
        let registry = component_partition_head(&next_partition);
        let record = RootComponentDrainingRecord {
            operation_id,
            component,
            previous_registry: current_registry,
            registry,
            descendant_count: next_partition.committed_descendants,
            descendant_content_hash: next_partition.descendant_content_hash,
            directory_authority_hash: component_directory_authority_hash(
                &next_partition.binding,
                next_partition.revision,
                next_partition.content_hash,
                started_at_ns,
                next_partition.committed_descendants,
                &fleet_directory,
            )?,
            started_at_ns,
            quiescence: None,
            subtree_operation_id: None,
            final_inventory: None,
            deletion: None,
        };
        let (next_partition, next_meta) = component_draining_state(
            &current,
            &partition,
            next_partition,
            &record,
            maximum_component_registry_bytes,
        )?;
        validate_partition_record(&next_partition)?;
        validate_component_draining_record(&next_partition, &record)?;
        RootComponentRegistryStore::begin_component_draining(
            &current,
            next_meta,
            &partition,
            next_partition,
            record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(component_draining_record_to_view(record))
    }

    pub(crate) fn prepare_component_quiescence(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_registry: ComponentRegistryHead,
        evidence: ComponentRuntimeDirectoryConvergenceEvidence,
        expected_module_hash: [u8; 32],
        prepared_at_ns: u64,
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentDrainingView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::component_draining(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        validate_component_draining_record(&partition, &record)?;
        if operation_id != record.operation_id || expected_registry != record.registry {
            return Err(InternalError::conflict());
        }
        if record.quiescence.is_some() {
            return Ok(component_draining_record_to_view(record));
        }
        if component_partition_head(&partition) != record.registry
            || partition.committed_descendants != record.descendant_count
            || partition.descendant_content_hash != record.descendant_content_hash
        {
            return Err(InternalError::conflict());
        }
        if expected_module_hash == [0; 32] || prepared_at_ns < record.started_at_ns {
            return Err(InternalError::invalid_input());
        }
        let expected_binding = ManagedCanisterBinding::Component(partition.binding.clone());
        let (coverage, convergence) =
            subtree_directory_convergence_record(&partition, &expected_binding, evidence)?;
        if coverage.component_registry_revision != record.registry.revision
            || coverage.component_registry_content_hash != record.registry.content_hash
        {
            return Err(InternalError::conflict());
        }

        let mut intent = RootComponentQuiescenceStopIntentRecord {
            registry: record.registry.clone(),
            descendant_count: record.descendant_count,
            descendant_content_hash: record.descendant_content_hash,
            canister_id: partition.binding.canister_id,
            controller: partition.binding.fleet_subnet_root,
            expected_module_hash,
            covered_fleet_registry_revision: coverage.fleet_registry_revision,
            covered_fleet_registry_content_hash: coverage.fleet_registry_content_hash,
            covered_authority_hash: coverage.authority_hash,
            runtime_operation_id: convergence.operation_id,
            activation: convergence.activation,
            prepared_at_ns,
            charged_entry_bytes: 0,
        };
        intent.charged_entry_bytes = component_quiescence_terminal_entry_bytes(&record, &intent)?;
        let mut next_record = record.clone();
        next_record.quiescence = Some(RootComponentQuiescenceProgressRecord::StopIntent(intent));
        let (next_partition, next_meta) = component_quiescence_intent_state(
            &current,
            &partition,
            &record,
            &next_record,
            maximum_component_registry_bytes,
        )?;
        validate_partition_record(&next_partition)?;
        validate_component_draining_record(&next_partition, &next_record)?;
        RootComponentRegistryStore::prepare_component_quiescence(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(component_draining_record_to_view(next_record))
    }

    pub(crate) fn mark_component_quiescent(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        observed_module_hash: [u8; 32],
        quiesced_at_ns: u64,
    ) -> Result<RootComponentDrainingView, InternalError> {
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::component_draining(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        validate_component_draining_record(&partition, &record)?;
        if operation_id != record.operation_id {
            return Err(InternalError::conflict());
        }
        let intent = match &record.quiescence {
            Some(RootComponentQuiescenceProgressRecord::StopIntent(intent)) => intent,
            Some(RootComponentQuiescenceProgressRecord::Quiescent(receipt)) => {
                return if receipt.observed_module_hash == observed_module_hash {
                    Ok(component_draining_record_to_view(record))
                } else {
                    Err(InternalError::conflict())
                };
            }
            None => {
                return Err(InternalError::unavailable());
            }
        };
        if observed_module_hash != intent.expected_module_hash
            || quiesced_at_ns < intent.prepared_at_ns
        {
            return Err(InternalError::conflict());
        }
        let receipt = RootComponentQuiescentReceiptRecord {
            stop: intent.clone(),
            observed_module_hash,
            quiesced_at_ns,
        };
        let mut next_record = record.clone();
        next_record.quiescence = Some(RootComponentQuiescenceProgressRecord::Quiescent(receipt));
        validate_component_draining_record(&partition, &next_record)?;
        RootComponentRegistryStore::mark_component_quiescent(&record, next_record.clone())
            .map_err(map_allocation_commit_error)?;
        Ok(component_draining_record_to_view(next_record))
    }

    pub(crate) fn advance_component_draining(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
    ) -> Result<RootComponentDrainingAdvanceView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        let draining = RootComponentRegistryStore::component_draining(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_component_draining_record(&partition, &draining)?;
        if operation_id != draining.operation_id
            || partition.status != ComponentLifecycleStatus::Draining
            || !component_has_terminal_quiescence(&partition)?
        {
            return Err(InternalError::conflict());
        }

        if let Some(subtree_operation_id) = draining.subtree_operation_id {
            let existing =
                RootComponentRegistryStore::subtree_removal(component, subtree_operation_id)
                    .ok_or_else(InternalError::invariant)?;
            validate_subtree_removal_record(&existing)?;
            validate_subtree_removal_root(&existing, &current.root)?;
            validate_subtree_removal_progress(&partition, &existing)?;
            if !matches!(
                existing.progress,
                RootComponentSubtreeRemovalProgressRecord::Completed(_)
            ) {
                return Ok(RootComponentDrainingAdvanceView::DescendantRemoval(
                    Box::new(subtree_removal_record_to_view(existing)),
                ));
            }
        }

        let Some(target) = first_registered_child(&partition, partition.binding.canister_id)?
        else {
            if partition.committed_descendants != 0
                || partition.descendant_content_hash
                    != empty_component_descendant_content_hash(component)
            {
                return Err(InternalError::invariant());
            }
            return Ok(RootComponentDrainingAdvanceView::DescendantsEmpty {
                registry: component_partition_head(&partition),
                descendant_content_hash: partition.descendant_content_hash,
            });
        };
        let subtree_operation_id =
            component_draining_subtree_operation_id(&draining, target.canister_id);
        if let Some(existing) =
            RootComponentRegistryStore::subtree_removal(component, subtree_operation_id)
        {
            validate_subtree_removal_record(&existing)?;
            validate_subtree_removal_root(&existing, &current.root)?;
            validate_subtree_removal_progress(&partition, &existing)?;
            if existing.target != target {
                return Err(InternalError::invariant());
            }
            return Ok(RootComponentDrainingAdvanceView::DescendantRemoval(
                Box::new(subtree_removal_record_to_view(existing)),
            ));
        }

        Ok(RootComponentDrainingAdvanceView::DescendantSubtreePending {
            operation_id: subtree_operation_id,
            target_canister_id: target.canister_id,
            reserved_against_registry: component_partition_head(&partition),
        })
    }

    pub(crate) fn finalize_component_inventory(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_registry: ComponentRegistryHead,
        fleet_directory: FleetDirectorySnapshot,
        finalized_at_ns: u64,
    ) -> Result<RootComponentFinalInventoryView, InternalError> {
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        let draining = RootComponentRegistryStore::component_draining(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_component_draining_record(&partition, &draining)?;
        if operation_id != draining.operation_id {
            return Err(InternalError::conflict());
        }
        if let Some(existing) = draining.final_inventory {
            return if expected_registry == existing.registry {
                Ok(component_final_inventory_record_to_view(existing))
            } else {
                Err(InternalError::conflict())
            };
        }

        let current_registry = component_partition_head(&partition);
        ensure_component_final_inventory_candidate(&partition, &expected_registry)?;
        let quiesced_at_ns =
            terminal_component_quiesced_at_ns(&draining).ok_or_else(InternalError::conflict)?;
        ensure_component_final_inventory_time(&partition, quiesced_at_ns, finalized_at_ns)?;
        ensure_component_final_inventory_indexes_are_empty(&partition)?;
        ensure_component_lifecycle_history_is_terminal(&partition)?;
        ensure_component_final_inventory_fleet_authority(&partition, &fleet_directory)?;

        let mut inventory = RootComponentFinalInventoryRecord {
            registry: current_registry,
            descendant_content_hash: partition.descendant_content_hash,
            registry_encoded_bytes: partition.encoded_bytes,
            directory_synchronized_at_ns: partition.directory_synchronized_at_ns,
            covered_fleet_registry_revision: fleet_directory.provenance.registry.revision,
            covered_fleet_registry_content_hash: fleet_directory.provenance.registry.content_hash,
            directory_authority_hash: component_directory_authority_hash(
                &partition.binding,
                partition.revision,
                partition.content_hash,
                partition.directory_synchronized_at_ns,
                0,
                &fleet_directory,
            )?,
            inventory_hash: [0; 32],
            finalized_at_ns,
        };
        inventory.inventory_hash = component_final_inventory_hash(&partition, &inventory)?;
        let mut next_draining = draining.clone();
        next_draining.final_inventory = Some(inventory.clone());
        validate_component_draining_record(&partition, &next_draining)?;
        RootComponentRegistryStore::mark_component_final_inventory(&draining, next_draining)
            .map_err(map_allocation_commit_error)?;
        Ok(component_final_inventory_record_to_view(inventory))
    }

    pub(crate) fn prepare_component_deletion(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_inventory_hash: [u8; 32],
        prepared_at_ns: u64,
    ) -> Result<RootComponentDrainingView, InternalError> {
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        let draining = RootComponentRegistryStore::component_draining(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        validate_component_draining_record(&partition, &draining)?;
        ensure_component_deletion_operation(&draining, operation_id)?;
        if let Some(progress) = &draining.deletion {
            ensure_component_deletion_inventory(progress, expected_inventory_hash)?;
            return Ok(component_draining_record_to_view(draining));
        }

        let final_inventory = draining
            .final_inventory
            .clone()
            .ok_or_else(InternalError::unavailable)?;
        if final_inventory.inventory_hash != expected_inventory_hash {
            return Err(InternalError::conflict());
        }
        let quiescence = terminal_component_quiescence(&draining)
            .cloned()
            .ok_or_else(InternalError::unavailable)?;
        if prepared_at_ns < final_inventory.finalized_at_ns {
            return Err(InternalError::invalid_input());
        }

        let mut next_draining = draining.clone();
        next_draining.deletion = Some(RootComponentDeletionProgressRecord::DeleteIntent(
            RootComponentDeletionIntentRecord {
                final_inventory,
                quiescence,
                prepared_at_ns,
            },
        ));
        validate_component_draining_record(&partition, &next_draining)?;
        RootComponentRegistryStore::prepare_component_deletion(&draining, next_draining.clone())
            .map_err(map_allocation_commit_error)?;
        Ok(component_draining_record_to_view(next_draining))
    }

    pub(crate) fn mark_component_deleted(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_inventory_hash: [u8; 32],
        deleted_at_ns: u64,
    ) -> Result<RootComponentDrainingView, InternalError> {
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        let draining = RootComponentRegistryStore::component_draining(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        validate_component_draining_record(&partition, &draining)?;
        ensure_component_deletion_operation(&draining, operation_id)?;
        let Some(progress) = &draining.deletion else {
            return Err(InternalError::unavailable());
        };
        ensure_component_deletion_inventory(progress, expected_inventory_hash)?;
        let intent = match progress {
            RootComponentDeletionProgressRecord::DeleteIntent(intent) => intent,
            RootComponentDeletionProgressRecord::Deleted(_)
            | RootComponentDeletionProgressRecord::MembershipRemoved(_) => {
                return Ok(component_draining_record_to_view(draining));
            }
        };
        if deleted_at_ns < intent.prepared_at_ns {
            return Err(InternalError::invalid_input());
        }

        let mut next_draining = draining.clone();
        next_draining.deletion = Some(RootComponentDeletionProgressRecord::Deleted(
            RootComponentDeletedReceiptRecord {
                deletion: intent.clone(),
                deleted_at_ns,
            },
        ));
        validate_component_draining_record(&partition, &next_draining)?;
        RootComponentRegistryStore::mark_component_deleted(&draining, next_draining.clone())
            .map_err(map_allocation_commit_error)?;
        Ok(component_draining_record_to_view(next_draining))
    }

    pub(crate) fn remove_component_membership(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_inventory_hash: [u8; 32],
        removed_at_ns: u64,
    ) -> Result<RootComponentDrainingView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let draining = RootComponentRegistryStore::component_draining(component)
            .ok_or_else(InternalError::unavailable)?;
        ensure_component_deletion_operation(&draining, operation_id)?;
        let progress = draining
            .deletion
            .as_ref()
            .ok_or_else(InternalError::unavailable)?;
        ensure_component_deletion_inventory(progress, expected_inventory_hash)?;
        if matches!(
            progress,
            RootComponentDeletionProgressRecord::MembershipRemoved(_)
        ) {
            validate_removed_component_authority(&draining)?;
            return Ok(component_draining_record_to_view(draining));
        }
        let RootComponentDeletionProgressRecord::Deleted(deleted) = progress else {
            return Err(InternalError::unavailable());
        };
        if removed_at_ns < deleted.deleted_at_ns {
            return Err(InternalError::invalid_input());
        }

        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        validate_component_draining_record(&partition, &draining)?;
        let allocation = committed_component_allocation(&partition)?;
        let records = component_membership_removal_records(
            &current,
            &partition,
            &allocation,
            &draining,
            deleted,
            removed_at_ns,
        )?;
        RootComponentRegistryStore::remove_component_membership(
            RootComponentMembershipRemovalCommit {
                expected_meta: &current,
                next_meta: records.next_meta,
                expected_partition: &partition,
                expected_allocation: &allocation,
                next_allocation: records.next_allocation,
                expected_draining: &draining,
                next_draining: records.next_draining.clone(),
            },
        )
        .map_err(map_allocation_commit_error)?;
        validate_removed_component_authority(&records.next_draining)?;
        Ok(component_draining_record_to_view(records.next_draining))
    }
}
