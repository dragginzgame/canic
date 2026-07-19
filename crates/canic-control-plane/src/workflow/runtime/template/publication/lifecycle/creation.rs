use super::super::super::store_pid_for_binding;
use super::super::{
    WASM_STORE_ROLE, WasmStorePublicationWorkflow,
    fleet::{PublicationPlacement, PublicationPlacementAction, PublicationStoreFleet},
};
use crate::{
    config,
    ops::storage::state::subnet::SubnetStateOps,
    workflow::{deployment, runtime::template::publication::error::PublicationWorkflowError},
};
use canic_core::control_plane_support::{
    error::{InternalError, InternalErrorOrigin},
    ops::{cost_guard::CostGuardPermit, ic::IcOps, storage::registry::subnet::SubnetRegistryOps},
};
use canic_core::{log, log::Topic};

impl WasmStorePublicationWorkflow {
    // Create one new wasm store canister and register its runtime-managed binding.
    async fn create_publication_store(
        _publication_permit: &CostGuardPermit,
    ) -> Result<crate::ids::WasmStoreBinding, InternalError> {
        let result = deployment::create_canister_with_deployment_guard(
            deployment::PUBLICATION_WASM_STORE_CREATE_COMMAND_KIND,
            WASM_STORE_ROLE,
            IcOps::canister_self(),
            None,
        )
        .await?;
        let pid = result.new_canister_pid.ok_or_else(|| {
            InternalError::workflow(
                InternalErrorOrigin::Workflow,
                "wasm store creation did not return a pid",
            )
        })?;
        let binding = Self::binding_for_store_pid(pid);
        let registration = SubnetRegistryOps::registration(pid).ok_or_else(|| {
            PublicationWorkflowError::InvalidState(format!(
                "new wasm store {pid} is missing from the subnet registry"
            ))
        })?;
        SubnetStateOps::upsert_wasm_store(
            binding.clone(),
            registration.pid,
            registration.created_at,
        )?;

        log!(Topic::Wasm, Ok, "ws created {} ({})", binding, pid);

        Ok(binding)
    }

    // Allocate one additional empty store and add it to the managed publication fleet.
    pub(in crate::workflow::runtime::template::publication) async fn create_store_for_fleet(
        fleet: &mut PublicationStoreFleet,
        publication_permit: &CostGuardPermit,
    ) -> Result<PublicationPlacement, InternalError> {
        let binding = match fleet.preferred_binding.clone() {
            Some(_) => Self::create_publication_store(publication_permit).await?,
            None => Self::create_and_activate_first_publication_store(publication_permit).await?,
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
    async fn create_and_activate_first_publication_store(
        publication_permit: &CostGuardPermit,
    ) -> Result<crate::ids::WasmStoreBinding, InternalError> {
        let binding = Self::create_publication_store(publication_permit).await?;
        Self::ensure_retired_binding_slot_available_for_promotion()?;
        let changed_at = IcOps::now_secs();
        let previous = SubnetStateOps::publication_store_state();
        let activated =
            SubnetStateOps::activate_publication_store_binding(binding.clone(), changed_at);
        let current = SubnetStateOps::publication_store_state();
        if !activated && current.active_binding.as_ref() != Some(&binding) {
            return Err(InternalError::workflow(
                InternalErrorOrigin::Workflow,
                format!("new ws '{binding}' was not activated"),
            ));
        }
        Self::log_publication_state_transition(
            "activate_first_publication_binding",
            &previous,
            &current,
            changed_at,
        );

        Ok(binding)
    }
}
