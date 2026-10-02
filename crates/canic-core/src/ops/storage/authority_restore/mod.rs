//! Module: ops::storage::authority_restore
//!
//! Responsibility: validate and commit authority snapshot/release-seal transitions.
//! Does not own: IC history observation, endpoint authentication, or timer suspension.
//! Boundary: workflow supplies independently observed history and ambient authority identity.

#[cfg(test)]
mod tests;

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::authority_restore::{
        AuthorityReleaseRequest, AuthorityRestoreFencePhase, AuthorityRestoreFenceStatusResponse,
        AuthoritySnapshotRequest,
    },
    storage::stable::authority_restore::{
        AuthorityRestoreFenceRecord, AuthorityRestoreFenceStateRecord, AuthorityRestoreFenceStore,
        AuthorityRestoreResumeReceiptRecord,
    },
    view::authority_restore::AuthorityMutationFence,
};

/// Deterministic storage owner for the authority snapshot/release fence.
pub struct AuthorityRestoreFenceOps;

impl AuthorityRestoreFenceOps {
    /// Initialize one fresh authority canister in the mutation-open phase.
    pub fn initialize(authority_canister: Principal) -> Result<(), InternalError> {
        let record = AuthorityRestoreFenceRecord {
            authority_canister,
            state: AuthorityRestoreFenceStateRecord::Open { last_resume: None },
        };
        if AuthorityRestoreFenceStore::initialize(record.clone()) {
            return Ok(());
        }
        match AuthorityRestoreFenceStore::get() {
            Some(existing) if existing == record => Ok(()),
            Some(_) => Err(InternalError::conflict()),
            None => Err(InternalError::invariant()),
        }
    }

    /// Return the exact durable fence projection.
    pub fn status() -> Result<AuthorityRestoreFenceStatusResponse, InternalError> {
        AuthorityRestoreFenceStore::get()
            .map(record_to_status)
            .ok_or_else(fence_uninitialized)
    }

    /// Validate a snapshot seal without changing durable authority state.
    pub fn validate_prepare(
        request: AuthoritySnapshotRequest,
        authority_canister: Principal,
    ) -> Result<(), InternalError> {
        require_operation_id(request.operation_id)?;
        let record = require_authority(authority_canister)?;
        match record.state {
            AuthorityRestoreFenceStateRecord::Open { .. } => Ok(()),
            AuthorityRestoreFenceStateRecord::Sealed { operation_id, .. }
                if operation_id == request.operation_id =>
            {
                Ok(())
            }
            AuthorityRestoreFenceStateRecord::Sealed { .. }
            | AuthorityRestoreFenceStateRecord::ReleaseSealed { .. } => {
                Err(InternalError::conflict())
            }
        }
    }

    /// Validate a live resume without opening durable authority state.
    pub fn validate_resume(
        request: AuthoritySnapshotRequest,
        authority_canister: Principal,
        history_total_num_changes: u64,
    ) -> Result<(), InternalError> {
        require_operation_id(request.operation_id)?;
        let record = require_authority(authority_canister)?;
        match record.state {
            AuthorityRestoreFenceStateRecord::Open {
                last_resume: Some(receipt),
            } if receipt.operation_id == request.operation_id => Ok(()),
            AuthorityRestoreFenceStateRecord::Open { .. } => Err(InternalError::conflict()),
            AuthorityRestoreFenceStateRecord::Sealed { operation_id, .. }
                if operation_id != request.operation_id =>
            {
                Err(InternalError::conflict())
            }
            AuthorityRestoreFenceStateRecord::Sealed {
                history_total_num_changes: sealed_history,
                ..
            } if sealed_history != history_total_num_changes => Err(InternalError::unavailable()),
            AuthorityRestoreFenceStateRecord::Sealed { .. } => Ok(()),
            AuthorityRestoreFenceStateRecord::ReleaseSealed { .. } => {
                Err(InternalError::conflict())
            }
        }
    }

