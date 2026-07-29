//! Module: ops::component_registry
//!
//! Responsibility: read and commit Component Registry authority and lifecycle progress.
//! Does not own: Store, Fleet Registry, topology, admission, or lifecycle validation.
//! Boundary: converts stable records into read-only views before workflow use.

use crate::{
    storage::stable::component_registry::{
        ComponentRegistryChildRecord, ComponentRegistryChildTraversalRecord,
        ComponentRegistryParentRoleCountRecord, ComponentRegistryPartitionRecord,
        RootComponentAllocationCommitError, RootComponentAllocationProgressRecord,
        RootComponentAllocationRecord, RootComponentChildAllocationProgressRecord,
        RootComponentChildAllocationRecord, RootComponentChildCommitmentRecord,
        RootComponentChildInstallEffectRecord, RootComponentChildMembershipRecord,
        RootComponentCommitmentRecord, RootComponentCreationEffectRecord,
        RootComponentInitialInventoryRecord, RootComponentInstallEffectRecord,
        RootComponentMembershipRecord, RootComponentRegistryCommitError,
        RootComponentRegistryMetaRecord, RootComponentRegistryStore,
        RootComponentSubtreeRemovalProgressRecord, RootComponentSubtreeRemovalRecord,
    },
    view::component_registry::{
        ComponentDirectoryCanonicalCursor, ComponentDirectoryChildView,
        ComponentDirectoryPageSelection, ComponentDirectoryPageView,
        ComponentRegistryPartitionView, RootComponentAllocationProgressView,
        RootComponentAllocationView, RootComponentChildAllocationProgressView,
        RootComponentChildAllocationView, RootComponentChildCommitmentView,
        RootComponentChildInstallEffectView, RootComponentChildMembershipView,
        RootComponentCommitmentView, RootComponentCreationEffectView,
        RootComponentInitialInventoryView, RootComponentInstallEffectView,
        RootComponentMembershipView, RootComponentRegistryView,
        RootComponentSubtreeRemovalProgressView, RootComponentSubtreeRemovalView,
    },
};
use candid::CandidType;
use canic_core::{
    cdk::types::{Cycles, Principal},
    control_plane_support::{
        error::InternalError,
        model::replay::ReplayCostGuardSettlement,
        ops::component_runtime::ComponentRuntimeOps,
        policy::{
            component_allocation::TopLevelComponentAllocationDecision,
            component_child_allocation::ComponentChildAllocationDecision,
        },
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
        CanisterRole, ComponentBinding, ComponentChildBinding, ComponentInstanceId,
        ComponentSpecId, FleetSubnetRootBinding, FleetSubnetRootReleaseSet, IntentId,
        ManagedCanisterBinding,
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
/// RootComponentInitialInventoryPlan
///
/// Exact sealed initial Component operations consumed by root activation workflow.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentInitialInventoryPlan {
    pub receipt: RootComponentInitialInventoryView,
    pub operation_ids: Vec<[u8; 32]>,
}

#[derive(CandidType)]
struct RootComponentInitialInventoryHashEntry {
    operation_id: [u8; 32],
    allocation_sequence: u64,
    component: ComponentInstanceId,
    component_spec: ComponentSpecId,
    spec_hash: [u8; 32],
    role: CanisterRole,
    provisioning_origin: ComponentProvisioningOrigin,
    release_set: FleetSubnetRootReleaseSet,
    prepared_registry: ComponentRegistryHead,
    prepared_registry_encoded_bytes: u64,
    prepared_directory_synchronized_at_ns: u64,
    prepared_directory_authority_hash: [u8; 32],
    active_binding: ComponentBinding,
    active_registry: ComponentRegistryHead,
    active_registry_encoded_bytes: u64,
    active_directory_synchronized_at_ns: u64,
    active_directory_authority_hash: [u8; 32],
}

struct CompleteInitialInventory {
    component_count: u32,
    inventory_hash: [u8; 32],
    operation_ids: Vec<[u8; 32]>,
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

///
/// RootComponentChildInstallPlan
///
/// Exact child module source and immutable target binding selected before installation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootComponentChildInstallPlan {
    pub raw_module_hash: [u8; 32],
    pub chunk_hashes: Vec<Vec<u8>>,
    pub binding: ComponentChildBinding,
    pub maximum_registry_bytes: u64,
}

impl ComponentRegistryOps {
    pub(crate) fn current() -> Option<RootComponentRegistryView> {
        RootComponentRegistryStore::current().map(record_to_view)
    }

    pub(crate) fn seal_initial_inventory(
        fleet_activation_operation_id: [u8; 32],
        sealed_at_ns: u64,
    ) -> Result<RootComponentInitialInventoryPlan, InternalError> {
        if sealed_at_ns == 0 {
            return Err(InternalError::invalid_input(
                "initial Component inventory seal time must be positive",
            ));
        }
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
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
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let receipt = current.initial_inventory.ok_or_else(|| {
            InternalError::unavailable("initial Component inventory has not been sealed")
        })?;
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
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let receipt = current.initial_inventory.ok_or_else(|| {
            InternalError::unavailable("initial Component inventory has not been sealed")
        })?;
        if receipt.fleet_activation_operation_id != fleet_activation_operation_id {
            return Err(InternalError::conflict(
                "initial Component inventory is bound to a different Fleet activation",
            ));
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
            known_created_component_canisters: 0,
            encoded_bytes: 0,
            initial_inventory: None,
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
        root_runtime_active: bool,
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
        match (current.initial_inventory.as_ref(), root_runtime_active) {
            (Some(_), false) => {
                return Err(InternalError::conflict(
                    "initial Component inventory is sealed while the root runtime is Prepared",
                ));
            }
            (Some(receipt), true) if !receipt.root_runtime_activated => {
                return Err(InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "active root runtime has no terminal initial-inventory receipt",
                ));
            }
            (None, true) => {
                return Err(InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "active root runtime has no sealed initial Component inventory",
                ));
            }
            (None, false) | (Some(_), true) => {}
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
        let mut next_meta = current.clone();
        next_meta.known_created_component_canisters = next_meta
            .known_created_component_canisters
            .checked_add(1)
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "known-created Component Canister count overflowed",
                )
            })?;
        let allocated_component_canisters = current
            .reserved_component_instances
            .checked_add(current.committed_component_instances)
            .and_then(|count| count.checked_add(current.managed_descendants))
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "allocated Component-tree Canister count overflowed",
                )
            })?;
        if next_meta.known_created_component_canisters > allocated_component_canisters {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "known-created Component Canisters exceed allocated Component-tree capacity",
            ));
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

    pub(crate) fn directory_page(
        component: ComponentInstanceId,
        selection: &ComponentDirectoryPageSelection,
        scan_limit: usize,
    ) -> Result<ComponentDirectoryPageView, InternalError> {
        if scan_limit == 0 {
            return Err(InternalError::invalid_input(
                "Component Directory page scan limit must be positive",
            ));
        }
        if selection.start_after.as_ref().is_some_and(|cursor| {
            selection
                .parent_canister_id
                .is_some_and(|parent| cursor.parent_canister_id != parent)
        }) {
            return Err(InternalError::invalid_input(
                "Component Directory cursor is outside the selected parent",
            ));
        }
        if selection.start_after.as_ref().is_some_and(|cursor| {
            selection.parent_canister_id.is_some()
                && selection
                    .role
                    .as_ref()
                    .is_some_and(|role| role != &cursor.role)
        }) {
            return Err(InternalError::invalid_input(
                "Component Directory cursor is outside the selected parent-role index",
            ));
        }

        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        let start_after = selection.start_after.as_ref().map(|cursor| {
            (
                &cursor.parent_canister_id,
                &cursor.role,
                &cursor.canister_id,
            )
        });
        let mut traversals = RootComponentRegistryStore::child_traversals_page(
            component,
            selection.parent_canister_id,
            selection.role.as_ref(),
            start_after,
            scan_limit.saturating_add(1),
        );
        let has_more = traversals.len() > scan_limit;
        traversals.truncate(scan_limit);
        let next_cursor = has_more
            .then(|| traversals.last().map(traversal_record_to_cursor))
            .flatten();

        let mut entries = Vec::with_capacity(traversals.len());
        for traversal in traversals {
            validate_child_traversal_record(component, &traversal)?;
            let child = RootComponentRegistryStore::child(component, traversal.canister_id)
                .ok_or_else(|| {
                    InternalError::invariant(
                        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                        "Component Directory traversal has no normalized child row",
                    )
                })?;
            validate_child_record(&partition, &child)?;
            if traversal.parent_canister_id != child.parent_canister_id
                || traversal.role != child.role
                || traversal.canister_id != child.canister_id
                || RootComponentRegistryStore::component_for_principal(traversal.parent_canister_id)
                    != Some(component)
            {
                return Err(InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "Component Directory traversal differs from normalized child authority",
                ));
            }
            if selection
                .role
                .as_ref()
                .is_some_and(|role| role != &child.role)
                || selection
                    .status
                    .is_some_and(|status| status != child.status)
            {
                continue;
            }
            entries.push(child_record_to_directory_view(&partition, child));
        }

        Ok(ComponentDirectoryPageView {
            entries,
            next_cursor,
        })
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

    pub(crate) fn committed_child_authority(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        fleet_directory: &FleetDirectorySnapshot,
    ) -> Result<
        (
            RootComponentChildAllocationView,
            ComponentRegistryPartitionView,
        ),
        InternalError,
    > {
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        let RootComponentChildAllocationProgressRecord::Committed { commitment, .. } =
            &record.progress
        else {
            return Err(InternalError::conflict(
                "Component Child allocation has no committed Registry authority",
            ));
        };
        let committed = exact_committed_child_partition(&record, commitment)?;
        validate_child_directory_authority_hash(&committed, fleet_directory, commitment)?;
        Ok((
            child_allocation_record_to_view(record),
            partition_record_to_view(committed),
        ))
    }

    pub(crate) fn component_for_principal(
        canister: Principal,
    ) -> Option<canic_core::ids::ComponentInstanceId> {
        RootComponentRegistryStore::component_for_principal(canister)
    }

    pub(crate) fn registered_parent(
        component: ComponentInstanceId,
        canister: Principal,
    ) -> Result<Option<(ManagedCanisterBinding, ComponentLifecycleStatus)>, InternalError> {
        if RootComponentRegistryStore::component_for_principal(canister) != Some(component) {
            return Ok(None);
        }
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "indexed Component Registry member has no partition",
            )
        })?;
        validate_partition_record(&partition)?;
        if partition.binding.canister_id == canister {
            return Ok(Some((
                ManagedCanisterBinding::Component(partition.binding),
                partition.status,
            )));
        }
        let child = RootComponentRegistryStore::child(component, canister).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "indexed Component Registry member has no normalized child row",
            )
        })?;
        validate_child_record(&partition, &child)?;
        let traversal = ComponentRegistryChildTraversalRecord {
            component,
            parent_canister_id: child.parent_canister_id,
            role: child.role.clone(),
            canister_id: child.canister_id,
        };
        if RootComponentRegistryStore::component_for_principal(child.parent_canister_id)
            != Some(component)
            || RootComponentRegistryStore::child_traversal(
                component,
                traversal.parent_canister_id,
                &traversal.role,
                traversal.canister_id,
            )
            .as_ref()
                != Some(&traversal)
        {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "indexed Component Registry child differs from its parent or traversal index",
            ));
        }
        Ok(Some((
            ManagedCanisterBinding::ComponentChild(ComponentChildBinding {
                component: partition.binding,
                parent_canister_id: child.parent_canister_id,
                role: child.role,
                canister_id: child.canister_id,
            }),
            child.status,
        )))
    }

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

    pub(crate) fn subtree_removal(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
    ) -> Result<Option<RootComponentSubtreeRemovalView>, InternalError> {
        let Some(record) = RootComponentRegistryStore::subtree_removal(component, operation_id)
        else {
            return Ok(None);
        };
        validate_subtree_removal_record(&record)?;
        Ok(Some(subtree_removal_record_to_view(record)))
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one synchronous transaction validates and durably charges the exact subtree fence"
    )]
    pub(crate) fn begin_subtree_removal(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        target_canister_id: Principal,
        reserved_against_registry: ComponentRegistryHead,
        maximum_component_registry_bytes: u64,
    ) -> Result<RootComponentSubtreeRemovalView, InternalError> {
        if let Some(existing) = RootComponentRegistryStore::subtree_removal(component, operation_id)
        {
            validate_subtree_removal_record(&existing)?;
            return if existing.target.canister_id == target_canister_id
                && existing.reserved_against_registry == reserved_against_registry
            {
                Ok(subtree_removal_record_to_view(existing))
            } else {
                Err(InternalError::conflict(
                    "Component subtree-removal operation is already bound to a different fence",
                ))
            };
        }

        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        if partition.status != ComponentLifecycleStatus::Active
            || reserved_against_registry
                != (ComponentRegistryHead {
                    component,
                    revision: partition.revision,
                    content_hash: partition.content_hash,
                })
        {
            return Err(InternalError::conflict(
                "Component subtree-removal fence authority changed before durable mutation",
            ));
        }
        if !RootComponentRegistryStore::subtree_removals(component).is_empty() {
            return Err(InternalError::conflict(
                "Component already has an in-progress subtree-removal operation",
            ));
        }

        let target =
            RootComponentRegistryStore::child(component, target_canister_id).ok_or_else(|| {
                InternalError::unavailable(
                    "Component subtree-removal target is not a registered child",
                )
            })?;
        validate_registered_child_record(&partition, &target)?;
        if target.status != ComponentLifecycleStatus::Active {
            return Err(InternalError::conflict(
                "ordinary Component subtree removal requires an Active target",
            ));
        }
        let traversal_limit = partition
            .committed_descendants
            .checked_add(1)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component descendant count overflow")
            })?;
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
                return Err(InternalError::unavailable(
                    "Component subtree has an incomplete child lifecycle operation",
                ));
            }
        }

        let record = RootComponentSubtreeRemovalRecord {
            operation_id,
            component,
            target,
            reserved_against_registry,
            progress: RootComponentSubtreeRemovalProgressRecord::Fenced,
        };
        validate_subtree_removal_record(&record)?;
        let (next_partition, registry_delta) = subtree_fence_partition(&partition, &record)?;
        if next_partition.encoded_bytes > maximum_component_registry_bytes {
            return Err(InternalError::resource_exhausted(format!(
                "Component subtree-removal fence requires {} bytes, exceeding protected Component limit {maximum_component_registry_bytes}",
                next_partition.encoded_bytes
            )));
        }
        let mut next_meta = current.clone();
        next_meta.encoded_bytes = next_meta
            .encoded_bytes
            .checked_add(registry_delta)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        if next_meta.encoded_bytes > next_meta.root.limits.maximum_registry_bytes {
            return Err(InternalError::resource_exhausted(format!(
                "Component subtree-removal fence requires {} root Registry bytes, exceeding protected limit {}",
                next_meta.encoded_bytes, next_meta.root.limits.maximum_registry_bytes
            )));
        }

        RootComponentRegistryStore::begin_subtree_removal(
            &current,
            next_meta,
            &partition,
            next_partition,
            &record.target,
            record.clone(),
        )
        .map_err(map_allocation_commit_error)?;
        Ok(subtree_removal_record_to_view(record))
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
        if record.component != component
            || record.parent_canister_id != parent_canister_id
            || &record.child_role != child_role
            || record.instances == 0
        {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Registry parent-role count index is invalid",
            ));
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
        reserved_against_registry: ComponentRegistryHead,
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition =
            RootComponentRegistryStore::partition(decision.component).ok_or_else(|| {
                InternalError::unavailable("Component Registry partition has not been committed")
            })?;
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
                Err(InternalError::conflict(
                    "Component Child allocation operation is already bound to different intent",
                ))
            };
        }
        if partition.binding.component_spec != decision.component_spec
            || partition.binding.spec_hash != decision.spec_hash
            || partition.release_set != current.release_set
            || partition.status != ComponentLifecycleStatus::Active
            || record.reserved_against_registry
                != (ComponentRegistryHead {
                    component: decision.component,
                    revision: partition.revision,
                    content_hash: partition.content_hash,
                })
        {
            return Err(InternalError::conflict(
                "Component Child reservation authority changed before durable mutation",
            ));
        }
        let traversal_limit = partition
            .committed_descendants
            .checked_add(1)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component descendant count overflow")
            })?;
        for removal in RootComponentRegistryStore::subtree_removals(record.component) {
            validate_subtree_removal_record(&removal)?;
            if canister_is_in_subtree(
                &partition,
                record.parent_canister_id,
                removal.target.canister_id,
                traversal_limit,
            )? {
                return Err(InternalError::conflict(
                    "Component Child parent is fenced by an in-progress subtree removal",
                ));
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
                .ok_or_else(|| {
                    InternalError::resource_exhausted("per-parent child count overflow")
                })?,
        };
        if next_count.instances > decision.maximum_instances_per_parent {
            return Err(InternalError::resource_exhausted(
                "registered parent exhausted its direct-child role capacity",
            ));
        }
        let (next_partition, registry_delta) =
            child_reservation_partition(&partition, &record, current_count.as_ref(), &next_count)?;
        let component_descendants = next_partition
            .reserved_descendants
            .checked_add(next_partition.committed_descendants)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component descendant count overflow")
            })?;
        if component_descendants > decision.maximum_descendants {
            return Err(InternalError::resource_exhausted(
                "Component descendant capacity is exhausted",
            ));
        }
        if next_partition.encoded_bytes > decision.maximum_registry_bytes {
            return Err(InternalError::resource_exhausted(format!(
                "Component Child reservation requires {} bytes, exceeding protected Component limit {}",
                next_partition.encoded_bytes, decision.maximum_registry_bytes
            )));
        }
        let mut next_meta = current.clone();
        next_meta.managed_descendants =
            next_meta
                .managed_descendants
                .checked_add(1)
                .ok_or_else(|| {
                    InternalError::resource_exhausted("root managed descendant count overflow")
                })?;
        next_meta.encoded_bytes = next_meta
            .encoded_bytes
            .checked_add(registry_delta)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let managed_canisters = 1_u32
            .checked_add(next_meta.reserved_component_instances)
            .and_then(|count| count.checked_add(next_meta.committed_component_instances))
            .and_then(|count| count.checked_add(next_meta.managed_descendants))
            .ok_or_else(|| {
                InternalError::resource_exhausted("root managed-Canister count overflow")
            })?;
        if managed_canisters > next_meta.root.limits.maximum_managed_canisters {
            return Err(InternalError::resource_exhausted(
                "root managed-Canister capacity is exhausted",
            ));
        }
        if next_meta.encoded_bytes > next_meta.root.limits.maximum_registry_bytes {
            return Err(InternalError::resource_exhausted(format!(
                "Component Child reservation requires {} root Registry bytes, exceeding protected limit {}",
                next_meta.encoded_bytes, next_meta.root.limits.maximum_registry_bytes
            )));
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
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        validate_child_creation_authority(&current, &partition, &record, plan)?;
        if !matches!(
            record.progress,
            RootComponentChildAllocationProgressRecord::Reserved
        ) {
            return Err(InternalError::conflict(
                "Component Child allocation has already crossed its creation-intent boundary",
            ));
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
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        validate_child_creation_authority(&current, &partition, &record, &plan)?;
        if !matches!(
            record.progress,
            RootComponentChildAllocationProgressRecord::Reserved
        ) {
            return Err(InternalError::conflict(
                "Component Child allocation has already crossed its creation-intent boundary",
            ));
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
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;

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
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
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
            | RootComponentChildAllocationProgressRecord::Committed { .. } => {
                return Err(InternalError::conflict(
                    "Component Child allocation is already bound to a different created Canister",
                ));
            }
            RootComponentChildAllocationProgressRecord::Reserved => {
                return Err(InternalError::conflict(
                    "Component Child allocation has no durable creation intent",
                ));
            }
        };
        if canister == Principal::anonymous()
            || canister == current.root.fleet_subnet_root
            || canister == current.root.authority.binding.coordinator
            || canister == partition.binding.canister_id
            || canister == record.parent_canister_id
            || RootComponentRegistryStore::component_for_principal(canister).is_some()
        {
            return Err(InternalError::conflict(
                "created Component Child principal conflicts with protected Registry authority",
            ));
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
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "known-created Component Canister count overflowed",
                )
            })?;
        let allocated_component_canisters = current
            .reserved_component_instances
            .checked_add(current.committed_component_instances)
            .and_then(|count| count.checked_add(current.managed_descendants))
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "allocated Component-tree Canister count overflowed",
                )
            })?;
        if next_meta.known_created_component_canisters > allocated_component_canisters {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "known-created Component Canisters exceed allocated Component-tree capacity",
            ));
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
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        validate_child_install_authority(&current, &partition, &record, plan)?;
        if !matches!(
            record.progress,
            RootComponentChildAllocationProgressRecord::Created { .. }
        ) {
            return Err(InternalError::conflict(
                "Component Child allocation is not ready to cross its install-intent boundary",
            ));
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
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        validate_child_install_authority(&current, &partition, &record, &plan)?;
        let (creation, canister) = match &record.progress {
            RootComponentChildAllocationProgressRecord::Created { effect, canister } => {
                (effect.clone(), *canister)
            }
            _ => {
                return Err(InternalError::conflict(
                    "Component Child allocation is not ready for installation",
                ));
            }
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
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;

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
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        validate_child_install_authority(&current, &partition, &record, plan)?;
        let (creation, canister, existing) = match &record.progress {
            RootComponentChildAllocationProgressRecord::InstallIntent {
                creation,
                canister,
                installation,
            } => (creation.clone(), *canister, installation),
            _ => {
                return Err(InternalError::conflict(
                    "Component Child allocation has no renewable install intent",
                ));
            }
        };
        validate_child_install_effect_record(existing, plan)?;
        let charged_entry_bytes = existing.charged_entry_bytes;
        let mut next_record = record.clone();
        next_record.progress = RootComponentChildAllocationProgressRecord::InstallIntent {
            creation,
            canister,
            installation: RootComponentChildInstallEffectRecord {
                raw_module_hash: plan.raw_module_hash,
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

    #[expect(
        clippy::too_many_lines,
        reason = "one synchronous operation validates and atomically commits every child index"
    )]
    pub(crate) fn commit_verified_child(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        directory_synchronized_at_ns: u64,
        fleet_directory: FleetDirectorySnapshot,
    ) -> Result<
        (
            RootComponentChildAllocationView,
            ComponentRegistryPartitionView,
        ),
        InternalError,
    > {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        if let RootComponentChildAllocationProgressRecord::Committed { commitment, .. } =
            &record.progress
        {
            let committed = exact_committed_child_partition(&record, commitment)?;
            validate_child_directory_authority_hash(&committed, &fleet_directory, commitment)?;
            return Ok((
                child_allocation_record_to_view(record),
                partition_record_to_view(committed),
            ));
        }
        if directory_synchronized_at_ns <= partition.directory_synchronized_at_ns {
            return Err(InternalError::invalid_input(
                "Component Child Directory synchronization must advance the current Component authority",
            ));
        }
        let RootComponentChildAllocationProgressRecord::Verified {
            creation,
            canister,
            installation,
        } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component Child allocation is not ready for Registry commitment",
            ));
        };

        let (next_record, next_partition, child, traversal) = committed_child_records(
            &record,
            creation,
            *canister,
            installation,
            &partition,
            directory_synchronized_at_ns,
            &fleet_directory,
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
                .ok_or_else(|| {
                    InternalError::resource_exhausted("Component Registry bytes overflow")
                })?;
        if actual_terminal_bytes > installation.charged_entry_bytes {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Child commitment exceeds its pre-install Registry byte reservation",
            ));
        }
        if next_partition.encoded_bytes > record.maximum_registry_bytes {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "pre-install child reservation exceeds the protected Component limit at commitment",
            ));
        }
        let registry_reduction = partition
            .encoded_bytes
            .checked_sub(next_partition.encoded_bytes)
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "exact Component Child commitment exceeded its maximum terminal precharge",
                )
            })?;
        let mut next_meta = current.clone();
        next_meta.encoded_bytes = next_meta
            .encoded_bytes
            .checked_sub(registry_reduction)
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "root Component Registry cannot release excess child precharge",
                )
            })?;

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
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        let RootComponentChildAllocationProgressRecord::Committed {
            creation,
            canister,
            installation,
            commitment,
        } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component Child allocation is not committed for Directory preparation",
            ));
        };
        let _committed = exact_committed_child_partition(&record, commitment)?;
        if commitment.directory_authority_hash != expected_authority_hash {
            return Err(InternalError::conflict(
                "Component Child Directory authority differs from its committed root receipt",
            ));
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
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Child Directory receipt changed its precharged stable footprint",
            ));
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
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        let RootComponentChildAllocationProgressRecord::Committed {
            creation,
            canister,
            installation,
            commitment,
        } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component Child allocation is not committed for runtime activation",
            ));
        };
        let _committed = exact_committed_child_partition(&record, commitment)?;
        if commitment.directory_authority_hash != expected_authority_hash
            || !commitment.directory_prepared
        {
            return Err(InternalError::conflict(
                "Component Child runtime activation requires its exact prepared Directory authority",
            ));
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
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Child runtime receipt changed its precharged stable footprint",
            ));
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
    ) -> Result<
        (
            RootComponentChildAllocationView,
            ComponentRegistryPartitionView,
        ),
        InternalError,
    > {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        let RootComponentChildAllocationProgressRecord::Committed {
            canister,
            commitment,
            ..
        } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component Child allocation is not committed for membership activation",
            ));
        };
        let _committed = exact_committed_child_partition(&record, commitment)?;
        if let Some(membership) = &commitment.membership {
            let active = exact_active_child_partition(&record, commitment, membership)?;
            validate_child_membership_directory_authority_hash(
                &active,
                &fleet_directory,
                membership,
            )?;
            return Ok((
                child_allocation_record_to_view(record),
                partition_record_to_view(active),
            ));
        }
        if !commitment.directory_prepared || !commitment.runtime_activated {
            return Err(InternalError::conflict(
                "Component Child membership activation requires terminal Directory and runtime receipts",
            ));
        }
        if directory_synchronized_at_ns <= partition.directory_synchronized_at_ns {
            return Err(InternalError::invalid_input(
                "active Component Child Directory synchronization must follow current authority",
            ));
        }
        let child = RootComponentRegistryStore::child(component, *canister).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "committed Component Child allocation has no normalized row",
            )
        })?;
        validate_child_record(&partition, &child)?;
        if child.status != ComponentLifecycleStatus::Prepared {
            return Err(InternalError::conflict(
                "Component Child membership activation requires a Prepared Registry row",
            ));
        }

        persist_child_membership_activation(
            &current,
            &partition,
            &record,
            &child,
            directory_synchronized_at_ns,
            &fleet_directory,
        )
    }

    pub(crate) fn mark_child_membership_synchronized(
        component: ComponentInstanceId,
        operation_id: [u8; 32],
        expected_authority_hash: [u8; 32],
    ) -> Result<RootComponentChildAllocationView, InternalError> {
        let current = RootComponentRegistryStore::current().ok_or_else(|| {
            InternalError::unavailable("root Component Registry authority has not been prepared")
        })?;
        let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
            InternalError::unavailable("Component Registry partition has not been committed")
        })?;
        validate_partition_record(&partition)?;
        let record = RootComponentRegistryStore::child_allocation(component, operation_id)
            .ok_or_else(|| {
                InternalError::unavailable(
                    "Component Child allocation operation has not been reserved",
                )
            })?;
        let RootComponentChildAllocationProgressRecord::Committed {
            creation,
            canister,
            installation,
            commitment,
        } = &record.progress
        else {
            return Err(InternalError::conflict(
                "Component Child allocation is not committed for membership synchronization",
            ));
        };
        let membership = commitment.membership.as_ref().ok_or_else(|| {
            InternalError::conflict("Component Child Registry membership has not been activated")
        })?;
        let _active = exact_active_child_partition(&record, commitment, membership)?;
        if membership.directory_authority_hash != expected_authority_hash {
            return Err(InternalError::conflict(
                "current Component Child Directory differs from its active membership authority",
            ));
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
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Child membership receipt changed its precharged stable footprint",
            ));
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

