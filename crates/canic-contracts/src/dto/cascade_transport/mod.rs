//! Native cascade topology; internal snapshot and persistence encodings remain unchanged.
use crate::ids::CanisterRole;
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Exact receiver-relative ancestry and child inventory.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TopologySnapshot {
    pub parents: Vec<PathNode>,
    pub children_map: Vec<Children>,
}
/// One ancestry link with explicit role and parent binding.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct PathNode {
    pub pid: Principal,
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
}
/// Exact child inventory for one parent.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Children {
    pub parent_pid: Principal,
    pub children: Vec<DirectChild>,
}
/// Role and identity of one direct child.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct DirectChild {
    pub pid: Principal,
    pub role: CanisterRole,
}
