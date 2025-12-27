use crate::{
    Error,
    dto::state::AppCommand,
    ops::{OpsError, storage::state::AppStateOps},
    workflow::cascade::state::{StateBundleBuilder, root_cascade_state},
};

///
/// AppStateOrchestrator
///

pub struct AppStateOrchestrator;

impl AppStateOrchestrator {
    pub async fn apply_command(cmd: AppCommand) -> Result<(), Error> {
        OpsError::require_root()?;
        AppStateOps::command(cmd)?;

        let bundle = StateBundleBuilder::new().with_app_state().build();
        root_cascade_state(&bundle).await?;

        Ok(())
    }
}
