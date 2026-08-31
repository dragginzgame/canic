//! Module: ops::component_registry::top_level_allocation
//!
//! Responsibility: reserve and advance one top-level Component allocation through verification.
//! Does not own: canister effects, workflow ordering, child allocation, membership, or retirement.
//! Boundary: commits exact pre-journalled allocation transitions through the existing Registry store.

use super::{
    ComponentRegistryOps, ComponentSpecInstanceCounts, PeerComponentInstanceCounts,
    RootComponentCreationPlan, RootComponentInstallPlan, advance_install_phase,
    allocation_record_to_view, creation_charged_entry_bytes,
    ensure_root_accepts_top_level_allocation, exact_active_partition, install_charged_entry_bytes,
    map_allocation_commit_error, partition_record_to_view, validate_charged_record_size,
    validate_creation_capacity, validate_install_capacity, validate_install_effect_record,
};
use crate::{
    storage::stable::component_registry::{
        RootComponentAllocationProgressRecord, RootComponentAllocationRecord,
        RootComponentCreationEffectRecord, RootComponentInstallEffectRecord,
        RootComponentRegistryStore,
    },
    view::component_registry::{ComponentRegistryPartitionView, RootComponentAllocationView},
};
use canic_core::{
    cdk::types::Principal,
    control_plane_support::{
        error::InternalError, model::replay::ReplayCostGuardSettlement,
        policy::component_allocation::TopLevelComponentAllocationDecision,
    },
    dto::component_registry::ComponentProvisioningOrigin,
    ids::{ComponentBinding, ComponentSpecId},
};

impl ComponentRegistryOps {
    pub(crate) fn allocation(operation_id: [u8; 32]) -> Option<RootComponentAllocationView> {
        RootComponentRegistryStore::allocation(operation_id).map(allocation_record_to_view)
    }

    /// Reconstruct the immutable Registry partition established by active membership.
    ///
    /// The current partition may have advanced through later descendant work, so callers must
    /// validate terminal activation against this receipt-bound historical authority instead of
    /// comparing the current head with the earlier prepared head.
    pub(crate) fn active_membership_partition(
        operation_id: [u8; 32],
    ) -> Result<ComponentRegistryPartitionView, InternalError> {
        let record = RootComponentRegistryStore::allocation(operation_id)
            .ok_or_else(InternalError::unavailable)?;
        let RootComponentAllocationProgressRecord::Committed { commitment, .. } = &record.progress
        else {
            return Err(InternalError::conflict());
        };
        let membership = commitment
            .membership
            .as_ref()
            .ok_or_else(InternalError::conflict)?;
        let active = exact_active_partition(&record, commitment, membership)?;
        Ok(partition_record_to_view(active))
    }

    pub(crate) fn component_spec_counts(
        component_spec: &ComponentSpecId,
    ) -> Result<ComponentSpecInstanceCounts, InternalError> {
        let (reserved, committed) = RootComponentRegistryStore::allocation_counts(component_spec);
        Ok(ComponentSpecInstanceCounts {
            reserved: u32::try_from(reserved).map_err(|_| InternalError::invariant())?,
            committed: u32::try_from(committed).map_err(|_| InternalError::invariant())?,
        })
    }

    pub(crate) fn peer_component_counts(
        requester: &ComponentBinding,
        target_component_spec: &ComponentSpecId,
    ) -> Result<PeerComponentInstanceCounts, InternalError> {
        let (reserved, committed) =
            RootComponentRegistryStore::peer_allocation_counts(requester, target_component_spec);
        Ok(PeerComponentInstanceCounts {
            reserved: u32::try_from(reserved).map_err(|_| InternalError::invariant())?,
            committed: u32::try_from(committed).map_err(|_| InternalError::invariant())?,
        })
    }

