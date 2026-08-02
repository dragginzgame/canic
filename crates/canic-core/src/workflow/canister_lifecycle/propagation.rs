//! Module: workflow::canister_lifecycle::propagation
//!
//! Responsibility: propagate topology and state after lifecycle mutations.
//! Does not own: canister creation, stable registry schemas, or endpoint DTOs.
//! Boundary: lifecycle workflow helper coordinating cascades and consistency checks.

use crate::{
    InternalError,
    cdk::types::Principal,
    domain::policy::pure::topology::TopologyPolicy,
    dto::cascade::{StateSnapshotInput, TopologySnapshotInput},
    ids::CanisterRole,
    ops::{
        storage::{directory::fleet::FleetDirectoryOps, registry::subnet::SubnetRegistryOps},
        topology::input::mapper::TopologyRegistryMapper,
    },
    workflow::{
        cascade::{
            snapshot::adapter::StateSnapshotAdapter, state::StateCascadeWorkflow,
            topology::TopologyCascadeWorkflow,
        },
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
    pub async fn propagate_topology(
        target: Principal,
    ) -> Result<TopologySnapshotInput, InternalError> {
        TopologyCascadeWorkflow::root_cascade_topology_for_pid(target).await
    }

    /// Propagate Fleet state and Directory snapshots after structural mutations.
    ///
    /// This rebuilds Directory snapshots from the registry, applies current
    /// Fleet state, cascades it to root children, and finally re-asserts
    /// Directory ↔ Registry consistency.
    pub async fn propagate_state(role: &CanisterRole) -> Result<StateSnapshotInput, InternalError> {
        // Shared Directory/Fleet-state changes are sibling-visible, so create/adopt
        // state propagation must refresh all root children, not only the target branch.
        let snapshot = ProvisionWorkflow::rebuild_directories_from_registry(Some(role))?
            .with_fleet_state()
            .build();
        let input = StateSnapshotAdapter::to_input(&snapshot);

        StateCascadeWorkflow::root_cascade_state(&snapshot).await?;

        let registry_data = SubnetRegistryOps::data();
        let registry_input = TopologyRegistryMapper::data_to_registry(registry_data);
        let app_policy_input = FleetDirectoryOps::topology_entries();

        TopologyPolicy::assert_directory_consistent_with_registry(
            &registry_input,
            &app_policy_input,
        )
        .map_err(InternalError::from)?;

        Ok(input)
    }
}
