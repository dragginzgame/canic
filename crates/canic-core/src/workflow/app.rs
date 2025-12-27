use crate::{
    Error,
    dto::{app::AppCommand, bundle::StateBundle},
    ops::{OpsError, storage::state::AppStateOps},
    workflow::cascade::state::cascade_root_state,
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