fn complete_initial_inventory(
    current: &RootComponentRegistryMetaRecord,
) -> Result<CompleteInitialInventory, InternalError> {
    if current.reserved_component_instances != 0 {
        return Err(InternalError::unavailable(
            "initial Component inventory still contains nonterminal allocations",
        ));
    }

    let mut allocations = RootComponentRegistryStore::allocations();
    allocations.sort_by_key(|record| record.allocation_sequence);
    let component_count = u32::try_from(allocations.len()).map_err(|_| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "initial Component inventory exceeds u32",
        )
    })?;
    if component_count != current.committed_component_instances
        || current.next_allocation_sequence != u64::from(component_count) + 1
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component Registry counters differ from the initial allocation inventory",
        ));
    }
    let maximum_known_created = component_count
        .checked_add(current.managed_descendants)
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "initial Component-tree Canister count overflowed",
            )
        })?;
    if current.known_created_component_canisters < component_count
        || current.known_created_component_canisters > maximum_known_created
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "known-created Canister counter differs from the complete initial inventory",
        ));
    }

    let partitions = RootComponentRegistryStore::partitions();
    if partitions.len() != allocations.len() {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "initial Component allocations and Registry partitions differ in cardinality",
        ));
    }

    let mut entries = Vec::with_capacity(allocations.len());
    let mut operation_ids = Vec::with_capacity(allocations.len());
    let mut encoded_bytes = 0_u64;
    for (index, record) in allocations.iter().enumerate() {
        let (entry, partition_bytes) = initial_inventory_hash_entry(record, index)?;
        encoded_bytes = encoded_bytes.checked_add(partition_bytes).ok_or_else(|| {
            InternalError::resource_exhausted("Component Registry bytes overflow")
        })?;
        operation_ids.push(record.operation_id);
        entries.push(entry);
    }
    if encoded_bytes != current.encoded_bytes {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "initial Component inventory differs from root Registry byte accounting",
        ));
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
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "initial Component allocation sequences are not consecutive",
        ));
    }
    let RootComponentAllocationProgressRecord::Committed { commitment, .. } = &record.progress
    else {
        return Err(InternalError::unavailable(
            "initial Component inventory contains an allocation without Registry commitment",
        ));
    };
    let membership = commitment.membership.as_ref().ok_or_else(|| {
        InternalError::unavailable(
            "initial Component inventory contains an allocation without active membership",
        )
    })?;
    if !commitment.directory_prepared
        || !commitment.runtime_activated
        || !membership.directory_synchronized
    {
        return Err(InternalError::unavailable(
            "initial Component inventory lacks terminal Directory, runtime or membership evidence",
        ));
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
    let payload = candid::encode_one(entries).map_err(|error| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            format!("initial Component inventory cannot be encoded: {error}"),
        )
    })?;
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
        return Err(InternalError::conflict(
            "initial Component inventory is bound to a different Fleet activation",
        ));
    }
    if receipt.component_count != component_count
        || receipt.inventory_hash != inventory_hash
        || receipt.sealed_at_ns == 0
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "sealed initial Component inventory differs from current protected authority",
        ));
    }
    Ok(())
}

fn update_initial_inventory_receipt(
    fleet_activation_operation_id: [u8; 32],
    expected_inventory_hash: [u8; 32],
    directories_converged: bool,
    root_runtime_activated: bool,
) -> Result<RootComponentInitialInventoryView, InternalError> {
    let current = RootComponentRegistryStore::current().ok_or_else(|| {
        InternalError::unavailable("root Component Registry authority has not been prepared")
    })?;
    let mut receipt = current.initial_inventory.ok_or_else(|| {
        InternalError::unavailable("initial Component inventory has not been sealed")
    })?;
    if receipt.fleet_activation_operation_id != fleet_activation_operation_id
        || receipt.inventory_hash != expected_inventory_hash
    {
        return Err(InternalError::conflict(
            "root activation receipt differs from its sealed initial Component inventory",
        ));
    }
    if root_runtime_activated && !directories_converged {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "root runtime activation cannot precede initial Directory convergence",
        ));
    }
    receipt.directories_converged |= directories_converged;
    receipt.root_runtime_activated |= root_runtime_activated;
    if receipt.root_runtime_activated && !receipt.directories_converged {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "root runtime receipt has no initial Directory convergence evidence",
        ));
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
        known_created_component_canisters: record.known_created_component_canisters,
        encoded_bytes: record.encoded_bytes,
        initial_inventory: record
            .initial_inventory
            .map(initial_inventory_record_to_view),
    }
}

const fn initial_inventory_record_to_view(
    record: RootComponentInitialInventoryRecord,
) -> RootComponentInitialInventoryView {
    RootComponentInitialInventoryView {
        fleet_activation_operation_id: record.fleet_activation_operation_id,
        component_count: record.component_count,
        inventory_hash: record.inventory_hash,
        sealed_at_ns: record.sealed_at_ns,
        directories_converged: record.directories_converged,
        root_runtime_activated: record.root_runtime_activated,
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
        descendant_content_hash: record.descendant_content_hash,
        directory_synchronized_at_ns: record.directory_synchronized_at_ns,
        reserved_descendants: record.reserved_descendants,
        committed_descendants: record.committed_descendants,
        encoded_bytes: record.encoded_bytes,
    }
}

fn child_allocation_record_to_view(
    record: RootComponentChildAllocationRecord,
) -> RootComponentChildAllocationView {
    RootComponentChildAllocationView {
        operation_id: record.operation_id,
        component: record.component,
        parent_canister_id: record.parent_canister_id,
        parent_role: record.parent_role,
        child_role: record.child_role,
        child_kind: record.child_kind,
        maximum_instances_per_parent: record.maximum_instances_per_parent,
        maximum_descendants: record.maximum_descendants,
        maximum_registry_bytes: record.maximum_registry_bytes,
        reserved_against_registry: record.reserved_against_registry,
        release_set: record.release_set,
        progress: match record.progress {
            RootComponentChildAllocationProgressRecord::Reserved => {
                RootComponentChildAllocationProgressView::Reserved
            }
            RootComponentChildAllocationProgressRecord::CreationIntent(effect) => {
                RootComponentChildAllocationProgressView::CreationIntent(
                    creation_effect_record_to_view(effect),
                )
            }
            RootComponentChildAllocationProgressRecord::Created { effect, canister } => {
                RootComponentChildAllocationProgressView::Created {
                    effect: creation_effect_record_to_view(effect),
                    canister,
                }
            }
            RootComponentChildAllocationProgressRecord::InstallIntent {
                creation,
                canister,
                installation,
            } => RootComponentChildAllocationProgressView::InstallIntent {
                creation: creation_effect_record_to_view(creation),
                canister,
                installation: child_install_effect_record_to_view(installation),
            },
            RootComponentChildAllocationProgressRecord::Installed {
                creation,
                canister,
                installation,
            } => RootComponentChildAllocationProgressView::Installed {
                creation: creation_effect_record_to_view(creation),
                canister,
                installation: child_install_effect_record_to_view(installation),
            },
            RootComponentChildAllocationProgressRecord::Verified {
                creation,
                canister,
                installation,
            } => RootComponentChildAllocationProgressView::Verified {
                creation: creation_effect_record_to_view(creation),
                canister,
                installation: child_install_effect_record_to_view(installation),
            },
            RootComponentChildAllocationProgressRecord::Committed {
                creation,
                canister,
                installation,
                commitment,
            } => RootComponentChildAllocationProgressView::Committed {
                creation: creation_effect_record_to_view(creation),
                canister,
                installation: child_install_effect_record_to_view(installation),
                commitment: child_commitment_record_to_view(commitment),
            },
        },
    }
}

