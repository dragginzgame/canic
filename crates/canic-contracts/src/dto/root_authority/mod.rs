//! Complete native Root installation authority; persisted authority is unchanged.
use serde::{Deserialize, Serialize};

/// Exact Root binding, initial release, module and reciprocal Store identity.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Authority {
    pub binding: crate::dto::fleet_snapshot::FleetSubnetRootBinding,
    pub initial_release_set: crate::ids::FleetSubnetRootReleaseSet,
    pub expected_module_hash: [u8; 32],
    pub wasm_store_authority: crate::dto::store_status::Authority,
}
