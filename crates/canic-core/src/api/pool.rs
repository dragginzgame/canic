use crate::{
    PublicError,
    dto::pool::{CanisterPoolView, PoolAdminCommand, PoolAdminResponse},
    workflow,
};

pub fn canic_pool_list() -> Result<CanisterPoolView, PublicError> {
    Ok(workflow::pool::query::pool_list_view())
}

pub async fn canic_pool_admin(cmd: PoolAdminCommand) -> Result<PoolAdminResponse, PublicError> {
    workflow::pool::admin::handle_admin(cmd)
        .await
        .map_err(PublicError::from)
}
