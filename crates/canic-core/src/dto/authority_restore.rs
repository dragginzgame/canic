//! Module: dto::authority_restore
//!
//! Responsibility: carry authority snapshot-seal requests and status at the canister boundary.
//! Does not own: history proof, transition validation, persistence, or timer suspension.
//! Boundary: controller endpoints accept and return these passive v1 shapes.

use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

/// Controller-selected identity for one authority snapshot operation.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthoritySnapshotRequest {
    pub operation_id: [u8; 32],
}

/// Durable phase of one authority canister's restore fence.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AuthorityRestoreFencePhase {
    Open,
    Sealed,
}

/// Current durable authority snapshot/restore-fence state.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityRestoreFenceStatusResponse {
    pub authority_canister: Principal,
    pub phase: AuthorityRestoreFencePhase,
    pub operation_id: Option<[u8; 32]>,
    pub history_total_num_changes: Option<u64>,
    pub changed_at_ns: Option<u64>,
}