    /// Seal one authority snapshot operation before the external stop/capture sequence.
    pub fn prepare(
        request: AuthoritySnapshotRequest,
        authority_canister: Principal,
        history_total_num_changes: u64,
        sealed_at_ns: u64,
    ) -> Result<AuthorityRestoreFenceStatusResponse, InternalError> {
        require_operation_id(request.operation_id)?;
        let mut record = require_authority(authority_canister)?;
        match &record.state {
            AuthorityRestoreFenceStateRecord::Open { .. } => {
                record.state = AuthorityRestoreFenceStateRecord::Sealed {
                    operation_id: request.operation_id,
                    history_total_num_changes,
                    sealed_at_ns,
                };
                replace(record)
            }
            AuthorityRestoreFenceStateRecord::Sealed { operation_id, .. }
                if *operation_id == request.operation_id =>
            {
                Ok(record_to_status(record))
            }
            AuthorityRestoreFenceStateRecord::Sealed { .. }
            | AuthorityRestoreFenceStateRecord::ReleaseSealed { .. } => {
                Err(InternalError::conflict())
            }
        }
    }

    /// Resume only the live authority whose management history still matches the seal.
    pub fn resume(
        request: AuthoritySnapshotRequest,
        authority_canister: Principal,
        history_total_num_changes: u64,
        resumed_at_ns: u64,
    ) -> Result<AuthorityRestoreFenceStatusResponse, InternalError> {
        require_operation_id(request.operation_id)?;
        let mut record = require_authority(authority_canister)?;
        match &record.state {
            AuthorityRestoreFenceStateRecord::Open {
                last_resume: Some(receipt),
            } if receipt.operation_id == request.operation_id => Ok(record_to_status(record)),
            AuthorityRestoreFenceStateRecord::Open { .. } => Err(InternalError::conflict()),
            AuthorityRestoreFenceStateRecord::Sealed { operation_id, .. }
                if *operation_id != request.operation_id =>
            {
                Err(InternalError::conflict())
            }
            AuthorityRestoreFenceStateRecord::Sealed {
                history_total_num_changes: sealed_history,
                ..
            } if *sealed_history != history_total_num_changes => Err(InternalError::unavailable()),
            AuthorityRestoreFenceStateRecord::Sealed { .. } => {
                record.state = AuthorityRestoreFenceStateRecord::Open {
                    last_resume: Some(AuthorityRestoreResumeReceiptRecord {
                        operation_id: request.operation_id,
                        history_total_num_changes,
                        resumed_at_ns,
                    }),
                };
                replace(record)
            }
            AuthorityRestoreFenceStateRecord::ReleaseSealed { .. } => {
                Err(InternalError::conflict())
            }
        }
    }

    /// Validate the exact release identity before any role owner suspends producers.
    pub fn validate_release(
        request: AuthorityReleaseRequest,
        authority_canister: Principal,
    ) -> Result<(), InternalError> {
        require_release_request(request, authority_canister)?;
        let record = require_authority(authority_canister)?;
        match record.state {
            AuthorityRestoreFenceStateRecord::Open { .. } => Ok(()),
            AuthorityRestoreFenceStateRecord::ReleaseSealed {
                operation_id,
                review_sha256,
                recipient,
                ..
            } if request
                == (AuthorityReleaseRequest {
                    operation_id,
                    review_sha256,
                    recipient,
                }) =>
            {
                Ok(())
            }
            _ => Err(InternalError::conflict()),
        }
    }

    /// Commit only after synchronous role quiescence; this performs no IC effect.
    /// Release has no snapshot-resume transition, even with the same operation ID.
    pub fn seal_release(
        request: AuthorityReleaseRequest,
        authority_canister: Principal,
        sealed_at_ns: u64,
    ) -> Result<AuthorityRestoreFenceStatusResponse, InternalError> {
        Self::validate_release(request, authority_canister)?;
        let mut record = require_authority(authority_canister)?;
        if matches!(
            record.state,
            AuthorityRestoreFenceStateRecord::ReleaseSealed { .. }
        ) {
            return Ok(record_to_status(record));
        }
        record.state = AuthorityRestoreFenceStateRecord::ReleaseSealed {
            operation_id: request.operation_id,
            review_sha256: request.review_sha256,
            recipient: request.recipient,
            sealed_at_ns,
        };
        replace(record)
    }

