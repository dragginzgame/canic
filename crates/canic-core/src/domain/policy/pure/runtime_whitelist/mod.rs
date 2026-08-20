//! Module: domain::policy::pure::runtime_whitelist
//!
//! Responsibility: decide canonical whitelist bootstrap, validation, mutation and replay.
//! Does not own: storage, IC calls, endpoint authorization, DTO conversion, or serialization.
//! Boundary: callers provide complete model state and commit only returned complete states.

use crate::{
    cdk::types::Principal,
    dto::runtime_whitelist::RuntimeWhitelistMutationOutcome,
    model::runtime_whitelist::{
        MAX_RUNTIME_WHITELIST_PRINCIPALS, RUNTIME_WHITELIST_SCHEMA_VERSION, RuntimeWhitelistAction,
        RuntimeWhitelistMutationResponseModel, RuntimeWhitelistOperation, RuntimeWhitelistState,
    },
};
use sha2::{Digest, Sha256};
use thiserror::Error as ThisError;

const MEMBERSHIP_DOMAIN: &[u8] = b"canic.runtime-whitelist.membership.v1\0";
const OPERATION_DOMAIN: &[u8] = b"canic.runtime-whitelist.operation.v1\0";

/// Pure runtime-whitelist invariant or request rejection.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ThisError)]
pub enum RuntimeWhitelistPolicyError {
    #[error("runtime whitelist operation ID is all zero")]
    EmptyOperationId,
    #[error("runtime whitelist operation ID was reused for another request")]
    OperationConflict,
    #[error("runtime whitelist expected revision does not match")]
    RevisionConflict,
    #[error("runtime whitelist principal capacity is exhausted")]
    CapacityExhausted,
    #[error("runtime whitelist revision is exhausted")]
    RevisionExhausted,
    #[error("runtime whitelist schema is unsupported")]
    UnsupportedSchema,
    #[error("runtime whitelist principals are not canonical")]
    NonCanonicalPrincipals,
    #[error("runtime whitelist membership digest is invalid")]
    MembershipDigestMismatch,
    #[error("runtime whitelist retained operation is inconsistent")]
    InvalidRetainedOperation,
}

/// Complete result of one accepted request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeWhitelistMutationDecision {
    pub state: RuntimeWhitelistState,
    pub response: RuntimeWhitelistMutationResponseModel,
    pub replayed: bool,
}

/// Build the fresh-install authority from the compiled seed.
pub fn bootstrap(
    mut principals: Vec<Principal>,
) -> Result<RuntimeWhitelistState, RuntimeWhitelistPolicyError> {
    principals.sort_by(|left, right| left.as_slice().cmp(right.as_slice()));
    principals.dedup();
    if principals.len() > MAX_RUNTIME_WHITELIST_PRINCIPALS {
        return Err(RuntimeWhitelistPolicyError::CapacityExhausted);
    }
    let membership_digest = membership_digest(&principals);
    Ok(RuntimeWhitelistState {
        schema_version: RUNTIME_WHITELIST_SCHEMA_VERSION,
        principals,
        revision: 0,
        membership_digest,
        last_operation: None,
    })
}

/// Validate one restored canonical state without repairing it.
pub fn validate(state: &RuntimeWhitelistState) -> Result<(), RuntimeWhitelistPolicyError> {
    if state.schema_version != RUNTIME_WHITELIST_SCHEMA_VERSION {
        return Err(RuntimeWhitelistPolicyError::UnsupportedSchema);
    }
    if state.principals.len() > MAX_RUNTIME_WHITELIST_PRINCIPALS
        || state
            .principals
            .windows(2)
            .any(|pair| pair[0].as_slice().cmp(pair[1].as_slice()) != std::cmp::Ordering::Less)
    {
        return Err(RuntimeWhitelistPolicyError::NonCanonicalPrincipals);
    }
    if state.membership_digest != membership_digest(&state.principals) {
        return Err(RuntimeWhitelistPolicyError::MembershipDigestMismatch);
    }
    if let Some(operation) = &state.last_operation
        && (operation.operation_id == [0; 32]
            || operation.response.revision != state.revision
            || operation.response.membership_digest != state.membership_digest)
    {
        return Err(RuntimeWhitelistPolicyError::InvalidRetainedOperation);
    }
    if let Some(operation) = &state.last_operation {
        let present = state
            .principals
            .binary_search_by(|candidate| {
                candidate
                    .as_slice()
                    .cmp(operation.response.principal.as_slice())
            })
            .is_ok();
        let outcome_consistent = match operation.response.outcome {
            RuntimeWhitelistMutationOutcome::Added
            | RuntimeWhitelistMutationOutcome::AlreadyPresent => present,
            RuntimeWhitelistMutationOutcome::Removed
            | RuntimeWhitelistMutationOutcome::AlreadyAbsent => !present,
        };
        if !outcome_consistent {
            return Err(RuntimeWhitelistPolicyError::InvalidRetainedOperation);
        }
        let (action, expected_revision) = match operation.response.outcome {
            RuntimeWhitelistMutationOutcome::Added => (
                RuntimeWhitelistAction::Add,
                state
                    .revision
                    .checked_sub(1)
                    .ok_or(RuntimeWhitelistPolicyError::InvalidRetainedOperation)?,
            ),
            RuntimeWhitelistMutationOutcome::AlreadyPresent => {
                (RuntimeWhitelistAction::Add, state.revision)
            }
            RuntimeWhitelistMutationOutcome::Removed => (
                RuntimeWhitelistAction::Remove,
                state
                    .revision
                    .checked_sub(1)
                    .ok_or(RuntimeWhitelistPolicyError::InvalidRetainedOperation)?,
            ),
            RuntimeWhitelistMutationOutcome::AlreadyAbsent => {
                (RuntimeWhitelistAction::Remove, state.revision)
            }
        };
        if operation.request_hash
            != operation_request_hash(action, operation.response.principal, expected_revision)
        {
            return Err(RuntimeWhitelistPolicyError::InvalidRetainedOperation);
        }
    }
    Ok(())
}

