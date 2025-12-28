use crate::{
    Error,
    dto::state::AppCommand,
    ops::{OpsError, storage::state::AppStateOps},
    workflow::{cascade::state::root_cascade_state, snapshot::StateSnapshotBuilder},
};

///
/// AppStateOrchestrator
///

pub struct AppStateOrchestrator;

impl AppStateOrchestrator {
    pub async fn apply_command(cmd: AppCommand) -> Result<(), Error> {
        OpsError::require_root()?;
        AppStateOps::command(cmd)?;

        let snapshot = StateSnapshotBuilder::new().with_app_state().build();
        root_cascade_state(&snapshot).await?;

        Ok(())
    }
}
