use super::call_store_result;
use super::{WASM_STORE_BOOTSTRAP_BINDING, WASM_STORE_ROLE, embedded_template_id, local_chunks};
use super::{
    store_begin_gc, store_binding_for_pid, store_catalog, store_chunk_set_info, store_chunks,
    store_complete_gc, store_pid_for_binding, store_prepare_gc, store_status,
};
use crate::{
    InternalError, InternalErrorOrigin, VERSION,
    cdk::types::{Principal, WasmModule},
    dto::template::{
        TemplateChunkInput, TemplateChunkSetInfoResponse, TemplateChunkSetPrepareInput,
        TemplateManifestInput, WasmStoreAdminCommand, WasmStoreAdminResponse,
        WasmStoreCatalogEntryResponse, WasmStoreFinalizedStoreResponse,
    },
    ids::{
        CanisterRole, TemplateChunkingMode, TemplateManifestState, TemplateVersion,
        WasmStoreBinding, WasmStoreGcMode,
    },
    log,
    log::Topic,
    ops::{
        config::ConfigOps,
        ic::IcOps,
        ic::mgmt::MgmtOps,
        runtime::template::EmbeddedTemplatePayloadOps,
        storage::{
            registry::subnet::SubnetRegistryOps, state::subnet::SubnetStateOps,
            template::TemplateManifestOps,
        },
    },
    protocol,
    storage::stable::state::subnet::PublicationStoreStateRecord,
    workflow::{
        canister_lifecycle::{CanisterLifecycleEvent, CanisterLifecycleWorkflow},
        ic::provision::ProvisionWorkflow,
    },
};
use std::collections::BTreeSet;

///
/// WasmStorePublicationWorkflow
///

pub struct WasmStorePublicationWorkflow;

