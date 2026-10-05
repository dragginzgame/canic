//! Module: dto::caller_authority
//!
//! Protected managed-caller publication and original-operation receipt boundaries.

use crate::{
    dto::prelude::*,
    ids::{CallerComponentInstallation, CallerInstallation, CallerReceiverAuthority},
};

/// Exact source or receiver lifecycle change issued by the owning Root.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum CallerAuthorityChange {
    StageSource(CallerInstallation),
    Grant(CallerInstallation),
    DenySource(CallerInstallation),
    DenyComponent(CallerComponentInstallation),
    OpenReceiver,
    RetireReceiver,
}

/// Immutable recipient-specific original publication; generation and hash are issuer-owned.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct CallerAuthorityPublication {
    pub operation_id: [u8; 32],
    pub authority: CallerReceiverAuthority,
    pub previous_generation: u64,
    pub generation: u64,
    pub change: CallerAuthorityChange,
    pub content_hash: [u8; 32],
}

/// Protected publication phases, serialized with source membership changes.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum CallerAuthorityCommand {
    Prepare(CallerAuthorityPublication),
    Commit(CallerAuthorityPublication),
    Complete(CallerAuthorityPublication),
}

/// Exact durable receiver progress for lost-response reconciliation.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum CallerAuthorityPhase {
    Prepared,
    Committed,
    Complete,
}

/// Retained immutable original publication and observed durable phase.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct CallerAuthorityReceipt {
    pub publication: CallerAuthorityPublication,
    pub phase: CallerAuthorityPhase,
}

/// Receiver authority and optional original-operation receipt, without copying the source census.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
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
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum CallerAuthorityReadiness {
    FrameworkPending,
    FrameworkReady,
    ApplicationReady,
}
