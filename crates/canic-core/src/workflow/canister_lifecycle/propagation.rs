//! Module: workflow::canister_lifecycle::propagation
//!
//! Responsibility: propagate topology and state after lifecycle mutations.
//! Does not own: canister creation, stable registry schemas, or endpoint DTOs.
//! Boundary: lifecycle workflow helper coordinating cascades and consistency checks.

use crate::{
    InternalError,
    cdk::types::Principal,
    domain::policy::pure::topology::TopologyPolicy,
    ids::CanisterRole,
    ops::{
        storage::{
            index::{app::AppIndexOps, subnet::SubnetIndexOps},
            registry::subnet::SubnetRegistryOps,
        },
        topology::input::mapper::TopologyRegistryMapper,
    },
    workflow::{
        cascade::{state::StateCascadeWorkflow, topology::TopologyCascadeWorkflow},
        ic::provision::ProvisionWorkflow,
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

    /// Propagate application/subnet state and index snapshots after structural mutations.
    ///
    /// This rebuilds index snapshots from the registry, applies current
    /// app state, cascades it to root children, and finally re-asserts
    /// index ↔ registry consistency.
    pub async fn propagate_state(role: &CanisterRole) -> Result<(), InternalError> {
        // The implicit wasm_store receives the normal topology cascade, but its
        // publication inventory is synchronized in root-owned subnet state after
        // creation rather than via the immediate create-time state cascade.
        if role.is_wasm_store() {
            return Ok(());
        }

        // Shared index/app-state changes are sibling-visible, so create/adopt
        // state propagation must refresh all root children, not only the target branch.
        let snapshot = ProvisionWorkflow::rebuild_indexes_from_registry(Some(role))?
            .with_app_state()
            .build();

        StateCascadeWorkflow::root_cascade_state(&snapshot).await?;

        let registry_data = SubnetRegistryOps::data();
        let registry_input = TopologyRegistryMapper::data_to_registry(registry_data);
        let app_policy_input = AppIndexOps::topology_entries();
        let subnet_policy_input = SubnetIndexOps::topology_entries();

        TopologyPolicy::assert_index_consistent_with_registry(&registry_input, &app_policy_input)
            .map_err(InternalError::from)?;

        TopologyPolicy::assert_index_consistent_with_registry(
            &registry_input,
            &subnet_policy_input,
        )
        .map_err(InternalError::from)?;

        Ok(())
    }
}