fn subtree_removal_record_to_view(
    record: RootComponentSubtreeRemovalRecord,
) -> RootComponentSubtreeRemovalView {
    RootComponentSubtreeRemovalView {
        operation_id: record.operation_id,
        component: record.component,
        target_canister_id: record.target.canister_id,
        target_parent_canister_id: record.target.parent_canister_id,
        target_role: record.target.role,
        target_status: record.target.status,
        reserved_against_registry: record.reserved_against_registry,
        progress: match record.progress {
            RootComponentSubtreeRemovalProgressRecord::Fenced => {
                RootComponentSubtreeRemovalProgressView::Fenced
            }
        },
    }
}

fn child_commitment_record_to_view(
    commitment: RootComponentChildCommitmentRecord,
) -> RootComponentChildCommitmentView {
    RootComponentChildCommitmentView {
        registry: commitment.registry,
        descendant_content_hash: commitment.descendant_content_hash,
        registry_encoded_bytes: commitment.registry_encoded_bytes,
        reserved_descendants: commitment.reserved_descendants,
        committed_descendants: commitment.committed_descendants,
        directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
        directory_authority_hash: commitment.directory_authority_hash,
        directory_prepared: commitment.directory_prepared,
        runtime_activated: commitment.runtime_activated,
        membership: commitment.membership.map(child_membership_record_to_view),
    }
}

const fn child_membership_record_to_view(
    membership: RootComponentChildMembershipRecord,
) -> RootComponentChildMembershipView {
    RootComponentChildMembershipView {
        registry: membership.registry,
        descendant_content_hash: membership.descendant_content_hash,
        registry_encoded_bytes: membership.registry_encoded_bytes,
        reserved_descendants: membership.reserved_descendants,
        committed_descendants: membership.committed_descendants,
        directory_synchronized_at_ns: membership.directory_synchronized_at_ns,
        directory_authority_hash: membership.directory_authority_hash,
        directory_synchronized: membership.directory_synchronized,
    }
}

fn child_install_effect_record_to_view(
    effect: RootComponentChildInstallEffectRecord,
) -> RootComponentChildInstallEffectView {
    RootComponentChildInstallEffectView {
        raw_module_hash: effect.raw_module_hash,
        chunk_hashes: effect.chunk_hashes,
        binding: effect.binding,
        cost_guard_settlement: effect.cost_guard_settlement,
        charged_entry_bytes: effect.charged_entry_bytes,
    }
}

fn child_reservation_partition(
    current: &ComponentRegistryPartitionRecord,
    allocation: &RootComponentChildAllocationRecord,
    current_count: Option<&ComponentRegistryParentRoleCountRecord>,
    next_count: &ComponentRegistryParentRoleCountRecord,
) -> Result<(ComponentRegistryPartitionRecord, u64), InternalError> {
    if let Some(count) = current_count
        && (count.component != allocation.component
            || count.parent_canister_id != allocation.parent_canister_id
            || count.child_role != allocation.child_role
            || count.instances == 0)
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component Registry parent-role count index is invalid",
        ));
    }
    let current_partition_bytes = RootComponentRegistryStore::partition_entry_bytes(current);
    let current_count_bytes = current_count
        .map(RootComponentRegistryStore::parent_role_count_entry_bytes)
        .unwrap_or_default();
    let allocation_bytes = RootComponentRegistryStore::child_allocation_entry_bytes(allocation);
    let next_count_bytes = RootComponentRegistryStore::parent_role_count_entry_bytes(next_count);
    let mut next = current.clone();
    next.reserved_descendants = next.reserved_descendants.checked_add(1).ok_or_else(|| {
        InternalError::resource_exhausted("reserved Component descendant count overflow")
    })?;

    for _ in 0..8 {
        let next_partition_bytes = RootComponentRegistryStore::partition_entry_bytes(&next);
        let next_total = next_partition_bytes
            .checked_add(allocation_bytes)
            .and_then(|value| value.checked_add(next_count_bytes))
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let current_total = current_partition_bytes
            .checked_add(current_count_bytes)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let delta = next_total.checked_sub(current_total).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Child reservation unexpectedly reduced Registry bytes",
            )
        })?;
        let encoded_bytes = current.encoded_bytes.checked_add(delta).ok_or_else(|| {
            InternalError::resource_exhausted("Component Registry bytes overflow")
        })?;
        if next.encoded_bytes == encoded_bytes {
            return Ok((next, delta));
        }
        next.encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component Child reservation byte accounting did not converge",
    ))
}

fn subtree_fence_partition(
    current: &ComponentRegistryPartitionRecord,
    removal: &RootComponentSubtreeRemovalRecord,
) -> Result<(ComponentRegistryPartitionRecord, u64), InternalError> {
    let current_partition_bytes = RootComponentRegistryStore::partition_entry_bytes(current);
    let removal_bytes = RootComponentRegistryStore::subtree_removal_entry_bytes(removal);
    let mut next = current.clone();

    for _ in 0..8 {
        let next_total = RootComponentRegistryStore::partition_entry_bytes(&next)
            .checked_add(removal_bytes)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let registry_delta = next_total
            .checked_sub(current_partition_bytes)
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "Component subtree-removal fence unexpectedly reduced Registry bytes",
                )
            })?;
        let encoded_bytes = current
            .encoded_bytes
            .checked_add(registry_delta)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        if next.encoded_bytes == encoded_bytes {
            return Ok((next, registry_delta));
        }
        next.encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component subtree-removal fence byte accounting did not converge",
    ))
}

fn validate_child_creation_authority(
    current: &RootComponentRegistryMetaRecord,
    partition: &ComponentRegistryPartitionRecord,
    record: &RootComponentChildAllocationRecord,
    plan: &RootComponentCreationPlan,
) -> Result<(), InternalError> {
    validate_partition_record(partition)?;
    validate_child_allocation_record(record)?;
    if partition.binding.component != record.component
        || partition.release_set != record.release_set
        || partition.status != ComponentLifecycleStatus::Active
        || plan.controller != current.root.fleet_subnet_root
        || plan.wasm_store == Principal::anonymous()
        || plan.payload_hash == [0; 32]
        || plan.payload_size_bytes == 0
    {
        return Err(InternalError::conflict(
            "Component Child creation authority differs from its active reservation",
        ));
    }
    Ok(())
}

fn child_creation_capacity(
    current: &RootComponentRegistryMetaRecord,
    partition: &ComponentRegistryPartitionRecord,
    record: &RootComponentChildAllocationRecord,
    charged_entry_bytes: u64,
) -> Result<(ComponentRegistryPartitionRecord, u64), InternalError> {
    let current_partition_bytes = RootComponentRegistryStore::partition_entry_bytes(partition);
    let current_record_bytes = RootComponentRegistryStore::child_allocation_entry_bytes(record);
    if charged_entry_bytes < current_record_bytes {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "Component Child creation charge is smaller than its reservation record",
        ));
    }
    let current_total = current_partition_bytes
        .checked_add(current_record_bytes)
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))?;
    let mut next = partition.clone();

    for _ in 0..8 {
        let next_total = RootComponentRegistryStore::partition_entry_bytes(&next)
            .checked_add(charged_entry_bytes)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let registry_delta = next_total.checked_sub(current_total).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
                "Component Child creation precharge unexpectedly reduced Registry bytes",
            )
        })?;
        let encoded_bytes = partition
            .encoded_bytes
            .checked_add(registry_delta)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        if next.encoded_bytes == encoded_bytes {
            if encoded_bytes > record.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted(format!(
                    "Component Child creation requires {encoded_bytes} bytes, exceeding protected Component limit {}",
                    record.maximum_registry_bytes
                )));
            }
            let root_encoded_bytes = current
                .encoded_bytes
                .checked_add(registry_delta)
                .ok_or_else(|| {
                    InternalError::resource_exhausted("Component Registry bytes overflow")
                })?;
            if root_encoded_bytes > current.root.limits.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted(format!(
                    "Component Child creation requires {root_encoded_bytes} root Registry bytes, exceeding protected limit {}",
                    current.root.limits.maximum_registry_bytes
                )));
            }
            return Ok((next, registry_delta));
        }
        next.encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component Child creation byte accounting did not converge",
    ))
}

fn child_creation_charged_entry_bytes(
    record: &RootComponentChildAllocationRecord,
    plan: &RootComponentCreationPlan,
) -> u64 {
    let mut maximum = record.clone();
    maximum.progress = RootComponentChildAllocationProgressRecord::Created {
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
    RootComponentRegistryStore::child_allocation_entry_bytes(&maximum)
}

fn validate_charged_child_record_size(
    record: &RootComponentChildAllocationRecord,
    charged_entry_bytes: u64,
) -> Result<(), InternalError> {
    if RootComponentRegistryStore::child_allocation_entry_bytes(record) > charged_entry_bytes {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component Child allocation exceeded its precharged stable footprint",
        ));
    }
    Ok(())
}

fn validate_child_install_authority(
    current: &RootComponentRegistryMetaRecord,
    partition: &ComponentRegistryPartitionRecord,
    record: &RootComponentChildAllocationRecord,
    plan: &RootComponentChildInstallPlan,
) -> Result<(), InternalError> {
    validate_partition_record(partition)?;
    validate_child_allocation_record(record)?;
    let canister = match &record.progress {
        RootComponentChildAllocationProgressRecord::Created { canister, .. }
        | RootComponentChildAllocationProgressRecord::InstallIntent { canister, .. }
        | RootComponentChildAllocationProgressRecord::Installed { canister, .. }
        | RootComponentChildAllocationProgressRecord::Verified { canister, .. }
        | RootComponentChildAllocationProgressRecord::Committed { canister, .. } => *canister,
        RootComponentChildAllocationProgressRecord::Reserved
        | RootComponentChildAllocationProgressRecord::CreationIntent(_) => {
            return Err(InternalError::conflict(
                "Component Child allocation has no created Canister",
            ));
        }
    };
    if partition.binding != plan.binding.component
        || partition.release_set != record.release_set
        || current.release_set != record.release_set
        || partition.status != ComponentLifecycleStatus::Active
        || plan.binding.parent_canister_id != record.parent_canister_id
        || plan.binding.role != record.child_role
        || plan.binding.canister_id != canister
        || plan.maximum_registry_bytes != record.maximum_registry_bytes
        || plan.raw_module_hash == [0; 32]
        || plan.chunk_hashes.is_empty()
    {
        return Err(InternalError::conflict(
            "Component Child install authority differs from its active reservation",
        ));
    }
    Ok(())
}

fn child_install_charged_entry_bytes(
    record: &RootComponentChildAllocationRecord,
    plan: &RootComponentChildInstallPlan,
) -> Result<u64, InternalError> {
    let (creation, canister) = match &record.progress {
        RootComponentChildAllocationProgressRecord::Created { effect, canister } => {
            (effect.clone(), *canister)
        }
        _ => {
            return Err(InternalError::conflict(
                "Component Child allocation is not ready for installation",
            ));
        }
    };
    let installation = RootComponentChildInstallEffectRecord {
        raw_module_hash: plan.raw_module_hash,
        chunk_hashes: plan.chunk_hashes.clone(),
        binding: plan.binding.clone(),
        cost_guard_settlement: ReplayCostGuardSettlement {
            quota_intent_id: IntentId(u64::MAX),
            reservation_intent_id: IntentId(u64::MAX),
        },
        charged_entry_bytes: u64::MAX,
    };
    let mut maximum = record.clone();
    maximum.progress = RootComponentChildAllocationProgressRecord::Committed {
        creation,
        canister,
        installation,
        commitment: RootComponentChildCommitmentRecord {
            registry: ComponentRegistryHead {
                component: record.component,
                revision: u64::MAX,
                content_hash: [u8::MAX; 32],
            },
            descendant_content_hash: [u8::MAX; 32],
            registry_encoded_bytes: u64::MAX,
            reserved_descendants: u32::MAX,
            committed_descendants: u32::MAX,
            directory_synchronized_at_ns: u64::MAX,
            directory_authority_hash: [u8::MAX; 32],
            directory_prepared: true,
            runtime_activated: true,
            membership: Some(RootComponentChildMembershipRecord {
                registry: ComponentRegistryHead {
                    component: record.component,
                    revision: u64::MAX,
                    content_hash: [u8::MAX; 32],
                },
                descendant_content_hash: [u8::MAX; 32],
                registry_encoded_bytes: u64::MAX,
                reserved_descendants: u32::MAX,
                committed_descendants: u32::MAX,
                directory_synchronized_at_ns: u64::MAX,
                directory_authority_hash: [u8::MAX; 32],
                directory_synchronized: true,
            }),
        },
    };
    let child = ComponentRegistryChildRecord {
        component: record.component,
        canister_id: canister,
        parent_canister_id: record.parent_canister_id,
        role: record.child_role.clone(),
        kind: record.child_kind,
        installed_artifact_hash: plan.raw_module_hash,
        status: ComponentLifecycleStatus::Active,
    };
    let traversal = ComponentRegistryChildTraversalRecord {
        component: record.component,
        parent_canister_id: record.parent_canister_id,
        role: record.child_role.clone(),
        canister_id: canister,
    };
    RootComponentRegistryStore::child_allocation_entry_bytes(&maximum)
        .checked_add(RootComponentRegistryStore::child_entry_bytes(&child))
        .and_then(|value| {
            value.checked_add(RootComponentRegistryStore::child_traversal_entry_bytes(
                &traversal,
            ))
        })
        .and_then(|value| {
            value.checked_add(RootComponentRegistryStore::principal_index_entry_bytes(
                canister,
                record.component,
            ))
        })
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))
}

fn child_install_capacity(
    current: &RootComponentRegistryMetaRecord,
    partition: &ComponentRegistryPartitionRecord,
    record: &RootComponentChildAllocationRecord,
    charged_entry_bytes: u64,
) -> Result<(ComponentRegistryPartitionRecord, u64), InternalError> {
    let current_reserved_bytes = match &record.progress {
        RootComponentChildAllocationProgressRecord::Created { effect, .. } => {
            effect.charged_entry_bytes
        }
        _ => {
            return Err(InternalError::conflict(
                "Component Child allocation is not ready to reserve install capacity",
            ));
        }
    };
    let current_total = RootComponentRegistryStore::partition_entry_bytes(partition)
        .checked_add(current_reserved_bytes)
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))?;
    let mut next = partition.clone();

    for _ in 0..8 {
        let next_total = RootComponentRegistryStore::partition_entry_bytes(&next)
            .checked_add(charged_entry_bytes)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let registry_delta = next_total.checked_sub(current_total).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
                "Component Child install precharge unexpectedly reduced Registry bytes",
            )
        })?;
        let encoded_bytes = partition
            .encoded_bytes
            .checked_add(registry_delta)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        if next.encoded_bytes == encoded_bytes {
            if encoded_bytes > record.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted(format!(
                    "Component Child installation requires {encoded_bytes} bytes, exceeding protected Component limit {}",
                    record.maximum_registry_bytes
                )));
            }
            let root_encoded_bytes = current
                .encoded_bytes
                .checked_add(registry_delta)
                .ok_or_else(|| {
                    InternalError::resource_exhausted("Component Registry bytes overflow")
                })?;
            if root_encoded_bytes > current.root.limits.maximum_registry_bytes {
                return Err(InternalError::resource_exhausted(format!(
                    "Component Child installation requires {root_encoded_bytes} root Registry bytes, exceeding protected limit {}",
                    current.root.limits.maximum_registry_bytes
                )));
            }
            return Ok((next, registry_delta));
        }
        next.encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component Child installation byte accounting did not converge",
    ))
}

fn validate_child_install_effect_record(
    effect: &RootComponentChildInstallEffectRecord,
    plan: &RootComponentChildInstallPlan,
) -> Result<(), InternalError> {
    if effect.raw_module_hash != plan.raw_module_hash
        || effect.chunk_hashes != plan.chunk_hashes
        || effect.binding != plan.binding
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "durable Component Child install intent differs from verified module or binding authority",
        ));
    }
    Ok(())
}

