//! Module: ops::component_registry::subtree_retirement
//!
//! Responsibility: advance one exact Component descendant subtree through terminal removal.
//! Does not own: management effects, workflow ordering, top-level Component retirement, or Root retirement.
//! Boundary: validates and commits retained traversal, stop, delete, Directory, and membership evidence.

use super::*;

impl ComponentRegistryOps {
    pub(crate) fn subtree_removal(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
    ) -> Result<Option<RootComponentSubtreeRemovalView>, InternalError> {
        let Some(record) = RootComponentRegistryStore::subtree_removal(component, operation_id)
        else {
            return Ok(None);
        };
        validate_subtree_removal_record(&record)?;
        let current = RootComponentRegistryStore::current().ok_or_else(InternalError::invariant)?;
        validate_subtree_removal_root(&record, &current.root)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::invariant)?;
        validate_partition_record(&partition)?;
        validate_subtree_removal_progress(&partition, &record)?;
        Ok(Some(subtree_removal_record_to_view(record)))
    }

    /// Resolve one subtree removal through its domain-owned operation identity.
    pub(crate) fn subtree_removal_by_operation(
        operation_id: [u8; 32],
    ) -> Result<Option<RootComponentSubtreeRemovalView>, InternalError> {
        let mut matches = RootComponentRegistryStore::registry_components()
            .into_iter()
            .flat_map(RootComponentRegistryStore::subtree_removals)
            .filter(|record| record.operation_id == operation_id);
        let Some(record) = matches.next() else {
            return Ok(None);
        };
        if matches.next().is_some() {
            return Err(InternalError::invariant());
        }
        validate_subtree_removal_record(&record)?;
        let current = RootComponentRegistryStore::current().ok_or_else(InternalError::invariant)?;
        validate_subtree_removal_root(&record, &current.root)?;
        let partition = RootComponentRegistryStore::partition(record.component)
            .ok_or_else(InternalError::invariant)?;
        validate_partition_record(&partition)?;
        validate_subtree_removal_progress(&partition, &record)?;
        Ok(Some(subtree_removal_record_to_view(record)))
    }

    pub(crate) fn subtree_removal_completed_leaf_matches(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        traversal_steps: u32,
        leaf_canister_id: Principal,
        leaf_parent_canister_id: Principal,
    ) -> Result<bool, InternalError> {
        let Some(removal) = RootComponentRegistryStore::subtree_removal(component, operation_id)
        else {
            return Ok(false);
        };
        let Some(partition) = RootComponentRegistryStore::partition(component) else {
            return Ok(false);
        };
        let selection =
            SubtreeLeafSelection::new(traversal_steps, leaf_canister_id, leaf_parent_canister_id);
        completed_subtree_leaf_for_selection(&removal, &partition, selection)
            .map(|leaf| leaf.is_some())
    }

    pub(crate) fn begin_draining_subtree_removal(
        component: ComponentInstanceId,
        draining_operation_id: [u8; 32],
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        let RootComponentDrainingAdvanceView::DescendantSubtreePending {
            operation_id,
            target_canister_id,
            reserved_against_registry,
        } = Self::advance_component_draining(component, draining_operation_id)?
        else {
            return Err(InternalError::conflict());
        };
        Self::begin_subtree_removal_with_origin(
            component,
            operation_id,
            target_canister_id,
            reserved_against_registry,
            maximum_component_registry_bytes,
            SubtreeRemovalOrigin::DrainingDriver,
        )
    }

    pub(crate) fn begin_subtree_removal(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        target_canister_id: Principal,
        reserved_against_registry: ComponentRegistryHead,
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        Self::begin_subtree_removal_with_origin(
            component,
            operation_id,
            target_canister_id,
            reserved_against_registry,
            maximum_component_registry_bytes,
            SubtreeRemovalOrigin::Ordinary,
        )
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one synchronous transaction validates and durably charges the exact subtree fence"
    )]
    pub(super) fn begin_subtree_removal_with_origin(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        target_canister_id: Principal,
        reserved_against_registry: ComponentRegistryHead,
        maximum_component_registry_bytes: u64,
        origin: SubtreeRemovalOrigin,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        let lifecycle_matches_origin = match origin {
            SubtreeRemovalOrigin::Ordinary => partition.status == ComponentLifecycleStatus::Active,
            SubtreeRemovalOrigin::DrainingDriver => {
                partition.status == ComponentLifecycleStatus::Draining
                    && component_has_terminal_quiescence(&partition)?
            }
        };
        if !lifecycle_matches_origin {
            return Err(InternalError::conflict());
        }
        if let Some(existing) = RootComponentRegistryStore::subtree_removal(component, operation_id)
        {
            validate_subtree_removal_record(&existing)?;
            validate_subtree_removal_root(&existing, &current.root)?;
            validate_subtree_removal_progress(&partition, &existing)?;
            return if existing.target.canister_id == target_canister_id
                && existing.reserved_against_registry == reserved_against_registry
            {
                Ok(subtree_removal_record_to_view(existing))
            } else {
                Err(InternalError::conflict())
            };
        }

        if reserved_against_registry
            != (ComponentRegistryHead {
                component,
                revision: partition.revision,
                content_hash: partition.content_hash,
            })
        {
            return Err(InternalError::conflict());
        }
        if origin == SubtreeRemovalOrigin::Ordinary
            && RootComponentRegistryStore::subtree_removals(component)
                .iter()
                .any(|removal| {
                    !matches!(
                        &removal.progress,
                        RootComponentSubtreeRemovalProgressRecord::Completed(_)
                    )
                })
        {
            return Err(InternalError::conflict());
        }

        let target = RootComponentRegistryStore::child(component, target_canister_id)
            .ok_or_else(InternalError::unavailable)?;
        validate_registered_child_record(&partition, &target)?;
        if target.status != ComponentLifecycleStatus::Active {
            return Err(InternalError::conflict());
        }
        let traversal_limit = partition
            .committed_descendants
            .checked_add(1)
            .ok_or_else(InternalError::resource_exhausted)?;
        for allocation in RootComponentRegistryStore::child_allocations(component) {
            validate_child_allocation_record(&allocation)?;
            if !child_allocation_is_terminal(&allocation)
                && canister_is_in_subtree(
                    &partition,
                    allocation.parent_canister_id,
                    target_canister_id,
                    traversal_limit,
                )?
            {
                return Err(InternalError::unavailable());
            }
        }

        let record = RootComponentSubtreeRemovalRecord {
            operation_id,
            component,
            target,
            reserved_against_registry,
            maximum_completed_leaves: partition.committed_descendants,
            completed_leaves: 0,
            traversal_steps: 0,
            progress: RootComponentSubtreeRemovalProgressRecord::Fenced,
        };
        validate_subtree_removal_record(&record)?;
        let (next_partition, registry_delta) = subtree_fence_partition(&partition, &record)?;
        if next_partition.encoded_bytes > maximum_component_registry_bytes {
            return Err(InternalError::resource_exhausted());
        }
        let mut next_meta = current.clone();
        next_meta.encoded_bytes = next_meta
            .encoded_bytes
            .checked_add(registry_delta)
            .ok_or_else(InternalError::resource_exhausted)?;
        if next_meta.encoded_bytes > next_meta.root.limits.maximum_registry_bytes {
            return Err(InternalError::resource_exhausted());
        }

        let draining_transition = match origin {
            SubtreeRemovalOrigin::Ordinary => None,
            SubtreeRemovalOrigin::DrainingDriver => {
                let current_draining = RootComponentRegistryStore::component_draining(component)
                    .ok_or_else(InternalError::invariant)?;
                validate_component_draining_record(&partition, &current_draining)?;
                let mut next_draining = current_draining.clone();
                next_draining.subtree_operation_id = Some(operation_id);
                validate_component_draining_record(&partition, &next_draining)?;
                Some((current_draining, next_draining))
            }
        };
        RootComponentRegistryStore::begin_subtree_removal(RootComponentSubtreeRemovalBeginCommit {
            expected_meta: &current,
            next_meta,
            expected_partition: &partition,
            next_partition,
            expected_target: &record.target,
            record: record.clone(),
            expected_draining: draining_transition.as_ref().map(|(expected, _)| expected),
            next_draining: draining_transition.as_ref().map(|(_, next)| next.clone()),
        })
        .map_err(map_allocation_commit_error)?;
        Ok(subtree_removal_record_to_view(record))
    }

    pub(crate) fn advance_subtree_removal(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_traversal_steps: u32,
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        let record = RootComponentRegistryStore::subtree_removal(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        validate_subtree_removal_record(&record)?;
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        validate_subtree_removal_root(&record, &current.root)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        validate_subtree_removal_progress(&partition, &record)?;
        if expected_traversal_steps < record.traversal_steps {
            return Ok(subtree_removal_record_to_view(record));
        }
        if expected_traversal_steps > record.traversal_steps {
            return Err(InternalError::conflict());
        }
        if matches!(
            &record.progress,
            RootComponentSubtreeRemovalProgressRecord::LeafSelected { .. }
                | RootComponentSubtreeRemovalProgressRecord::StopIntent(_)
                | RootComponentSubtreeRemovalProgressRecord::Stopped(_)
                | RootComponentSubtreeRemovalProgressRecord::DeleteIntent(_)
                | RootComponentSubtreeRemovalProgressRecord::Deleted(_)
                | RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(_)
                | RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(_)
                | RootComponentSubtreeRemovalProgressRecord::Completed(_)
        ) {
            return Ok(subtree_removal_record_to_view(record));
        }

        let mut next_record = record.clone();
        for _ in 0..SUBTREE_REMOVAL_TRAVERSAL_BATCH_SIZE {
            let cursor = match &next_record.progress {
                RootComponentSubtreeRemovalProgressRecord::Fenced => next_record.target.clone(),
                RootComponentSubtreeRemovalProgressRecord::Traversing { cursor } => cursor.clone(),
                RootComponentSubtreeRemovalProgressRecord::LeafSelected { .. }
                | RootComponentSubtreeRemovalProgressRecord::StopIntent(_)
                | RootComponentSubtreeRemovalProgressRecord::Stopped(_)
                | RootComponentSubtreeRemovalProgressRecord::DeleteIntent(_)
                | RootComponentSubtreeRemovalProgressRecord::Deleted(_)
                | RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(_)
                | RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(_)
                | RootComponentSubtreeRemovalProgressRecord::Completed(_) => break,
            };
            next_record.progress = match first_registered_child(&partition, cursor.canister_id)? {
                Some(child) => {
                    RootComponentSubtreeRemovalProgressRecord::Traversing { cursor: child }
                }
                None => RootComponentSubtreeRemovalProgressRecord::LeafSelected { leaf: cursor },
            };
            next_record.traversal_steps = next_record
                .traversal_steps
                .checked_add(1)
                .ok_or_else(InternalError::resource_exhausted)?;
        }
        validate_subtree_removal_record(&next_record)?;
        validate_subtree_removal_root(&next_record, &current.root)?;
        validate_subtree_removal_progress(&partition, &next_record)?;
        let (next_partition, next_meta) = subtree_removal_progress_state(
            &current,
            &partition,
            &record,
            &next_record,
            maximum_component_registry_bytes,
        )?;
        RootComponentRegistryStore::replace_subtree_removal(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(subtree_removal_record_to_view(next_record))
    }

    pub(crate) fn prepare_subtree_leaf_stop(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_traversal_steps: u32,
        expected_leaf_canister_id: Principal,
        expected_leaf_parent_canister_id: Principal,
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        let record = RootComponentRegistryStore::subtree_removal(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        validate_subtree_removal_record(&record)?;
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        validate_subtree_removal_root(&record, &current.root)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        validate_subtree_removal_progress(&partition, &record)?;

        let expected_selection = SubtreeLeafSelection::new(
            expected_traversal_steps,
            expected_leaf_canister_id,
            expected_leaf_parent_canister_id,
        );
        if completed_subtree_leaf_for_selection(&record, &partition, expected_selection)?.is_some()
        {
            return Ok(subtree_removal_record_to_view(record));
        }
        let expected_stop =
            SubtreeLeafStopAuthority::new(expected_selection, current.root.fleet_subnet_root);
        let leaf = match &record.progress {
            RootComponentSubtreeRemovalProgressRecord::LeafSelected { leaf } => leaf,
            RootComponentSubtreeRemovalProgressRecord::Fenced
            | RootComponentSubtreeRemovalProgressRecord::Traversing { .. } => {
                return Err(InternalError::unavailable());
            }
            progress => {
                let durable_stop =
                    retained_subtree_stop_effect(progress).ok_or_else(InternalError::invariant)?;
                if SubtreeLeafStopAuthority::from_record(record.traversal_steps, durable_stop)
                    == expected_stop
                {
                    return Ok(subtree_removal_record_to_view(record));
                }
                return Err(InternalError::conflict());
            }
        };
        if SubtreeLeafSelection::from_record(record.traversal_steps, leaf) != expected_selection {
            return Err(InternalError::conflict());
        }
        if current.root.fleet_subnet_root == Principal::anonymous() {
            return Err(InternalError::invariant());
        }

        let mut next_record = record.clone();
        next_record.progress = RootComponentSubtreeRemovalProgressRecord::StopIntent(
            RootComponentSubtreeStopEffectRecord {
                leaf: leaf.clone(),
                controller: current.root.fleet_subnet_root,
            },
        );
        validate_subtree_removal_record(&next_record)?;
        validate_subtree_removal_root(&next_record, &current.root)?;
        validate_subtree_removal_progress(&partition, &next_record)?;
        let (next_partition, next_meta) = subtree_removal_progress_state(
            &current,
            &partition,
            &record,
            &next_record,
            maximum_component_registry_bytes,
        )?;
        RootComponentRegistryStore::replace_subtree_removal(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(subtree_removal_record_to_view(next_record))
    }

    pub(crate) fn mark_subtree_leaf_stopped(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_traversal_steps: u32,
        expected_leaf_canister_id: Principal,
        expected_leaf_parent_canister_id: Principal,
        observed_module_hash: [u8; 32],
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        let record = RootComponentRegistryStore::subtree_removal(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        validate_subtree_removal_record(&record)?;
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        validate_subtree_removal_root(&record, &current.root)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        validate_subtree_removal_progress(&partition, &record)?;

        let expected_selection = SubtreeLeafSelection::new(
            expected_traversal_steps,
            expected_leaf_canister_id,
            expected_leaf_parent_canister_id,
        );
        if let Some(history) =
            completed_subtree_leaf_for_selection(&record, &partition, expected_selection)?
        {
            if history.observed_module_hash == observed_module_hash {
                return Ok(subtree_removal_record_to_view(record));
            }
            return Err(InternalError::conflict());
        }
        let expected_stop =
            SubtreeLeafStopAuthority::new(expected_selection, current.root.fleet_subnet_root);
        let expected_stopped = SubtreeLeafStoppedAuthority {
            stop: expected_stop,
            observed_module_hash,
        };
        let stop = match &record.progress {
            RootComponentSubtreeRemovalProgressRecord::StopIntent(effect) => effect,
            RootComponentSubtreeRemovalProgressRecord::Fenced
            | RootComponentSubtreeRemovalProgressRecord::Traversing { .. }
            | RootComponentSubtreeRemovalProgressRecord::LeafSelected { .. } => {
                return Err(InternalError::unavailable());
            }
            progress => {
                let durable_stopped = retained_subtree_stopped_effect(progress)
                    .ok_or_else(InternalError::invariant)?;
                if SubtreeLeafStoppedAuthority::from_record(record.traversal_steps, durable_stopped)
                    == expected_stopped
                {
                    return Ok(subtree_removal_record_to_view(record));
                }
                return Err(InternalError::conflict());
            }
        };
        if SubtreeLeafStopAuthority::from_record(record.traversal_steps, stop) != expected_stop {
            return Err(InternalError::conflict());
        }

        let mut next_record = record.clone();
        next_record.progress = RootComponentSubtreeRemovalProgressRecord::Stopped(
            RootComponentSubtreeStoppedEffectRecord {
                stop: stop.clone(),
                observed_module_hash,
            },
        );
        validate_subtree_removal_record(&next_record)?;
        validate_subtree_removal_root(&next_record, &current.root)?;
        validate_subtree_removal_progress(&partition, &next_record)?;
        let (next_partition, next_meta) = subtree_removal_progress_state(
            &current,
            &partition,
            &record,
            &next_record,
            maximum_component_registry_bytes,
        )?;
        RootComponentRegistryStore::replace_subtree_removal(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(subtree_removal_record_to_view(next_record))
    }

    #[expect(
        clippy::too_many_lines,
        reason = "deletion preparation reconciles every later durable removal phase"
    )]
    pub(crate) fn prepare_subtree_leaf_delete(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_traversal_steps: u32,
        expected_leaf_canister_id: Principal,
        expected_leaf_parent_canister_id: Principal,
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        let record = RootComponentRegistryStore::subtree_removal(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        validate_subtree_removal_record(&record)?;
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        validate_subtree_removal_root(&record, &current.root)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        validate_subtree_removal_progress(&partition, &record)?;

        let expected_selection = SubtreeLeafSelection::new(
            expected_traversal_steps,
            expected_leaf_canister_id,
            expected_leaf_parent_canister_id,
        );
        if completed_subtree_leaf_for_selection(&record, &partition, expected_selection)?.is_some()
        {
            return Ok(subtree_removal_record_to_view(record));
        }
        let expected_stop =
            SubtreeLeafStopAuthority::new(expected_selection, current.root.fleet_subnet_root);
        let stopped = match &record.progress {
            RootComponentSubtreeRemovalProgressRecord::Stopped(receipt) => receipt,
            RootComponentSubtreeRemovalProgressRecord::DeleteIntent(deletion) => {
                if SubtreeLeafStopAuthority::from_record(
                    record.traversal_steps,
                    &deletion.stopped.stop,
                ) == expected_stop
                {
                    return Ok(subtree_removal_record_to_view(record));
                }
                return Err(InternalError::conflict());
            }
            RootComponentSubtreeRemovalProgressRecord::Deleted(receipt) => {
                if SubtreeLeafStopAuthority::from_record(
                    record.traversal_steps,
                    &receipt.deletion.stopped.stop,
                ) == expected_stop
                {
                    return Ok(subtree_removal_record_to_view(record));
                }
                return Err(InternalError::conflict());
            }
            RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(receipt) => {
                if SubtreeLeafStopAuthority::from_record(
                    record.traversal_steps,
                    &receipt.deleted.deletion.stopped.stop,
                ) == expected_stop
                {
                    return Ok(subtree_removal_record_to_view(record));
                }
                return Err(InternalError::conflict());
            }
            RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(receipt) => {
                if SubtreeLeafStopAuthority::from_record(
                    record.traversal_steps,
                    &receipt.membership_removed.deleted.deletion.stopped.stop,
                ) == expected_stop
                {
                    return Ok(subtree_removal_record_to_view(record));
                }
                return Err(InternalError::conflict());
            }
            RootComponentSubtreeRemovalProgressRecord::Fenced
            | RootComponentSubtreeRemovalProgressRecord::Traversing { .. }
            | RootComponentSubtreeRemovalProgressRecord::LeafSelected { .. }
            | RootComponentSubtreeRemovalProgressRecord::StopIntent(_)
            | RootComponentSubtreeRemovalProgressRecord::Completed(_) => {
                return Err(InternalError::unavailable());
            }
        };
        if SubtreeLeafStopAuthority::from_record(record.traversal_steps, &stopped.stop)
            != expected_stop
        {
            return Err(InternalError::conflict());
        }

        let mut next_record = record.clone();
        next_record.progress = RootComponentSubtreeRemovalProgressRecord::DeleteIntent(
            RootComponentSubtreeDeleteEffectRecord {
                stopped: stopped.clone(),
            },
        );
        validate_subtree_removal_record(&next_record)?;
        validate_subtree_removal_root(&next_record, &current.root)?;
        validate_subtree_removal_progress(&partition, &next_record)?;
        let (next_partition, next_meta) = subtree_removal_progress_state(
            &current,
            &partition,
            &record,
            &next_record,
            maximum_component_registry_bytes,
        )?;
        RootComponentRegistryStore::replace_subtree_removal(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(subtree_removal_record_to_view(next_record))
    }

    pub(crate) fn mark_subtree_leaf_deleted(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_traversal_steps: u32,
        expected_leaf_canister_id: Principal,
        expected_leaf_parent_canister_id: Principal,
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        let record = RootComponentRegistryStore::subtree_removal(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        validate_subtree_removal_record(&record)?;
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        validate_subtree_removal_root(&record, &current.root)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        validate_subtree_removal_progress(&partition, &record)?;

        let expected_selection = SubtreeLeafSelection::new(
            expected_traversal_steps,
            expected_leaf_canister_id,
            expected_leaf_parent_canister_id,
        );
        if completed_subtree_leaf_for_selection(&record, &partition, expected_selection)?.is_some()
        {
            return Ok(subtree_removal_record_to_view(record));
        }
        let expected_stop =
            SubtreeLeafStopAuthority::new(expected_selection, current.root.fleet_subnet_root);
        let RootComponentSubtreeRemovalProgressRecord::DeleteIntent(deletion) = &record.progress
        else {
            let Some(receipt) = retained_subtree_deleted_effect(&record.progress) else {
                return Err(InternalError::unavailable());
            };
            let durable_stop = SubtreeLeafStopAuthority::from_record(
                record.traversal_steps,
                &receipt.deletion.stopped.stop,
            );
            if durable_stop == expected_stop {
                return Ok(subtree_removal_record_to_view(record));
            }
            return Err(InternalError::conflict());
        };
        if SubtreeLeafStopAuthority::from_record(record.traversal_steps, &deletion.stopped.stop)
            != expected_stop
        {
            return Err(InternalError::conflict());
        }

        let mut next_record = record.clone();
        next_record.progress = RootComponentSubtreeRemovalProgressRecord::Deleted(
            RootComponentSubtreeDeletedEffectRecord {
                deletion: deletion.clone(),
            },
        );
        validate_subtree_removal_record(&next_record)?;
        validate_subtree_removal_root(&next_record, &current.root)?;
        validate_subtree_removal_progress(&partition, &next_record)?;
        let (next_partition, next_meta) = subtree_removal_progress_state(
            &current,
            &partition,
            &record,
            &next_record,
            maximum_component_registry_bytes,
        )?;
        RootComponentRegistryStore::replace_subtree_removal(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(subtree_removal_record_to_view(next_record))
    }

    #[expect(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        reason = "one synchronous operation validates and atomically removes every leaf index"
    )]
    pub(crate) fn remove_subtree_leaf_membership(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_traversal_steps: u32,
        expected_leaf_canister_id: Principal,
        expected_leaf_parent_canister_id: Principal,
        directory_synchronized_at_ns: u64,
        maximum_component_registry_bytes: u64,
        fleet_directory: FleetDirectorySnapshot,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        let record = RootComponentRegistryStore::subtree_removal(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        validate_subtree_removal_record(&record)?;
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        validate_subtree_removal_root(&record, &current.root)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        validate_subtree_removal_progress(&partition, &record)?;

        let expected_selection = SubtreeLeafSelection::new(
            expected_traversal_steps,
            expected_leaf_canister_id,
            expected_leaf_parent_canister_id,
        );
        if completed_subtree_leaf_for_selection(&record, &partition, expected_selection)?.is_some()
        {
            return Ok(subtree_removal_record_to_view(record));
        }
        let deleted = match &record.progress {
            RootComponentSubtreeRemovalProgressRecord::Deleted(receipt) => receipt,
            RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(receipt) => {
                let durable_selection = SubtreeLeafSelection::from_record(
                    record.traversal_steps,
                    &receipt.deleted.deletion.stopped.stop.leaf,
                );
                if durable_selection == expected_selection {
                    return Ok(subtree_removal_record_to_view(record));
                }
                return Err(InternalError::conflict());
            }
            RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(receipt) => {
                let durable_selection = SubtreeLeafSelection::from_record(
                    record.traversal_steps,
                    &receipt
                        .membership_removed
                        .deleted
                        .deletion
                        .stopped
                        .stop
                        .leaf,
                );
                if durable_selection == expected_selection {
                    return Ok(subtree_removal_record_to_view(record));
                }
                return Err(InternalError::conflict());
            }
            RootComponentSubtreeRemovalProgressRecord::Fenced
            | RootComponentSubtreeRemovalProgressRecord::Traversing { .. }
            | RootComponentSubtreeRemovalProgressRecord::LeafSelected { .. }
            | RootComponentSubtreeRemovalProgressRecord::StopIntent(_)
            | RootComponentSubtreeRemovalProgressRecord::Stopped(_)
            | RootComponentSubtreeRemovalProgressRecord::DeleteIntent(_)
            | RootComponentSubtreeRemovalProgressRecord::Completed(_) => {
                return Err(InternalError::unavailable());
            }
        };
        let leaf = &deleted.deletion.stopped.stop.leaf;
        if SubtreeLeafSelection::from_record(record.traversal_steps, leaf) != expected_selection {
            return Err(InternalError::conflict());
        }
        if directory_synchronized_at_ns <= partition.directory_synchronized_at_ns {
            return Err(InternalError::invalid_input());
        }
        if first_registered_child(&partition, leaf.canister_id)?.is_some() {
            return Err(InternalError::conflict());
        }
        let traversal = ComponentRegistryChildTraversalRecord {
            component,
            parent_canister_id: leaf.parent_canister_id,
            role: leaf.role.clone(),
            canister_id: leaf.canister_id,
        };
        let parent_role_count = RootComponentRegistryStore::parent_role_count(
            component,
            leaf.parent_canister_id,
            &leaf.role,
        )
        .ok_or_else(InternalError::invariant)?;
        if parent_role_count.instances == 0 {
            return Err(InternalError::invariant());
        }
        let next_parent_role_count =
            parent_role_count
                .instances
                .checked_sub(1)
                .and_then(|instances| {
                    (instances > 0).then(|| ComponentRegistryParentRoleCountRecord {
                        component,
                        parent_canister_id: leaf.parent_canister_id,
                        child_role: leaf.role.clone(),
                        instances,
                    })
                });

        let revision = partition
            .revision
            .checked_add(1)
            .ok_or_else(InternalError::resource_exhausted)?;
        let committed_descendants = partition
            .committed_descendants
            .checked_sub(1)
            .ok_or_else(InternalError::invariant)?;
        let descendant_content_hash = removed_component_descendant_content_hash(
            component,
            partition.descendant_content_hash,
            partition.revision,
            partition.committed_descendants,
            revision,
            leaf,
        )?;
        let content_hash = component_partition_content_hash(
            &partition.binding,
            partition.protocol_profile_digest,
            &partition.provisioning_origin,
            partition.release_set,
            partition.status,
            revision,
            descendant_content_hash,
            committed_descendants,
        )?;
        let directory_authority_hash = component_directory_authority_hash(
            &partition.binding,
            revision,
            content_hash,
            directory_synchronized_at_ns,
            committed_descendants,
            &fleet_directory,
        )?;
        let mut next_meta = current.clone();
        next_meta.managed_descendants = next_meta
            .managed_descendants
            .checked_sub(1)
            .ok_or_else(InternalError::invariant)?;
        next_meta.known_created_component_canisters = next_meta
            .known_created_component_canisters
            .checked_sub(1)
            .ok_or_else(InternalError::invariant)?;
        let registry = ComponentRegistryHead {
            component,
            revision,
            content_hash,
        };
        let mut next_partition = partition.clone();
        next_partition.revision = revision;
        next_partition.content_hash = content_hash;
        next_partition.descendant_content_hash = descendant_content_hash;
        next_partition.directory_synchronized_at_ns = directory_synchronized_at_ns;
        next_partition.committed_descendants = committed_descendants;
        let mut next_record = record.clone();
        next_record.progress = RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(
            RootComponentSubtreeMembershipRemovedRecord {
                deleted: deleted.clone(),
                removed_from_registry: ComponentRegistryHead {
                    component,
                    revision: partition.revision,
                    content_hash: partition.content_hash,
                },
                previous_descendant_content_hash: partition.descendant_content_hash,
                previous_committed_descendants: partition.committed_descendants,
                registry,
                descendant_content_hash,
                registry_encoded_bytes: 0,
                reserved_descendants: partition.reserved_descendants,
                committed_descendants,
                directory_synchronized_at_ns,
                directory_authority_hash,
                parent_role_instances: next_parent_role_count
                    .as_ref()
                    .map_or(0, |count| count.instances),
                root_managed_descendants: next_meta.managed_descendants,
                root_known_created_component_canisters: next_meta.known_created_component_canisters,
            },
        );
        converge_subtree_membership_removal_bytes(
            &current,
            &partition,
            &record,
            leaf,
            &traversal,
            &parent_role_count,
            next_parent_role_count.as_ref(),
            &mut next_meta,
            &mut next_partition,
            &mut next_record,
            maximum_component_registry_bytes,
        )?;
        validate_subtree_removal_record(&next_record)?;
        RootComponentRegistryStore::remove_subtree_leaf_membership(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record.clone(),
            leaf,
            &traversal,
            &parent_role_count,
            next_parent_role_count,
        )
        .map_err(map_allocation_commit_error)?;
        Ok(subtree_removal_record_to_view(next_record))
    }

    #[expect(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        reason = "the exact leaf selection and both independently observed recipients are one durable transition"
    )]
    pub(crate) fn mark_subtree_leaf_directory_synchronized(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_traversal_steps: u32,
        expected_leaf_canister_id: Principal,
        expected_leaf_parent_canister_id: Principal,
        authority: ComponentRuntimeDirectoryAuthority,
        authority_hash: [u8; 32],
        owning_component: Option<ComponentRuntimeDirectoryConvergenceEvidence>,
        parent: Option<ComponentRuntimeDirectoryConvergenceEvidence>,
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        let record = RootComponentRegistryStore::subtree_removal(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        validate_subtree_removal_record(&record)?;
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        validate_subtree_removal_root(&record, &current.root)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        validate_subtree_removal_progress(&partition, &record)?;

        let expected_selection = SubtreeLeafSelection::new(
            expected_traversal_steps,
            expected_leaf_canister_id,
            expected_leaf_parent_canister_id,
        );
        if completed_subtree_leaf_for_selection(&record, &partition, expected_selection)?.is_some()
        {
            return Ok(subtree_removal_record_to_view(record));
        }
        let membership_removed = match &record.progress {
            RootComponentSubtreeRemovalProgressRecord::MembershipRemoved(receipt) => receipt,
            RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(receipt) => {
                let durable_selection = SubtreeLeafSelection::from_record(
                    record.traversal_steps,
                    &receipt
                        .membership_removed
                        .deleted
                        .deletion
                        .stopped
                        .stop
                        .leaf,
                );
                if durable_selection == expected_selection {
                    return Ok(subtree_removal_record_to_view(record));
                }
                return Err(InternalError::conflict());
            }
            RootComponentSubtreeRemovalProgressRecord::Fenced
            | RootComponentSubtreeRemovalProgressRecord::Traversing { .. }
            | RootComponentSubtreeRemovalProgressRecord::LeafSelected { .. }
            | RootComponentSubtreeRemovalProgressRecord::StopIntent(_)
            | RootComponentSubtreeRemovalProgressRecord::Stopped(_)
            | RootComponentSubtreeRemovalProgressRecord::DeleteIntent(_)
            | RootComponentSubtreeRemovalProgressRecord::Deleted(_)
            | RootComponentSubtreeRemovalProgressRecord::Completed(_) => {
                return Err(InternalError::unavailable());
            }
        };
        let leaf = &membership_removed.deleted.deletion.stopped.stop.leaf;
        if SubtreeLeafSelection::from_record(record.traversal_steps, leaf) != expected_selection {
            return Err(InternalError::conflict());
        }

        let owning_binding = ManagedCanisterBinding::Component(partition.binding.clone());
        let coverage = subtree_directory_coverage(&partition, &authority, authority_hash)?;
        let owning_component = match (partition.status, owning_component) {
            (ComponentLifecycleStatus::Active, Some(evidence)) => {
                let (observed_coverage, evidence) =
                    subtree_directory_convergence_record(&partition, &owning_binding, evidence)?;
                if observed_coverage != coverage {
                    return Err(InternalError::conflict());
                }
                Some(evidence)
            }
            (ComponentLifecycleStatus::Draining, None)
                if component_has_terminal_quiescence(&partition)? =>
            {
                None
            }
            _ => {
                return Err(InternalError::conflict());
            }
        };
        let parent = subtree_directory_parent_convergence_record(
            &partition,
            component,
            leaf.parent_canister_id,
            parent,
            &coverage,
        )?;

        let mut next_record = record.clone();
        next_record.progress = RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(
            RootComponentSubtreeDirectorySynchronizedRecord {
                membership_removed: membership_removed.clone(),
                covered_fleet_registry_revision: coverage.fleet_registry_revision,
                covered_fleet_registry_content_hash: coverage.fleet_registry_content_hash,
                covered_component_registry_revision: coverage.component_registry_revision,
                covered_component_registry_content_hash: coverage.component_registry_content_hash,
                covered_authority_hash: coverage.authority_hash,
                owning_component,
                parent,
            },
        );
        validate_subtree_removal_record(&next_record)?;
        validate_subtree_removal_root(&next_record, &current.root)?;
        validate_subtree_removal_progress(&partition, &next_record)?;
        let (next_partition, next_meta) = subtree_removal_progress_state(
            &current,
            &partition,
            &record,
            &next_record,
            maximum_component_registry_bytes,
        )?;
        RootComponentRegistryStore::replace_subtree_removal(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(subtree_removal_record_to_view(next_record))
    }

    pub(crate) fn finalize_subtree_leaf(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_traversal_steps: u32,
        expected_leaf_canister_id: Principal,
        expected_leaf_parent_canister_id: Principal,
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        let record = RootComponentRegistryStore::subtree_removal(component, operation_id)
            .ok_or_else(InternalError::unavailable)?;
        validate_subtree_removal_record(&record)?;
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        validate_subtree_removal_root(&record, &current.root)?;
        let partition = RootComponentRegistryStore::partition(component)
            .ok_or_else(InternalError::unavailable)?;
        validate_partition_record(&partition)?;
        validate_subtree_removal_progress(&partition, &record)?;

        let expected_selection = SubtreeLeafSelection::new(
            expected_traversal_steps,
            expected_leaf_canister_id,
            expected_leaf_parent_canister_id,
        );
        if completed_subtree_leaf_for_selection(&record, &partition, expected_selection)?.is_some()
        {
            return Ok(subtree_removal_record_to_view(record));
        }

        let RootComponentSubtreeRemovalProgressRecord::DirectorySynchronized(receipt) =
            &record.progress
        else {
            return Err(InternalError::unavailable());
        };
        let leaf = &receipt
            .membership_removed
            .deleted
            .deletion
            .stopped
            .stop
            .leaf;
        if SubtreeLeafSelection::from_record(record.traversal_steps, leaf) != expected_selection {
            return Err(InternalError::conflict());
        }

        let completed_leaves = record
            .completed_leaves
            .checked_add(1)
            .ok_or_else(InternalError::resource_exhausted)?;
        if completed_leaves > record.maximum_completed_leaves {
            return Err(InternalError::resource_exhausted());
        }
        let completed_leaf = completed_subtree_leaf_record(&record, receipt)?;
        validate_subtree_removal_completed_leaf(&record, &partition, &completed_leaf)?;
        let next_progress =
            finalized_subtree_removal_progress(component, &partition, &record, receipt)?;

        let mut next_record = record.clone();
        next_record.completed_leaves = completed_leaves;
        next_record.progress = next_progress;
        validate_subtree_removal_record(&next_record)?;
        validate_subtree_removal_root(&next_record, &current.root)?;
        if matches!(
            &next_record.progress,
            RootComponentSubtreeRemovalProgressRecord::Traversing { .. }
        ) {
            validate_subtree_removal_progress(&partition, &next_record)?;
        }
        let (next_partition, next_meta) = subtree_removal_leaf_finalization_state(
            &current,
            &partition,
            &record,
            &next_record,
            &completed_leaf,
            maximum_component_registry_bytes,
        )?;
        RootComponentRegistryStore::finalize_subtree_removal_leaf(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record,
            next_record,
            completed_leaf,
        )
        .map_err(map_allocation_commit_error)?;
        Self::subtree_removal(component, operation_id)?.ok_or_else(InternalError::invariant)
    }
}
