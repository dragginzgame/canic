use crate::{
    Error,
    ops::{
        OpsError,
        storage::state::{AppCommand, AppStateOps},
    },
    workflow::cascade::state::{StateBundle, cascade_root_state},
};

///
/// AppStateOrchestrator
///

pub struct AppStateOrchestrator;

impl AppStateOrchestrator {
    pub async fn apply_command(cmd: AppCommand) -> Result<(), Error> {
        OpsError::require_root()?;
        AppStateOps::command(cmd)?;

        let bundle = StateBundle::new().with_app_state();
        cascade_root_state(bundle).await?;

        Ok(())
    }
}
