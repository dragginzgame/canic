//! Module: workflow::runtime_whitelist
//!
//! Responsibility: coordinate seed, restore, mutation, exact retry and bounded status.
//! Does not own: endpoint authentication, stable encoding, config parsing, or pure policy.
//! Boundary: managed-role lifecycle and API facades call this after caller authorization.

use crate::{
    InternalError,
    cdk::types::Principal,
    domain::policy::pure::runtime_whitelist::{
        RuntimeWhitelistPolicyError, bootstrap, mutate, validate,
    },
    dto::{
        page::{Page, PageRequest},
        runtime_whitelist::{
            RuntimeWhitelistCommand, RuntimeWhitelistMutationRequest,
            RuntimeWhitelistMutationResponse, RuntimeWhitelistStatusResponse,
        },
    },
    model::runtime_whitelist::{
        MAX_RUNTIME_WHITELIST_PAGE, MAX_RUNTIME_WHITELIST_PRINCIPALS, RuntimeWhitelistAction,
    },
    ops::{config::ConfigOps, storage::runtime_whitelist::RuntimeWhitelistOps},
};

/// Orchestrator for the one managed-role runtime-whitelist authority.
pub struct RuntimeWhitelistWorkflow;

impl RuntimeWhitelistWorkflow {
    /// Seed a fresh managed installation exactly once from compiled configuration.
    pub(crate) fn initialize_from_compiled_seed() -> Result<(), InternalError> {
        let seed = ConfigOps::runtime_whitelist_seed()?;
        let state = bootstrap(seed).map_err(policy_error)?;
        RuntimeWhitelistOps::initialize(state)
    }

    /// Validate same-release restored state without reseeding or repair.
    pub(crate) fn restore() -> Result<(), InternalError> {
        let state = require_state()?;
        validate(&state).map_err(policy_error)
    }

    /// Apply or exactly replay one accepted mutation.
    pub(crate) fn command(
        command: RuntimeWhitelistCommand,
    ) -> Result<RuntimeWhitelistMutationResponse, InternalError> {
        let (action, request) = match command {
            RuntimeWhitelistCommand::Add(request) => (RuntimeWhitelistAction::Add, request),
            RuntimeWhitelistCommand::Remove(request) => (RuntimeWhitelistAction::Remove, request),
        };
        let RuntimeWhitelistMutationRequest {
            principal,
            expected_revision,
            operation_id,
        } = request;
        let state = require_state()?;
        let decision = mutate(&state, action, principal, expected_revision, operation_id)
            .map_err(policy_error)?;
        if !decision.replayed {
            RuntimeWhitelistOps::replace(decision.state)?;
        }
        Ok(RuntimeWhitelistOps::response_to_dto(decision.response))
    }

    /// Return one bounded canonical membership page.
    pub(crate) fn status(
        request: PageRequest,
    ) -> Result<RuntimeWhitelistStatusResponse, InternalError> {
        let state = require_state()?;
        validate(&state).map_err(policy_error)?;
        status_from_state(&state, request)
    }

    /// Read membership without mutation or compiled-config fallback.
    pub(crate) fn contains(principal: Principal) -> Result<bool, InternalError> {
        let state = require_state()?;
        validate(&state).map_err(policy_error)?;
        Ok(state
            .principals
            .binary_search_by(|candidate| candidate.as_slice().cmp(principal.as_slice()))
            .is_ok())
    }
}

fn status_from_state(
    state: &crate::model::runtime_whitelist::RuntimeWhitelistState,
    request: PageRequest,
) -> Result<RuntimeWhitelistStatusResponse, InternalError> {
    let total = u64::try_from(state.principals.len()).map_err(|_| InternalError::invariant())?;
    let limit = request.limit.min(MAX_RUNTIME_WHITELIST_PAGE);
    let entries = usize::try_from(request.offset)
        .ok()
        .filter(|offset| *offset < state.principals.len())
        .map_or_else(Vec::new, |offset| {
            let take = usize::try_from(limit).expect("page limit fits usize");
            state
                .principals
                .iter()
                .skip(offset)
                .take(take)
                .copied()
                .collect()
        });
    Ok(RuntimeWhitelistStatusResponse {
        principals: Page { entries, total },
        revision: state.revision,
        membership_digest: state.membership_digest,
        maximum_principals: u16::try_from(MAX_RUNTIME_WHITELIST_PRINCIPALS)
            .expect("runtime whitelist maximum fits u16"),
    })
}

fn require_state() -> Result<crate::model::runtime_whitelist::RuntimeWhitelistState, InternalError>
{
    RuntimeWhitelistOps::load().ok_or_else(InternalError::unavailable)
}

const fn policy_error(error: RuntimeWhitelistPolicyError) -> InternalError {
    use crate::diagnostics::codes;
    match error {
        RuntimeWhitelistPolicyError::EmptyOperationId => {
            InternalError::public(codes::REQUEST_INCOMPLETE)
        }
        RuntimeWhitelistPolicyError::OperationConflict => {
            InternalError::public(codes::REQUEST_CONFLICT)
        }
        RuntimeWhitelistPolicyError::RevisionConflict => {
            InternalError::public(codes::VERSION_CONFLICT)
        }
        RuntimeWhitelistPolicyError::CapacityExhausted => InternalError::resource_exhausted(),
        RuntimeWhitelistPolicyError::RevisionExhausted => {
            InternalError::public(codes::VERSION_CAPACITY)
        }
        RuntimeWhitelistPolicyError::UnsupportedSchema
        | RuntimeWhitelistPolicyError::NonCanonicalPrincipals
        | RuntimeWhitelistPolicyError::MembershipDigestMismatch
        | RuntimeWhitelistPolicyError::InvalidRetainedOperation => InternalError::invariant(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::policy::pure::runtime_whitelist::bootstrap;

    #[test]
    fn status_pages_are_canonical_clamped_and_bounded() {
        let principals = (0..=MAX_RUNTIME_WHITELIST_PAGE)
            .map(|index| Principal::from_slice(&index.to_be_bytes()))
            .collect();
        let state = bootstrap(principals).expect("bounded canonical state");

        let first = status_from_state(
            &state,
            PageRequest {
                offset: 0,
                limit: u64::MAX,
            },
        )
        .expect("clamped first page");
        assert_eq!(first.principals.total, MAX_RUNTIME_WHITELIST_PAGE + 1);
        assert_eq!(first.principals.entries.len(), 128);

        let empty = status_from_state(
            &state,
            PageRequest {
                offset: u64::MAX,
                limit: 1,
            },
        )
        .expect("unrepresentable or out-of-range offset is empty");
        assert!(empty.principals.entries.is_empty());

        let zero = status_from_state(
            &state,
            PageRequest {
                offset: 0,
                limit: 0,
            },
        )
        .expect("zero limit is valid");
        assert!(zero.principals.entries.is_empty());
    }
}
