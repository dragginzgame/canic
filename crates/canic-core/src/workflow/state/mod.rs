pub mod mapper;
pub mod query;

use crate::{
    Error,
    access::env,
    dto::state::AppCommand,
    ops::storage::state::app::AppStateOps,
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
    /// Returns internal [`Error`]. Public error mapping is handled
    /// exclusively at the API boundary.
    pub async fn execute_command(cmd: AppCommand) -> Result<(), Error> {
        env::require_root()?;
        let cmd = mapper::AppCommandMapper::from_dto(cmd);
        AppStateOps::execute_command(cmd)?;

        let snapshot = StateSnapshotBuilder::new()?.with_app_state().build();
        StateCascadeWorkflow::root_cascade_state(&snapshot).await?;

        Ok(())
    }
}
