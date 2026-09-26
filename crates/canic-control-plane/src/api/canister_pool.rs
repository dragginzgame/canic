//! Endpoint-facing facade for the Fleet Subnet Root Canister pool.

use canic_core::cdk::types::Principal;
use canic_core::dto::pool_import::{
    PoolImportCommand, PoolImportContext, PoolImportIdentity, PoolImportReservation,
    PoolImportStatus,
};
use canic_core::dto::{
    error::Error,
    pool::{CanisterPoolResponse, CanisterPoolStatusRequest, PoolAdminCommand, PoolAdminResponse},
};

pub struct CanisterPoolApi;

impl CanisterPoolApi {
    /// Validate the same bootstrap declaration in host review and Root initialization.
    pub fn validate_bootstrap(
        args: &canic_core::dto::fleet_subnet_root::FleetSubnetRootInitArgs,
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
        binding: &canic_core::ids::FleetSubnetRootBinding,
    ) -> Result<[u8; 32], Error> {
        crate::ops::canister_pool::capacity_import::CanisterPoolImportOps::authority_hash(binding)
            .map_err(Into::into)
    }

    /// Return protected current authority for host import review.
    pub fn import_context() -> Result<PoolImportContext, Error> {
        crate::workflow::canister_pool::capacity_import::context().map_err(Into::into)
    }

    /// Reserve the exact reviewed source set before controller handoff.
    pub fn reserve_import(request: PoolImportReservation) -> Result<PoolImportStatus, Error> {
        crate::workflow::canister_pool::capacity_import::reserve(request).map_err(Into::into)
    }

    /// Read retained protected import evidence without platform effects.
    pub fn import_status(identity: PoolImportIdentity) -> Result<PoolImportStatus, Error> {
        crate::workflow::canister_pool::capacity_import::status(identity).map_err(Into::into)
    }

    /// Advance one operation-bound source reset transition.
    pub async fn advance_import(
        identity: PoolImportIdentity,
        canister_id: Principal,
    ) -> Result<PoolImportStatus, Error> {
        crate::workflow::canister_pool::capacity_import::advance(identity, canister_id)
            .await
            .map_err(Into::into)
    }

    /// Retain the final Root cycle receipt after all imported sources are cleared.
    pub async fn settle_import(identity: PoolImportIdentity) -> Result<PoolImportStatus, Error> {
        crate::workflow::canister_pool::capacity_import::settle(identity)
            .await
            .map_err(Into::into)
    }

    /// Release allocation after durable host inventory publication.
    pub fn release_import(
        identity: PoolImportIdentity,
        publication_sha256: [u8; 32],
    ) -> Result<PoolImportStatus, Error> {
        crate::workflow::canister_pool::capacity_import::release(identity, publication_sha256)
            .map_err(Into::into)
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
