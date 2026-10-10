//! Endpoint-facing facade for the Fleet Subnet Root Canister pool.

use canic_contracts::dto::{
    error::Error,
    pool::{CanisterPoolResponse, CanisterPoolStatusRequest, PoolAdminCommand, PoolAdminResponse},
    pool_import::{PoolImportCommand, PoolImportContext, PoolImportIdentity, PoolImportStatus},
};
use canic_core::cdk::types::Principal;

pub struct CanisterPoolApi;

impl CanisterPoolApi {
    /// Validate the same bootstrap declaration in host review and Root initialization.
    pub fn validate_bootstrap(
        args: &canic_contracts::dto::fleet_subnet_root::FleetSubnetRootInitArgs,
    ) -> Result<(), Error> {
        crate::ops::canister_pool::capacity_import::bootstrap::validate(args).map_err(Into::into)
    }

    /// Resolve the exact operator for endpoint authentication before dispatch.
    pub fn import_operator(command: &PoolImportCommand) -> Result<Principal, Error> {
        crate::ops::canister_pool::capacity_import::CanisterPoolImportOps::operator(command)
            .map_err(Into::into)
    }

    /// Dispatch an import command after endpoint controller and operator authentication.
    pub async fn import_command(command: PoolImportCommand) -> Result<PoolImportStatus, Error> {
        crate::workflow::canister_pool::capacity_import::command(command)
            .await
            .map_err(Into::into)
    }

    /// Hash the full Root binding using the same reviewed-import authority contract.
    pub fn import_authority_hash(
        binding: &canic_contracts::ids::FleetSubnetRootBinding,
    ) -> Result<[u8; 32], Error> {
        crate::ops::canister_pool::capacity_import::CanisterPoolImportOps::authority_hash(binding)
            .map_err(Into::into)
    }

    /// Return protected current authority for host import review.
    pub fn import_context() -> Result<PoolImportContext, Error> {
        crate::workflow::canister_pool::capacity_import::context().map_err(Into::into)
    }

    /// Read retained protected import evidence without platform effects.
    pub fn import_status(identity: PoolImportIdentity) -> Result<PoolImportStatus, Error> {
        crate::workflow::canister_pool::capacity_import::status(identity).map_err(Into::into)
    }

    /// Observe obligations even while a competing pool operation prevents new work.
    pub fn release_status() -> Result<crate::dto::root::RootPoolReleaseResponse, Error> {
        crate::workflow::canister_pool::release_status().map_err(Into::into)
    }

    pub fn status(request: CanisterPoolStatusRequest) -> Result<CanisterPoolResponse, Error> {
        crate::workflow::canister_pool::status(request).map_err(Into::into)
    }

    pub async fn admin(command: PoolAdminCommand) -> Result<PoolAdminResponse, Error> {
        crate::workflow::canister_pool::admin(command)
            .await
            .map_err(Into::into)
    }
}
