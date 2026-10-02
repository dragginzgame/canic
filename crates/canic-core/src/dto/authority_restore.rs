//! Module: dto::authority_restore
//!
//! Responsibility: carry authority snapshot/release-seal requests and status at the boundary.
//! Does not own: history proof, transition validation, persistence, or timer suspension.
//! Boundary: controller endpoints accept and return these passive v1 shapes.

use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

/// Controller-selected identity for one authority snapshot operation.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthoritySnapshotRequest {
    pub operation_id: [u8; 32],
}

/// Exact reviewed identity for irreversible release fencing within one installation.
/// This request alone proves neither quiescence nor permission to clear owner state.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityReleaseRequest {
    pub operation_id: [u8; 32],
    pub review_sha256: [u8; 32],
    pub recipient: Principal,
}

/// Durable phase of one authority canister's restore fence.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AuthorityRestoreFencePhase {
    Open,
    Sealed,
    ReleaseSealed {
        review_sha256: [u8; 32],
        recipient: Principal,
    },
}

/// Current durable authority snapshot/release-fence state.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityRestoreFenceStatusResponse {
    pub authority_canister: Principal,
    pub phase: AuthorityRestoreFencePhase,
    pub operation_id: Option<[u8; 32]>,
    pub history_total_num_changes: Option<u64>,
    pub changed_at_ns: Option<u64>,
}
