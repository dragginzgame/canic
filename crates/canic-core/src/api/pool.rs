use crate::{
    PublicError,
    dto::pool::{CanisterPoolView, PoolAdminCommand, PoolAdminResponse},
    workflow,
};

///
/// Pool API
///

#[must_use]
pub fn pool_list() -> CanisterPoolView {
    workflow::pool::query::pool_list_view()
}

pub async fn pool_admin(cmd: PoolAdminCommand) -> Result<PoolAdminResponse, PublicError> {
    workflow::pool::admin::handle_admin(cmd)
        .await
        .map_err(PublicError::from)
}
