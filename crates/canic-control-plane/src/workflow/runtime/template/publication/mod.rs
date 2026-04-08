mod fleet;
mod store;

use super::{WASM_STORE_BOOTSTRAP_BINDING, store_pid_for_binding};
use crate::{
    config,
    dto::template::{
        TemplateChunkSetInfoResponse, TemplateChunkSetPrepareInput, TemplateManifestInput,
        TemplateManifestResponse, WasmStoreAdminCommand, WasmStoreAdminResponse,
        WasmStoreFinalizedStoreResponse, WasmStorePublicationSlotResponse,
        WasmStorePublicationStatusResponse, WasmStorePublicationStoreStatusResponse,
        WasmStoreRetiredStoreStatusResponse,
    },
    ids::{
        CanisterRole, TemplateChunkingMode, TemplateManifestState, WasmStoreBinding,
        WasmStoreGcMode,
    },
    ops::storage::{
        state::subnet::SubnetStateOps,
        template::{TemplateChunkedOps, TemplateManifestOps},
    },
    storage::stable::state::subnet::PublicationStoreStateRecord,
};
use canic_core::{__control_plane_core as cp_core, log, log::Topic};
use cp_core::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    ops::{ic::IcOps, storage::registry::subnet::SubnetRegistryOps},
    workflow::{
        canister_lifecycle::{CanisterLifecycleEvent, CanisterLifecycleWorkflow},
        ic::provision::ProvisionWorkflow,
    },
};
use std::collections::BTreeMap;

use fleet::{
    PublicationPlacement, PublicationPlacementAction, PublicationStoreFleet,
    PublicationStoreSnapshot,
};
use store::{
    TemplateChunkInputRef, local_chunk, store_begin_gc, store_binding_for_pid, store_catalog,
    store_chunk, store_chunk_set_info, store_complete_gc, store_prepare_gc, store_stage_manifest,
    store_status,
};

const WASM_STORE_ROLE: CanisterRole = CanisterRole::WASM_STORE;

///
/// WasmStorePublicationWorkflow
///

pub struct WasmStorePublicationWorkflow;

impl WasmStorePublicationWorkflow {
    const WASM_STORE_CAPACITY_EXCEEDED_MESSAGE: &str = "wasm store capacity exceeded";

    // Build the canonical runtime-managed binding for one wasm store canister id.
    fn binding_for_store_pid(store_pid: Principal) -> WasmStoreBinding {
        WasmStoreBinding::owned(store_pid.to_text())
    }

    // Import any already-registered wasm stores into runtime subnet state.
    pub fn sync_registered_wasm_store_inventory() -> Vec<WasmStoreBinding> {
        let mut bindings = Vec::new();

        for pid in SubnetRegistryOps::pids_for_role(&WASM_STORE_ROLE).unwrap_or_default() {
            let binding = Self::binding_for_store_pid(pid);
            let created_at = SubnetRegistryOps::get(pid).map_or(0, |record| record.created_at);
            let _ = SubnetStateOps::upsert_wasm_store(binding.clone(), pid, created_at);
            bindings.push(binding);
        }

        bindings
    }

    // Return the current retired runtime-managed publication store status, if one exists.
    pub async fn retired_publication_store_status()
    -> Result<Option<WasmStoreRetiredStoreStatusResponse>, InternalError> {
        let state = SubnetStateOps::publication_store_state();
        let Some(retired_binding) = state.retired_binding.clone() else {
            return Ok(None);
        };

        let store_pid = store_pid_for_binding(&retired_binding)?;
        let store = store_status(store_pid).await?;

        Ok(Some(WasmStoreRetiredStoreStatusResponse {
            retired_binding,
            generation: state.generation,
            retired_at: state.retired_at,
            gc_ready: store.gc.mode == WasmStoreGcMode::Complete,
            reclaimable_store_bytes: store.occupied_store_bytes,
            store,
        }))
    }