impl WasmStorePublicationWorkflow {
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
    fn binding_is_reserved_for_publication(
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
    fn ensure_retired_binding_slot_available_for_promotion() -> Result<(), InternalError> {
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
    fn ensure_retired_binding_slot_available_for_retirement() -> Result<(), InternalError> {
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

    // Promote one existing or newly-created runtime store into the active publication slot.
    fn promote_publication_binding(
        binding: WasmStoreBinding,
        transition_kind: &str,
    ) -> Result<(), InternalError> {
        let changed_at = IcOps::now_secs();
        Self::ensure_retired_binding_slot_available_for_promotion()?;
        let previous = SubnetStateOps::publication_store_state();
        let promoted = SubnetStateOps::activate_publication_store_binding(binding, changed_at);

        if promoted {
            let current = SubnetStateOps::publication_store_state();
            Self::log_publication_state_transition(
                transition_kind,
                &previous,
                &current,
                changed_at,
            );
        }

        Ok(())
    }

    // Pick the preferred active publication binding before capacity checks run.
    async fn preferred_publication_binding() -> Result<WasmStoreBinding, InternalError> {
        match SubnetStateOps::publication_store_binding() {
            Some(binding) if store_pid_for_binding(&binding).is_ok() => Ok(binding),
            Some(binding) => Self::clear_stale_publication_binding(binding),
            None => {
                if let Some(record) = Self::oldest_registered_store_binding() {
                    Ok(record)
                } else {
                    Self::create_and_activate_first_publication_store().await
                }
            }
        }
    }

    // Return true when a bound store is still outside its reserved publication headroom.
    async fn store_binding_accepts_publication(
        store_binding: &WasmStoreBinding,
    ) -> Result<bool, InternalError> {
        let store_pid = store_pid_for_binding(store_binding)?;
        let status = store_status(store_pid).await?;

        Ok(!status.within_headroom)
    }

    // Resolve the deterministic current publication target for this subnet.
    async fn resolve_current_publication_store_binding() -> Result<WasmStoreBinding, InternalError>
    {
        Self::sync_registered_wasm_store_inventory();

        let preferred_binding = Self::preferred_publication_binding().await?;

        let current_state = SubnetStateOps::publication_store_state();
        if Self::binding_is_reserved_for_publication(&current_state, &preferred_binding) {
            log!(
                Topic::Wasm,
                Info,
                "ws skip reserved binding {}",
                preferred_binding
            );
        } else if Self::store_binding_accepts_publication(&preferred_binding).await? {
            return Ok(preferred_binding);
        }

        for candidate in SubnetStateOps::wasm_stores()
            .into_iter()
            .map(|record| record.binding)
        {
            if candidate == preferred_binding {
                continue;
            }

            let current_state = SubnetStateOps::publication_store_state();
            if Self::binding_is_reserved_for_publication(&current_state, &candidate) {
                log!(Topic::Wasm, Info, "ws skip reserved binding {}", candidate);
                continue;
            }

            if Self::store_binding_accepts_publication(&candidate).await? {
                log!(
                    Topic::Wasm,
                    Info,
                    "ws preferred {} within headroom, using {}",
                    preferred_binding,
                    candidate
                );
                Self::promote_publication_binding(
                    candidate.clone(),
                    "promote_publication_binding",
                )?;
                return Ok(candidate);
            }
        }

        let binding = Self::create_publication_store().await?;
        Self::promote_publication_binding(binding.clone(), "promote_new_publication_binding")?;
        log!(
            Topic::Wasm,
            Info,
            "ws preferred {} within headroom, created {}",
            preferred_binding,
            binding
        );
        Ok(binding)
    }

    // Fail closed when a selected publication target is already inside reserved headroom.
    async fn ensure_store_accepts_publication(
        store_binding: &WasmStoreBinding,
        store_pid: Principal,
    ) -> Result<(), InternalError> {
        let status = store_status(store_pid).await?;

        if status.within_headroom {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!("ws binding '{store_binding}' at {store_pid} is within headroom",),
            ));
        }

        Ok(())
    }

    // Seed approved manifests and template-keyed embedded payloads from the current release table.
    pub fn import_embedded_release_set(wasms: &'static [(CanisterRole, &[u8])]) {
        if !cfg!(target_arch = "wasm32") {
            return;
        }

        let version = TemplateVersion::new(VERSION);
        let now_secs = IcOps::now_secs();

        for (role, bytes) in wasms {
            let wasm = WasmModule::new(bytes);
            let payload_hash = wasm.module_hash();
            let template_id = embedded_template_id(role);

            let input = TemplateManifestInput {
                template_id: template_id.clone(),
                role: role.clone(),
                version: version.clone(),
                payload_hash,
                payload_size_bytes: wasm.len() as u64,
                store_binding: WASM_STORE_BOOTSTRAP_BINDING,
                chunking_mode: TemplateChunkingMode::Inline,
                manifest_state: TemplateManifestState::Approved,
                approved_at: Some(now_secs),
                created_at: now_secs,
            };

            TemplateManifestOps::replace_approved_from_input(input);
            EmbeddedTemplatePayloadOps::import(template_id.clone(), bytes);

            crate::log!(
                crate::log::Topic::Wasm,
                Info,
                "tpl.import {} -> {} ({} bytes)",
                role,
                template_id,
                wasm.len()
            );
        }
    }

    // Import one store catalog into root-owned manifest state for one selected store.
    pub fn import_store_catalog(
        binding: WasmStoreBinding,
        entries: Vec<WasmStoreCatalogEntryResponse>,
    ) {
        let now_secs = IcOps::now_secs();

        for entry in entries {
            TemplateManifestOps::replace_approved_from_input(TemplateManifestInput {
                template_id: entry.template_id,
                role: entry.role,
                version: entry.version,
                payload_hash: entry.payload_hash,
                payload_size_bytes: entry.payload_size_bytes,
                store_binding: binding.clone(),
                chunking_mode: TemplateChunkingMode::Chunked,
                manifest_state: TemplateManifestState::Approved,
                approved_at: Some(now_secs),
                created_at: now_secs,
            });
        }
    }

    // Publish all root-local staged releases into the current subnet's selected wasm store.
    pub async fn publish_staged_release_set_to_current_store() -> Result<(), InternalError> {
        let target_store_binding = Self::resolve_current_publication_store_binding().await?;
        let target_store_pid = store_pid_for_binding(&target_store_binding)?;
        Self::ensure_store_accepts_publication(&target_store_binding, target_store_pid).await?;
        let manifests = TemplateManifestOps::approved_manifests_response()
            .into_iter()
            .filter(|manifest| {
                manifest.role != WASM_STORE_ROLE
                    && manifest.store_binding == WASM_STORE_BOOTSTRAP_BINDING
                    && manifest.chunking_mode == TemplateChunkingMode::Chunked
            })
            .collect::<Vec<_>>();

        // Fail closed before any store writes if one staged release is incomplete.
        for manifest in &manifests {
            TemplateManifestOps::validate_staged_release(manifest)?;
        }

        for manifest in manifests {
            Self::publish_bootstrap_release_to_store(
                target_store_pid,
                target_store_binding.clone(),
                manifest,
            )
            .await?;
        }

        Ok(())
    }

    // Publish the current release set from the current default store into one subnet-local wasm store.
    pub async fn publish_current_release_set_to_store(
        target_store_pid: Principal,
    ) -> Result<(), InternalError> {
        let source_store_binding = ConfigOps::current_subnet_default_wasm_store_binding();
        let source_store_pid = store_pid_for_binding(&source_store_binding)?;
        let target_store_binding = store_binding_for_pid(target_store_pid)?;
        Self::ensure_store_accepts_publication(&target_store_binding, target_store_pid).await?;
        let entries = store_catalog(source_store_pid).await?;

        for entry in entries {
            if entry.role == WASM_STORE_ROLE {
                continue;
            }

            Self::publish_release_to_store(
                source_store_pid,
                target_store_pid,
                target_store_binding.clone(),
                entry,
            )
            .await?;
        }

        Ok(())
    }

    // Import the current default store catalog into root-owned approved manifest state.
    pub async fn import_current_store_catalog() -> Result<(), InternalError> {
        Self::sync_registered_wasm_store_inventory();
        let store_binding = ConfigOps::current_subnet_default_wasm_store_binding();
        let store_pid = store_pid_for_binding(&store_binding)?;
        let entries = store_catalog(store_pid).await?;

        Self::import_store_catalog(store_binding, entries);
        Ok(())
    }

    /// Publish the current embedded release set into the current subnet's default wasm store.
    pub async fn publish_current_release_set_to_current_store() -> Result<(), InternalError> {
        let target_store_binding = Self::resolve_current_publication_store_binding().await?;
        let target_store_pid = store_pid_for_binding(&target_store_binding)?;
        Self::ensure_store_accepts_publication(&target_store_binding, target_store_pid).await?;
        let entries = store_catalog(target_store_pid).await?;

        for entry in entries {
            if entry.role == WASM_STORE_ROLE {
                continue;
            }

            Self::publish_release_to_store(
                target_store_pid,
                target_store_pid,
                target_store_binding.clone(),
                entry,
            )
            .await?;
        }

        Ok(())
    }

    // Publish one catalog-defined release from a source store into a target store.
    async fn publish_release_to_store(
        source_store_pid: Principal,
        target_store_pid: Principal,
        target_store_binding: WasmStoreBinding,
        entry: WasmStoreCatalogEntryResponse,
    ) -> Result<(), InternalError> {
        let info =
            store_chunk_set_info(source_store_pid, &entry.template_id, &entry.version).await?;
        let chunks = store_chunks(
            source_store_pid,
            &entry.template_id,
            &entry.version,
            info.chunk_hashes.len(),
        )
        .await?;
        let chunk_hashes = info.chunk_hashes.clone();
        let existing_hashes = MgmtOps::stored_chunks(target_store_pid)
            .await?
            .into_iter()
            .collect::<BTreeSet<_>>();

        let _: TemplateChunkSetInfoResponse = call_store_result(
            target_store_pid,
            protocol::CANIC_WASM_STORE_PREPARE,
            (TemplateChunkSetPrepareInput {
                template_id: entry.template_id.clone(),
                version: entry.version.clone(),
                payload_hash: entry.payload_hash.clone(),
                payload_size_bytes: entry.payload_size_bytes,
                chunk_hashes: chunk_hashes.clone(),
            },),
        )
        .await?;

        for (chunk_index, bytes) in chunks.into_iter().enumerate() {
            let chunk_index = u32::try_from(chunk_index).map_err(|_| {
                InternalError::workflow(
                    InternalErrorOrigin::Workflow,
                    format!(
                        "template '{}' exceeds chunk index bounds",
                        entry.template_id
                    ),
                )
            })?;
            let expected_hash = chunk_hashes[chunk_index as usize].clone();

            call_store_result::<(), _>(
                target_store_pid,
                protocol::CANIC_WASM_STORE_PUBLISH_CHUNK,
                (TemplateChunkInput {
                    template_id: entry.template_id.clone(),
                    version: entry.version.clone(),
                    chunk_index,
                    bytes: bytes.clone(),
                },),
            )
            .await?;

            if !existing_hashes.contains(&expected_hash) {
                let uploaded_hash = MgmtOps::upload_chunk(target_store_pid, bytes).await?;
                if uploaded_hash != expected_hash {
                    return Err(InternalError::workflow(
                        InternalErrorOrigin::Workflow,
                        format!(
                            "template '{}' chunk {} hash mismatch for {}",
                            entry.template_id, chunk_index, target_store_pid
                        ),
                    ));
                }
            }
        }

        TemplateManifestOps::replace_approved_from_input(TemplateManifestInput {
            template_id: entry.template_id.clone(),
            role: entry.role.clone(),
            version: entry.version.clone(),
            payload_hash: entry.payload_hash.clone(),
            payload_size_bytes: entry.payload_size_bytes,
            store_binding: target_store_binding,
            chunking_mode: TemplateChunkingMode::Chunked,
            manifest_state: TemplateManifestState::Approved,
            approved_at: Some(IcOps::now_secs()),
            created_at: IcOps::now_secs(),
        });

        crate::log!(
            crate::log::Topic::Wasm,
            Info,
            "tpl.publish {} -> {} (store={}, chunks={})",
            entry.role,
            entry.template_id,
            target_store_pid,
            chunk_hashes.len()
        );

        Ok(())
    }

    // Publish one root-local staged release into a target store and promote root manifest state.
    async fn publish_bootstrap_release_to_store(
        target_store_pid: Principal,
        target_store_binding: WasmStoreBinding,
        manifest: crate::dto::template::TemplateManifestResponse,
    ) -> Result<(), InternalError> {
        let info =
            TemplateManifestOps::chunk_set_info_response(&manifest.template_id, &manifest.version)?;
        let chunks = local_chunks(
            &manifest.template_id,
            &manifest.version,
            info.chunk_hashes.len(),
        )?;
        let chunk_hashes = info.chunk_hashes.clone();
        let existing_hashes = MgmtOps::stored_chunks(target_store_pid)
            .await?
            .into_iter()
            .collect::<BTreeSet<_>>();

        let _: TemplateChunkSetInfoResponse = call_store_result(
            target_store_pid,
            protocol::CANIC_WASM_STORE_PREPARE,
            (TemplateChunkSetPrepareInput {
                template_id: manifest.template_id.clone(),
                version: manifest.version.clone(),
                payload_hash: manifest.payload_hash.clone(),
                payload_size_bytes: manifest.payload_size_bytes,
                chunk_hashes: chunk_hashes.clone(),
            },),
        )
        .await?;

        for (chunk_index, bytes) in chunks.into_iter().enumerate() {
            let chunk_index = u32::try_from(chunk_index).map_err(|_| {
                InternalError::workflow(
                    InternalErrorOrigin::Workflow,
                    format!(
                        "template '{}' exceeds chunk index bounds",
                        manifest.template_id
                    ),
                )
            })?;

            if existing_hashes.contains(&chunk_hashes[chunk_index as usize]) {
                continue;
            }

            call_store_result::<(), _>(
                target_store_pid,
                protocol::CANIC_WASM_STORE_PUBLISH_CHUNK,
                (TemplateChunkInput {
                    template_id: manifest.template_id.clone(),
                    version: manifest.version.clone(),
                    chunk_index,
                    bytes,
                },),
            )
            .await?;
        }

        TemplateManifestOps::replace_approved_from_input(TemplateManifestInput {
            template_id: manifest.template_id.clone(),
            role: manifest.role.clone(),
            version: manifest.version.clone(),
            payload_hash: manifest.payload_hash,
            payload_size_bytes: manifest.payload_size_bytes,
            store_binding: target_store_binding.clone(),
            chunking_mode: TemplateChunkingMode::Chunked,
            manifest_state: TemplateManifestState::Approved,
            approved_at: Some(IcOps::now_secs()),
            created_at: manifest.created_at,
        });

        log!(
            Topic::Wasm,
            Ok,
            "tpl.publish {} -> {} (store={}, chunks={})",
            manifest.role,
            manifest.template_id,
            target_store_pid,
            chunk_hashes.len()
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::WasmStorePublicationWorkflow;
    use crate::{
        ids::WasmStoreBinding,
        ops::storage::state::subnet::SubnetStateOps,
        storage::stable::state::subnet::{PublicationStoreStateRecord, SubnetStateRecord},
    };

    #[test]
    fn promotion_is_blocked_when_it_would_overwrite_retired_binding() {
        SubnetStateOps::import(SubnetStateRecord {
            publication_store: PublicationStoreStateRecord {
                active_binding: Some(WasmStoreBinding::new("active")),
                detached_binding: Some(WasmStoreBinding::new("detached")),
                retired_binding: Some(WasmStoreBinding::new("retired")),
                generation: 3,
                changed_at: 30,
                retired_at: 20,
            },
            wasm_stores: Vec::new(),
        });

        let err =
            WasmStorePublicationWorkflow::ensure_retired_binding_slot_available_for_promotion()
                .expect_err("promotion must fail closed while retired binding is still pending");

        assert!(err.to_string().contains("rollover blocked"));
    }

    #[test]
    fn explicit_retirement_is_blocked_when_retired_binding_already_exists() {
        SubnetStateOps::import(SubnetStateRecord {
            publication_store: PublicationStoreStateRecord {
                active_binding: Some(WasmStoreBinding::new("active")),
                detached_binding: Some(WasmStoreBinding::new("detached")),
                retired_binding: Some(WasmStoreBinding::new("retired")),
                generation: 3,
                changed_at: 30,
                retired_at: 20,
            },
            wasm_stores: Vec::new(),
        });

        let err =
            WasmStorePublicationWorkflow::ensure_retired_binding_slot_available_for_retirement()
                .expect_err("retirement must fail closed while an older retired binding exists");

        assert!(err.to_string().contains("retirement blocked"));
    }

    #[test]
    fn detached_and_retired_bindings_are_not_publication_candidates() {
        let state = PublicationStoreStateRecord {
            active_binding: Some(WasmStoreBinding::new("active")),
            detached_binding: Some(WasmStoreBinding::new("detached")),
            retired_binding: Some(WasmStoreBinding::new("retired")),
            generation: 3,
            changed_at: 30,
            retired_at: 20,
        };

        assert!(
            !WasmStorePublicationWorkflow::binding_is_reserved_for_publication(
                &state,
                &WasmStoreBinding::new("active"),
            )
        );
        assert!(
            WasmStorePublicationWorkflow::binding_is_reserved_for_publication(
                &state,
                &WasmStoreBinding::new("detached"),
            )
        );
        assert!(
            WasmStorePublicationWorkflow::binding_is_reserved_for_publication(
                &state,
                &WasmStoreBinding::new("retired"),
            )
        );
    }
}
