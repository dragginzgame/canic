use crate::{
    Error, PublicError,
    dto::state::AppCommand,
    ops::{runtime::env::EnvOps, storage::state::AppStateOps},
    workflow::{cascade::state::root_cascade_state, snapshot::StateSnapshotBuilder},
};

///
/// AppStateOrchestrator
///

pub struct AppStateOrchestrator;

impl AppStateOrchestrator {
    pub(crate) async fn apply_command_internal(cmd: AppCommand) -> Result<(), Error> {
        EnvOps::require_root()?;
        AppStateOps::command(cmd)?;

        let snapshot = StateSnapshotBuilder::new().with_app_state().build();
        root_cascade_state(&snapshot).await?;

        Ok(())
    }

    pub async fn apply_command(cmd: AppCommand) -> Result<(), PublicError> {
        Self::apply_command_internal(cmd)
            .await
            .map_err(PublicError::from)
    }
}
