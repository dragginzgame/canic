use crate::{
    Error,
    access::env,
    dto::state::AppCommand,
    ops::storage::state::app::AppStateOps,
    workflow::cascade::{snapshot::StateSnapshotBuilder, state::root_cascade_state},
};

///
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
///
pub async fn apply_command(cmd: AppCommand) -> Result<(), Error> {
    env::require_root()?;
    AppStateOps::command(cmd)?;

    let snapshot = StateSnapshotBuilder::new()?.with_app_state().build();
    root_cascade_state(&snapshot).await?;

    Ok(())
}
