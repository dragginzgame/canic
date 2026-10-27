//! Lossless Store adoption boundary conversion; no authority or lifecycle changes.
use crate::dto::{
    fleet_subnet_root::FleetSubnetWasmStoreAdoptionRequest, root_store_control::AdoptRequest,
};
impl From<FleetSubnetWasmStoreAdoptionRequest> for AdoptRequest {
    fn from(value: FleetSubnetWasmStoreAdoptionRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            authority: value.authority.into(),
        }
    }
}
impl From<AdoptRequest> for FleetSubnetWasmStoreAdoptionRequest {
    fn from(value: AdoptRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            authority: value.authority.into(),
        }
    }
}

impl From<crate::dto::fleet_subnet_root::FleetSubnetWasmStoreAdoptionResponse>
    for crate::dto::root_store_control::Adoption
{
    fn from(value: crate::dto::fleet_subnet_root::FleetSubnetWasmStoreAdoptionResponse) -> Self {
        Self {
            operation_id: value.operation_id,
            authority: value.authority.into(),
            controllers: value.controllers.into_iter().map(Into::into).collect(),
            adopted_at_ns: value.adopted_at_ns,
        }
    }
}

impl From<crate::dto::root_store_control::Adoption>
    for crate::dto::fleet_subnet_root::FleetSubnetWasmStoreAdoptionResponse
{
    fn from(value: crate::dto::root_store_control::Adoption) -> Self {
        Self {
            operation_id: value.operation_id,
            authority: value.authority.into(),
            controllers: value.controllers.into_iter().map(Into::into).collect(),
            adopted_at_ns: value.adopted_at_ns,
        }
    }
}

impl From<crate::dto::root_store::RootStoreBootstrapResponse>
    for crate::dto::root_store_control::Bootstrap
{
    fn from(value: crate::dto::root_store::RootStoreBootstrapResponse) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            wasm_store: value.wasm_store.into(),
            release_set: value.release_set,
            catalog: value.catalog,
            fixtures: value.fixtures,
        }
    }
}

impl From<crate::dto::root_store_control::Bootstrap>
    for crate::dto::root_store::RootStoreBootstrapResponse
{
    fn from(value: crate::dto::root_store_control::Bootstrap) -> Self {
        Self {
            fleet_subnet_root: value.fleet_subnet_root.into(),
            wasm_store: value.wasm_store.into(),
            release_set: value.release_set,
            catalog: value.catalog,
            fixtures: value.fixtures,
        }
    }
}
