//! Module: workflow::state
//!
//! Responsibility: orchestrate app-state mutations and downstream state cascades.
//! Does not own: endpoint authorization, stable state records, or DTO schemas.
//! Boundary: workflow layer between state API calls, storage ops, and cascade workflow.

pub mod query;

use crate::{
    InternalError,
    dto::state::{AppCommand, AppCommandResponse},
    ops::{runtime::env::EnvOps, storage::state::app::AppStateOps},
    workflow::cascade::{snapshot::StateSnapshotBuilder, state::StateCascadeWorkflow},
};

///
/// AppStateWorkflow
/// Orchestrates application state mutations and downstream cascades
///

pub struct AppStateWorkflow;

impl AppStateWorkflow {
    /// Apply an application-level command (internal).
    ///
    /// Workflow-level orchestration for mutating application state.
    /// This function:
    /// - enforces execution context (root-only)
    /// - applies the command via storage ops
    /// - rebuilds the relevant state snapshot
    /// - cascades state changes to dependent components
    ///
    /// Returns internal [`InternalError`]. Public error mapping is handled
    /// exclusively at the API boundary.
    pub async fn execute_command(cmd: AppCommand) -> Result<AppCommandResponse, InternalError> {
        EnvOps::require_root()?;
        let response = AppStateOps::apply_command(cmd);
        if !app_command_response_changed(response) {
            return Ok(response);
        }

        let snapshot = StateSnapshotBuilder::new()?.with_app_state().build();
        StateCascadeWorkflow::root_cascade_state(&snapshot).await?;

        Ok(response)
    }
}

const fn app_command_response_changed(response: AppCommandResponse) -> bool {
    match response {
        AppCommandResponse::Status(status) => status.changed,
        AppCommandResponse::CyclesFundingEnabled(enabled) => enabled.changed,
    }
}
