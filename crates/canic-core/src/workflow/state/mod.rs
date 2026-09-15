//! Module: workflow::state
//!
//! Responsibility: orchestrate Fleet-state mutations and downstream state cascades.
//! Does not own: endpoint authorization, stable state records, or DTO schemas.
//! Boundary: workflow layer between state API calls, storage ops, and cascade workflow.

pub mod query;

use crate::{
    InternalError,
    dto::state::{FleetCommand, FleetCommandExecutionResponse},
    ops::{
        cascade_report::StateCascadeReportOps, runtime::env::EnvOps,
        storage::state::fleet::FleetStateOps,
    },
    view::state_cascade::StateCascadeTarget,
    workflow::{
        cascade::{snapshot::StateSnapshotBuilder, state::StateCascadeWorkflow},
        runtime::cycles::CycleWorkflow,
    },
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
        root_children: &[StateCascadeTarget],
        reconcile_funding: bool,
    ) -> Result<FleetCommandExecutionResponse, InternalError> {
        EnvOps::require_root()?;
        let builder = StateSnapshotBuilder::new()?;
        let response = FleetStateOps::apply_command(cmd);
        let reconciliation_error = if reconcile_funding {
            CycleWorkflow::start().err().map(Into::into)
        } else {
            None
        };
        let snapshot = builder.with_fleet_state().build();
        let propagation =
            StateCascadeWorkflow::root_cascade_state_to(&snapshot, root_children).await?;
        Ok(StateCascadeReportOps::command_response(
            response,
            propagation,
            reconciliation_error,
        ))
    }
}
