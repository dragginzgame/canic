use crate::{
    dto::{error::Error, state::AppCommand},
    workflow::state::AppStateWorkflow,
};

/// Workflow Query Re-export
pub use crate::workflow::state::query::{AppStateQuery, SubnetStateQuery};

///
/// AppStateApi
///

pub struct AppStateApi;

impl AppStateApi {
    pub async fn execute_command(cmd: AppCommand) -> Result<(), Error> {
        AppStateWorkflow::execute_command(cmd)
            .await
            .map_err(Error::from)
    }
}