fn advance_child_install_phase(
    component: ComponentInstanceId,
    operation_id: [u8; 32],
    verified: bool,
) -> Result<RootComponentChildAllocationView, InternalError> {
    let current = RootComponentRegistryStore::current().ok_or_else(|| {
        InternalError::unavailable("root Component Registry authority has not been prepared")
    })?;
    let partition = RootComponentRegistryStore::partition(component).ok_or_else(|| {
        InternalError::unavailable("Component Registry partition has not been committed")
    })?;
    let record =
        RootComponentRegistryStore::child_allocation(component, operation_id).ok_or_else(|| {
            InternalError::unavailable("Component Child allocation operation has not been reserved")
        })?;
    let next_progress = match (&record.progress, verified) {
        (
            RootComponentChildAllocationProgressRecord::InstallIntent {
                creation,
                canister,
                installation,
            },
            false,
        ) => RootComponentChildAllocationProgressRecord::Installed {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
        },
        (RootComponentChildAllocationProgressRecord::Installed { .. }, false)
        | (
            RootComponentChildAllocationProgressRecord::Verified { .. }
            | RootComponentChildAllocationProgressRecord::Committed { .. },
            _,
        ) => return Ok(child_allocation_record_to_view(record)),
        (
            RootComponentChildAllocationProgressRecord::Installed {
                creation,
                canister,
                installation,
            },
            true,
        ) => RootComponentChildAllocationProgressRecord::Verified {
            creation: creation.clone(),
            canister: *canister,
            installation: installation.clone(),
        },
        _ => {
            return Err(InternalError::conflict(if verified {
                "Component Child allocation has not recorded successful installation"
            } else {
                "Component Child allocation has no durable install intent"
            }));
        }
    };
    let charged_entry_bytes = match &next_progress {
        RootComponentChildAllocationProgressRecord::Installed { installation, .. }
        | RootComponentChildAllocationProgressRecord::Verified { installation, .. } => {
            installation.charged_entry_bytes
        }
        _ => unreachable!(),
    };
    let mut next_record = record.clone();
    next_record.progress = next_progress;
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
            empty_component_descendant_content_hash(record.component),
            0,
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
        descendant_content_hash: [u8::MAX; 32],
        directory_synchronized_at_ns: u64::MAX,
        reserved_descendants: u32::MAX,
        committed_descendants: u32::MAX,
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

#[expect(
    clippy::too_many_lines,
    reason = "one constructor converges the complete child receipt and Registry byte ledger"
)]
fn committed_child_records(
    record: &RootComponentChildAllocationRecord,
    creation: &RootComponentCreationEffectRecord,
    canister: Principal,
    installation: &RootComponentChildInstallEffectRecord,
    partition: &ComponentRegistryPartitionRecord,
    directory_synchronized_at_ns: u64,
    fleet_directory: &FleetDirectorySnapshot,
) -> Result<
    (
        RootComponentChildAllocationRecord,
        ComponentRegistryPartitionRecord,
        ComponentRegistryChildRecord,
        ComponentRegistryChildTraversalRecord,
    ),
    InternalError,
> {
    if RootComponentRegistryStore::child(record.component, canister).is_some() {
        return Err(InternalError::conflict(
            "Component Child principal is already committed",
        ));
    }
    let child = ComponentRegistryChildRecord {
        component: record.component,
        canister_id: canister,
        parent_canister_id: record.parent_canister_id,
        role: record.child_role.clone(),
        kind: record.child_kind,
        installed_artifact_hash: installation.raw_module_hash,
        status: ComponentLifecycleStatus::Prepared,
    };
    validate_child_record(partition, &child)?;

    let revision = partition
        .revision
        .checked_add(1)
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry revision overflow"))?;
    let reserved_descendants = partition
        .reserved_descendants
        .checked_sub(1)
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "Component Registry has no reserved descendant to commit",
            )
        })?;
    let committed_descendants =
        partition
            .committed_descendants
            .checked_add(1)
            .ok_or_else(|| {
                InternalError::resource_exhausted("committed Component descendant count overflow")
            })?;
    let descendant_content_hash = committed_component_descendant_content_hash(
        partition.descendant_content_hash,
        partition.committed_descendants,
        revision,
        &child,
    )?;
    let content_hash = component_partition_content_hash(
        &partition.binding,
        &partition.provisioning_origin,
        partition.release_set,
        partition.status,
        revision,
        descendant_content_hash,
        committed_descendants,
    )?;
    let registry = ComponentRegistryHead {
        component: record.component,
        revision,
        content_hash,
    };
    let directory_authority_hash = component_directory_authority_hash(
        &partition.binding,
        revision,
        content_hash,
        directory_synchronized_at_ns,
        committed_descendants,
        fleet_directory,
    )?;
    let traversal = ComponentRegistryChildTraversalRecord {
        component: record.component,
        parent_canister_id: record.parent_canister_id,
        role: record.child_role.clone(),
        canister_id: canister,
    };
    let mut next_record = record.clone();
    next_record.progress = RootComponentChildAllocationProgressRecord::Committed {
        creation: creation.clone(),
        canister,
        installation: installation.clone(),
        commitment: RootComponentChildCommitmentRecord {
            registry,
            descendant_content_hash,
            registry_encoded_bytes: 0,
            reserved_descendants,
            committed_descendants,
            directory_synchronized_at_ns,
            directory_authority_hash,
            directory_prepared: false,
            runtime_activated: false,
            membership: None,
        },
    };
    let mut next_partition = ComponentRegistryPartitionRecord {
        binding: partition.binding.clone(),
        provisioning_origin: partition.provisioning_origin.clone(),
        release_set: partition.release_set,
        status: partition.status,
        revision,
        content_hash,
        descendant_content_hash,
        directory_synchronized_at_ns,
        reserved_descendants,
        committed_descendants,
        encoded_bytes: partition.encoded_bytes,
    };
    let current_total = RootComponentRegistryStore::partition_entry_bytes(partition)
        .checked_add(installation.charged_entry_bytes)
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))?;
    let child_bytes = RootComponentRegistryStore::child_entry_bytes(&child);
    let traversal_bytes = RootComponentRegistryStore::child_traversal_entry_bytes(&traversal);
    let index_bytes =
        RootComponentRegistryStore::principal_index_entry_bytes(canister, record.component);

    for _ in 0..8 {
        let terminal_bytes = RootComponentRegistryStore::child_allocation_entry_bytes(&next_record)
            .checked_add(child_bytes)
            .and_then(|value| value.checked_add(traversal_bytes))
            .and_then(|value| value.checked_add(index_bytes))
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let next_total = RootComponentRegistryStore::partition_entry_bytes(&next_partition)
            .checked_add(terminal_bytes)
            .ok_or_else(|| {
                InternalError::resource_exhausted("Component Registry bytes overflow")
            })?;
        let released_precharge = current_total.checked_sub(next_total).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "exact Component Child commitment exceeds its maximum terminal precharge",
            )
        })?;
        let encoded_bytes = partition
            .encoded_bytes
            .checked_sub(released_precharge)
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "Component Registry cannot release excess child precharge",
                )
            })?;
        let RootComponentChildAllocationProgressRecord::Committed { commitment, .. } =
            &mut next_record.progress
        else {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
                "new Component Child commitment changed phase during byte accounting",
            ));
        };
        if next_partition.encoded_bytes == encoded_bytes
            && commitment.registry_encoded_bytes == encoded_bytes
        {
            return Ok((next_record, next_partition, child, traversal));
        }
        next_partition.encoded_bytes = encoded_bytes;
        commitment.registry_encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component Child commitment byte accounting did not converge",
    ))
}

fn persist_child_membership_activation(
    current: &RootComponentRegistryMetaRecord,
    partition: &ComponentRegistryPartitionRecord,
    record: &RootComponentChildAllocationRecord,
    child: &ComponentRegistryChildRecord,
    directory_synchronized_at_ns: u64,
    fleet_directory: &FleetDirectorySnapshot,
) -> Result<
    (
        RootComponentChildAllocationView,
        ComponentRegistryPartitionView,
    ),
    InternalError,
> {
    let RootComponentChildAllocationProgressRecord::Committed {
        installation,
        commitment,
        ..
    } = &record.progress
    else {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "membership activation persistence requires a committed Component Child allocation",
        ));
    };
    let (next_record, active_partition, active_child) = active_child_membership_records(
        record,
        commitment,
        partition,
        child,
        directory_synchronized_at_ns,
        fleet_directory,
    )?;
    let traversal = ComponentRegistryChildTraversalRecord {
        component: record.component,
        parent_canister_id: record.parent_canister_id,
        role: record.child_role.clone(),
        canister_id: child.canister_id,
    };
    let terminal_bytes = RootComponentRegistryStore::child_allocation_entry_bytes(&next_record)
        .checked_add(RootComponentRegistryStore::child_entry_bytes(&active_child))
        .and_then(|value| {
            value.checked_add(RootComponentRegistryStore::child_traversal_entry_bytes(
                &traversal,
            ))
        })
        .and_then(|value| {
            value.checked_add(RootComponentRegistryStore::principal_index_entry_bytes(
                child.canister_id,
                record.component,
            ))
        })
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))?;
    if terminal_bytes > installation.charged_entry_bytes {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component Child membership exceeds its pre-install Registry byte reservation",
        ));
    }
    if active_partition.encoded_bytes > record.maximum_registry_bytes {
        return Err(InternalError::resource_exhausted(format!(
            "active Component Registry requires {} bytes, exceeding protected Component limit {}",
            active_partition.encoded_bytes, record.maximum_registry_bytes
        )));
    }
    let encoded_bytes = current
        .encoded_bytes
        .checked_sub(partition.encoded_bytes)
        .and_then(|value| value.checked_add(active_partition.encoded_bytes))
        .ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "root Component Registry byte accounting cannot activate child membership",
            )
        })?;
    if encoded_bytes > current.root.limits.maximum_registry_bytes {
        return Err(InternalError::resource_exhausted(
            "active Component Child Registry exceeds the protected root byte limit",
        ));
    }
    let mut next_meta = current.clone();
    next_meta.encoded_bytes = encoded_bytes;
    RootComponentRegistryStore::activate_child_membership(
        current,
        next_meta,
        partition,
        active_partition.clone(),
        record,
        next_record.clone(),
        child,
        active_child,
    )
    .map_err(map_allocation_commit_error)?;
    Ok((
        child_allocation_record_to_view(next_record),
        partition_record_to_view(active_partition),
    ))
}

#[expect(
    clippy::too_many_lines,
    reason = "one constructor freezes the exact active child head and converges its byte ledger"
)]
fn active_child_membership_records(
    record: &RootComponentChildAllocationRecord,
    commitment: &RootComponentChildCommitmentRecord,
    partition: &ComponentRegistryPartitionRecord,
    child: &ComponentRegistryChildRecord,
    directory_synchronized_at_ns: u64,
    fleet_directory: &FleetDirectorySnapshot,
) -> Result<
    (
        RootComponentChildAllocationRecord,
        ComponentRegistryPartitionRecord,
        ComponentRegistryChildRecord,
    ),
    InternalError,
> {
    let RootComponentChildAllocationProgressRecord::Committed {
        creation,
        canister,
        installation,
        ..
    } = &record.progress
    else {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "membership activation requires a committed Component Child allocation",
        ));
    };
    let revision = partition
        .revision
        .checked_add(1)
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry revision overflow"))?;
    let mut active_child = child.clone();
    active_child.status = ComponentLifecycleStatus::Active;
    let descendant_content_hash = activated_component_descendant_content_hash(
        partition.descendant_content_hash,
        partition.revision,
        revision,
        &active_child,
    )?;
    let content_hash = component_partition_content_hash(
        &partition.binding,
        &partition.provisioning_origin,
        partition.release_set,
        partition.status,
        revision,
        descendant_content_hash,
        partition.committed_descendants,
    )?;
    let registry = ComponentRegistryHead {
        component: record.component,
        revision,
        content_hash,
    };
    let directory_authority_hash = component_directory_authority_hash(
        &partition.binding,
        revision,
        content_hash,
        directory_synchronized_at_ns,
        partition.committed_descendants,
        fleet_directory,
    )?;
    let mut next_record = record.clone();
    next_record.progress = RootComponentChildAllocationProgressRecord::Committed {
        creation: creation.clone(),
        canister: *canister,
        installation: installation.clone(),
        commitment: RootComponentChildCommitmentRecord {
            registry: commitment.registry.clone(),
            descendant_content_hash: commitment.descendant_content_hash,
            registry_encoded_bytes: commitment.registry_encoded_bytes,
            reserved_descendants: commitment.reserved_descendants,
            committed_descendants: commitment.committed_descendants,
            directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
            directory_authority_hash: commitment.directory_authority_hash,
            directory_prepared: commitment.directory_prepared,
            runtime_activated: commitment.runtime_activated,
            membership: Some(RootComponentChildMembershipRecord {
                registry,
                descendant_content_hash,
                registry_encoded_bytes: 0,
                reserved_descendants: partition.reserved_descendants,
                committed_descendants: partition.committed_descendants,
                directory_synchronized_at_ns,
                directory_authority_hash,
                directory_synchronized: false,
            }),
        },
    };
    let mut active_partition = ComponentRegistryPartitionRecord {
        binding: partition.binding.clone(),
        provisioning_origin: partition.provisioning_origin.clone(),
        release_set: partition.release_set,
        status: partition.status,
        revision,
        content_hash,
        descendant_content_hash,
        directory_synchronized_at_ns,
        reserved_descendants: partition.reserved_descendants,
        committed_descendants: partition.committed_descendants,
        encoded_bytes: partition.encoded_bytes,
    };
    let previous_variable_bytes = RootComponentRegistryStore::partition_entry_bytes(partition)
        .checked_add(RootComponentRegistryStore::child_allocation_entry_bytes(
            record,
        ))
        .and_then(|value| value.checked_add(RootComponentRegistryStore::child_entry_bytes(child)))
        .ok_or_else(|| InternalError::resource_exhausted("Component Registry bytes overflow"))?;

    for _ in 0..8 {
        let next_variable_bytes =
            RootComponentRegistryStore::partition_entry_bytes(&active_partition)
                .checked_add(RootComponentRegistryStore::child_allocation_entry_bytes(
                    &next_record,
                ))
                .and_then(|value| {
                    value.checked_add(RootComponentRegistryStore::child_entry_bytes(&active_child))
                })
                .ok_or_else(|| {
                    InternalError::resource_exhausted("Component Registry bytes overflow")
                })?;
        let encoded_bytes = partition
            .encoded_bytes
            .checked_sub(previous_variable_bytes)
            .and_then(|value| value.checked_add(next_variable_bytes))
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "Component Registry bytes cannot activate child membership",
                )
            })?;
        let RootComponentChildAllocationProgressRecord::Committed { commitment, .. } =
            &mut next_record.progress
        else {
            return Err(InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
                "active Component Child commitment changed phase during byte accounting",
            ));
        };
        let membership = commitment.membership.as_mut().ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
                "active Component Child commitment lost membership during byte accounting",
            )
        })?;
        if active_partition.encoded_bytes == encoded_bytes
            && membership.registry_encoded_bytes == encoded_bytes
        {
            return Ok((next_record, active_partition, active_child));
        }
        active_partition.encoded_bytes = encoded_bytes;
        membership.registry_encoded_bytes = encoded_bytes;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "active Component Child Registry byte accounting did not converge",
    ))
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
        empty_component_descendant_content_hash(record.component),
        0,
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
        descendant_content_hash: empty_component_descendant_content_hash(record.component),
        directory_synchronized_at_ns,
        reserved_descendants: 0,
        committed_descendants: 0,
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

#[expect(
    clippy::too_many_lines,
    reason = "one constructor freezes the complete top-level membership authority"
)]
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
        empty_component_descendant_content_hash(record.component),
        0,
    )?;
    let directory_authority_hash = component_directory_authority_hash(
        &installation.binding,
        revision,
        content_hash,
        directory_synchronized_at_ns,
        0,
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
        descendant_content_hash: empty_component_descendant_content_hash(record.component),
        directory_synchronized_at_ns,
        reserved_descendants: 0,
        committed_descendants: 0,
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
    descendant_count: u32,
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
            descendant_count,
        },
    })
}

fn exact_committed_child_partition(
    record: &RootComponentChildAllocationRecord,
    commitment: &RootComponentChildCommitmentRecord,
) -> Result<ComponentRegistryPartitionRecord, InternalError> {
    let RootComponentChildAllocationProgressRecord::Committed {
        canister,
        installation,
        ..
    } = &record.progress
    else {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "Component Child partition validation requires a committed allocation",
        ));
    };
    let current = RootComponentRegistryStore::partition(record.component).ok_or_else(|| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "committed Component Child allocation has no Registry partition",
        )
    })?;
    validate_partition_record(&current)?;
    let child =
        RootComponentRegistryStore::child(record.component, *canister).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "committed Component Child allocation has no normalized row",
            )
        })?;
    validate_child_record(&current, &child)?;
    let traversal = ComponentRegistryChildTraversalRecord {
        component: record.component,
        parent_canister_id: record.parent_canister_id,
        role: record.child_role.clone(),
        canister_id: *canister,
    };
    let committed = ComponentRegistryPartitionRecord {
        binding: installation.binding.component.clone(),
        provisioning_origin: current.provisioning_origin.clone(),
        release_set: record.release_set,
        status: ComponentLifecycleStatus::Active,
        revision: commitment.registry.revision,
        content_hash: commitment.registry.content_hash,
        descendant_content_hash: commitment.descendant_content_hash,
        directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
        reserved_descendants: commitment.reserved_descendants,
        committed_descendants: commitment.committed_descendants,
        encoded_bytes: commitment.registry_encoded_bytes,
    };
    validate_partition_snapshot(&committed)?;
    if commitment.registry.component != record.component
        || child.component != record.component
        || child.canister_id != *canister
        || child.parent_canister_id != record.parent_canister_id
        || child.role != record.child_role
        || child.kind != record.child_kind
        || child.installed_artifact_hash != installation.raw_module_hash
        || !matches!(
            child.status,
            ComponentLifecycleStatus::Prepared | ComponentLifecycleStatus::Active
        )
        || RootComponentRegistryStore::child_traversal(
            traversal.component,
            traversal.parent_canister_id,
            &traversal.role,
            traversal.canister_id,
        )
        .as_ref()
            != Some(&traversal)
        || current.binding != committed.binding
        || current.release_set != committed.release_set
        || current.status != ComponentLifecycleStatus::Active
        || current.revision < committed.revision
        || current.directory_synchronized_at_ns < committed.directory_synchronized_at_ns
        || current.committed_descendants < committed.committed_descendants
        || current.encoded_bytes > record.maximum_registry_bytes
        || (current.revision == committed.revision
            && (current.content_hash != committed.content_hash
                || current.directory_synchronized_at_ns != committed.directory_synchronized_at_ns
                || current.committed_descendants != committed.committed_descendants))
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "committed Component Child differs from its immutable Registry receipt",
        ));
    }
    Ok(committed)
}