/// Decide one add/remove request without touching storage.
pub fn mutate(
    state: &RuntimeWhitelistState,
    action: RuntimeWhitelistAction,
    principal: Principal,
    expected_revision: u64,
    operation_id: [u8; 32],
) -> Result<RuntimeWhitelistMutationDecision, RuntimeWhitelistPolicyError> {
    validate(state)?;
    if operation_id == [0; 32] {
        return Err(RuntimeWhitelistPolicyError::EmptyOperationId);
    }
    let request_hash = operation_request_hash(action, principal, expected_revision);
    if let Some(retained) = &state.last_operation
        && retained.operation_id == operation_id
    {
        if retained.request_hash != request_hash {
            return Err(RuntimeWhitelistPolicyError::OperationConflict);
        }
        return Ok(RuntimeWhitelistMutationDecision {
            state: state.clone(),
            response: retained.response.clone(),
            replayed: true,
        });
    }
    if expected_revision != state.revision {
        return Err(RuntimeWhitelistPolicyError::RevisionConflict);
    }

    let mut principals = state.principals.clone();
    let position =
        principals.binary_search_by(|candidate| candidate.as_slice().cmp(principal.as_slice()));
    let (outcome, changed) = match (action, position) {
        (RuntimeWhitelistAction::Add, Ok(_)) => {
            (RuntimeWhitelistMutationOutcome::AlreadyPresent, false)
        }
        (RuntimeWhitelistAction::Add, Err(index)) => {
            if principals.len() == MAX_RUNTIME_WHITELIST_PRINCIPALS {
                return Err(RuntimeWhitelistPolicyError::CapacityExhausted);
            }
            principals.insert(index, principal);
            (RuntimeWhitelistMutationOutcome::Added, true)
        }
        (RuntimeWhitelistAction::Remove, Ok(index)) => {
            principals.remove(index);
            (RuntimeWhitelistMutationOutcome::Removed, true)
        }
        (RuntimeWhitelistAction::Remove, Err(_)) => {
            (RuntimeWhitelistMutationOutcome::AlreadyAbsent, false)
        }
    };
    let revision = if changed {
        state
            .revision
            .checked_add(1)
            .ok_or(RuntimeWhitelistPolicyError::RevisionExhausted)?
    } else {
        state.revision
    };
    let membership_digest = membership_digest(&principals);
    let response = RuntimeWhitelistMutationResponseModel {
        outcome,
        principal,
        revision,
        membership_digest,
    };
    let next = RuntimeWhitelistState {
        schema_version: RUNTIME_WHITELIST_SCHEMA_VERSION,
        principals,
        revision,
        membership_digest,
        last_operation: Some(RuntimeWhitelistOperation {
            operation_id,
            request_hash,
            response: response.clone(),
        }),
    };
    Ok(RuntimeWhitelistMutationDecision {
        state: next,
        response,
        replayed: false,
    })
}

/// Compute the canonical membership digest.
#[must_use]
pub fn membership_digest(principals: &[Principal]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(MEMBERSHIP_DOMAIN);
    let count = u32::try_from(principals.len()).unwrap_or(u32::MAX);
    hasher.update(count.to_be_bytes());
    for principal in principals {
        let bytes = principal.as_slice();
        let length = u8::try_from(bytes.len()).expect("IC principals fit one-byte length");
        hasher.update([length]);
        hasher.update(bytes);
    }
    hasher.finalize().into()
}

