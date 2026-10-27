//! Lossless boundary conversions for retained Store authority.
use crate::{
    dto::store_status::Authority,
    ids::{FleetSubnetWasmStoreAuthority, SubnetId},
};
impl From<FleetSubnetWasmStoreAuthority> for Authority {
    fn from(value: FleetSubnetWasmStoreAuthority) -> Self {
        Self {
            authority: value.authority.into(),
            placement_subnet: value.placement_subnet.into_principal().into(),
            fleet_subnet_root: value.fleet_subnet_root.into(),
            wasm_store: value.wasm_store.into(),
            installation_controller: value.installation_controller.into(),
            release_build_id: value.release_build_id,
            wasm_module_hash: value.wasm_module_hash,
        }
    }
}
impl From<Authority> for FleetSubnetWasmStoreAuthority {
    fn from(value: Authority) -> Self {
        Self {
            authority: value.authority.into(),
            placement_subnet: SubnetId::from_principal(value.placement_subnet.into()),
            fleet_subnet_root: value.fleet_subnet_root.into(),
            wasm_store: value.wasm_store.into(),
            installation_controller: value.installation_controller.into(),
            release_build_id: value.release_build_id,
            wasm_module_hash: value.wasm_module_hash,
        }
    }
}
