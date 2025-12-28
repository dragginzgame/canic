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

use crate::dto::prelude::*;

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

/// Summary of pool entries by status.
#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct PoolStatusCounts {
    pub ready: u64,
    pub pending_reset: u64,
    pub failed: u64,
    pub total: u64,
}

/// Diagnostics for queued imports.
#[derive(CandidType, Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct PoolImportSummary {
    pub status_counts: PoolStatusCounts,

    pub skipped_in_registry: u64,
    pub skipped_already_ready: u64,
    pub skipped_already_pending_reset: u64,
    pub skipped_already_failed: u64,
    pub skipped_non_importable: u64,
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
    QueuedImported {
        added: u64,
        requeued: u64,
        skipped: u64,
        total: u64,
        summary: PoolImportSummary,
    },

    /// Failed pool entries were requeued.
    FailedRequeued {
        requeued: u64,
        skipped: u64,
        total: u64,
    },
}
