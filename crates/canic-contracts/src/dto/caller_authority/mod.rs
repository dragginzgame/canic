//! Module: dto::caller_authority
//!
//! Protected managed-caller publication and original-operation receipt boundaries.

use crate::{
    dto::prelude::*,
    ids::{CallerComponentInstallation, CallerInstallation, CallerReceiverAuthority},
};

/// Exact source or receiver lifecycle change issued by the owning Root.

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub enum CallerAuthorityChange {
    StageSource(CallerInstallation),
    Grant(CallerInstallation),
    DenySource(CallerInstallation),
    DenyComponent(CallerComponentInstallation),
    OpenReceiver,
    RetireReceiver,
}

/// Immutable recipient-specific original publication; generation and hash are issuer-owned.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct CallerAuthorityPublication {
    pub operation_id: [u8; 32],
    pub authority: CallerReceiverAuthority,
    pub previous_generation: u64,
    pub generation: u64,
    pub change: CallerAuthorityChange,
    pub content_hash: [u8; 32],
}

/// Protected publication phases, serialized with source membership changes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub enum CallerAuthorityCommand {
    Prepare(CallerAuthorityPublication),
    Commit(CallerAuthorityPublication),
    Complete(CallerAuthorityPublication),
}

/// Exact durable receiver progress for lost-response reconciliation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum CallerAuthorityPhase {
    Prepared,
    Committed,
    Complete,
}

/// Retained immutable original publication and observed durable phase.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct CallerAuthorityReceipt {
    pub publication: CallerAuthorityPublication,
    pub phase: CallerAuthorityPhase,
}

/// Receiver authority and optional original-operation receipt, without copying the source census.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
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

/// Framework bootstrap may finish before protected application initialization and fixtures.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum CallerAuthorityReadiness {
    FrameworkPending,
    FrameworkReady,
    ApplicationReady,
}
