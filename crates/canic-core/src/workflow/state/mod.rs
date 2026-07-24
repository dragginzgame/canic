//! Module: workflow::state
//!
//! Responsibility: orchestrate Fleet-state mutations and downstream state cascades.
//! Does not own: endpoint authorization, stable state records, or DTO schemas.
//! Boundary: workflow layer between state API calls, storage ops, and cascade workflow.

pub mod query;

use crate::{
    InternalError,
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
    /// Apply a Fleet-level command (internal).
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
    pub async fn execute_command(cmd: FleetCommand) -> Result<FleetCommandResponse, InternalError> {
        EnvOps::require_root()?;
        let response = FleetStateOps::apply_command(cmd);
        if !fleet_command_response_changed(response) {
            return Ok(response);
        }

        let snapshot = StateSnapshotBuilder::new()?.with_fleet_state().build();
        StateCascadeWorkflow::root_cascade_state(&snapshot).await?;

        Ok(response)
    }
}

const fn fleet_command_response_changed(response: FleetCommandResponse) -> bool {
    match response {
        FleetCommandResponse::Status(status) => status.changed,
        FleetCommandResponse::CyclesFundingEnabled(enabled) => enabled.changed,
    }
}