fn exact_active_child_partition(
    record: &RootComponentChildAllocationRecord,
    commitment: &RootComponentChildCommitmentRecord,
    membership: &RootComponentChildMembershipRecord,
) -> Result<ComponentRegistryPartitionRecord, InternalError> {
    let current = RootComponentRegistryStore::partition(record.component).ok_or_else(|| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "active Component Child allocation has no Registry partition",
        )
    })?;
    validate_active_child_partition(record, commitment, membership, &current)
}

fn validate_active_child_partition(
    record: &RootComponentChildAllocationRecord,
    commitment: &RootComponentChildCommitmentRecord,
    membership: &RootComponentChildMembershipRecord,
    current: &ComponentRegistryPartitionRecord,
) -> Result<ComponentRegistryPartitionRecord, InternalError> {
    let RootComponentChildAllocationProgressRecord::Committed {
        canister,
        installation,
        ..
    } = &record.progress
    else {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "active child validation requires a committed Component Child allocation",
        ));
    };
    let child =
        RootComponentRegistryStore::child(record.component, *canister).ok_or_else(|| {
            InternalError::invariant(
                canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                "active Component Child allocation has no normalized row",
            )
        })?;
    validate_child_record(current, &child)?;
    let historical = ComponentRegistryPartitionRecord {
        binding: current.binding.clone(),
        provisioning_origin: current.provisioning_origin.clone(),
        release_set: current.release_set,
        status: current.status,
        revision: membership.registry.revision,
        content_hash: membership.registry.content_hash,
        descendant_content_hash: membership.descendant_content_hash,
        directory_synchronized_at_ns: membership.directory_synchronized_at_ns,
        reserved_descendants: membership.reserved_descendants,
        committed_descendants: membership.committed_descendants,
        encoded_bytes: membership.registry_encoded_bytes,
    };
    validate_partition_snapshot(&historical)?;
    if !commitment.directory_prepared
        || !commitment.runtime_activated
        || membership.registry.component != record.component
        || membership.registry.revision <= commitment.registry.revision
        || membership.descendant_content_hash == commitment.descendant_content_hash
        || membership.directory_synchronized_at_ns <= commitment.directory_synchronized_at_ns
        || membership.directory_authority_hash == [0; 32]
        || child.canister_id != *canister
        || child.parent_canister_id != record.parent_canister_id
        || child.role != record.child_role
        || child.kind != record.child_kind
        || child.installed_artifact_hash != installation.raw_module_hash
        || child.status != ComponentLifecycleStatus::Active
        || current.binding != historical.binding
        || current.release_set != historical.release_set
        || current.status != ComponentLifecycleStatus::Active
        || current.revision < historical.revision
        || current.directory_synchronized_at_ns < historical.directory_synchronized_at_ns
        || (current.revision == historical.revision
            && (current.content_hash != historical.content_hash
                || current.descendant_content_hash != historical.descendant_content_hash
                || current.directory_synchronized_at_ns != historical.directory_synchronized_at_ns
                || current.reserved_descendants < historical.reserved_descendants
                || current.committed_descendants != historical.committed_descendants
                || current.encoded_bytes < historical.encoded_bytes))
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "active Component Child partition differs from its immutable membership receipt",
        ));
    }
    validate_partition_record(current)?;
    Ok(historical)
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
        descendant_content_hash: empty_component_descendant_content_hash(record.component),
        directory_synchronized_at_ns: commitment.directory_synchronized_at_ns,
        reserved_descendants: 0,
        committed_descendants: 0,
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
    validate_partition_snapshot(&prepared)?;
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
    let historical = ComponentRegistryPartitionRecord {
        binding: current.binding.clone(),
        provisioning_origin: current.provisioning_origin.clone(),
        release_set: current.release_set,
        status: ComponentLifecycleStatus::Active,
        revision: expected_revision,
        content_hash: component_partition_content_hash(
            &current.binding,
            &current.provisioning_origin,
            current.release_set,
            ComponentLifecycleStatus::Active,
            expected_revision,
            empty_component_descendant_content_hash(record.component),
            0,
        )?,
        descendant_content_hash: empty_component_descendant_content_hash(record.component),
        directory_synchronized_at_ns: membership.directory_synchronized_at_ns,
        reserved_descendants: 0,
        committed_descendants: 0,
        encoded_bytes: membership.registry_encoded_bytes,
    };
    validate_partition_snapshot(&historical)?;
    // Later child reservations and commitments may advance charged bytes and
    // the current head without changing this immutable top-level receipt.
    let registry_encoded_bytes_covered = membership.registry_encoded_bytes <= current.encoded_bytes;
    if !commitment.directory_prepared
        || !commitment.runtime_activated
        || !registry_encoded_bytes_covered
        || membership.directory_synchronized_at_ns <= commitment.directory_synchronized_at_ns
        || membership.directory_authority_hash == [0; 32]
        || current.binding.component != record.component
        || current.status != ComponentLifecycleStatus::Active
        || current.revision < expected_revision
        || current.directory_synchronized_at_ns < membership.directory_synchronized_at_ns
        || (current.revision == expected_revision
            && (current.content_hash != historical.content_hash
                || current.directory_synchronized_at_ns != historical.directory_synchronized_at_ns
                || current.committed_descendants != 0))
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "active Component partition differs from its immutable membership receipt",
        ));
    }
    validate_partition_record(current)?;
    Ok(historical)
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

fn validate_child_directory_authority_hash(
    partition: &ComponentRegistryPartitionRecord,
    fleet_directory: &FleetDirectorySnapshot,
    commitment: &RootComponentChildCommitmentRecord,
) -> Result<(), InternalError> {
    let authority = ComponentRuntimeDirectoryAuthority {
        fleet: fleet_directory.clone(),
        component: ComponentDirectoryHead {
            provenance: ComponentDirectoryProvenance {
                component: partition.binding.clone(),
                source_fleet_subnet_root: partition.binding.fleet_subnet_root,
                component_registry_revision: commitment.registry.revision,
                component_registry_content_hash: commitment.registry.content_hash,
                synchronized_at_ns: commitment.directory_synchronized_at_ns,
            },
            descendant_count: commitment.committed_descendants,
        },
    };
    if ComponentRuntimeOps::directory_authority_hash(&authority)?
        != commitment.directory_authority_hash
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "committed Component Child Directory authority differs from its Registry receipt",
        ));
    }
    Ok(())
}

fn validate_child_membership_directory_authority_hash(
    partition: &ComponentRegistryPartitionRecord,
    fleet_directory: &FleetDirectorySnapshot,
    membership: &RootComponentChildMembershipRecord,
) -> Result<(), InternalError> {
    let authority = ComponentRuntimeDirectoryAuthority {
        fleet: fleet_directory.clone(),
        component: ComponentDirectoryHead {
            provenance: ComponentDirectoryProvenance {
                component: partition.binding.clone(),
                source_fleet_subnet_root: partition.binding.fleet_subnet_root,
                component_registry_revision: membership.registry.revision,
                component_registry_content_hash: membership.registry.content_hash,
                synchronized_at_ns: membership.directory_synchronized_at_ns,
            },
            descendant_count: membership.committed_descendants,
        },
    };
    if ComponentRuntimeOps::directory_authority_hash(&authority)?
        != membership.directory_authority_hash
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "active Component Child Directory differs from its membership receipt",
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
    validate_partition_snapshot(partition)
}

fn validate_partition_snapshot(
    partition: &ComponentRegistryPartitionRecord,
) -> Result<(), InternalError> {
    let empty_descendant_hash =
        empty_component_descendant_content_hash(partition.binding.component);
    if partition.revision == 0
        || partition.directory_synchronized_at_ns == 0
        || partition.descendant_content_hash == [0; 32]
        || (partition.committed_descendants == 0
            && partition.descendant_content_hash != empty_descendant_hash)
        || (partition.committed_descendants > 0
            && partition.descendant_content_hash == empty_descendant_hash)
        || partition.content_hash
            != component_partition_content_hash(
                &partition.binding,
                &partition.provisioning_origin,
                partition.release_set,
                partition.status,
                partition.revision,
                partition.descendant_content_hash,
                partition.committed_descendants,
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

fn validate_child_record(
    partition: &ComponentRegistryPartitionRecord,
    child: &ComponentRegistryChildRecord,
) -> Result<(), InternalError> {
    if child.component != partition.binding.component
        || child.canister_id == Principal::anonymous()
        || child.parent_canister_id == Principal::anonymous()
        || child.canister_id == child.parent_canister_id
        || child.canister_id == partition.binding.canister_id
        || child.canister_id == partition.binding.fleet_subnet_root
        || child.canister_id == partition.binding.authority.binding.coordinator
        || child.parent_canister_id == partition.binding.fleet_subnet_root
        || child.parent_canister_id == partition.binding.authority.binding.coordinator
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component Registry child row has invalid tree identity",
        ));
    }
    Ok(())
}

fn validate_registered_child_record(
    partition: &ComponentRegistryPartitionRecord,
    child: &ComponentRegistryChildRecord,
) -> Result<(), InternalError> {
    validate_child_record(partition, child)?;
    let traversal = ComponentRegistryChildTraversalRecord {
        component: child.component,
        parent_canister_id: child.parent_canister_id,
        role: child.role.clone(),
        canister_id: child.canister_id,
    };
    if RootComponentRegistryStore::component_for_principal(child.canister_id)
        != Some(child.component)
        || RootComponentRegistryStore::component_for_principal(child.parent_canister_id)
            != Some(child.component)
        || RootComponentRegistryStore::child_traversal(
            child.component,
            child.parent_canister_id,
            &child.role,
            child.canister_id,
        )
        .as_ref()
            != Some(&traversal)
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component Registry child differs from its principal or traversal index",
        ));
    }
    Ok(())
}

fn validate_subtree_removal_record(
    record: &RootComponentSubtreeRemovalRecord,
) -> Result<(), InternalError> {
    if record.operation_id == [0; 32]
        || record.component != record.target.component
        || record.component != record.reserved_against_registry.component
        || record.reserved_against_registry.revision == 0
        || record.target.canister_id == Principal::anonymous()
        || record.target.parent_canister_id == Principal::anonymous()
        || record.target.status != ComponentLifecycleStatus::Active
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component subtree-removal fence has invalid protected identity",
        ));
    }
    Ok(())
}

const fn child_allocation_is_terminal(record: &RootComponentChildAllocationRecord) -> bool {
    matches!(
        &record.progress,
        RootComponentChildAllocationProgressRecord::Committed {
            commitment:
                RootComponentChildCommitmentRecord {
                    membership: Some(membership),
                    ..
                },
            ..
        } if membership.directory_synchronized
    )
}

fn canister_is_in_subtree(
    partition: &ComponentRegistryPartitionRecord,
    candidate: Principal,
    target: Principal,
    traversal_limit: u32,
) -> Result<bool, InternalError> {
    let mut current = candidate;
    for _ in 0..traversal_limit {
        if current == target {
            return Ok(true);
        }
        if current == partition.binding.canister_id {
            return Ok(false);
        }
        let child = RootComponentRegistryStore::child(partition.binding.component, current)
            .ok_or_else(|| {
                InternalError::invariant(
                    canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
                    "Component subtree ancestry references an unregistered child",
                )
            })?;
        validate_registered_child_record(partition, &child)?;
        current = child.parent_canister_id;
    }
    Err(InternalError::invariant(
        canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
        "Component subtree ancestry exceeded its committed descendant bound",
    ))
}

fn validate_child_traversal_record(
    component: ComponentInstanceId,
    traversal: &ComponentRegistryChildTraversalRecord,
) -> Result<(), InternalError> {
    if traversal.component != component
        || traversal.parent_canister_id == Principal::anonymous()
        || traversal.canister_id == Principal::anonymous()
        || traversal.parent_canister_id == traversal.canister_id
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component Directory traversal has invalid tree identity",
        ));
    }
    Ok(())
}

fn traversal_record_to_cursor(
    traversal: &ComponentRegistryChildTraversalRecord,
) -> ComponentDirectoryCanonicalCursor {
    ComponentDirectoryCanonicalCursor {
        parent_canister_id: traversal.parent_canister_id,
        role: traversal.role.clone(),
        canister_id: traversal.canister_id,
    }
}

fn child_record_to_directory_view(
    partition: &ComponentRegistryPartitionRecord,
    child: ComponentRegistryChildRecord,
) -> ComponentDirectoryChildView {
    ComponentDirectoryChildView {
        binding: ComponentChildBinding {
            component: partition.binding.clone(),
            parent_canister_id: child.parent_canister_id,
            role: child.role,
            canister_id: child.canister_id,
        },
        kind: child.kind,
        installed_artifact_hash: child.installed_artifact_hash,
        status: child.status,
    }
}