    // Return one root-facing live publication snapshot that explains slot state and candidate order.
    pub async fn publication_status() -> Result<WasmStorePublicationStatusResponse, InternalError> {
        let managed_manifests = Self::managed_release_manifests()?;
        let fleet = Self::snapshot_publication_store_fleet().await?;
        let publication = SubnetStateOps::publication_store_state_response();
        let writable_indices = fleet.writable_store_indices();
        let mut candidate_orders = BTreeMap::new();

        for (order, index) in writable_indices.into_iter().enumerate() {
            let order = u32::try_from(order).unwrap_or(u32::MAX);
            candidate_orders.insert(index, order);
        }

        let stores = fleet
            .stores
            .iter()
            .enumerate()
            .map(|(index, store)| {
                let exact_managed_release_count = u32::try_from(
                    managed_manifests
                        .iter()
                        .filter(|manifest| store.has_exact_release(manifest))
                        .count(),
                )
                .unwrap_or(u32::MAX);
                let conflicting_managed_release_count = u32::try_from(
                    managed_manifests
                        .iter()
                        .filter(|manifest| store.conflicting_release(manifest).is_some())
                        .count(),
                )
                .unwrap_or(u32::MAX);
                let publication_slot =
                    if publication.active_binding.as_ref() == Some(&store.binding) {
                        Some(WasmStorePublicationSlotResponse::Active)
                    } else if publication.detached_binding.as_ref() == Some(&store.binding) {
                        Some(WasmStorePublicationSlotResponse::Detached)
                    } else if publication.retired_binding.as_ref() == Some(&store.binding) {
                        Some(WasmStorePublicationSlotResponse::Retired)
                    } else {
                        None
                    };
                let is_reserved_for_publication = Self::binding_is_reserved_for_publication(
                    &fleet.reserved_state,
                    &store.binding,
                );

                WasmStorePublicationStoreStatusResponse {
                    binding: store.binding.clone(),
                    pid: store.pid,
                    created_at: store.created_at,
                    publication_slot,
                    is_preferred_binding: fleet.preferred_binding.as_ref() == Some(&store.binding),
                    is_reserved_for_publication,
                    is_selectable_for_publication: !is_reserved_for_publication,
                    publication_candidate_order: candidate_orders.get(&index).copied(),
                    exact_managed_release_count,
                    conflicting_managed_release_count,
                    store: store.status.clone(),
                }
            })
            .collect();

        Ok(WasmStorePublicationStatusResponse {
            publication,
            preferred_binding: fleet.preferred_binding,
            managed_release_count: u32::try_from(managed_manifests.len()).unwrap_or(u32::MAX),
            stores,
        })
    }

    // Create one new wasm store canister and register its runtime-managed binding.
    async fn create_publication_store() -> Result<WasmStoreBinding, InternalError> {
        let result = CanisterLifecycleWorkflow::apply(CanisterLifecycleEvent::Create {
            role: WASM_STORE_ROLE,
            parent: IcOps::canister_self(),
            extra_arg: None,
        })
        .await?;
        let pid = result.new_canister_pid.ok_or_else(|| {
            InternalError::workflow(
                InternalErrorOrigin::Workflow,
                "wasm store creation did not return a pid",
            )
        })?;
        let binding = Self::binding_for_store_pid(pid);
        let created_at =
            SubnetRegistryOps::get(pid).map_or_else(IcOps::now_secs, |record| record.created_at);
        let _ = SubnetStateOps::upsert_wasm_store(binding.clone(), pid, created_at);

        log!(Topic::Wasm, Ok, "ws created {} ({})", binding, pid);

        Ok(binding)
    }

    // Snapshot the current writable store fleet and the current preferred write hint.
    async fn snapshot_publication_store_fleet() -> Result<PublicationStoreFleet, InternalError> {
        Self::sync_registered_wasm_store_inventory();

        let preferred_binding = match SubnetStateOps::publication_store_binding() {
            Some(binding) if store_pid_for_binding(&binding).is_ok() => Some(binding),
            Some(binding) => Some(Self::clear_stale_publication_binding(binding)?),
            None => Self::oldest_registered_store_binding(),
        };
        let reserved_state = SubnetStateOps::publication_store_state();
        let mut stores = Vec::new();

        for record in SubnetStateOps::wasm_stores() {
            let status = store_status(record.pid).await?;
            let releases = store_catalog(record.pid).await?;
            stores.push(PublicationStoreSnapshot {
                binding: record.binding,
                pid: record.pid,
                created_at: record.created_at,
                status,
                releases,
                stored_chunk_hashes: None,
            });
        }

        Ok(PublicationStoreFleet {
            preferred_binding,
            reserved_state,
            stores,
        })
    }

    // Allocate one additional empty store and add it to the managed publication fleet.
    async fn create_store_for_fleet(
        fleet: &mut PublicationStoreFleet,
    ) -> Result<PublicationPlacement, InternalError> {
        let binding = match fleet.preferred_binding.clone() {
            Some(_) => Self::create_publication_store().await?,
            None => Self::create_and_activate_first_publication_store().await?,
        };
        let store_pid = store_pid_for_binding(&binding)?;
        let record = SubnetStateOps::wasm_stores()
            .into_iter()
            .find(|record| record.binding == binding)
            .ok_or_else(|| {
                InternalError::workflow(
                    InternalErrorOrigin::Workflow,
                    format!("new ws '{binding}' missing from subnet state"),
                )
            })?;

        fleet.push_store(record, config::current_subnet_default_wasm_store());
        if fleet.preferred_binding.is_none() {
            fleet.preferred_binding = Some(binding.clone());
        }
        fleet.reserved_state = SubnetStateOps::publication_store_state();

        Ok(PublicationPlacement {
            binding,
            pid: store_pid,
            action: PublicationPlacementAction::Create,
        })
    }

