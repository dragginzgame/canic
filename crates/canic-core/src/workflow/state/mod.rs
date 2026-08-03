//! Module: workflow::state
//!
//! Responsibility: orchestrate Fleet-state mutations and downstream state cascades.
//! Does not own: endpoint authorization, stable state records, or DTO schemas.
//! Boundary: workflow layer between state API calls, storage ops, and cascade workflow.

pub mod query;

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::state::{FleetCommand, FleetCommandResponse},
    ops::{runtime::env::EnvOps, storage::state::fleet::FleetStateOps},
    workflow::cascade::{snapshot::StateSnapshotBuilder, state::StateCascadeWorkflow},
};

///
/// FleetStateWorkflow
/// Orchestrates Fleet-state mutations and downstream cascades
///

pub struct FleetStateWorkflow;

impl FleetStateWorkflow {
    /// Apply a Fleet-level command and cascade to an explicit root-owned inventory.
    ///
    /// Workflow-level orchestration for mutating Fleet state.
    /// This function:
    /// - enforces execution context (root-only)
    /// - applies the command via storage ops
    /// - rebuilds the relevant state snapshot
    /// - cascades state changes to dependent components
    ///
    /// Returns internal [`InternalError`]. Public error mapping is handled
    /// exclusively at the API boundary.
    pub async fn execute_command_to(
        cmd: FleetCommand,
        root_children: &[Principal],
    ) -> Result<FleetCommandResponse, InternalError> {
        EnvOps::require_root()?;
        let response = FleetStateOps::apply_command(cmd);
        let snapshot = StateSnapshotBuilder::new()?.with_fleet_state().build();
        StateCascadeWorkflow::root_cascade_state_to(&snapshot, root_children).await?;

        Ok(response)
    }
}