/// Compute the canonical accepted-operation request hash.
#[must_use]
pub fn operation_request_hash(
    action: RuntimeWhitelistAction,
    principal: Principal,
    expected_revision: u64,
) -> [u8; 32] {
    let bytes = principal.as_slice();
    let length = u8::try_from(bytes.len()).expect("IC principals fit one-byte length");
    let mut hasher = Sha256::new();
    hasher.update(OPERATION_DOMAIN);
    hasher.update([action.hash_byte(), length]);
    hasher.update(bytes);
    hasher.update(expected_revision.to_be_bytes());
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal(byte: u8) -> Principal {
        Principal::from_slice(&[byte; 29])
    }

    #[test]
    fn bootstrap_sorts_collapses_and_hashes_the_seed() {
        let state =
            bootstrap(vec![principal(2), principal(1), principal(2)]).expect("canonical bootstrap");
        assert_eq!(state.principals, vec![principal(1), principal(2)]);
        assert_eq!(state.revision, 0);
        assert_eq!(
            state.membership_digest,
            membership_digest(&state.principals)
        );
        validate(&state).expect("valid state");
    }

    #[test]
    fn effective_and_idempotent_mutations_retain_exact_replay() {
        let state = bootstrap(vec![principal(1)]).expect("bootstrap");
        let added = mutate(
            &state,
            RuntimeWhitelistAction::Add,
            principal(2),
            0,
            [1; 32],
        )
        .expect("add");
        assert_eq!(
            added.response.outcome,
            RuntimeWhitelistMutationOutcome::Added
        );
        assert_eq!(added.response.revision, 1);
        let replay = mutate(
            &added.state,
            RuntimeWhitelistAction::Add,
            principal(2),
            0,
            [1; 32],
        )
        .expect("exact replay");
        assert!(replay.replayed);
        assert_eq!(replay.response, added.response);
        assert_eq!(replay.state, added.state);

        let present = mutate(
            &added.state,
            RuntimeWhitelistAction::Add,
            principal(2),
            1,
            [2; 32],
        )
        .expect("idempotent add");
        assert_eq!(
            present.response.outcome,
            RuntimeWhitelistMutationOutcome::AlreadyPresent
        );
        assert_eq!(present.response.revision, 1);

        let removed = mutate(
            &present.state,
            RuntimeWhitelistAction::Remove,
            principal(2),
            1,
            [3; 32],
        )
        .expect("remove");
        assert_eq!(
            removed.response.outcome,
            RuntimeWhitelistMutationOutcome::Removed
        );
        assert_eq!(removed.response.revision, 2);
        validate(&removed.state).expect("removed state remains canonical");

        let absent = mutate(
            &removed.state,
            RuntimeWhitelistAction::Remove,
            principal(2),
            2,
            [4; 32],
        )
        .expect("idempotent remove");
        assert_eq!(
            absent.response.outcome,
            RuntimeWhitelistMutationOutcome::AlreadyAbsent
        );
        assert_eq!(absent.response.revision, 2);
        validate(&absent.state).expect("idempotent result remains canonical");
    }

    #[test]
    fn conflicts_capacity_and_corruption_leave_no_accepted_decision() {
        let mut state = bootstrap(vec![principal(1)]).expect("bootstrap");
        let accepted = mutate(
            &state,
            RuntimeWhitelistAction::Add,
            principal(2),
            0,
            [1; 32],
        )
        .expect("accepted");
        assert_eq!(
            mutate(
                &accepted.state,
                RuntimeWhitelistAction::Remove,
                principal(2),
                0,
                [1; 32],
            ),
            Err(RuntimeWhitelistPolicyError::OperationConflict)
        );
        assert_eq!(
            mutate(
                &accepted.state,
                RuntimeWhitelistAction::Remove,
                principal(2),
                0,
                [3; 32],
            ),
            Err(RuntimeWhitelistPolicyError::RevisionConflict)
        );
        assert_eq!(
            mutate(
                &accepted.state,
                RuntimeWhitelistAction::Remove,
                principal(2),
                1,
                [0; 32],
            ),
            Err(RuntimeWhitelistPolicyError::EmptyOperationId)
        );

        state.principals = (0..MAX_RUNTIME_WHITELIST_PRINCIPALS)
            .map(|index| {
                let index = u16::try_from(index).expect("whitelist fixture index fits u16");
                Principal::from_slice(&index.to_be_bytes())
            })
            .collect();
        state.membership_digest = membership_digest(&state.principals);
        assert_eq!(
            mutate(
                &state,
                RuntimeWhitelistAction::Add,
                Principal::from_slice(&[0xff; 29]),
                0,
                [4; 32],
            ),
            Err(RuntimeWhitelistPolicyError::CapacityExhausted)
        );

        state.membership_digest = [9; 32];
        assert_eq!(
            validate(&state),
            Err(RuntimeWhitelistPolicyError::MembershipDigestMismatch)
        );

        let mut corrupt_operation = accepted.state;
        corrupt_operation
            .last_operation
            .as_mut()
            .expect("accepted operation")
            .request_hash = [0x7f; 32];
        assert_eq!(
            validate(&corrupt_operation),
            Err(RuntimeWhitelistPolicyError::InvalidRetainedOperation)
        );
    }
}
