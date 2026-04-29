use crate::{
    InternalError,
    domain::policy::topology::TopologyPolicy,
    ops::{
        storage::{
            index::{app::AppIndexOps, subnet::SubnetIndexOps},
            registry::subnet::SubnetRegistryOps,
        },
        topology::policy::mapper::RegistryPolicyInputMapper,
    },
    workflow::{
        cascade::{state::StateCascadeWorkflow, topology::TopologyCascadeWorkflow},
        ic::provision::ProvisionWorkflow,
        prelude::*,
        runtime::auth::RuntimeAuthWorkflow,
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

    /// Propagate application/subnet state and index views after structural mutations.
    ///
    /// This rebuilds index snapshots from the registry, applies current
    /// app state, cascades it to root children, and finally re-asserts
    /// index ↔ registry consistency.
    pub async fn propagate_state(
        _target: Principal,
        role: &CanisterRole,
    ) -> Result<(), InternalError> {
        // The implicit wasm_store receives the normal topology cascade, but its
        // publication inventory is synchronized in root-owned subnet state after
        // creation rather than via the immediate create-time state cascade.
        if role.is_wasm_store() {
            return Ok(());
        }

        // Shared index/app-state changes are sibling-visible, so create/adopt
        // state propagation must refresh all root children, not only the target branch.
        RuntimeAuthWorkflow::publish_root_delegated_key_to_subnet_state().await?;
        let snapshot = ProvisionWorkflow::rebuild_indexes_from_registry(Some(role))?
            .with_app_state()
            .with_subnet_state()
            .build();

        StateCascadeWorkflow::root_cascade_state(&snapshot).await?;

        let registry_data = SubnetRegistryOps::data();
        let registry_input = RegistryPolicyInputMapper::record_to_policy_input(registry_data);
        let app_data = AppIndexOps::data();
        let subnet_data = SubnetIndexOps::data();

        TopologyPolicy::assert_index_consistent_with_registry(&registry_input, &app_data.entries)
            .map_err(InternalError::from)?;

        TopologyPolicy::assert_index_consistent_with_registry(
            &registry_input,
            &subnet_data.entries,
        )
        .map_err(InternalError::from)?;

        Ok(())
    }
}
