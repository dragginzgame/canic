pub mod mapper;

use crate::{
    dto::auth::DelegationProof,
    storage::stable::auth::{DelegationProofRecord, DelegationState, DelegationStateRecord},
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
    /// Export the full delegation state.
    ///
    /// Intended usage:
    /// - Stable memory migration
    /// - Snapshotting for upgrades
    ///
    /// MUST NOT be used during request handling or verification.
    #[must_use]
    #[expect(dead_code)]
    pub fn data() -> DelegationStateRecord {
        DelegationState::export()
    }

    /// Import a previously exported delegation state.
    ///
    /// Intended usage:
    /// - Post-upgrade restoration
    /// - Controlled administrative recovery
    ///
    /// Callers MUST ensure the imported data has already been validated.
    #[expect(dead_code)]
    pub fn import(data: DelegationStateRecord) {
        DelegationState::import(data);
    }

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

    /// Clear the active delegation proof.
    ///
    /// Intended usage:
    /// - Emergency revocation
    /// - Controlled teardown during tests
    ///
    /// After this call, all delegated token verification MUST fail.
    #[allow(dead_code)]
    pub fn clear_proof() {
        DelegationState::clear_proof();
    }
}
