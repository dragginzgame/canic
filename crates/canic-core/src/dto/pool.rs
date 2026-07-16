//! Pool admin DTOs.
//!
//! This module defines the command and response types used at the
//! boundary of the pool workflow (endpoints, admin APIs).
//!
//! These types:
//! - are pure data
//! - contain no logic
//! - are safe to serialize / expose
//!
//! They must NOT:
//! - perform validation
//! - call ops or workflow
//! - embed policy or orchestration logic

use crate::{
    cdk::types::Cycles,
    dto::{prelude::*, rpc::RootRequestMetadata},
};

pub use crate::domain::pool::CanisterPoolStatus;

//
// CanisterPoolResponse
// Read-only pool snapshot for endpoints.
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CanisterPoolResponse {
    pub entries: Vec<CanisterPoolEntry>,
}

//
// CanisterPoolEntry
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CanisterPoolEntry {
    pub pid: Principal,
    pub created_at: u64,
    pub cycles: Cycles,
    pub status: CanisterPoolStatus,
    pub role: Option<CanisterRole>,
    pub parent: Option<Principal>,
    pub module_hash: Option<Vec<u8>>,
}

//
// PoolAdminCommand
//
// These represent *intent*, not execution.
// Validation and authorization are handled elsewhere.
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum PoolAdminCommand {
    // Create a fresh empty pool canister.
    CreateEmpty(CreateEmptyPoolRequest),

    // Recycle an existing canister back into the pool.
    Recycle { pid: Principal },

    // Import a canister into the pool immediately (synchronous).
    ImportImmediate { pid: Principal },

    // Queue one or more canisters for pool import.
    ImportQueued { pids: Vec<Principal> },
}

//
// CreateEmptyPoolRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct CreateEmptyPoolRequest {
    #[serde(default)]
    pub metadata: Option<RootRequestMetadata>,
}

//
// PoolAdminResponse
// These describe *what happened*, not *how* it happened.
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum PoolAdminResponse {
    // A new pool canister was created.
    Created { pid: Principal },

    // A canister was successfully recycled into the pool.
    Recycled,

    // A canister was imported immediately.
    Imported,

    // One or more canisters were queued for import.
    QueuedImported { result: PoolBatchResult },
}

//
// PoolBatchResult
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PoolBatchResult {
    pub total: u64,
    pub added: u64,
    pub requeued: u64,
    pub skipped: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reexported_pool_status_roundtrips_through_candid() {
        let entry = CanisterPoolEntry {
            pid: Principal::from_slice(&[3; 29]),
            created_at: 42,
            cycles: Cycles::new(10_000),
            status: crate::domain::pool::CanisterPoolStatus::Failed {
                reason: "bounded reset failure".to_string(),
            },
            role: Some(CanisterRole::new("worker")),
            parent: None,
            module_hash: Some(vec![1, 2, 3]),
        };

        let bytes = candid::encode_one(&entry).expect("encode pool entry");
        let decoded: CanisterPoolEntry = candid::decode_one(&bytes).expect("decode pool entry");

        let dto_status: CanisterPoolStatus = crate::domain::pool::CanisterPoolStatus::Failed {
            reason: "bounded reset failure".to_string(),
        };

        assert_eq!(decoded.pid, Principal::from_slice(&[3; 29]));
        assert_eq!(decoded.created_at, 42);
        assert_eq!(decoded.cycles, Cycles::new(10_000));
        assert_eq!(decoded.status, dto_status);
        assert_eq!(decoded.role, Some(CanisterRole::new("worker")));
        assert_eq!(decoded.parent, None);
        assert_eq!(decoded.module_hash, Some(vec![1, 2, 3]));
    }
}
