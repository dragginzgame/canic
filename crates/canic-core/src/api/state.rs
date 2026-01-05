use crate::{
    PublicError,
    dto::state::{AppCommand, AppStateView, SubnetStateView},
    workflow,
};

///
/// AppStateApi
///

pub struct AppStateApi;

impl AppStateApi {
    #[must_use]
    pub fn view() -> AppStateView {
        workflow::state::query::AppStateQuery::view()
    }

    pub async fn execute_command(cmd: AppCommand) -> Result<(), PublicError> {
        workflow::state::AppStateWorkflow::execute_command(cmd)
            .await
            .map_err(PublicError::from)
    }
}

///
/// SubnetState Api
///

pub struct SubnetStateApi;

impl SubnetStateApi {
    #[must_use]
    pub fn view() -> SubnetStateView {
        workflow::state::query::SubnetStateQuery::view()
    }
}
