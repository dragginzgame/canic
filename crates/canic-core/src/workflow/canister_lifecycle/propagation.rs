use crate::{
    InternalError,
    domain::policy::topology::TopologyPolicy,
    ops::{
        storage::{
            directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
            registry::subnet::SubnetRegistryOps,
        },
        topology::policy::mapper::RegistryPolicyInputMapper,
    },
    workflow::{
        cascade::{state::StateCascadeWorkflow, topology::TopologyCascadeWorkflow},
        ic::provision::ProvisionWorkflow,
        prelude::*,
    },
};

///
/// PropagationWorkflow
///

pub struct PropagationWorkflow;

impl PropagationWorkflow {
    /// Propagate topology changes starting from the given canister.
    ///
    /// Used after structural mutations (create/adopt) to update
    /// parent/child relationships and derived topology views.
    pub async fn propagate_topology(target: Principal) -> Result<(), InternalError> {
        TopologyCascadeWorkflow::root_cascade_topology_for_pid(target).await
    }

    /// Propagate application/subnet state and directory views to newly created or adopted canisters.
    ///
    /// This rebuilds directory snapshots from the registry, applies current
    /// app state, cascades it to dependents, and finally re-asserts
    /// directory ↔ registry consistency.
    pub async fn propagate_state(
        target: Principal,
        role: &CanisterRole,
    ) -> Result<(), InternalError> {
        // The implicit wasm_store receives the normal topology cascade, but its
        // publication inventory is synchronized in root-owned subnet state after
        // creation rather than via the immediate create-time state cascade.
        if role.is_wasm_store() {
            return Ok(());
        }

        // Ensure newly created/adopted canisters inherit the current app
        // state and directory projections.
        let snapshot = ProvisionWorkflow::rebuild_directories_from_registry(Some(role))?
            .with_app_state()
            .with_subnet_state()
            .build();

        StateCascadeWorkflow::root_cascade_state_for_pid(target, &snapshot).await?;

        let registry_data = SubnetRegistryOps::data();
        let registry_input = RegistryPolicyInputMapper::record_to_policy_input(registry_data);
        let app_data = AppDirectoryOps::data();
        let subnet_data = SubnetDirectoryOps::data();

        TopologyPolicy::assert_directory_consistent_with_registry(
            &registry_input,
            &app_data.entries,
        )
        .map_err(InternalError::from)?;

        TopologyPolicy::assert_directory_consistent_with_registry(
            &registry_input,
            &subnet_data.entries,
        )
        .map_err(InternalError::from)?;

        Ok(())
    }
}
