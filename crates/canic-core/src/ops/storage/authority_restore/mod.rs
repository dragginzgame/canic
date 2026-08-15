//! Module: ops::storage::authority_restore
//!
//! Responsibility: validate and commit authority snapshot-seal transitions.
//! Does not own: IC history observation, endpoint authentication, or timer suspension.
//! Boundary: workflow supplies independently observed history and ambient authority identity.

#[cfg(test)]
mod tests;

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    dto::authority_restore::{
        AuthorityRestoreFencePhase, AuthorityRestoreFenceStatusResponse, AuthoritySnapshotRequest,
    },
    storage::stable::authority_restore::{
        AuthorityRestoreFenceRecord, AuthorityRestoreFenceStateRecord, AuthorityRestoreFenceStore,
        AuthorityRestoreResumeReceiptRecord,
    },
};

/// Deterministic storage owner for the authority snapshot/restore fence.
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
            Some(_) => Err(InternalError::conflict(
                "authority restore fence is already initialized for different authority",
            )),
            None => Err(InternalError::invariant(
                InternalErrorOrigin::Ops,
                "authority restore fence initialization did not persist",
            )),
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
            AuthorityRestoreFenceStateRecord::Sealed { .. } => Err(InternalError::conflict(
                "authority is sealed by a different snapshot operation",
            )),
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
            AuthorityRestoreFenceStateRecord::Open { .. } => Err(InternalError::conflict(
                "authority snapshot operation is not sealed",
            )),
            AuthorityRestoreFenceStateRecord::Sealed { operation_id, .. }
                if operation_id != request.operation_id =>
            {
                Err(InternalError::conflict(
                    "authority snapshot resume names a different sealed operation",
                ))
            }
            AuthorityRestoreFenceStateRecord::Sealed {
                history_total_num_changes: sealed_history,
                ..
            } if sealed_history != history_total_num_changes => Err(InternalError::unavailable(
                "authority management history advanced after the snapshot seal; restored or ambiguous authority remains mutation-fenced",
            )),
            AuthorityRestoreFenceStateRecord::Sealed { .. } => Ok(()),
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
            AuthorityRestoreFenceStateRecord::Sealed { .. } => Err(InternalError::conflict(
                "authority is sealed by a different snapshot operation",
            )),
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
            AuthorityRestoreFenceStateRecord::Open { .. } => Err(InternalError::conflict(
                "authority snapshot operation is not sealed",
            )),
            AuthorityRestoreFenceStateRecord::Sealed { operation_id, .. }
                if *operation_id != request.operation_id =>
            {
                Err(InternalError::conflict(
                    "authority snapshot resume names a different sealed operation",
                ))
            }
            AuthorityRestoreFenceStateRecord::Sealed {
                history_total_num_changes: sealed_history,
                ..
            } if *sealed_history != history_total_num_changes => Err(InternalError::unavailable(
                "authority management history advanced after the snapshot seal; restored or ambiguous authority remains mutation-fenced",
            )),
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
        }
    }

    /// Return validated sealed state for the ambient authority Canister.
    pub fn is_sealed_for(authority_canister: Principal) -> Result<bool, InternalError> {
        let record = require_authority(authority_canister)?;
        Ok(matches!(
            record.state,
            AuthorityRestoreFenceStateRecord::Sealed { .. }
        ))
    }
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
        return Err(InternalError::conflict(
            "authority restore fence is bound to a different Canister",
        ));
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
    };
    AuthorityRestoreFenceStatusResponse {
        authority_canister: record.authority_canister,
        phase,
        operation_id,
        history_total_num_changes,
        changed_at_ns,
    }
}

fn fence_uninitialized() -> InternalError {
    InternalError::invariant(
        InternalErrorOrigin::Ops,
        "authority restore fence is not initialized",
    )
}
