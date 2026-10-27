//! Native caller-authority observations; receipt hashing and storage remain unchanged.
use crate::dto::{
    caller_authority::{CallerAuthorityPhase, CallerAuthorityReadiness},
    component_directory::{ComponentBinding, FleetRegistryAuthority, ManagedCanisterBinding},
};
use ic_platform::Principal;
use serde::{Deserialize, Serialize};

/// Complete CallerInstallation with native principal authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CallerInstallation {
    pub binding: ManagedCanisterBinding,
    pub install_id: [u8; 32],
    pub component_install_id: [u8; 32],
}

/// Complete CallerComponentInstallation with native principal authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CallerComponentInstallation {
    pub binding: ComponentBinding,
    pub install_id: [u8; 32],
}

/// Complete CallerRootAuthority with native principal authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CallerRootAuthority {
    pub registry: FleetRegistryAuthority,
    pub root: Principal,
    pub install_id: [u8; 32],
}

/// Complete CallerReceiverAuthority with native principal authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CallerReceiverAuthority {
    pub receiver: CallerInstallation,
    pub issuer: CallerRootAuthority,
    pub policy_digest: [u8; 32],
}

/// Complete CallerAuthorityPublication with native principal authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CallerAuthorityPublication {
    pub operation_id: [u8; 32],
    pub authority: CallerReceiverAuthority,
    pub previous_generation: u64,
    pub generation: u64,
    pub change: CallerAuthorityChange,
    pub content_hash: [u8; 32],
}

/// Complete CallerAuthorityReceipt with native principal authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CallerAuthorityReceipt {
    pub publication: CallerAuthorityPublication,
    pub phase: CallerAuthorityPhase,
}

/// Complete CallerAuthorityStatus with native principal authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CallerAuthorityStatus {
    pub readiness: CallerAuthorityReadiness,
    pub authority: CallerReceiverAuthority,
    pub generation: u64,
    pub open: bool,
    pub retired: bool,
    pub entries: u32,
    pub reserved_bytes: u32,
    pub pending_operation: Option<[u8; 32]>,
    pub receipt: Option<CallerAuthorityReceipt>,
}

/// Original membership change carried by the retained publication.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum CallerAuthorityChange {
    StageSource(CallerInstallation),
    Grant(CallerInstallation),
    DenySource(CallerInstallation),
    DenyComponent(CallerComponentInstallation),
    OpenReceiver,
    RetireReceiver,
}