    // Execute one typed root-owned WasmStore publication or lifecycle admin command.
    pub async fn handle_admin(
        cmd: WasmStoreAdminCommand,
    ) -> Result<WasmStoreAdminResponse, InternalError> {
        match cmd {
            WasmStoreAdminCommand::PublishCurrentReleaseToStore { store_pid } => {
                Self::publish_current_release_set_to_store(store_pid).await?;
                Ok(WasmStoreAdminResponse::PublishedCurrentReleaseToStore { store_pid })
            }
            WasmStoreAdminCommand::PublishCurrentReleaseToCurrentStore => {
                Self::publish_current_release_set_to_current_store().await?;
                Ok(WasmStoreAdminResponse::PublishedCurrentReleaseToCurrentStore)
            }
            WasmStoreAdminCommand::SetPublicationBinding { binding } => {
                Self::set_current_publication_store_binding(binding.clone())?;
                Ok(WasmStoreAdminResponse::SetPublicationBinding { binding })
            }
            WasmStoreAdminCommand::ClearPublicationBinding => {
                Self::clear_current_publication_store_binding();
                Ok(WasmStoreAdminResponse::ClearedPublicationBinding)
            }
            WasmStoreAdminCommand::RetireDetachedBinding => {
                let binding = Self::retire_detached_publication_store_binding();
                Ok(WasmStoreAdminResponse::RetiredDetachedBinding { binding })
            }
            WasmStoreAdminCommand::PrepareRetiredStoreGc => {
                let binding = Self::prepare_retired_publication_store_for_gc().await?;
                Ok(WasmStoreAdminResponse::PreparedRetiredStoreGc { binding })
            }
            WasmStoreAdminCommand::BeginRetiredStoreGc => {
                let binding = Self::begin_retired_publication_store_gc().await?;
                Ok(WasmStoreAdminResponse::BeganRetiredStoreGc { binding })
            }
            WasmStoreAdminCommand::CompleteRetiredStoreGc => {
                let binding = Self::complete_retired_publication_store_gc().await?;
                Ok(WasmStoreAdminResponse::CompletedRetiredStoreGc { binding })
            }
            WasmStoreAdminCommand::FinalizeRetiredBinding => {
                let result = Self::finalize_retired_publication_store_binding()
                    .await?
                    .map(|(binding, store_pid)| WasmStoreFinalizedStoreResponse {
                        binding,
                        store_pid,
                    });
                Ok(WasmStoreAdminResponse::FinalizedRetiredBinding { result })
            }
            WasmStoreAdminCommand::DeleteFinalizedStore { binding, store_pid } => {
                Self::delete_finalized_publication_store(binding.clone(), store_pid).await?;
                Ok(WasmStoreAdminResponse::DeletedFinalizedStore { binding, store_pid })
            }
        }
    }

    // Format one publication-state binding slot for structured transition logs.
    fn binding_slot(slot: Option<&WasmStoreBinding>) -> String {
        slot.map_or_else(|| "-".to_string(), std::string::ToString::to_string)
    }

    // Return true when a binding is already reserved for detached or retired lifecycle state.
    pub(super) fn binding_is_reserved_for_publication(
        state: &PublicationStoreStateRecord,
        binding: &WasmStoreBinding,
    ) -> bool {
        state.detached_binding.as_ref() == Some(binding)
            || state.retired_binding.as_ref() == Some(binding)
    }

