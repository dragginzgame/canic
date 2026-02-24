pub mod mapper;

use crate::{
    cdk::types::Principal,
    dto::auth::DelegationProof,
    storage::stable::auth::{DelegationProofRecord, DelegationState},
};
use mapper::DelegationProofRecordMapper;

///
/// DelegationStateOps
///
/// WHY THIS FILE EXISTS
/// --------------------
/// This module defines the **only authorized access path** to persisted
/// delegation state stored in stable memory.
///
/// It intentionally sits between:
///   - access / auth logic
///   - stable storage implementation details
///
/// Responsibilities:
/// - Provide a narrow, explicit API for delegation state access
/// - Prevent access-layer code from depending on storage internals
/// - Serve as the choke point for future changes (migration, versioning)
///
/// This is a **security-sensitive boundary**:
/// delegation state determines which signer authorities are trusted.
///

pub struct DelegationStateOps;

impl DelegationStateOps {
    /// Get the currently active delegation proof.
    ///
    /// Semantics:
    /// - Returns `Some` if delegation is initialized
    /// - Returns `None` if delegation is not configured or not yet established
    ///
    /// This value represents the *current trust anchor* for delegated tokens.
    #[must_use]
    pub fn proof() -> Option<DelegationProofRecord> {
        DelegationState::get_proof()
    }

    /// Get the current delegation proof as a DTO.
    #[must_use]
    pub fn proof_dto() -> Option<DelegationProof> {
        Self::proof().map(DelegationProofRecordMapper::record_to_view)
    }

    /// Set the active delegation proof.
    ///
    /// Intended usage:
    /// - Delegation initialization
    /// - Delegation rotation
    ///
    /// IMPORTANT:
    /// - This operation invalidates all previously issued delegated tokens.
    /// - Callers MUST ensure atomicity at a higher level if required.
    pub fn set_proof(proof: DelegationProofRecord) {
        DelegationState::set_proof(proof);
    }

    /// Set the active delegation proof from a DTO.
    pub fn set_proof_from_dto(proof: DelegationProof) {
        Self::set_proof(DelegationProofRecordMapper::dto_to_record(proof));
    }

    #[must_use]
    pub fn root_public_key() -> Option<Vec<u8>> {
        DelegationState::get_root_public_key()
    }

    pub fn set_root_public_key(public_key_sec1: Vec<u8>) {
        DelegationState::set_root_public_key(public_key_sec1);
    }

    #[must_use]
    pub fn shard_public_key(shard_pid: Principal) -> Option<Vec<u8>> {
        DelegationState::get_shard_public_key(shard_pid)
    }

    pub fn set_shard_public_key(shard_pid: Principal, public_key_sec1: Vec<u8>) {
        DelegationState::set_shard_public_key(shard_pid, public_key_sec1);
    }
}
