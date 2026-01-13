use crate::{
    Error,
    dto::pool::{CanisterPoolView, PoolAdminCommand, PoolAdminResponse},
    workflow::pool::{PoolWorkflow, query::PoolQuery},
};

///
/// CanisterPoolApi
///

pub struct CanisterPoolApi;

impl CanisterPoolApi {
    #[must_use]
    pub fn list_view() -> CanisterPoolView {
        PoolQuery::pool_list_view()
    }

    pub async fn admin(cmd: PoolAdminCommand) -> Result<PoolAdminResponse, Error> {
        PoolWorkflow::handle_admin(cmd).await.map_err(Error::from)
    }
}
