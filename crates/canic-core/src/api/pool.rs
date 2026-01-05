use crate::{
    PublicError,
    dto::pool::{CanisterPoolView, PoolAdminCommand, PoolAdminResponse},
    workflow,
};

///
/// CanisterPoolApi
///

pub struct CanisterPoolApi;

impl CanisterPoolApi {
    #[must_use]
    pub fn list_view() -> CanisterPoolView {
        workflow::pool::query::pool_list_view()
    }

    pub async fn admin(cmd: PoolAdminCommand) -> Result<PoolAdminResponse, PublicError> {
        workflow::pool::PoolWorkflow::handle_admin(cmd)
            .await
            .map_err(PublicError::from)
    }
}
