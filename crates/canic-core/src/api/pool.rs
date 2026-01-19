use crate::{
    dto::{
        error::Error,
        pool::{CanisterPoolResponse, PoolAdminCommand, PoolAdminResponse},
    },
    workflow::pool::{PoolWorkflow, query::PoolQuery},
};

///
/// CanisterPoolApi
///

pub struct CanisterPoolApi;

impl CanisterPoolApi {
    #[must_use]
    pub fn list() -> CanisterPoolResponse {
        PoolQuery::pool_list()
    }

    pub async fn admin(cmd: PoolAdminCommand) -> Result<PoolAdminResponse, Error> {
        PoolWorkflow::handle_admin(cmd).await.map_err(Error::from)
    }
}