    /// Return validated sealed state for the ambient authority Canister.
    pub fn is_sealed_for(authority_canister: Principal) -> Result<bool, InternalError> {
        Ok(Self::mutation_fence_for(authority_canister)? != AuthorityMutationFence::Open)
    }

    /// Project purpose without granting access from the presence of any seal.
    pub fn mutation_fence_for(
        authority_canister: Principal,
    ) -> Result<AuthorityMutationFence, InternalError> {
        let record = require_authority(authority_canister)?;
        Ok(match record.state {
            AuthorityRestoreFenceStateRecord::Open { .. } => AuthorityMutationFence::Open,
            AuthorityRestoreFenceStateRecord::Sealed { .. } => AuthorityMutationFence::Snapshot,
            AuthorityRestoreFenceStateRecord::ReleaseSealed { .. } => {
                AuthorityMutationFence::Release
            }
        })
    }
}

fn require_release_request(
    request: AuthorityReleaseRequest,
    authority_canister: Principal,
) -> Result<(), InternalError> {
    require_operation_id(request.operation_id)?;
    if request.review_sha256 == [0; 32]
        || [
            Principal::anonymous(),
            Principal::management_canister(),
            authority_canister,
        ]
        .contains(&request.recipient)
    {
        return Err(InternalError::invalid_input());
    }
    Ok(())
}

fn require_operation_id(operation_id: [u8; 32]) -> Result<(), InternalError> {
    if operation_id == [0; 32] {
        return Err(InternalError::operation_id_required());
    }
    Ok(())
}

fn require_authority(
    authority_canister: Principal,
) -> Result<AuthorityRestoreFenceRecord, InternalError> {
    let record = AuthorityRestoreFenceStore::get().ok_or_else(fence_uninitialized)?;
    if record.authority_canister != authority_canister {
        return Err(InternalError::conflict());
    }
    Ok(record)
}

fn replace(
    record: AuthorityRestoreFenceRecord,
) -> Result<AuthorityRestoreFenceStatusResponse, InternalError> {
    if !AuthorityRestoreFenceStore::replace(record.clone()) {
        return Err(fence_uninitialized());
    }
    Ok(record_to_status(record))
}

const fn record_to_status(
    record: AuthorityRestoreFenceRecord,
) -> AuthorityRestoreFenceStatusResponse {
    let (phase, operation_id, history_total_num_changes, changed_at_ns) = match record.state {
        AuthorityRestoreFenceStateRecord::Open { last_resume: None } => {
            (AuthorityRestoreFencePhase::Open, None, None, None)
        }
        AuthorityRestoreFenceStateRecord::Open {
            last_resume: Some(receipt),
        } => (
            AuthorityRestoreFencePhase::Open,
            Some(receipt.operation_id),
            Some(receipt.history_total_num_changes),
            Some(receipt.resumed_at_ns),
        ),
        AuthorityRestoreFenceStateRecord::Sealed {
            operation_id,
            history_total_num_changes,
            sealed_at_ns,
        } => (
            AuthorityRestoreFencePhase::Sealed,
            Some(operation_id),
            Some(history_total_num_changes),
            Some(sealed_at_ns),
        ),
        AuthorityRestoreFenceStateRecord::ReleaseSealed {
            operation_id,
            review_sha256,
            recipient,
            sealed_at_ns,
        } => (
            AuthorityRestoreFencePhase::ReleaseSealed {
                review_sha256,
                recipient,
            },
            Some(operation_id),
            None,
            Some(sealed_at_ns),
        ),
    };
    AuthorityRestoreFenceStatusResponse {
        authority_canister: record.authority_canister,
        phase,
        operation_id,
        history_total_num_changes,
        changed_at_ns,
    }
}

const fn fence_uninitialized() -> InternalError {
    InternalError::invariant()
}
