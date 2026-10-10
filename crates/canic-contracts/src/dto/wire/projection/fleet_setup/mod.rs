//! Passive fleet setup transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::{
    component_registry::{
        RootComponentRegistryPreparationRequest, RootComponentRegistryStatusResponse,
    },
    fleet_registry::FleetSubnetRootRegistrySyncRequest,
    fleet_subnet_root::{FleetSubnetRootAuthority, FleetSubnetWasmStoreAdoptionRequest},
    role::{OperationReceipt, OperationStatusRequest},
    root::RootOperationStatusResponse,
    root_store::RootStoreBootstrapRequest,
};
use candid::CandidType;
use serde::Deserialize;

#[derive(CandidType)]
pub enum RootCommandFragment {
    MaintainPool,
    ImportPoolCanister(crate::dto::pool::PoolCanisterRequest),
    AdoptStore(FleetSubnetWasmStoreAdoptionRequest),
    BootstrapStore(RootStoreBootstrapRequest),
    PrepareStoreFixture(crate::dto::root_store::RootStoreFixturePrepareRequest),
    PrepareComponentRegistry(RootComponentRegistryPreparationRequest),
    SynchronizeRegistry(FleetSubnetRootRegistrySyncRequest),
}

#[derive(CandidType, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "the private decoder mirrors the exact Root command response wire"
)]
pub enum RootCommandResponseFragment {
    PrepareStoreFixture(
        Result<
            crate::dto::fixture_provisioning::FixtureSourceStatus,
            crate::dto::fixture_provisioning::FixtureStoreError,
        >,
    ),
    MaintainPool(crate::dto::pool::PoolMaintenanceResponse),
    ImportPoolCanister(crate::dto::pool::PoolImportResponse),
    OperationAccepted(OperationReceipt),
    PrepareComponentRegistry(RootComponentRegistryStatusResponse),
}

#[derive(CandidType)]
#[expect(
    clippy::large_enum_variant,
    reason = "the private encoder mirrors the exact Root status request wire"
)]
pub enum RootStatusRequestFragment {
    Pool(crate::dto::pool::CanisterPoolStatusRequest),
    ComponentRegistry(RootComponentRegistryPreparationRequest),
    FleetAuthority,
    Operation(OperationStatusRequest),
}

#[derive(CandidType, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "the private decoder mirrors the exact Root status response wire"
)]
pub enum RootStatusResponseFragment {
    Pool(crate::dto::pool::CanisterPoolResponse),
    ComponentRegistry(RootComponentRegistryStatusResponse),
    FleetAuthority(FleetSubnetRootAuthority),
    Operation(RootOperationStatusResponse),
}
