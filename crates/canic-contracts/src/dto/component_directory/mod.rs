//! Passive native-Wire directory authority. Persisted bindings retain their owning encoding.

use crate::{
    deployment::ComponentChildKind,
    dto::component_registry::{ComponentDirectoryPageCursor, ComponentLifecycleStatus},
    ids::{CanisterRole, ComponentInstanceId, ComponentSpecId, FleetBinding},
    protocol::profile::ProtocolProfileDigest,
};
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Complete FleetCoordinatorBinding transported without changing stored authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetCoordinatorBinding {
    pub fleet: FleetBinding,
    pub coordinator_subnet: Principal,
    pub coordinator: Principal,
    pub recovery_controllers: Vec<Principal>,
}

/// Complete FleetRegistryAuthority transported without changing stored authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct FleetRegistryAuthority {
    pub binding: FleetCoordinatorBinding,
    pub epoch: u64,
}

/// Complete ComponentBinding transported without changing stored authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ComponentBinding {
    pub authority: FleetRegistryAuthority,
    pub component: ComponentInstanceId,
    pub component_spec: ComponentSpecId,
    pub spec_hash: [u8; 32],
    pub role: CanisterRole,
    pub placement_subnet: Principal,
    pub fleet_subnet_root: Principal,
    pub canister_id: Principal,
}

/// Complete ComponentChildBinding transported without changing stored authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ComponentChildBinding {
    pub component: ComponentBinding,
    pub parent_canister_id: Principal,
    pub role: CanisterRole,
    pub canister_id: Principal,
}

/// Complete ComponentDirectoryProvenance transported without changing stored authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ComponentDirectoryProvenance {
    pub component: ComponentBinding,
    pub source_fleet_subnet_root: Principal,
    pub component_registry_revision: u64,
    pub component_registry_content_hash: [u8; 32],
    pub synchronized_at_ns: u64,
}

/// Complete ComponentDirectoryHead transported without changing stored authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ComponentDirectoryHead {
    pub provenance: ComponentDirectoryProvenance,
    pub descendant_count: u32,
}

/// Complete ComponentDirectoryPageRequest transported without changing stored authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ComponentDirectoryPageRequest {
    pub directory: ComponentDirectoryHead,
    pub parent_canister_id: Option<Principal>,
    pub role: Option<CanisterRole>,
    pub status: Option<ComponentLifecycleStatus>,
    pub cursor: Option<ComponentDirectoryPageCursor>,
    pub limit: u16,
}

/// Complete ComponentDirectoryChildEntry transported without changing stored authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ComponentDirectoryChildEntry {
    pub binding: ComponentChildBinding,
    pub kind: ComponentChildKind,
    pub installed_artifact_hash: [u8; 32],
    pub protocol_profile_digest: ProtocolProfileDigest,
    pub status: ComponentLifecycleStatus,
}

/// Complete ComponentDirectoryPageResponse transported without changing stored authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct ComponentDirectoryPageResponse {
    pub directory: ComponentDirectoryHead,
    pub entries: Vec<ComponentDirectoryChildEntry>,
    pub next_cursor: Option<ComponentDirectoryPageCursor>,
}

/// Exact managed identity with native principal fields.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum ManagedCanisterBinding {
    Component(ComponentBinding),
    ComponentChild(ComponentChildBinding),
}
