use crate::{
    Error,
    dto::state::AppCommand,
    ops::{runtime::env::EnvOps, storage::state::app::AppStateOps},
    workflow::{cascade::state::root_cascade_state, snapshot::StateSnapshotBuilder},
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
pub(crate) async fn apply_command(cmd: AppCommand) -> Result<(), Error> {
    EnvOps::require_root()?;
    AppStateOps::command(cmd)?;

    let snapshot = StateSnapshotBuilder::new().with_app_state().build();
    root_cascade_state(&snapshot).await?;

    Ok(())
}
