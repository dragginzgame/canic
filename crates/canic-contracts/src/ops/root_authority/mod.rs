//! Lossless Root authority conversion at the transport boundary.
use crate::dto::{fleet_subnet_root::FleetSubnetRootAuthority, root_authority::Authority};
impl From<FleetSubnetRootAuthority> for Authority {
    fn from(value: FleetSubnetRootAuthority) -> Self {
        Self {
            binding: value.binding.into(),
            initial_release_set: value.initial_release_set,
            expected_module_hash: value.expected_module_hash,
            wasm_store_authority: value.wasm_store_authority.into(),
        }
    }
}
impl From<Authority> for FleetSubnetRootAuthority {
    fn from(value: Authority) -> Self {
        Self {
            binding: value.binding.into(),
            initial_release_set: value.initial_release_set,
            expected_module_hash: value.expected_module_hash,
            wasm_store_authority: value.wasm_store_authority.into(),
        }
    }
}
