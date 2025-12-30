//! Pool admin DTOs.
//!
//! This module defines the command and response types used at the
//! boundary of the pool workflow (endpoints, admin APIs).
//!
//! These types:
//! - are pure data
//! - contain no logic
//! - are safe to serialize / persist / expose
//!
//! They must NOT:
//! - perform validation
//! - call ops or workflow
//! - embed policy or orchestration logic

use crate::{cdk::types::Cycles, dto::prelude::*};

///
/// CanisterPoolView
/// Read-only pool snapshot for endpoints.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterPoolView(pub Vec<(Principal, CanisterPoolEntryView)>);

///
/// CanisterPoolStatusView
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterPoolStatusView {
    PendingReset,
    Ready,
    Failed { reason: String },
}

///
/// CanisterPoolEntryView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterPoolEntryView {
    pub created_at: u64,
    pub cycles: Cycles,
    pub status: CanisterPoolStatusView,
    pub role: Option<CanisterRole>,
    pub parent: Option<Principal>,
    pub module_hash: Option<Vec<u8>>,
}

///
/// PoolAdminCommand
///
/// These represent *intent*, not execution.
/// Validation and authorization are handled elsewhere.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum PoolAdminCommand {
    /// Create a fresh empty pool canister.
    CreateEmpty,

    /// Recycle an existing canister back into the pool.
    Recycle { pid: Principal },

    /// Import a canister into the pool immediately (synchronous).
    ImportImmediate { pid: Principal },

    /// Queue one or more canisters for pool import.
    ImportQueued { pids: Vec<Principal> },
}

///
/// PoolAdminResponse
/// These describe *what happened*, not *how* it happened.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum PoolAdminResponse {
    /// A new pool canister was created.
    Created { pid: Principal },

    /// A canister was successfully recycled into the pool.
    Recycled,

    /// A canister was imported immediately.
    Imported,

    /// One or more canisters were queued for import.
    QueuedImported { result: PoolBatchResult },

    /// Failed pool entries were requeued.
    FailedRequeued { result: PoolBatchResult },
}

///
/// PoolBatchResult
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PoolBatchResult {
    pub total: u64,
    pub added: u64,
    pub requeued: u64,
    pub skipped: u64,
}
