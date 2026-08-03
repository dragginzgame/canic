//! Endpoint-facing facade for the Fleet Subnet Root Canister pool.

use canic_core::dto::{
    error::Error,
    pool::{CanisterPoolResponse, CanisterPoolStatusRequest, PoolAdminCommand, PoolAdminResponse},
};

pub struct CanisterPoolApi;

impl CanisterPoolApi {
    pub fn status(request: CanisterPoolStatusRequest) -> Result<CanisterPoolResponse, Error> {
        crate::workflow::canister_pool::status(request).map_err(Into::into)
    }

    pub async fn admin(command: PoolAdminCommand) -> Result<PoolAdminResponse, Error> {
        crate::workflow::canister_pool::admin(command)
            .await
            .map_err(Into::into)
    }
}
