use super::super::super::store_pid_for_binding;
use super::super::{
    WASM_STORE_ROLE, WasmStorePublicationWorkflow,
    fleet::{PublicationPlacement, PublicationPlacementAction, PublicationStoreFleet},
};
use crate::{config, ops::storage::state::subnet::SubnetStateOps};
use canic_core::{__control_plane_core as cp_core, log, log::Topic};
use cp_core::{
    InternalError, InternalErrorOrigin,
    ops::{ic::IcOps, storage::registry::subnet::SubnetRegistryOps},
    workflow::canister_lifecycle::{CanisterLifecycleEvent, CanisterLifecycleWorkflow},
};

impl WasmStorePublicationWorkflow {
    // Create one new wasm store canister and register its runtime-managed binding.
    async fn create_publication_store() -> Result<crate::ids::WasmStoreBinding, InternalError> {
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

    // Allocate one additional empty store and add it to the managed publication fleet.
    pub(in crate::workflow::runtime::template::publication) async fn create_store_for_fleet(
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

    // Create the first runtime-managed store and promote it into the active publication slot.
    async fn create_and_activate_first_publication_store()
    -> Result<crate::ids::WasmStoreBinding, InternalError> {
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
}
