pub mod mapper;

use crate::{
    cdk::types::Principal,
    dto::auth::AttestationKeySet,
    dto::auth::{AttestationKey, DelegationProof},
    storage::stable::auth::{DelegationProofRecord, DelegationState},
};
use mapper::{AttestationPublicKeyRecordMapper, DelegationProofRecordMapper};

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

    #[must_use]
    pub fn attestation_public_key(key_id: u32) -> Option<AttestationKey> {
        DelegationState::get_attestation_public_key(key_id)
            .map(AttestationPublicKeyRecordMapper::record_to_dto)
    }

    #[must_use]
    pub fn attestation_public_key_sec1(key_id: u32) -> Option<Vec<u8>> {
        Self::attestation_public_key(key_id).map(|entry| entry.public_key)
    }

    #[must_use]
    pub fn attestation_keys() -> Vec<AttestationKey> {
        DelegationState::get_attestation_public_keys()
            .into_iter()
            .map(AttestationPublicKeyRecordMapper::record_to_dto)
            .collect()
    }

    pub fn set_attestation_key_set(key_set: AttestationKeySet) {
        let keys = key_set
            .keys
            .into_iter()
            .map(AttestationPublicKeyRecordMapper::dto_to_record)
            .collect();
        DelegationState::set_attestation_public_keys(keys);
    }

    pub fn upsert_attestation_key(key: AttestationKey) {
        DelegationState::upsert_attestation_public_key(
            AttestationPublicKeyRecordMapper::dto_to_record(key),
        );
    }
}
