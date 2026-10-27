//! Native retained installation authority; stored identities keep their canonical representation.
use ic_platform::Principal;
use serde::{Deserialize, Serialize};
/// Complete Store authority, including its exact release and installed module hash.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Authority {
    pub authority: crate::dto::component_directory::FleetRegistryAuthority,
    pub placement_subnet: Principal,
    pub fleet_subnet_root: Principal,
    pub wasm_store: Principal,
    pub installation_controller: Principal,
    pub release_build_id: crate::ids::ReleaseBuildId,
    pub wasm_module_hash: [u8; 32],
}