    // Reject explicit publication selection when the binding is already detached or retired.
    fn ensure_binding_is_selectable_for_publication(
        state: &PublicationStoreStateRecord,
        binding: &WasmStoreBinding,
    ) -> Result<(), InternalError> {
        if Self::binding_is_reserved_for_publication(state, binding) {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!("ws binding '{binding}' is detached/retired"),
            ));
        }

        Ok(())
    }

    // Emit one structured publication-binding transition record after root-owned state changes.
    fn log_publication_state_transition(
        transition_kind: &str,
        previous: &PublicationStoreStateRecord,
        current: &PublicationStoreStateRecord,
        changed_at: u64,
    ) {
        if previous == current {
            return;
        }

        log!(
            Topic::Wasm,
            Info,
            "ws.transition kind={} gen={} at={} old_a={} old_d={} old_r={} new_a={} new_d={} new_r={}",
            transition_kind,
            current.generation,
            changed_at,
            Self::binding_slot(previous.active_binding.as_ref()),
            Self::binding_slot(previous.detached_binding.as_ref()),
            Self::binding_slot(previous.retired_binding.as_ref()),
            Self::binding_slot(current.active_binding.as_ref()),
            Self::binding_slot(current.detached_binding.as_ref()),
            Self::binding_slot(current.retired_binding.as_ref()),
        );
    }

    // Reject rollover when it would overwrite an older retired store.
    pub(super) fn ensure_retired_binding_slot_available_for_promotion() -> Result<(), InternalError>
    {
        let state = SubnetStateOps::publication_store_state();

        if state.detached_binding.is_some() && state.retired_binding.is_some() {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                "ws rollover blocked: retired slot occupied".to_string(),
            ));
        }

        Ok(())
    }

    // Reject explicit retirement when one retired store is already pending cleanup.
    pub(super) fn ensure_retired_binding_slot_available_for_retirement() -> Result<(), InternalError>
    {
        let state = SubnetStateOps::publication_store_state();

        if state.retired_binding.is_some() {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                "ws retirement blocked: retired slot occupied".to_string(),
            ));
        }

        Ok(())
    }

    // Mark the current retired publication store as prepared for store-local GC execution.
    pub async fn prepare_retired_publication_store_for_gc()
    -> Result<Option<WasmStoreBinding>, InternalError> {
        let state = SubnetStateOps::publication_store_state();
        let Some(retired_binding) = state.retired_binding.clone() else {
            return Ok(None);
        };

        let store_pid = store_pid_for_binding(&retired_binding)?;
        store_prepare_gc(store_pid).await?;
        let _ = SubnetStateOps::transition_wasm_store_gc(
            &retired_binding,
            WasmStoreGcMode::Prepared,
            IcOps::now_secs(),
        );

        log!(
            Topic::Wasm,
            Ok,
            "ws gc prepared {} gen={} retired_at={}",
            retired_binding,
            state.generation,
            state.retired_at
        );

        Ok(Some(retired_binding))
    }

    // Mark the current retired publication store as actively executing store-local GC.
    pub async fn begin_retired_publication_store_gc()
    -> Result<Option<WasmStoreBinding>, InternalError> {
        let state = SubnetStateOps::publication_store_state();
        let Some(retired_binding) = state.retired_binding.clone() else {
            return Ok(None);
        };

        let store_pid = store_pid_for_binding(&retired_binding)?;
        store_begin_gc(store_pid).await?;
        let _ = SubnetStateOps::transition_wasm_store_gc(
            &retired_binding,
            WasmStoreGcMode::InProgress,
            IcOps::now_secs(),
        );

        log!(
            Topic::Wasm,
            Ok,
            "ws gc begin {} gen={} retired_at={}",
            retired_binding,
            state.generation,
            state.retired_at
        );

        Ok(Some(retired_binding))
    }

    // Mark the current retired publication store as having completed its local GC pass.
    pub async fn complete_retired_publication_store_gc()
    -> Result<Option<WasmStoreBinding>, InternalError> {
        let state = SubnetStateOps::publication_store_state();
        let Some(retired_binding) = state.retired_binding.clone() else {
            return Ok(None);
        };

        let store_pid = store_pid_for_binding(&retired_binding)?;
        store_complete_gc(store_pid).await?;
        let _ = SubnetStateOps::transition_wasm_store_gc(
            &retired_binding,
            WasmStoreGcMode::Complete,
            IcOps::now_secs(),
        );

        log!(
            Topic::Wasm,
            Ok,
            "ws gc complete {} gen={} retired_at={}",
            retired_binding,
            state.generation,
            state.retired_at
        );

        Ok(Some(retired_binding))
    }

    // Finalize the current retired publication store after its local GC run has completed.
    pub async fn finalize_retired_publication_store_binding()
    -> Result<Option<(WasmStoreBinding, Principal)>, InternalError> {
        let state = SubnetStateOps::publication_store_state();
        let Some(retired_binding) = state.retired_binding.clone() else {
            return Ok(None);
        };

        let store_pid = store_pid_for_binding(&retired_binding)?;
        let store = store_status(store_pid).await?;

        if store.gc.mode != WasmStoreGcMode::Complete {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!(
                    "retired ws '{}' not ready for finalize; gc={:?}",
                    retired_binding, store.gc.mode
                ),
            ));
        }

        let changed_at = IcOps::now_secs();
        let previous = SubnetStateOps::publication_store_state();
        let finalized = SubnetStateOps::finalize_retired_publication_store_binding(changed_at)
            .map(|binding| (binding, store_pid));

        if let Some((binding, finalized_store_pid)) = finalized.as_ref() {
            let current = SubnetStateOps::publication_store_state();
            Self::log_publication_state_transition(
                "finalize_retired_binding",
                &previous,
                &current,
                changed_at,
            );
            log!(
                Topic::Wasm,
                Ok,
                "ws finalized {} ({})",
                binding,
                finalized_store_pid
            );
        }

        Ok(finalized)
    }

    // Delete one previously finalized retired publication store after local GC and root finalization complete.
    pub async fn delete_finalized_publication_store(
        binding: WasmStoreBinding,
        store_pid: Principal,
    ) -> Result<(), InternalError> {
        let state = SubnetStateOps::publication_store_state();

        if state.active_binding.as_ref() == Some(&binding)
            || state.detached_binding.as_ref() == Some(&binding)
            || state.retired_binding.as_ref() == Some(&binding)
        {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!("ws '{binding}' is still referenced"),
            ));
        }

        let store = store_status(store_pid).await?;

        if store.gc.mode != WasmStoreGcMode::Complete {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!(
                    "finalized ws '{}' not ready for delete; gc={:?}",
                    binding, store.gc.mode
                ),
            ));
        }

        if store.occupied_store_bytes != 0 || store.template_count != 0 || store.release_count != 0
        {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!(
                    "finalized ws '{}' not empty after gc; bytes={} templates={} releases={}",
                    binding, store.occupied_store_bytes, store.template_count, store.release_count
                ),
            ));
        }

        ProvisionWorkflow::uninstall_and_delete_canister(store_pid).await?;
        let _ = SubnetStateOps::remove_wasm_store(&binding);

        log!(Topic::Wasm, Ok, "ws deleted {} ({})", binding, store_pid);

        Ok(())
    }

    // Move the current detached publication binding into retired state.
    pub fn retire_detached_publication_store_binding() -> Option<WasmStoreBinding> {
        if let Err(err) = Self::ensure_retired_binding_slot_available_for_retirement() {
            log!(Topic::Wasm, Warn, "{err}");
            return None;
        }

        let changed_at = IcOps::now_secs();
        let previous = SubnetStateOps::publication_store_state();
        let retired = SubnetStateOps::retire_detached_publication_store_binding(changed_at);

        if let Some(binding) = retired.as_ref() {
            let current = SubnetStateOps::publication_store_state();
            Self::log_publication_state_transition(
                "retire_detached_binding",
                &previous,
                &current,
                changed_at,
            );
            log!(Topic::Wasm, Ok, "ws retired {}", binding);
        }

        retired
    }

    // Persist one explicit publication binding after validating that it exists in subnet config.
    pub fn set_current_publication_store_binding(
        binding: WasmStoreBinding,
    ) -> Result<(), InternalError> {
        let _ = store_pid_for_binding(&binding)?;
        Self::ensure_retired_binding_slot_available_for_promotion()?;
        let previous = SubnetStateOps::publication_store_state();
        Self::ensure_binding_is_selectable_for_publication(&previous, &binding)?;
        let changed_at = IcOps::now_secs();

        if SubnetStateOps::activate_publication_store_binding(binding, changed_at) {
            let current = SubnetStateOps::publication_store_state();
            Self::log_publication_state_transition(
                "pin_publication_binding",
                &previous,
                &current,
                changed_at,
            );
        }

        Ok(())
    }

    // Clear the explicit publication binding and fall back to configured store selection.
    pub fn clear_current_publication_store_binding() {
        if let Err(err) = Self::ensure_retired_binding_slot_available_for_promotion() {
            log!(Topic::Wasm, Warn, "{err}");
            return;
        }

        let changed_at = IcOps::now_secs();
        let previous = SubnetStateOps::publication_store_state();

        if SubnetStateOps::clear_publication_store_binding(changed_at) {
            let current = SubnetStateOps::publication_store_state();
            Self::log_publication_state_transition(
                "clear_publication_binding",
                &previous,
                &current,
                changed_at,
            );
        }
    }

    // Return the oldest known runtime-managed wasm-store binding for this subnet.
    fn oldest_registered_store_binding() -> Option<WasmStoreBinding> {
        SubnetStateOps::wasm_stores()
            .into_iter()
            .min_by(|left, right| left.created_at.cmp(&right.created_at))
            .map(|record| record.binding)
    }

    // Clear one stale publication binding and fall back to the oldest known runtime store.
    fn clear_stale_publication_binding(
        binding: WasmStoreBinding,
    ) -> Result<WasmStoreBinding, InternalError> {
        log!(Topic::Wasm, Warn, "ws clear stale binding {}", binding);
        let changed_at = IcOps::now_secs();
        Self::ensure_retired_binding_slot_available_for_promotion()?;
        let previous = SubnetStateOps::publication_store_state();
        let _ = SubnetStateOps::clear_publication_store_binding(changed_at);
        let current = SubnetStateOps::publication_store_state();
        Self::log_publication_state_transition(
            "clear_stale_publication_binding",
            &previous,
            &current,
            changed_at,
        );

        Self::oldest_registered_store_binding().ok_or_else(|| {
            InternalError::workflow(
                InternalErrorOrigin::Workflow,
                "no registered wasm stores after clearing stale publication binding",
            )
        })
    }

    // Create the first runtime-managed store and promote it into the active publication slot.
    async fn create_and_activate_first_publication_store() -> Result<WasmStoreBinding, InternalError>
    {
        let binding = Self::create_publication_store().await?;
        Self::ensure_retired_binding_slot_available_for_promotion()?;
        let changed_at = IcOps::now_secs();
        let previous = SubnetStateOps::publication_store_state();
        let _ = SubnetStateOps::activate_publication_store_binding(binding.clone(), changed_at);
        let current = SubnetStateOps::publication_store_state();
        Self::log_publication_state_transition(
            "activate_first_publication_binding",
            &previous,
            &current,
            changed_at,
        );

        Ok(binding)
    }

    // Return the deterministic approved manifests that still belong to the configured managed fleet.
    fn managed_release_manifests() -> Result<Vec<TemplateManifestResponse>, InternalError> {
        let roles = config::current_subnet_managed_release_roles()?;

        Ok(
            TemplateManifestOps::approved_manifests_for_roles_response(&roles)
                .into_iter()
                .filter(|manifest| manifest.chunking_mode == TemplateChunkingMode::Chunked)
                .collect(),
        )
    }

    // Deprecate any currently approved managed release that no longer belongs to the configured fleet.
    pub fn prune_unconfigured_managed_releases() -> Result<usize, InternalError> {
        let roles = config::current_subnet_managed_release_roles()?;
        Ok(TemplateManifestOps::deprecate_approved_roles_not_in(&roles))
    }

    // Return the exact fleet stores that already carry one approved release.
    fn exact_release_candidates<'a>(
        fleet: &'a PublicationStoreFleet,
        manifest: &TemplateManifestResponse,
    ) -> Vec<&'a PublicationStoreSnapshot> {
        let mut stores = fleet
            .stores
            .iter()
            .filter(|store| store.has_exact_release(manifest))
            .collect::<Vec<_>>();

        stores.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then(left.binding.cmp(&right.binding))
        });

        stores
    }

    // Reconcile the approved release for one role against the exact matching fleet entries.
    fn reconciled_binding_for_manifest(
        fleet: &PublicationStoreFleet,
        manifest: &TemplateManifestResponse,
    ) -> Result<WasmStoreBinding, InternalError> {
        let candidates = Self::exact_release_candidates(fleet, manifest);

        if candidates.is_empty() {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!(
                    "fleet import missing exact release for role '{}': expected {}@{} on {}",
                    manifest.role, manifest.template_id, manifest.version, manifest.store_binding
                ),
            ));
        }

        if candidates
            .iter()
            .any(|store| store.binding == manifest.store_binding)
        {
            return Ok(manifest.store_binding.clone());
        }

        if let Some(binding) = fleet.preferred_binding.as_ref()
            && candidates.iter().any(|store| &store.binding == binding)
        {
            return Ok(binding.clone());
        }

        Ok(candidates[0].binding.clone())
    }

    // Build the source label used in placement logs for one approved manifest.
    fn release_label(manifest: &TemplateManifestResponse) -> String {
        format!("{}@{}", manifest.template_id, manifest.version)
    }

    // Resolve the source store pid for one manifest-backed release, if it is store-backed.
    fn source_store_pid_for_manifest(
        manifest: &TemplateManifestResponse,
    ) -> Result<Option<Principal>, InternalError> {
        if manifest.store_binding == WASM_STORE_BOOTSTRAP_BINDING {
            Ok(None)
        } else {
            store_pid_for_binding(&manifest.store_binding).map(Some)
        }
    }

    // Resolve deterministic chunk-set metadata for one manifest from its authoritative source.
    async fn source_chunk_set_info_for_manifest(
        manifest: &TemplateManifestResponse,
    ) -> Result<TemplateChunkSetInfoResponse, InternalError> {
        match Self::source_store_pid_for_manifest(manifest)? {
            Some(store_pid) => {
                store_chunk_set_info(store_pid, &manifest.template_id, &manifest.version).await
            }
            None => TemplateChunkedOps::chunk_set_info_response(
                &manifest.template_id,
                &manifest.version,
            ),
        }
    }

    // Resolve one deterministic chunk for one manifest from its authoritative source.
    async fn source_chunk_for_manifest(
        manifest: &TemplateManifestResponse,
        chunk_index: u32,
    ) -> Result<Vec<u8>, InternalError> {
        match Self::source_store_pid_for_manifest(manifest)? {
            Some(store_pid) => {
                store_chunk(
                    store_pid,
                    &manifest.template_id,
                    &manifest.version,
                    chunk_index,
                )
                .await
            }
            None => local_chunk(&manifest.template_id, &manifest.version, chunk_index),
        }
    }

    // Return true when one failed store call represents store-capacity exhaustion.
    fn is_store_capacity_exceeded(err: &InternalError) -> bool {
        err.public_error().is_some_and(|public| {
            public
                .message
                .contains(Self::WASM_STORE_CAPACITY_EXCEEDED_MESSAGE)
        }) || err
            .to_string()
            .contains(Self::WASM_STORE_CAPACITY_EXCEEDED_MESSAGE)
    }

    // Mirror one approved manifest into root-owned state without mutating a live store.
    fn mirror_manifest_to_root_state(
        target_store_binding: WasmStoreBinding,
        manifest: &TemplateManifestResponse,
    ) {
        TemplateManifestOps::replace_approved_from_input(TemplateManifestInput {
            template_id: manifest.template_id.clone(),
            role: manifest.role.clone(),
            version: manifest.version.clone(),
            payload_hash: manifest.payload_hash.clone(),
            payload_size_bytes: manifest.payload_size_bytes,
            store_binding: target_store_binding,
            chunking_mode: TemplateChunkingMode::Chunked,
            manifest_state: TemplateManifestState::Approved,
            approved_at: Some(IcOps::now_secs()),
            created_at: manifest.created_at,
        });
    }

    // Resolve one automatic managed placement from the live fleet snapshot.
    async fn resolve_managed_publication_placement(
        fleet: &mut PublicationStoreFleet,
        manifest: &TemplateManifestResponse,
    ) -> Result<PublicationPlacement, InternalError> {
        if let Some(placement) = fleet.select_existing_store_for_release(manifest)? {
            return Ok(placement);
        }

        let store_config = config::current_subnet_default_wasm_store();
        if manifest.payload_size_bytes > store_config.max_store_bytes() {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!(
                    "release {} exceeds empty wasm store capacity: bytes {} > {}",
                    Self::release_label(manifest),
                    manifest.payload_size_bytes,
                    store_config.max_store_bytes()
                ),
            ));
        }

        let created = Self::create_store_for_fleet(fleet).await?;
        let created_store = fleet
            .stores
            .iter()
            .find(|store| store.binding == created.binding)
            .ok_or_else(|| {
                InternalError::workflow(
                    InternalErrorOrigin::Workflow,
                    format!("new ws '{}' missing from fleet snapshot", created.binding),
                )
            })?;

        if !created_store.can_accept_release(manifest) {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!(
                    "release {} does not fit empty store {}",
                    Self::release_label(manifest),
                    created.binding
                ),
            ));
        }

        Ok(created)
    }

    // Publish one approved manifest into the target store from its authoritative source.
    async fn publish_manifest_to_store(
        target_store: &mut PublicationStoreSnapshot,
        manifest: TemplateManifestResponse,
    ) -> Result<(), InternalError> {
        let info = Self::source_chunk_set_info_for_manifest(&manifest).await?;
        let chunk_hashes = info.chunk_hashes.clone();
        target_store.ensure_stored_chunk_hashes().await?;

        let _: TemplateChunkSetInfoResponse = super::call_store_result(
            target_store.pid,
            cp_core::protocol::CANIC_WASM_STORE_PREPARE,
            (TemplateChunkSetPrepareInput {
                template_id: manifest.template_id.clone(),
                version: manifest.version.clone(),
                payload_hash: manifest.payload_hash.clone(),
                payload_size_bytes: manifest.payload_size_bytes,
                chunk_hashes: chunk_hashes.clone(),
            },),
        )
        .await?;
        canic_core::perf!("publish_prepare_store");

        for (chunk_index, expected_hash) in chunk_hashes.iter().cloned().enumerate() {
            let chunk_index = u32::try_from(chunk_index).map_err(|_| {
                InternalError::workflow(
                    InternalErrorOrigin::Workflow,
                    format!(
                        "template '{}' exceeds chunk index bounds",
                        manifest.template_id
                    ),
                )
            })?;
            let already_uploaded = target_store
                .stored_chunk_hashes
                .as_ref()
                .is_some_and(|hashes| hashes.contains(&expected_hash));
            let bytes = Self::source_chunk_for_manifest(&manifest, chunk_index).await?;

            super::call_store_result::<(), _>(
                target_store.pid,
                cp_core::protocol::CANIC_WASM_STORE_PUBLISH_CHUNK,
                (TemplateChunkInputRef {
                    template_id: &manifest.template_id,
                    version: &manifest.version,
                    chunk_index,
                    bytes: &bytes,
                },),
            )
            .await?;
            canic_core::perf!("publish_push_store_chunk");

            if !already_uploaded {
                let uploaded_hash =
                    cp_core::ops::ic::mgmt::MgmtOps::upload_chunk(target_store.pid, bytes).await?;
                if uploaded_hash != expected_hash {
                    return Err(InternalError::workflow(
                        InternalErrorOrigin::Workflow,
                        format!(
                            "template '{}' chunk {} hash mismatch for {}",
                            manifest.template_id, chunk_index, target_store.pid
                        ),
                    ));
                }
                target_store
                    .stored_chunk_hashes
                    .as_mut()
                    .expect("stored chunk hashes must be initialized")
                    .insert(expected_hash);
            }
        }

        Self::promote_manifest_to_target_store(
            target_store.pid,
            target_store.binding.clone(),
            TemplateManifestInput {
                template_id: manifest.template_id.clone(),
                role: manifest.role.clone(),
                version: manifest.version.clone(),
                payload_hash: manifest.payload_hash.clone(),
                payload_size_bytes: manifest.payload_size_bytes,
                store_binding: manifest.store_binding,
                chunking_mode: TemplateChunkingMode::Chunked,
                manifest_state: TemplateManifestState::Approved,
                approved_at: Some(IcOps::now_secs()),
                created_at: manifest.created_at,
            },
        )
        .await?;
        canic_core::perf!("publish_promote_manifest");

        log!(
            Topic::Wasm,
            Ok,
            "tpl.publish {} -> {}@{} (store={}, chunks={})",
            manifest.role,
            manifest.template_id,
            manifest.version,
            target_store.pid,
            chunk_hashes.len()
        );

        Ok(())
    }

    // Publish one approved manifest through the managed store fleet or reuse an exact existing release.
    async fn publish_manifest_to_managed_fleet(
        fleet: &mut PublicationStoreFleet,
        manifest: TemplateManifestResponse,
    ) -> Result<(), InternalError> {
        let release_label = Self::release_label(&manifest);
        let placement = Self::resolve_managed_publication_placement(fleet, &manifest).await?;

        match placement.action {
            PublicationPlacementAction::Reuse => {
                Self::mirror_manifest_to_root_state(placement.binding.clone(), &manifest);
                log!(
                    Topic::Wasm,
                    Info,
                    "ws reuse {} on {} ({})",
                    release_label,
                    placement.binding,
                    placement.pid
                );
            }
            PublicationPlacementAction::Publish | PublicationPlacementAction::Create => {
                let action_label = if placement.action == PublicationPlacementAction::Create {
                    "create"
                } else {
                    "publish"
                };
                let store_index = fleet
                    .store_index_for_binding(&placement.binding)
                    .ok_or_else(|| {
                        InternalError::workflow(
                            InternalErrorOrigin::Workflow,
                            format!("ws '{}' missing from fleet snapshot", placement.binding),
                        )
                    })?;

                let publish_result = {
                    let target_store = &mut fleet.stores[store_index];
                    Self::publish_manifest_to_store(target_store, manifest.clone()).await
                };

                match publish_result {
                    Ok(()) => {
                        log!(
                            Topic::Wasm,
                            Info,
                            "ws place {} mode={} binding={} pid={}",
                            release_label,
                            action_label,
                            placement.binding,
                            placement.pid
                        );
                    }
                    Err(err) if Self::is_store_capacity_exceeded(&err) => {
                        if placement.action == PublicationPlacementAction::Create {
                            return Err(err);
                        }

                        let retry = Self::create_store_for_fleet(fleet).await?;
                        let retry_index = fleet
                            .store_index_for_binding(&retry.binding)
                            .ok_or_else(|| {
                                InternalError::workflow(
                                    InternalErrorOrigin::Workflow,
                                    format!("ws '{}' missing from fleet snapshot", retry.binding),
                                )
                            })?;
                        {
                            let target_store = &mut fleet.stores[retry_index];
                            Self::publish_manifest_to_store(target_store, manifest.clone()).await?;
                        }
                        log!(
                            Topic::Wasm,
                            Warn,
                            "ws rollover {} from {} to {}",
                            release_label,
                            placement.binding,
                            retry.binding
                        );
                        fleet.record_placement(&retry.binding, &manifest);
                        return Ok(());
                    }
                    Err(err) => return Err(err),
                }
            }
        }

        fleet.record_placement(&placement.binding, &manifest);
        Ok(())
    }

    // Publish all root-local staged releases into the current subnet's selected wasm store.
    pub async fn publish_staged_release_set_to_current_store() -> Result<(), InternalError> {
        let manifests = Self::managed_release_manifests()?
            .into_iter()
            .filter(|manifest| manifest.store_binding == WASM_STORE_BOOTSTRAP_BINDING)
            .collect::<Vec<_>>();

        for manifest in &manifests {
            TemplateChunkedOps::validate_staged_release(manifest)?;
        }

        let mut fleet = Self::snapshot_publication_store_fleet().await?;
        for manifest in manifests {
            Self::publish_manifest_to_managed_fleet(&mut fleet, manifest).await?;
        }

        Ok(())
    }

    // Publish the current release set from the current default store into one subnet-local wasm store.
    pub async fn publish_current_release_set_to_store(
        target_store_pid: Principal,
    ) -> Result<(), InternalError> {
        let target_store_binding = store_binding_for_pid(target_store_pid)?;
        let target_status = store_status(target_store_pid).await?;
        let target_catalog = store_catalog(target_store_pid).await?;
        let mut target_store = PublicationStoreSnapshot {
            binding: target_store_binding.clone(),
            pid: target_store_pid,
            created_at: IcOps::now_secs(),
            status: target_status,
            releases: target_catalog,
            stored_chunk_hashes: None,
        };

        for manifest in Self::managed_release_manifests()? {
            if target_store.has_exact_release(&manifest) {
                Self::mirror_manifest_to_root_state(target_store_binding.clone(), &manifest);
                continue;
            }

            if !target_store.can_accept_release(&manifest) {
                return Err(InternalError::workflow(
                    InternalErrorOrigin::Workflow,
                    format!(
                        "target ws '{}' cannot fit {}",
                        target_store_binding,
                        Self::release_label(&manifest)
                    ),
                ));
            }

            Self::publish_manifest_to_store(&mut target_store, manifest.clone()).await?;
            target_store.record_release(&manifest);
        }

        Ok(())
    }

    // Reconcile root-owned approved manifest bindings against exact releases present in the fleet.
    pub async fn import_current_store_catalog() -> Result<(), InternalError> {
        let fleet = Self::snapshot_publication_store_fleet().await?;
        for manifest in Self::managed_release_manifests()? {
            let binding = Self::reconciled_binding_for_manifest(&fleet, &manifest)?;
            TemplateManifestOps::replace_approved_from_input(TemplateManifestInput {
                template_id: manifest.template_id,
                role: manifest.role,
                version: manifest.version,
                payload_hash: manifest.payload_hash,
                payload_size_bytes: manifest.payload_size_bytes,
                store_binding: binding,
                chunking_mode: manifest.chunking_mode,
                manifest_state: manifest.manifest_state,
                approved_at: manifest.approved_at,
                created_at: manifest.created_at,
            });
        }

        Ok(())
    }

    /// Publish the current managed release set into the managed subnet-local store fleet.
    pub async fn publish_current_release_set_to_current_store() -> Result<(), InternalError> {
        let mut fleet = Self::snapshot_publication_store_fleet().await?;

        for manifest in Self::managed_release_manifests()? {
            Self::publish_manifest_to_managed_fleet(&mut fleet, manifest).await?;
        }

        Ok(())
    }

    // Stage one approved manifest into the target store and mirror it into root-owned state.
    async fn promote_manifest_to_target_store(
        target_store_pid: Principal,
        target_store_binding: WasmStoreBinding,
        manifest: TemplateManifestInput,
    ) -> Result<(), InternalError> {
        store_stage_manifest(
            target_store_pid,
            TemplateManifestInput {
                store_binding: target_store_binding.clone(),
                ..manifest.clone()
            },
        )
        .await?;

        TemplateManifestOps::replace_approved_from_input(TemplateManifestInput {
            store_binding: target_store_binding,
            ..manifest
        });

        Ok(())
    }
}