    pub(crate) fn require_top_level_allocation_open() -> Result<(), InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        ensure_root_accepts_top_level_allocation(&current)
    }

    pub(crate) fn reserve_allocation(
        decision: TopLevelComponentAllocationDecision,
        operation_id: [u8; 32],
        provisioning_origin: ComponentProvisioningOrigin,
        root_runtime_active: bool,
    ) -> Result<RootComponentAllocationView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
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
                Err(InternalError::conflict())
            };
        }
        ensure_root_accepts_top_level_allocation(&current)?;
        match (current.initial_inventory.as_ref(), root_runtime_active) {
            (Some(_), false) => return Err(InternalError::conflict()),
            (Some(receipt), true) if !receipt.root_runtime_activated => {
                return Err(InternalError::invariant());
            }
            (None, true) => return Err(InternalError::invariant()),
            (None, false) | (Some(_), true) => {}
        }

        if current.next_allocation_sequence != record.allocation_sequence {
            return Err(InternalError::conflict());
        }
        let entry_bytes = RootComponentRegistryStore::allocation_entry_bytes(&record);
        let encoded_bytes = current
            .encoded_bytes
            .checked_add(entry_bytes)
            .ok_or_else(InternalError::resource_exhausted)?;
        if encoded_bytes > current.root.limits.maximum_registry_bytes {
            return Err(InternalError::resource_exhausted());
        }
        let mut next = current.clone();
        next.next_allocation_sequence = next
            .next_allocation_sequence
            .checked_add(1)
            .ok_or_else(InternalError::resource_exhausted)?;
        next.reserved_component_instances = next
            .reserved_component_instances
            .checked_add(1)
            .ok_or_else(InternalError::resource_exhausted)?;
        next.encoded_bytes = encoded_bytes;

        RootComponentRegistryStore::reserve_allocation(&current, next, record.clone())
            .map_err(map_allocation_commit_error)?;
        Ok(allocation_record_to_view(record))
    }

    pub(crate) fn validate_creation_capacity(
        operation_id: [u8; 32],
        plan: &RootComponentCreationPlan,
    ) -> Result<(), InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::allocation(operation_id)
            .ok_or_else(InternalError::unavailable)?;
        if !matches!(
            record.progress,
            RootComponentAllocationProgressRecord::Reserved
        ) {
            return Err(InternalError::conflict());
        }

        let charged_entry_bytes = creation_charged_entry_bytes(&record, plan);
        validate_creation_capacity(&current, &record, charged_entry_bytes).map(|_| ())
    }

    pub(crate) fn begin_creation(
        operation_id: [u8; 32],
        plan: RootComponentCreationPlan,
        cost_guard_settlement: ReplayCostGuardSettlement,
    ) -> Result<RootComponentAllocationView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::allocation(operation_id)
            .ok_or_else(InternalError::unavailable)?;
        if !matches!(
            record.progress,
            RootComponentAllocationProgressRecord::Reserved
        ) {
            return Err(InternalError::conflict());
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
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::allocation(operation_id)
            .ok_or_else(InternalError::unavailable)?;
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
            }
            | RootComponentAllocationProgressRecord::Removed {
                canister: existing, ..
            } if existing == &canister => return Ok(allocation_record_to_view(record)),
            RootComponentAllocationProgressRecord::Created { .. }
            | RootComponentAllocationProgressRecord::InstallIntent { .. }
            | RootComponentAllocationProgressRecord::Installed { .. }
            | RootComponentAllocationProgressRecord::Verified { .. }
            | RootComponentAllocationProgressRecord::Committed { .. }
            | RootComponentAllocationProgressRecord::Removed { .. }
            | RootComponentAllocationProgressRecord::Reserved => {
                return Err(InternalError::conflict());
            }
        };
        let charged_entry_bytes = effect.charged_entry_bytes;
        let mut next_record = record.clone();
        next_record.progress = RootComponentAllocationProgressRecord::Created { effect, canister };
        validate_charged_record_size(&next_record, charged_entry_bytes)?;
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
        RootComponentRegistryStore::replace_allocation(
            &current,
            next_meta,
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
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::allocation(operation_id)
            .ok_or_else(InternalError::unavailable)?;
        if !matches!(
            record.progress,
            RootComponentAllocationProgressRecord::Created { .. }
        ) {
            return Err(InternalError::conflict());
        }

        let charged_entry_bytes = install_charged_entry_bytes(&record, plan)?;
        validate_install_capacity(&current, &record, charged_entry_bytes).map(|_| ())
    }

    pub(crate) fn begin_install(
        operation_id: [u8; 32],
        plan: RootComponentInstallPlan,
        cost_guard_settlement: ReplayCostGuardSettlement,
    ) -> Result<RootComponentAllocationView, InternalError> {
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::allocation(operation_id)
            .ok_or_else(InternalError::unavailable)?;
        let (creation, canister) = match &record.progress {
            RootComponentAllocationProgressRecord::Created { effect, canister } => {
                (effect.clone(), *canister)
            }
            _ => return Err(InternalError::conflict()),
        };
        let charged_entry_bytes = install_charged_entry_bytes(&record, &plan)?;
        let next_encoded_bytes = validate_install_capacity(&current, &record, charged_entry_bytes)?;
        let mut next_record = record.clone();
        next_record.progress = RootComponentAllocationProgressRecord::InstallIntent {
            creation,
            canister,
            installation: RootComponentInstallEffectRecord {
                raw_module_hash: plan.raw_module_hash,
                protocol_profile_digest: plan.protocol_profile_digest,
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
        let current =
            RootComponentRegistryStore::current().ok_or_else(InternalError::unavailable)?;
        let record = RootComponentRegistryStore::allocation(operation_id)
            .ok_or_else(InternalError::unavailable)?;
        let (creation, canister, existing) = match &record.progress {
            RootComponentAllocationProgressRecord::InstallIntent {
                creation,
                canister,
                installation,
            } => (creation.clone(), *canister, installation),
            _ => return Err(InternalError::conflict()),
        };
        validate_install_effect_record(existing, plan)?;
        let charged_entry_bytes = existing.charged_entry_bytes;
        let mut next_record = record.clone();
        next_record.progress = RootComponentAllocationProgressRecord::InstallIntent {
            creation,
            canister,
            installation: RootComponentInstallEffectRecord {
                raw_module_hash: plan.raw_module_hash,
                protocol_profile_digest: plan.protocol_profile_digest,
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
}