fn validate_child_allocation_record(
    record: &RootComponentChildAllocationRecord,
) -> Result<(), InternalError> {
    if record.operation_id == [0; 32]
        || record.component != record.reserved_against_registry.component
        || record.reserved_against_registry.revision == 0
        || record.parent_canister_id == Principal::anonymous()
        || record.maximum_instances_per_parent == 0
        || record.maximum_descendants == 0
        || record.maximum_registry_bytes == 0
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Storage,
            "Component Child allocation record has invalid protected identity",
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
    descendant_content_hash: [u8; 32],
    committed_descendants: u32,
) -> Result<[u8; 32], InternalError> {
    const DOMAIN: &[u8] = b"canic.component-registry.partition.v1";
    let payload = candid::encode_one((
        binding.clone(),
        provisioning_origin.clone(),
        release_set,
        status,
        revision,
        descendant_content_hash,
        committed_descendants,
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

fn empty_component_descendant_content_hash(component: ComponentInstanceId) -> [u8; 32] {
    const DOMAIN: &[u8] = b"canic.component-registry.descendants.v1";
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hasher.update(component.as_bytes());
    hasher.finalize().into()
}

fn committed_component_descendant_content_hash(
    previous: [u8; 32],
    previous_committed_descendants: u32,
    revision: u64,
    child: &ComponentRegistryChildRecord,
) -> Result<[u8; 32], InternalError> {
    const DOMAIN: &[u8] = b"canic.component-registry.descendant-commit.v1";
    if previous == [0; 32] || child.status != ComponentLifecycleStatus::Prepared {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "Component descendant digest input is invalid",
        ));
    }
    let payload = candid::encode_one((
        previous,
        previous_committed_descendants,
        revision,
        child.canister_id,
        child.parent_canister_id,
        child.role.clone(),
        child.kind,
        child.installed_artifact_hash,
        child.status,
    ))
    .map_err(|error| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            format!("Component descendant digest input cannot be encoded: {error}"),
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    Ok(hasher.finalize().into())
}

fn activated_component_descendant_content_hash(
    previous: [u8; 32],
    previous_revision: u64,
    revision: u64,
    child: &ComponentRegistryChildRecord,
) -> Result<[u8; 32], InternalError> {
    const DOMAIN: &[u8] = b"canic.component-registry.descendant-activate.v1";
    if previous == [0; 32]
        || previous_revision == 0
        || revision <= previous_revision
        || child.status != ComponentLifecycleStatus::Active
    {
        return Err(InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            "Component descendant activation digest input is invalid",
        ));
    }
    let payload = candid::encode_one((
        previous,
        previous_revision,
        revision,
        child.canister_id,
        child.parent_canister_id,
        child.role.clone(),
        child.kind,
        child.installed_artifact_hash,
        ComponentLifecycleStatus::Prepared,
        child.status,
    ))
    .map_err(|error| {
        InternalError::invariant(
            canic_core::control_plane_support::error::InternalErrorOrigin::Ops,
            format!("Component descendant activation digest input cannot be encoded: {error}"),
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
        RootComponentAllocationCommitError::ConflictingChildEntry => InternalError::conflict(
            "Component Child reservation differs from its Registry partition or count index",
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
        RootComponentAllocationCommitError::ParentPrincipalConflict => InternalError::forbidden(
            "Component Child reservation parent is not indexed by its Component Registry",
        ),
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
        control_plane_support::{
            config::schema::ComponentChildKind,
            policy::{
                component_allocation::TopLevelComponentAllocationDecision,
                component_child_allocation::ComponentChildAllocationDecision,
            },
        },
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

    fn restart_component_registry() -> RootComponentRegistryData {
        let snapshot = RootComponentRegistryStore::export();
        RootComponentRegistryStore::import(snapshot.clone());
        assert_eq!(RootComponentRegistryStore::export(), snapshot);
        snapshot
    }

    fn exact_registry_entry_bytes(data: &RootComponentRegistryData) -> u64 {
        data.allocations
            .iter()
            .map(RootComponentRegistryStore::allocation_entry_bytes)
            .chain(
                data.partitions
                    .iter()
                    .map(RootComponentRegistryStore::partition_entry_bytes),
            )
            .chain(data.partitions.iter().map(|partition| {
                RootComponentRegistryStore::principal_index_entry_bytes(
                    partition.binding.canister_id,
                    partition.binding.component,
                )
            }))
            .chain(
                data.children
                    .iter()
                    .map(RootComponentRegistryStore::child_entry_bytes),
            )
            .chain(data.children.iter().map(|child| {
                RootComponentRegistryStore::principal_index_entry_bytes(
                    child.canister_id,
                    child.component,
                )
            }))
            .chain(
                data.child_traversals
                    .iter()
                    .map(RootComponentRegistryStore::child_traversal_entry_bytes),
            )
            .chain(
                data.child_allocations
                    .iter()
                    .map(RootComponentRegistryStore::child_allocation_entry_bytes),
            )
            .chain(
                data.subtree_removals
                    .iter()
                    .map(RootComponentRegistryStore::subtree_removal_entry_bytes),
            )
            .chain(
                data.parent_role_counts
                    .iter()
                    .map(RootComponentRegistryStore::parent_role_count_entry_bytes),
            )
            .sum()
    }

    fn exact_component_registry_entry_bytes(
        data: &RootComponentRegistryData,
        component: ComponentInstanceId,
    ) -> u64 {
        data.partitions
            .iter()
            .filter(|partition| partition.binding.component == component)
            .map(RootComponentRegistryStore::partition_entry_bytes)
            .chain(
                data.partitions
                    .iter()
                    .filter(|partition| partition.binding.component == component)
                    .map(|partition| {
                        RootComponentRegistryStore::principal_index_entry_bytes(
                            partition.binding.canister_id,
                            component,
                        )
                    }),
            )
            .chain(
                data.children
                    .iter()
                    .filter(|child| child.component == component)
                    .map(RootComponentRegistryStore::child_entry_bytes),
            )
            .chain(
                data.children
                    .iter()
                    .filter(|child| child.component == component)
                    .map(|child| {
                        RootComponentRegistryStore::principal_index_entry_bytes(
                            child.canister_id,
                            component,
                        )
                    }),
            )
            .chain(
                data.child_traversals
                    .iter()
                    .filter(|traversal| traversal.component == component)
                    .map(RootComponentRegistryStore::child_traversal_entry_bytes),
            )
            .chain(
                data.child_allocations
                    .iter()
                    .filter(|allocation| allocation.component == component)
                    .map(charged_child_allocation_entry_bytes),
            )
            .chain(
                data.subtree_removals
                    .iter()
                    .filter(|removal| removal.component == component)
                    .map(RootComponentRegistryStore::subtree_removal_entry_bytes),
            )
            .chain(
                data.parent_role_counts
                    .iter()
                    .filter(|count| count.component == component)
                    .map(RootComponentRegistryStore::parent_role_count_entry_bytes),
            )
            .sum()
    }

    fn charged_child_allocation_entry_bytes(record: &RootComponentChildAllocationRecord) -> u64 {
        match &record.progress {
            RootComponentChildAllocationProgressRecord::Reserved => {
                RootComponentRegistryStore::child_allocation_entry_bytes(record)
            }
            RootComponentChildAllocationProgressRecord::CreationIntent(creation)
            | RootComponentChildAllocationProgressRecord::Created {
                effect: creation, ..
            } => creation.charged_entry_bytes,
            RootComponentChildAllocationProgressRecord::InstallIntent { installation, .. }
            | RootComponentChildAllocationProgressRecord::Installed { installation, .. }
            | RootComponentChildAllocationProgressRecord::Verified { installation, .. }
            | RootComponentChildAllocationProgressRecord::Committed { installation, .. } => {
                installation.charged_entry_bytes
            }
        }
    }

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
        assert_eq!(prepared.known_created_component_canisters, 0);
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
        let empty_inventory = ComponentRegistryOps::seal_initial_inventory([10; 32], 11)
            .expect("seal empty initial inventory");
        assert_eq!(empty_inventory.receipt.component_count, 0);
        assert_ne!(empty_inventory.receipt.inventory_hash, [0; 32]);
        assert!(empty_inventory.operation_ids.is_empty());
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

        let reserved = ComponentRegistryOps::reserve_allocation(
            decision.clone(),
            [12; 32],
            origin.clone(),
            false,
        )
        .expect("reserve");
        let interrupted_snapshot = RootComponentRegistryStore::export();
        RootComponentRegistryStore::import(interrupted_snapshot);
        let repeated = ComponentRegistryOps::reserve_allocation(decision, [12; 32], origin, false)
            .expect("exact retry");

        assert_eq!(reserved, repeated);
        assert_eq!(reserved.allocation_sequence, 1);
        let status = ComponentRegistryOps::current().expect("Registry status");
        assert_eq!(status.next_allocation_sequence, 2);
        assert_eq!(status.reserved_component_instances, 1);
        assert_eq!(status.committed_component_instances, 0);
        assert_eq!(status.known_created_component_canisters, 0);
        assert!(status.encoded_bytes > 0);
        assert_eq!(
            ComponentRegistryOps::component_spec_counts(&reserved.component_spec)
                .expect("Spec counts"),
            ComponentSpecInstanceCounts {
                reserved: 1,
                committed: 0,
            }
        );
        assert!(
            ComponentRegistryOps::seal_initial_inventory([20; 32], 21).is_err(),
            "a nonterminal allocation must prevent initial inventory sealing"
        );
        assert!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .initial_inventory
                .is_none()
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
                false,
            )
            .is_err()
        );
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one test proves the durable fence across a multi-level Component tree"
    )]
    fn subtree_removal_fence_is_durable_scoped_and_capacity_bounded() {
        let fixture = import_active_component_tree();
        let initial = RootComponentRegistryStore::export();
        let registry = component_registry_head(&fixture.partition);

        ComponentRegistryOps::reserve_child_allocation(
            child_allocation_decision_for_parent(
                &fixture.partition,
                fixture.descendant.canister_id,
                &fixture.descendant.role,
                "project_machine",
            ),
            [69; 32],
            registry.clone(),
        )
        .expect("reserve in-flight descendant");
        let before_inflight_rejection = RootComponentRegistryStore::export();
        ComponentRegistryOps::begin_subtree_removal(
            fixture.component,
            [70; 32],
            fixture.target.canister_id,
            registry.clone(),
            16_777_216,
        )
        .expect_err("in-flight descendant lifecycle must prevent fencing");
        assert_eq!(
            RootComponentRegistryStore::export(),
            before_inflight_rejection
        );
        RootComponentRegistryStore::import(initial.clone());

        ComponentRegistryOps::begin_subtree_removal(
            fixture.component,
            [70; 32],
            fixture.target.canister_id,
            registry.clone(),
            fixture.partition.encoded_bytes,
        )
        .expect_err("subtree fence must fit before mutation");
        assert_eq!(RootComponentRegistryStore::export(), initial);

        let fenced = ComponentRegistryOps::begin_subtree_removal(
            fixture.component,
            [70; 32],
            fixture.target.canister_id,
            registry.clone(),
            16_777_216,
        )
        .expect("durably fence target subtree");
        assert_eq!(fenced.target_canister_id, fixture.target.canister_id);
        assert_eq!(
            fenced.target_parent_canister_id,
            fixture.target.parent_canister_id
        );
        assert_eq!(
            fenced.progress,
            RootComponentSubtreeRemovalProgressView::Fenced
        );
        let durable_fence = restart_component_registry();
        assert_eq!(
            ComponentRegistryOps::subtree_removal(fixture.component, [70; 32])
                .expect("valid subtree removal")
                .expect("durable subtree removal"),
            fenced
        );
        assert_eq!(
            ComponentRegistryOps::begin_subtree_removal(
                fixture.component,
                [70; 32],
                fixture.target.canister_id,
                registry.clone(),
                16_777_216,
            )
            .expect("exact fence retry"),
            fenced
        );
        let current = ComponentRegistryOps::current().expect("Registry status");
        let partition = ComponentRegistryOps::partition(fixture.component)
            .expect("partition read")
            .expect("active partition");
        assert_eq!(
            partition.encoded_bytes,
            exact_component_registry_entry_bytes(&durable_fence, fixture.component)
        );
        assert_eq!(
            current.encoded_bytes,
            exact_registry_entry_bytes(&durable_fence)
        );

        for (operation_id, parent) in [([71; 32], &fixture.target), ([72; 32], &fixture.descendant)]
        {
            let before = RootComponentRegistryStore::export();
            ComponentRegistryOps::reserve_child_allocation(
                child_allocation_decision_for_parent(
                    &fixture.partition,
                    parent.canister_id,
                    &parent.role,
                    "project_machine",
                ),
                operation_id,
                registry.clone(),
            )
            .expect_err("fenced subtree member cannot reserve a new child");
            assert_eq!(RootComponentRegistryStore::export(), before);
        }

        let before_second_fence = RootComponentRegistryStore::export();
        ComponentRegistryOps::begin_subtree_removal(
            fixture.component,
            [73; 32],
            fixture.unrelated.canister_id,
            registry.clone(),
            16_777_216,
        )
        .expect_err("one Component admits only one in-progress subtree removal");
        assert_eq!(RootComponentRegistryStore::export(), before_second_fence);

        ComponentRegistryOps::reserve_child_allocation(
            child_allocation_decision_for_parent(
                &fixture.partition,
                fixture.unrelated.canister_id,
                &fixture.unrelated.role,
                "project_machine",
            ),
            [74; 32],
            registry.clone(),
        )
        .expect("unrelated branch remains mutable");
        assert_eq!(
            ComponentRegistryOps::begin_subtree_removal(
                fixture.component,
                [70; 32],
                fixture.target.canister_id,
                registry,
                16_777_216,
            )
            .expect("fence retry survives unrelated progress"),
            fenced
        );
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one test proves independent partition progress and exact shared root capacity"
    )]
    fn incomplete_component_operation_does_not_block_an_unrelated_partition() {
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
        let root = root_binding();
        let release_set = FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [8; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
        };
        let component_a = ComponentInstanceId::from_generated_bytes([10; 32]);
        let component_b = ComponentInstanceId::from_generated_bytes([11; 32]);
        let parent_a = candid::Principal::from_slice(&[18; 29]);
        let parent_b = candid::Principal::from_slice(&[19; 29]);
        let partition_a = active_component_partition(&root, release_set, component_a, parent_a);
        let partition_b = active_component_partition(&root, release_set, component_b, parent_b);
        let initial_encoded_bytes = partition_a
            .encoded_bytes
            .checked_add(partition_b.encoded_bytes)
            .expect("initial Registry bytes");
        RootComponentRegistryStore::import(RootComponentRegistryData {
            current: Some(RootComponentRegistryMetaRecord {
                root: root.clone(),
                prepared_against_registry: FleetRegistryVersion {
                    authority: root.authority.clone(),
                    revision: 4,
                    content_hash: [5; 32],
                },
                release_set,
                store_bootstrap: RootStoreBootstrapRequest {
                    manifest_payload_size_bytes: 128,
                },
                next_allocation_sequence: 3,
                reserved_component_instances: 0,
                committed_component_instances: 2,
                managed_descendants: 0,
                known_created_component_canisters: 2,
                encoded_bytes: initial_encoded_bytes,
                initial_inventory: None,
            }),
            partitions: vec![partition_a.clone(), partition_b.clone()],
            ..RootComponentRegistryData::default()
        });

        let operation_a = [44; 32];
        let decision_a = child_allocation_decision(&partition_a, "project_instance");
        let registry_a = component_registry_head(&partition_a);
        ComponentRegistryOps::reserve_child_allocation(
            decision_a.clone(),
            operation_a,
            registry_a.clone(),
        )
        .expect("reserve Component A child");
        let incomplete_a = ComponentRegistryOps::begin_child_creation(
            component_a,
            operation_a,
            child_creation_plan(&root, 50),
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(51),
                reservation_intent_id: IntentId(52),
            },
        )
        .expect("record Component A creation intent");
        let partition_a_after_intent = ComponentRegistryOps::partition(component_a)
            .expect("Component A partition read")
            .expect("Component A partition");
        let before_failed_a = RootComponentRegistryStore::export();
        ComponentRegistryOps::mark_child_created(component_a, operation_a, parent_a)
            .expect_err("Component A cannot create over its registered parent");
        assert_eq!(RootComponentRegistryStore::export(), before_failed_a);

        let operation_b = [54; 32];
        let decision_b = child_allocation_decision(&partition_b, "project_instance");
        let registry_b = component_registry_head(&partition_b);
        ComponentRegistryOps::reserve_child_allocation(decision_b, operation_b, registry_b.clone())
            .expect("reserve unrelated Component B child");
        ComponentRegistryOps::begin_child_creation(
            component_b,
            operation_b,
            child_creation_plan(&root, 55),
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(56),
                reservation_intent_id: IntentId(57),
            },
        )
        .expect("record Component B creation intent");
        let child_b = candid::Principal::from_slice(&[58; 29]);
        let progressed_b =
            ComponentRegistryOps::mark_child_created(component_b, operation_b, child_b)
                .expect("Component B progresses independently");
        assert!(matches!(
            progressed_b.progress,
            RootComponentChildAllocationProgressView::Created { canister, .. }
                if canister == child_b
        ));

        let durable = restart_component_registry();
        let retried_a =
            ComponentRegistryOps::reserve_child_allocation(decision_a, operation_a, registry_a)
                .expect("retry preserves incomplete Component A intent");
        assert_eq!(retried_a, incomplete_a);
        assert_eq!(
            ComponentRegistryOps::partition(component_a)
                .expect("Component A partition read")
                .expect("Component A partition"),
            partition_a_after_intent
        );
        let current = ComponentRegistryOps::current().expect("Registry status");
        assert_eq!(current.managed_descendants, 2);
        assert_eq!(current.known_created_component_canisters, 3);
        assert!(
            exact_registry_entry_bytes(&durable) <= current.encoded_bytes,
            "persisted entries must fit inside their exact pre-effect charges"
        );
        assert_eq!(
            current.encoded_bytes,
            durable
                .partitions
                .iter()
                .map(|partition| partition.encoded_bytes)
                .sum::<u64>()
        );
        for component in [component_a, component_b] {
            let partition = ComponentRegistryOps::partition(component)
                .expect("partition read")
                .expect("active partition");
            assert_eq!(
                partition.encoded_bytes,
                exact_component_registry_entry_bytes(&durable, component)
            );
        }

        let mut capacity_bounded = durable;
        let status = capacity_bounded.current.as_mut().expect("Registry status");
        let managed_canisters = 1
            + status.reserved_component_instances
            + status.committed_component_instances
            + status.managed_descendants;
        assert_eq!(managed_canisters, 5);
        status.root.limits.maximum_managed_canisters = managed_canisters;
        RootComponentRegistryStore::import(capacity_bounded);
        let before_capacity_failure = RootComponentRegistryStore::export();
        let capacity_error = ComponentRegistryOps::reserve_child_allocation(
            child_allocation_decision(&partition_b, "project_machine"),
            [59; 32],
            registry_b,
        )
        .expect_err("Component A reservation remains charged to the shared root limit");
        assert!(capacity_error.is_public_resource_exhausted());
        assert_eq!(
            RootComponentRegistryStore::export(),
            before_capacity_failure
        );
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one test follows the complete direct-child reserve-through-commit lifecycle"
    )]
    fn child_reservation_is_parent_indexed_idempotent_and_capacity_bounded() {
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
        let root = root_binding();
        let root_canister = root.fleet_subnet_root;
        let component = ComponentInstanceId::from_generated_bytes([10; 32]);
        let parent = candid::Principal::from_slice(&[18; 29]);
        let release_set = FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [8; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
        };
        let binding = ComponentBinding {
            authority: root.authority.clone(),
            component,
            component_spec: "projects".parse().expect("Component Spec"),
            spec_hash: [6; 32],
            role: CanisterRole::new("project_hub"),
            placement_subnet: root.placement_subnet,
            fleet_subnet_root: root.fleet_subnet_root,
            canister_id: parent,
        };
        let mut partition = ComponentRegistryPartitionRecord {
            binding: binding.clone(),
            provisioning_origin: ComponentProvisioningOrigin::FleetAdministrator {
                caller: candid::Principal::from_slice(&[11; 29]),
            },
            release_set,
            status: ComponentLifecycleStatus::Active,
            revision: 2,
            content_hash: component_partition_content_hash(
                &binding,
                &ComponentProvisioningOrigin::FleetAdministrator {
                    caller: candid::Principal::from_slice(&[11; 29]),
                },
                release_set,
                ComponentLifecycleStatus::Active,
                2,
                empty_component_descendant_content_hash(component),
                0,
            )
            .expect("partition hash"),
            descendant_content_hash: empty_component_descendant_content_hash(component),
            directory_synchronized_at_ns: 33,
            reserved_descendants: 0,
            committed_descendants: 0,
            encoded_bytes: 0,
        };
        let component_principal_index_bytes =
            RootComponentRegistryStore::principal_index_entry_bytes(parent, component);
        for _ in 0..8 {
            let encoded_bytes = RootComponentRegistryStore::partition_entry_bytes(&partition)
                + component_principal_index_bytes;
            if partition.encoded_bytes == encoded_bytes {
                break;
            }
            partition.encoded_bytes = encoded_bytes;
        }
        assert_eq!(
            partition.encoded_bytes,
            RootComponentRegistryStore::partition_entry_bytes(&partition)
                + component_principal_index_bytes
        );
        let initial_encoded_bytes = partition.encoded_bytes;
        RootComponentRegistryStore::import(RootComponentRegistryData {
            current: Some(RootComponentRegistryMetaRecord {
                root: root.clone(),
                prepared_against_registry: FleetRegistryVersion {
                    authority: root.authority.clone(),
                    revision: 4,
                    content_hash: [5; 32],
                },
                release_set,
                store_bootstrap: RootStoreBootstrapRequest {
                    manifest_payload_size_bytes: 128,
                },
                next_allocation_sequence: 2,
                reserved_component_instances: 0,
                committed_component_instances: 1,
                managed_descendants: 0,
                known_created_component_canisters: 1,
                encoded_bytes: partition.encoded_bytes,
                initial_inventory: None,
            }),
            partitions: vec![partition.clone()],
            ..RootComponentRegistryData::default()
        });
        let decision = ComponentChildAllocationDecision {
            component,
            component_spec: binding.component_spec.clone(),
            spec_hash: binding.spec_hash,
            parent_canister_id: parent,
            parent_role: binding.role.clone(),
            child_role: CanisterRole::new("project_instance"),
            child_kind: ComponentChildKind::Instance,
            maximum_instances_per_parent: 10_000,
            maximum_descendants: 20_000,
            maximum_registry_bytes: 16_777_216,
        };
        let registry = ComponentRegistryHead {
            component,
            revision: partition.revision,
            content_hash: partition.content_hash,
        };

        let reserved = ComponentRegistryOps::reserve_child_allocation(
            decision.clone(),
            [44; 32],
            registry.clone(),
        )
        .expect("reserve child");
        let interrupted = RootComponentRegistryStore::export();
        RootComponentRegistryStore::import(interrupted);
        let repeated =
            ComponentRegistryOps::reserve_child_allocation(decision.clone(), [44; 32], registry)
                .expect("retry child reservation");

        assert_eq!(reserved, repeated);
        assert_eq!(
            ComponentRegistryOps::registered_parent(component, parent)
                .expect("registered parent")
                .expect("top-level parent")
                .0,
            ManagedCanisterBinding::Component(binding.clone())
        );
        assert_eq!(
            ComponentRegistryOps::parent_role_instances(component, parent, &decision.child_role,)
                .expect("parent-role count"),
            1
        );
        let partition = ComponentRegistryOps::partition(component)
            .expect("partition read")
            .expect("partition");
        assert_eq!(partition.reserved_descendants, 1);
        assert_eq!(partition.committed_descendants, 0);
        let current = ComponentRegistryOps::current().expect("Registry status");
        assert_eq!(current.managed_descendants, 1);
        assert_eq!(current.encoded_bytes, partition.encoded_bytes);
        assert!(partition.encoded_bytes > initial_encoded_bytes);

        let mut exhausted = decision.clone();
        exhausted.maximum_instances_per_parent = 1;
        let before = RootComponentRegistryStore::export();
        let error = ComponentRegistryOps::reserve_child_allocation(
            exhausted,
            [45; 32],
            repeated.reserved_against_registry.clone(),
        )
        .expect_err("per-parent capacity must reject reservation");
        assert!(error.is_public_resource_exhausted());
        assert_eq!(RootComponentRegistryStore::export(), before);

        let mut conflicting = decision.clone();
        conflicting.maximum_descendants -= 1;
        assert!(
            ComponentRegistryOps::reserve_child_allocation(
                conflicting,
                [44; 32],
                repeated.reserved_against_registry.clone(),
            )
            .is_err()
        );

        let plan = RootComponentCreationPlan {
            wasm_store: candid::Principal::from_slice(&[50; 29]),
            payload_hash: [51; 32],
            payload_size_bytes: 4_096,
            initial_cycles: Cycles::new(5_000_000_000_000),
            controller: root_canister,
        };
        let before_creation = RootComponentRegistryStore::export();
        let mut capacity_exhausted = before_creation.clone();
        let maximum_registry_bytes = capacity_exhausted
            .current
            .as_ref()
            .expect("Registry status")
            .encoded_bytes;
        capacity_exhausted
            .current
            .as_mut()
            .expect("Registry status")
            .root
            .limits
            .maximum_registry_bytes = maximum_registry_bytes;
        RootComponentRegistryStore::import(capacity_exhausted);
        let error =
            ComponentRegistryOps::validate_child_creation_capacity(component, [44; 32], &plan)
                .expect_err("creation must fit before the paid effect");
        assert!(error.is_public_resource_exhausted());
        assert!(matches!(
            ComponentRegistryOps::child_allocation(component, [44; 32])
                .expect("child allocation")
                .expect("reserved child")
                .progress,
            RootComponentChildAllocationProgressView::Reserved
        ));
        RootComponentRegistryStore::import(before_creation);

        ComponentRegistryOps::validate_child_creation_capacity(component, [44; 32], &plan)
            .expect("child creation capacity");
        let intent = ComponentRegistryOps::begin_child_creation(
            component,
            [44; 32],
            plan,
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(52),
                reservation_intent_id: IntentId(53),
            },
        )
        .expect("child creation intent");
        let intent_bytes = ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes;
        assert!(intent_bytes > current.encoded_bytes);
        assert!(matches!(
            intent.progress,
            RootComponentChildAllocationProgressView::CreationIntent(_)
        ));

        restart_component_registry();
        let canister = candid::Principal::from_slice(&[54; 29]);
        let created = ComponentRegistryOps::mark_child_created(component, [44; 32], canister)
            .expect("record created child");
        restart_component_registry();
        let repeated_created =
            ComponentRegistryOps::mark_child_created(component, [44; 32], canister)
                .expect("exact created child retry");

        assert_eq!(created, repeated_created);
        assert!(matches!(
            created.progress,
            RootComponentChildAllocationProgressView::Created {
                canister: created_canister,
                ..
            } if created_canister == canister
        ));
        let created_status = ComponentRegistryOps::current().expect("Registry status");
        assert_eq!(created_status.known_created_component_canisters, 2);
        assert_eq!(created_status.managed_descendants, 1);
        assert_eq!(created_status.encoded_bytes, intent_bytes);
        assert_eq!(
            ComponentRegistryOps::partition(component)
                .expect("partition read")
                .expect("partition")
                .reserved_descendants,
            1
        );
        assert!(
            ComponentRegistryOps::mark_child_created(
                component,
                [44; 32],
                candid::Principal::from_slice(&[55; 29]),
            )
            .is_err()
        );
        let install_plan = RootComponentChildInstallPlan {
            raw_module_hash: [56; 32],
            chunk_hashes: vec![vec![57; 32], vec![58; 32]],
            binding: ComponentChildBinding {
                component: binding,
                parent_canister_id: parent,
                role: decision.child_role.clone(),
                canister_id: canister,
            },
            maximum_registry_bytes: decision.maximum_registry_bytes,
        };
        let before_install = RootComponentRegistryStore::export();
        let mut install_capacity_exhausted = before_install.clone();
        let maximum_registry_bytes = install_capacity_exhausted
            .current
            .as_ref()
            .expect("Registry status")
            .encoded_bytes;
        install_capacity_exhausted
            .current
            .as_mut()
            .expect("Registry status")
            .root
            .limits
            .maximum_registry_bytes = maximum_registry_bytes;
        RootComponentRegistryStore::import(install_capacity_exhausted);
        let error = ComponentRegistryOps::validate_child_install_capacity(
            component,
            [44; 32],
            &install_plan,
        )
        .expect_err("installation must fit before the paid effect");
        assert!(error.is_public_resource_exhausted());
        assert!(matches!(
            ComponentRegistryOps::child_allocation(component, [44; 32])
                .expect("child allocation")
                .expect("created child")
                .progress,
            RootComponentChildAllocationProgressView::Created { .. }
        ));
        RootComponentRegistryStore::import(before_install);

        ComponentRegistryOps::validate_child_install_capacity(component, [44; 32], &install_plan)
            .expect("child install capacity");
        let install_intent = ComponentRegistryOps::begin_child_install(
            component,
            [44; 32],
            install_plan.clone(),
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(59),
                reservation_intent_id: IntentId(60),
            },
        )
        .expect("child install intent");
        let install_intent_bytes = ComponentRegistryOps::current()
            .expect("Registry status")
            .encoded_bytes;
        assert!(install_intent_bytes > intent_bytes);
        assert!(matches!(
            install_intent.progress,
            RootComponentChildAllocationProgressView::InstallIntent { .. }
        ));

        let mut conflicting_install = install_plan.clone();
        conflicting_install.raw_module_hash = [61; 32];
        assert!(
            ComponentRegistryOps::renew_child_install_intent(
                component,
                [44; 32],
                &conflicting_install,
                ReplayCostGuardSettlement {
                    quota_intent_id: IntentId(62),
                    reservation_intent_id: IntentId(63),
                },
            )
            .is_err()
        );
        restart_component_registry();
        let renewed = ComponentRegistryOps::renew_child_install_intent(
            component,
            [44; 32],
            &install_plan,
            ReplayCostGuardSettlement {
                quota_intent_id: IntentId(64),
                reservation_intent_id: IntentId(65),
            },
        )
        .expect("renew exact child install intent");
        let RootComponentChildAllocationProgressView::InstallIntent { installation, .. } =
            &renewed.progress
        else {
            panic!("renewed child install intent");
        };
        assert_eq!(installation.binding, install_plan.binding);
        assert_eq!(
            installation.cost_guard_settlement.quota_intent_id,
            IntentId(64)
        );

        let installed = ComponentRegistryOps::mark_child_installed(component, [44; 32])
            .expect("mark child installed");
        restart_component_registry();
        let installed_retry = ComponentRegistryOps::mark_child_installed(component, [44; 32])
            .expect("installed child retry");
        assert_eq!(installed, installed_retry);
        assert!(matches!(
            installed.progress,
            RootComponentChildAllocationProgressView::Installed { .. }
        ));
        let verified = ComponentRegistryOps::mark_child_verified(component, [44; 32])
            .expect("mark child verified");
        restart_component_registry();
        let verified_retry = ComponentRegistryOps::mark_child_verified(component, [44; 32])
            .expect("verified child retry");
        assert_eq!(verified, verified_retry);
        assert!(matches!(
            verified.progress,
            RootComponentChildAllocationProgressView::Verified { .. }
        ));
        let verified_status = ComponentRegistryOps::current().expect("Registry status");
        assert_eq!(verified_status.known_created_component_canisters, 2);
        assert_eq!(verified_status.managed_descendants, 1);
        assert_eq!(verified_status.encoded_bytes, install_intent_bytes);
        let verified_partition = ComponentRegistryOps::partition(component)
            .expect("partition read")
            .expect("partition");
        assert_eq!(verified_partition.reserved_descendants, 1);
        assert_eq!(verified_partition.committed_descendants, 0);
        assert_eq!(verified_partition.encoded_bytes, install_intent_bytes);

        let committed = ComponentRegistryOps::commit_verified_child(
            component,
            [44; 32],
            66,
            fleet_directory(&root),
        )
        .expect("commit verified child");
        let committed_partition = ComponentRegistryOps::partition(component)
            .expect("partition read")
            .expect("partition");
        assert_eq!(committed_partition.revision, 3);
        assert_ne!(
            committed_partition.content_hash,
            verified_partition.content_hash
        );
        assert_eq!(committed_partition.directory_synchronized_at_ns, 66);
        assert_eq!(committed_partition.reserved_descendants, 0);
        assert_eq!(committed_partition.committed_descendants, 1);
        assert!(committed_partition.encoded_bytes <= install_intent_bytes);
        let RootComponentChildAllocationProgressView::Committed {
            commitment,
            installation,
            ..
        } = &committed.0.progress
        else {
            panic!("committed child progress");
        };
        assert_eq!(
            commitment.registry,
            ComponentRegistryHead {
                component,
                revision: committed_partition.revision,
                content_hash: committed_partition.content_hash,
            }
        );
        assert_eq!(
            commitment.registry_encoded_bytes,
            committed_partition.encoded_bytes
        );
        assert_eq!(commitment.reserved_descendants, 0);
        assert_eq!(commitment.committed_descendants, 1);
        assert_ne!(commitment.directory_authority_hash, [0; 32]);
        let child_directory_authority_hash = commitment.directory_authority_hash;
        assert_eq!(installation.binding, install_plan.binding);
        assert_eq!(committed.1, committed_partition);
        let committed_status = ComponentRegistryOps::current().expect("Registry status");
        assert_eq!(committed_status.managed_descendants, 1);
        assert_eq!(committed_status.known_created_component_canisters, 2);
        assert_eq!(
            committed_status.encoded_bytes,
            committed_partition.encoded_bytes
        );
        assert_eq!(
            ComponentRegistryOps::parent_role_instances(component, parent, &decision.child_role,)
                .expect("parent-role count"),
            1
        );
        let registered_child = ComponentRegistryOps::registered_parent(component, canister)
            .expect("registered child")
            .expect("normalized child");
        assert_eq!(
            registered_child,
            (
                ManagedCanisterBinding::ComponentChild(install_plan.binding),
                ComponentLifecycleStatus::Prepared,
            )
        );
        let durable = restart_component_registry();
        assert_eq!(durable.children.len(), 1);
        assert_eq!(durable.child_traversals.len(), 1);
        let progressed_partition = ComponentRegistryOps::partition(component)
            .expect("partition read")
            .expect("partition");
        let progressed_reservation = ComponentRegistryOps::reserve_child_allocation(
            decision.clone(),
            [68; 32],
            ComponentRegistryHead {
                component,
                revision: progressed_partition.revision,
                content_hash: progressed_partition.content_hash,
            },
        )
        .expect("later child reservation");
        assert!(matches!(
            progressed_reservation.progress,
            RootComponentChildAllocationProgressView::Reserved
        ));
        let committed_retry = ComponentRegistryOps::commit_verified_child(
            component,
            [44; 32],
            67,
            fleet_directory(&root),
        )
        .expect("exact child commit retry");
        assert_eq!(committed_retry, committed);

        let retried_reservation = ComponentRegistryOps::reserve_child_allocation(
            decision.clone(),
            [44; 32],
            repeated.reserved_against_registry,
        )
        .expect("reservation retry preserves install progress");
        assert_eq!(retried_reservation, committed.0);
        let before_directory_receipt = ComponentRegistryOps::current().expect("Registry status");
        let before_directory_partition = ComponentRegistryOps::partition(component)
            .expect("partition read")
            .expect("partition");
        assert!(
            ComponentRegistryOps::mark_child_runtime_activated(
                component,
                [44; 32],
                child_directory_authority_hash,
            )
            .is_err()
        );
        let prepared = ComponentRegistryOps::mark_child_directory_prepared(
            component,
            [44; 32],
            child_directory_authority_hash,
        )
        .expect("mark child Directory prepared");
        restart_component_registry();
        let prepared_again = ComponentRegistryOps::mark_child_directory_prepared(
            component,
            [44; 32],
            child_directory_authority_hash,
        )
        .expect("repeat child Directory preparation receipt");
        assert_eq!(prepared_again, prepared);
        assert!(matches!(
            prepared.progress,
            RootComponentChildAllocationProgressView::Committed {
                commitment: RootComponentChildCommitmentView {
                    directory_prepared: true,
                    runtime_activated: false,
                    membership: None,
                    ..
                },
                ..
            }
        ));
        assert!(
            ComponentRegistryOps::activate_child_membership(
                component,
                [44; 32],
                69,
                fleet_directory(&root),
            )
            .is_err()
        );
        let activated = ComponentRegistryOps::mark_child_runtime_activated(
            component,
            [44; 32],
            child_directory_authority_hash,
        )
        .expect("mark child runtime activated");
        restart_component_registry();
        let activated_again = ComponentRegistryOps::mark_child_runtime_activated(
            component,
            [44; 32],
            child_directory_authority_hash,
        )
        .expect("repeat child runtime activation receipt");
        assert_eq!(activated_again, activated);
        assert!(matches!(
            activated.progress,
            RootComponentChildAllocationProgressView::Committed {
                commitment: RootComponentChildCommitmentView {
                    directory_prepared: true,
                    runtime_activated: true,
                    membership: None,
                    ..
                },
                ..
            }
        ));
        assert_eq!(
            ComponentRegistryOps::current().expect("Registry status"),
            before_directory_receipt
        );
        assert_eq!(
            ComponentRegistryOps::partition(component)
                .expect("partition read")
                .expect("partition"),
            before_directory_partition
        );
        let membership = ComponentRegistryOps::activate_child_membership(
            component,
            [44; 32],
            69,
            fleet_directory(&root),
        )
        .expect("activate child membership");
        restart_component_registry();
        let membership_again = ComponentRegistryOps::activate_child_membership(
            component,
            [44; 32],
            70,
            fleet_directory(&root),
        )
        .expect("repeat child membership activation");
        assert_eq!(membership_again, membership);
        assert_eq!(
            membership.1.revision,
            before_directory_partition.revision + 1
        );
        assert_eq!(membership.1.status, ComponentLifecycleStatus::Active);
        assert_eq!(
            membership.1.reserved_descendants,
            before_directory_partition.reserved_descendants
        );
        assert_eq!(
            membership.1.committed_descendants,
            before_directory_partition.committed_descendants
        );
        assert_ne!(
            membership.1.descendant_content_hash,
            before_directory_partition.descendant_content_hash
        );
        assert_eq!(membership.1.directory_synchronized_at_ns, 69);
        let RootComponentChildAllocationProgressView::Committed {
            commitment:
                RootComponentChildCommitmentView {
                    membership: Some(active_membership),
                    ..
                },
            ..
        } = &membership.0.progress
        else {
            panic!("active child membership receipt");
        };
        assert_eq!(
            active_membership.registry,
            ComponentRegistryHead {
                component,
                revision: membership.1.revision,
                content_hash: membership.1.content_hash,
            }
        );
        assert_eq!(
            active_membership.descendant_content_hash,
            membership.1.descendant_content_hash
        );
        assert_eq!(
            active_membership.registry_encoded_bytes,
            membership.1.encoded_bytes
        );
        assert!(!active_membership.directory_synchronized);
        assert_eq!(
            ComponentRegistryOps::registered_parent(component, canister)
                .expect("registered active child")
                .expect("active child row")
                .1,
            ComponentLifecycleStatus::Active
        );
        assert!(
            ComponentRegistryOps::mark_child_membership_synchronized(
                component,
                [44; 32],
                [u8::MAX; 32],
            )
            .is_err()
        );
        let terminal = ComponentRegistryOps::mark_child_membership_synchronized(
            component,
            [44; 32],
            active_membership.directory_authority_hash,
        )
        .expect("mark child membership synchronized");
        let terminal_snapshot = restart_component_registry();
        let terminal_again = ComponentRegistryOps::mark_child_membership_synchronized(
            component,
            [44; 32],
            active_membership.directory_authority_hash,
        )
        .expect("repeat child membership synchronization receipt");
        assert_eq!(terminal_again, terminal);
        assert!(matches!(
            terminal.progress,
            RootComponentChildAllocationProgressView::Committed {
                commitment: RootComponentChildCommitmentView {
                    membership: Some(RootComponentChildMembershipView {
                        directory_synchronized: true,
                        ..
                    }),
                    ..
                },
                ..
            }
        ));
        let terminal_partition = ComponentRegistryOps::partition(component)
            .expect("partition read")
            .expect("terminal active partition");
        assert_eq!(terminal_partition, membership.1);
        let exact_terminal_bytes = exact_registry_entry_bytes(&terminal_snapshot);
        assert_eq!(terminal_partition.encoded_bytes, exact_terminal_bytes);
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("terminal Registry status")
                .encoded_bytes,
            exact_terminal_bytes
        );
        let complete_directory = ComponentRegistryOps::directory_page(
            component,
            &ComponentDirectoryPageSelection {
                parent_canister_id: None,
                role: None,
                status: None,
                start_after: None,
            },
            100,
        )
        .expect("complete Component Directory page");
        assert_eq!(complete_directory.entries.len(), 1);
        assert_eq!(
            complete_directory.entries[0].binding.component,
            terminal_partition.binding
        );
        assert_eq!(
            complete_directory.entries[0].binding.parent_canister_id,
            parent
        );
        assert_eq!(
            complete_directory.entries[0].binding.role,
            decision.child_role
        );
        assert_eq!(complete_directory.entries[0].binding.canister_id, canister);
        assert_eq!(
            complete_directory.entries[0].status,
            ComponentLifecycleStatus::Active
        );
        assert!(complete_directory.next_cursor.is_none());
        let direct_active_children = ComponentRegistryOps::directory_page(
            component,
            &ComponentDirectoryPageSelection {
                parent_canister_id: Some(parent),
                role: Some(decision.child_role.clone()),
                status: Some(ComponentLifecycleStatus::Active),
                start_after: None,
            },
            100,
        )
        .expect("filtered direct-child Directory page");
        assert_eq!(direct_active_children.entries, complete_directory.entries);
        let after_only_child = ComponentRegistryOps::directory_page(
            component,
            &ComponentDirectoryPageSelection {
                parent_canister_id: Some(parent),
                role: Some(decision.child_role.clone()),
                status: Some(ComponentLifecycleStatus::Active),
                start_after: Some(ComponentDirectoryCanonicalCursor {
                    parent_canister_id: parent,
                    role: decision.child_role.clone(),
                    canister_id: canister,
                }),
            },
            100,
        )
        .expect("Directory page after only child");
        assert!(after_only_child.entries.is_empty());
        assert!(after_only_child.next_cursor.is_none());
        let prepared_children = ComponentRegistryOps::directory_page(
            component,
            &ComponentDirectoryPageSelection {
                parent_canister_id: Some(parent),
                role: Some(decision.child_role.clone()),
                status: Some(ComponentLifecycleStatus::Prepared),
                start_after: None,
            },
            100,
        )
        .expect("status-filtered Directory page");
        assert!(prepared_children.entries.is_empty());
        let later_reservation = ComponentRegistryOps::reserve_child_allocation(
            decision,
            [71; 32],
            ComponentRegistryHead {
                component,
                revision: terminal_partition.revision,
                content_hash: terminal_partition.content_hash,
            },
        )
        .expect("reserve later child after membership");
        assert!(matches!(
            later_reservation.progress,
            RootComponentChildAllocationProgressView::Reserved
        ));
        let membership_after_later_reservation = ComponentRegistryOps::activate_child_membership(
            component,
            [44; 32],
            72,
            fleet_directory(&root),
        )
        .expect("membership retry after later reservation");
        assert_eq!(membership_after_later_reservation.0, terminal);
        assert_eq!(membership_after_later_reservation.1, membership.1);
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one test follows the complete paid creation lifecycle and its exact retry invariants"
    )]
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
            false,
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
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .known_created_component_canisters,
            0,
            "creation intent without a known principal must not count"
        );

        let interrupted = RootComponentRegistryStore::export();
        RootComponentRegistryStore::import(interrupted);
        let canister = candid::Principal::from_slice(&[18; 29]);
        let created =
            ComponentRegistryOps::mark_created([12; 32], canister).expect("record created");
        let repeated =
            ComponentRegistryOps::mark_created([12; 32], canister).expect("exact created retry");

        assert_eq!(created, repeated);
        let status = ComponentRegistryOps::current().expect("Registry status");
        assert_eq!(
            status.encoded_bytes, intent_bytes,
            "the intent must reserve terminal record capacity before the effect"
        );
        assert_eq!(
            status.known_created_component_canisters, 1,
            "a known created principal must be counted exactly once"
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
        assert_eq!(
            ComponentRegistryOps::current()
                .expect("Registry status")
                .known_created_component_canisters,
            1,
            "a conflicting retry must not change the count"
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
        assert_initial_inventory_receipt();
    }

    fn assert_initial_inventory_receipt() {
        let sealed = ComponentRegistryOps::seal_initial_inventory([40; 32], 41)
            .expect("seal initial inventory");
        assert_eq!(sealed.operation_ids, vec![[12; 32]]);
        assert_eq!(sealed.receipt.fleet_activation_operation_id, [40; 32]);
        assert_eq!(sealed.receipt.component_count, 1);
        assert_ne!(sealed.receipt.inventory_hash, [0; 32]);
        assert_eq!(sealed.receipt.sealed_at_ns, 41);
        assert!(!sealed.receipt.directories_converged);
        assert!(!sealed.receipt.root_runtime_activated);
        let repeated = ComponentRegistryOps::seal_initial_inventory([40; 32], 42)
            .expect("retry initial inventory seal");
        assert_eq!(repeated, sealed);
        assert!(
            ComponentRegistryOps::reserve_allocation(
                TopLevelComponentAllocationDecision {
                    allocation_sequence: 2,
                    component: ComponentInstanceId::from_generated_bytes([42; 32]),
                    component_spec: "projects".parse().expect("Component Spec"),
                    spec_hash: [43; 32],
                    role: CanisterRole::new("project_hub"),
                },
                [44; 32],
                ComponentProvisioningOrigin::FleetAdministrator {
                    caller: candid::Principal::from_slice(&[11; 29]),
                },
                false,
            )
            .is_err(),
            "a Prepared root cannot extend its sealed initial inventory"
        );

        let converged = ComponentRegistryOps::mark_initial_inventory_directories_converged(
            [40; 32],
            sealed.receipt.inventory_hash,
        )
        .expect("mark initial Directories converged");
        assert!(converged.directories_converged);
        assert!(!converged.root_runtime_activated);
        let terminal = ComponentRegistryOps::mark_initial_inventory_root_runtime_activated(
            [40; 32],
            sealed.receipt.inventory_hash,
        )
        .expect("mark root runtime activated");
        assert!(terminal.directories_converged);
        assert!(terminal.root_runtime_activated);
        assert_eq!(
            ComponentRegistryOps::initial_inventory([40; 32]).expect("terminal initial inventory"),
            terminal
        );
        assert_child_reservation_preserves_membership_receipt();
        ComponentRegistryOps::reserve_allocation(
            TopLevelComponentAllocationDecision {
                allocation_sequence: 2,
                component: ComponentInstanceId::from_generated_bytes([42; 32]),
                component_spec: "projects".parse().expect("Component Spec"),
                spec_hash: [43; 32],
                role: CanisterRole::new("project_hub"),
            },
            [44; 32],
            ComponentProvisioningOrigin::FleetAdministrator {
                caller: candid::Principal::from_slice(&[11; 29]),
            },
            true,
        )
        .expect("active root admits dynamic allocation after terminal initial receipt");
    }

    fn assert_child_reservation_preserves_membership_receipt() {
        let allocation =
            ComponentRegistryOps::allocation([12; 32]).expect("committed Component allocation");
        let membership = committed_membership(&allocation)
            .expect("active membership receipt")
            .clone();
        let partition = ComponentRegistryOps::partition(allocation.component)
            .expect("valid active partition")
            .expect("active partition");
        ComponentRegistryOps::reserve_child_allocation(
            ComponentChildAllocationDecision {
                component: allocation.component,
                component_spec: allocation.component_spec,
                spec_hash: allocation.spec_hash,
                parent_canister_id: partition.binding.canister_id,
                parent_role: partition.binding.role.clone(),
                child_role: CanisterRole::new("project_instance"),
                child_kind: ComponentChildKind::Instance,
                maximum_instances_per_parent: 10_000,
                maximum_descendants: 20_000,
                maximum_registry_bytes: 16_777_216,
            },
            [50; 32],
            ComponentRegistryHead {
                component: allocation.component,
                revision: partition.revision,
                content_hash: partition.content_hash,
            },
        )
        .expect("reserve active Component child");

        let retried = ComponentRegistryOps::mark_membership_synchronized(
            [12; 32],
            membership.directory_authority_hash,
        )
        .expect("immutable membership receipt remains valid after child reservation");
        assert_eq!(
            committed_membership(&retried).expect("membership receipt"),
            &membership
        );
        assert_eq!(
            ComponentRegistryOps::partition(allocation.component)
                .expect("valid active partition")
                .expect("active partition")
                .reserved_descendants,
            1
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
            false,
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
            false,
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

    struct ActiveComponentTreeFixture {
        component: ComponentInstanceId,
        partition: ComponentRegistryPartitionRecord,
        target: ComponentRegistryChildRecord,
        descendant: ComponentRegistryChildRecord,
        unrelated: ComponentRegistryChildRecord,
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the fixture assembles one exact normalized multi-level Component tree"
    )]
    fn import_active_component_tree() -> ActiveComponentTreeFixture {
        RootComponentRegistryStore::import(RootComponentRegistryData::default());
        let root = root_binding();
        let release_set = FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [8; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
        };
        let component = ComponentInstanceId::from_generated_bytes([10; 32]);
        let component_canister = candid::Principal::from_slice(&[18; 29]);
        let mut partition =
            active_component_partition(&root, release_set, component, component_canister);
        let target = ComponentRegistryChildRecord {
            component,
            canister_id: candid::Principal::from_slice(&[21; 29]),
            parent_canister_id: component_canister,
            role: CanisterRole::new("project_instance"),
            kind: ComponentChildKind::Instance,
            installed_artifact_hash: [31; 32],
            status: ComponentLifecycleStatus::Active,
        };
        let descendant = ComponentRegistryChildRecord {
            component,
            canister_id: candid::Principal::from_slice(&[22; 29]),
            parent_canister_id: target.canister_id,
            role: CanisterRole::new("project_ledger"),
            kind: ComponentChildKind::Singleton,
            installed_artifact_hash: [32; 32],
            status: ComponentLifecycleStatus::Active,
        };
        let unrelated = ComponentRegistryChildRecord {
            component,
            canister_id: candid::Principal::from_slice(&[23; 29]),
            parent_canister_id: component_canister,
            role: CanisterRole::new("project_instance"),
            kind: ComponentChildKind::Instance,
            installed_artifact_hash: [33; 32],
            status: ComponentLifecycleStatus::Active,
        };
        let children = vec![target.clone(), descendant.clone(), unrelated.clone()];
        let child_traversals = children
            .iter()
            .map(|child| ComponentRegistryChildTraversalRecord {
                component,
                parent_canister_id: child.parent_canister_id,
                role: child.role.clone(),
                canister_id: child.canister_id,
            })
            .collect();
        let parent_role_counts = vec![
            ComponentRegistryParentRoleCountRecord {
                component,
                parent_canister_id: component_canister,
                child_role: CanisterRole::new("project_instance"),
                instances: 2,
            },
            ComponentRegistryParentRoleCountRecord {
                component,
                parent_canister_id: target.canister_id,
                child_role: CanisterRole::new("project_ledger"),
                instances: 1,
            },
        ];
        partition.committed_descendants = 3;
        partition.descendant_content_hash = [77; 32];
        partition.content_hash = component_partition_content_hash(
            &partition.binding,
            &partition.provisioning_origin,
            partition.release_set,
            partition.status,
            partition.revision,
            partition.descendant_content_hash,
            partition.committed_descendants,
        )
        .expect("nonempty partition hash");
        partition.encoded_bytes = 0;
        let mut data = RootComponentRegistryData {
            partitions: vec![partition.clone()],
            children,
            child_traversals,
            parent_role_counts,
            ..RootComponentRegistryData::default()
        };
        for _ in 0..8 {
            data.partitions[0] = partition.clone();
            let encoded_bytes = exact_component_registry_entry_bytes(&data, component);
            if partition.encoded_bytes == encoded_bytes {
                break;
            }
            partition.encoded_bytes = encoded_bytes;
        }
        data.partitions[0] = partition.clone();
        assert_eq!(
            partition.encoded_bytes,
            exact_component_registry_entry_bytes(&data, component)
        );
        data.current = Some(RootComponentRegistryMetaRecord {
            root: root.clone(),
            prepared_against_registry: FleetRegistryVersion {
                authority: root.authority,
                revision: 4,
                content_hash: [5; 32],
            },
            release_set,
            store_bootstrap: RootStoreBootstrapRequest {
                manifest_payload_size_bytes: 128,
            },
            next_allocation_sequence: 2,
            reserved_component_instances: 0,
            committed_component_instances: 1,
            managed_descendants: 3,
            known_created_component_canisters: 4,
            encoded_bytes: partition.encoded_bytes,
            initial_inventory: None,
        });
        RootComponentRegistryStore::import(data);
        ActiveComponentTreeFixture {
            component,
            partition,
            target,
            descendant,
            unrelated,
        }
    }

    fn active_component_partition(
        root: &FleetSubnetRootBinding,
        release_set: FleetSubnetRootReleaseSet,
        component: ComponentInstanceId,
        canister_id: candid::Principal,
    ) -> ComponentRegistryPartitionRecord {
        let binding = ComponentBinding {
            authority: root.authority.clone(),
            component,
            component_spec: "projects".parse().expect("Component Spec"),
            spec_hash: [6; 32],
            role: CanisterRole::new("project_hub"),
            placement_subnet: root.placement_subnet,
            fleet_subnet_root: root.fleet_subnet_root,
            canister_id,
        };
        let provisioning_origin = ComponentProvisioningOrigin::FleetAdministrator {
            caller: candid::Principal::from_slice(&[11; 29]),
        };
        let descendant_content_hash = empty_component_descendant_content_hash(component);
        let mut partition = ComponentRegistryPartitionRecord {
            content_hash: component_partition_content_hash(
                &binding,
                &provisioning_origin,
                release_set,
                ComponentLifecycleStatus::Active,
                2,
                descendant_content_hash,
                0,
            )
            .expect("partition hash"),
            binding,
            provisioning_origin,
            release_set,
            status: ComponentLifecycleStatus::Active,
            revision: 2,
            descendant_content_hash,
            directory_synchronized_at_ns: 33,
            reserved_descendants: 0,
            committed_descendants: 0,
            encoded_bytes: 0,
        };
        let principal_index_bytes =
            RootComponentRegistryStore::principal_index_entry_bytes(canister_id, component);
        for _ in 0..8 {
            let encoded_bytes = RootComponentRegistryStore::partition_entry_bytes(&partition)
                + principal_index_bytes;
            if partition.encoded_bytes == encoded_bytes {
                break;
            }
            partition.encoded_bytes = encoded_bytes;
        }
        assert_eq!(
            partition.encoded_bytes,
            RootComponentRegistryStore::partition_entry_bytes(&partition) + principal_index_bytes
        );
        partition
    }

    fn child_allocation_decision(
        partition: &ComponentRegistryPartitionRecord,
        child_role: &'static str,
    ) -> ComponentChildAllocationDecision {
        child_allocation_decision_for_parent(
            partition,
            partition.binding.canister_id,
            &partition.binding.role,
            child_role,
        )
    }

    fn child_allocation_decision_for_parent(
        partition: &ComponentRegistryPartitionRecord,
        parent_canister_id: candid::Principal,
        parent_role: &CanisterRole,
        child_role: &'static str,
    ) -> ComponentChildAllocationDecision {
        ComponentChildAllocationDecision {
            component: partition.binding.component,
            component_spec: partition.binding.component_spec.clone(),
            spec_hash: partition.binding.spec_hash,
            parent_canister_id,
            parent_role: parent_role.clone(),
            child_role: CanisterRole::new(child_role),
            child_kind: ComponentChildKind::Instance,
            maximum_instances_per_parent: 10_000,
            maximum_descendants: 20_000,
            maximum_registry_bytes: 16_777_216,
        }
    }

    fn component_registry_head(
        partition: &ComponentRegistryPartitionRecord,
    ) -> ComponentRegistryHead {
        ComponentRegistryHead {
            component: partition.binding.component,
            revision: partition.revision,
            content_hash: partition.content_hash,
        }
    }

    fn child_creation_plan(
        root: &FleetSubnetRootBinding,
        evidence_seed: u8,
    ) -> RootComponentCreationPlan {
        RootComponentCreationPlan {
            wasm_store: candid::Principal::from_slice(&[evidence_seed; 29]),
            payload_hash: [evidence_seed; 32],
            payload_size_bytes: 4_096,
            initial_cycles: Cycles::new(5_000_000_000_000),
            controller: root.fleet_subnet_root,
        }
    }

    fn root_binding() -> FleetSubnetRootBinding {
        let coordinator_subnet = SubnetId::from_principal(candid::Principal::from_slice(&[2; 29]));
        FleetSubnetRootBinding {
            authority: FleetRegistryAuthority {
                binding: FleetCoordinatorBinding {
                    fleet: FleetBinding {
                        fleet: FleetKey {
                            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
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
