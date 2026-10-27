//! Native Root-owned sibling Store preparation; protected release authority remains canonical.
use crate::dto::store_status::Authority;
use serde::{Deserialize, Serialize};
/// Exact reviewed sibling Store controller handoff.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct AdoptRequest {
    pub operation_id: [u8; 32],
    pub authority: Authority,
}

/// Exact retained sibling Store adoption receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Adoption {
    pub operation_id: [u8; 32],
    pub authority: Authority,
    pub controllers: Vec<ic_platform::Principal>,
    pub adopted_at_ns: u64,
}

/// Exact retained bootstrap catalog and fixture authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Bootstrap {
    pub fleet_subnet_root: ic_platform::Principal,
    pub wasm_store: ic_platform::Principal,
    pub release_set: crate::ids::FleetSubnetRootReleaseSet,
    pub catalog: Vec<crate::dto::root_store::RootStoreCatalogEntry>,
    pub fixtures: Vec<crate::dto::root_store::RootStoreFixture>,
}
