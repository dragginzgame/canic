use crate::{PublicError, dto::state::AppCommand, workflow};

/// Workflow Query Re-export
pub use crate::workflow::state::query::{AppStateQuery, SubnetStateQuery};

///
/// AppStateApi
///

pub struct AppStateApi;

impl AppStateApi {
    pub async fn execute_command(cmd: AppCommand) -> Result<(), PublicError> {
        workflow::state::AppStateWorkflow::execute_command(cmd)
            .await
            .map_err(PublicError::from)
    }
}
