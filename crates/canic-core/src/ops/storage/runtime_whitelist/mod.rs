//! Module: ops::storage::runtime_whitelist
//!
//! Responsibility: access and convert complete runtime-whitelist state atomically.
//! Does not own: mutation policy, lifecycle order, endpoint authorization, or pagination.
//! Boundary: workflow receives model state and commits only policy-produced replacements.

use crate::{
    InternalError,
    dto::runtime_whitelist::{RuntimeWhitelistMutationOutcome, RuntimeWhitelistMutationResponse},
    model::runtime_whitelist::{
        RuntimeWhitelistMutationOutcomeModel, RuntimeWhitelistMutationResponseModel,
        RuntimeWhitelistOperation, RuntimeWhitelistState,
    },
    storage::stable::runtime_whitelist::{
        RuntimeWhitelistMutationOutcomeRecord, RuntimeWhitelistMutationResponseRecord,
        RuntimeWhitelistOperationRecord, RuntimeWhitelistRecord, RuntimeWhitelistStore,
    },
};

/// Deterministic storage facade for memory ID 61.
pub struct RuntimeWhitelistOps;

impl RuntimeWhitelistOps {
    /// Load the optional record without inventing bootstrap authority.
    pub(crate) fn load() -> Option<RuntimeWhitelistState> {
        RuntimeWhitelistStore::get().map(record_to_model)
    }

    /// Commit the fresh-install record exactly once.
    pub(crate) fn initialize(state: RuntimeWhitelistState) -> Result<(), InternalError> {
        if RuntimeWhitelistStore::initialize(model_to_record(state)) {
            Ok(())
        } else {
            Err(InternalError::conflict())
        }
    }

    /// Atomically replace an existing complete record.
    pub(crate) fn replace(state: RuntimeWhitelistState) -> Result<(), InternalError> {
        if RuntimeWhitelistStore::replace(model_to_record(state)) {
            Ok(())
        } else {
            Err(InternalError::unavailable())
        }
    }

    /// Project one internal accepted response onto the public wire contract.
    pub(crate) const fn response_to_dto(
        response: RuntimeWhitelistMutationResponseModel,
    ) -> RuntimeWhitelistMutationResponse {
        RuntimeWhitelistMutationResponse {
            outcome: model_outcome_to_dto(response.outcome),
            principal: response.principal,
            revision: response.revision,
            membership_digest: response.membership_digest,
        }
    }
}

fn record_to_model(record: RuntimeWhitelistRecord) -> RuntimeWhitelistState {
    RuntimeWhitelistState {
        schema_version: record.schema_version,
        principals: record.principals,
        revision: record.revision,
        membership_digest: record.membership_digest,
        last_operation: record
            .last_operation
            .map(|operation| RuntimeWhitelistOperation {
                operation_id: operation.operation_id,
                request_hash: operation.request_hash,
                response: RuntimeWhitelistMutationResponseModel {
                    outcome: record_outcome_to_model(operation.result.outcome),
                    principal: operation.result.principal,
                    revision: operation.result.revision,
                    membership_digest: operation.result.membership_digest,
                },
            }),
    }
}

fn model_to_record(state: RuntimeWhitelistState) -> RuntimeWhitelistRecord {
    RuntimeWhitelistRecord {
        schema_version: state.schema_version,
        principals: state.principals,
        revision: state.revision,
        membership_digest: state.membership_digest,
        last_operation: state
            .last_operation
            .map(|operation| RuntimeWhitelistOperationRecord {
                operation_id: operation.operation_id,
                request_hash: operation.request_hash,
                result: RuntimeWhitelistMutationResponseRecord {
                    outcome: model_outcome_to_record(operation.response.outcome),
                    principal: operation.response.principal,
                    revision: operation.response.revision,
                    membership_digest: operation.response.membership_digest,
                },
            }),
    }
}

const fn record_outcome_to_model(
    outcome: RuntimeWhitelistMutationOutcomeRecord,
) -> RuntimeWhitelistMutationOutcomeModel {
    match outcome {
        RuntimeWhitelistMutationOutcomeRecord::Added => RuntimeWhitelistMutationOutcomeModel::Added,
        RuntimeWhitelistMutationOutcomeRecord::AlreadyPresent => {
            RuntimeWhitelistMutationOutcomeModel::AlreadyPresent
        }
        RuntimeWhitelistMutationOutcomeRecord::Removed => {
            RuntimeWhitelistMutationOutcomeModel::Removed
        }
        RuntimeWhitelistMutationOutcomeRecord::AlreadyAbsent => {
            RuntimeWhitelistMutationOutcomeModel::AlreadyAbsent
        }
    }
}

const fn model_outcome_to_record(
    outcome: RuntimeWhitelistMutationOutcomeModel,
) -> RuntimeWhitelistMutationOutcomeRecord {
    match outcome {
        RuntimeWhitelistMutationOutcomeModel::Added => RuntimeWhitelistMutationOutcomeRecord::Added,
        RuntimeWhitelistMutationOutcomeModel::AlreadyPresent => {
            RuntimeWhitelistMutationOutcomeRecord::AlreadyPresent
        }
        RuntimeWhitelistMutationOutcomeModel::Removed => {
            RuntimeWhitelistMutationOutcomeRecord::Removed
        }
        RuntimeWhitelistMutationOutcomeModel::AlreadyAbsent => {
            RuntimeWhitelistMutationOutcomeRecord::AlreadyAbsent
        }
    }
}

const fn model_outcome_to_dto(
    outcome: RuntimeWhitelistMutationOutcomeModel,
) -> RuntimeWhitelistMutationOutcome {
    match outcome {
        RuntimeWhitelistMutationOutcomeModel::Added => RuntimeWhitelistMutationOutcome::Added,
        RuntimeWhitelistMutationOutcomeModel::AlreadyPresent => {
            RuntimeWhitelistMutationOutcome::AlreadyPresent
        }
        RuntimeWhitelistMutationOutcomeModel::Removed => RuntimeWhitelistMutationOutcome::Removed,
        RuntimeWhitelistMutationOutcomeModel::AlreadyAbsent => {
            RuntimeWhitelistMutationOutcome::AlreadyAbsent
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal,
        domain::policy::pure::runtime_whitelist::{bootstrap, mutate},
        model::runtime_whitelist::RuntimeWhitelistAction,
    };

    #[test]
    fn stable_conversion_preserves_complete_canonical_authority() {
        let state = bootstrap(vec![Principal::from_slice(&[1; 29])]).expect("bootstrap");
        let accepted = mutate(
            &state,
            RuntimeWhitelistAction::Add,
            Principal::from_slice(&[2; 29]),
            0,
            [3; 32],
        )
        .expect("accepted mutation")
        .state;

        assert_eq!(record_to_model(model_to_record(accepted.clone())), accepted);
    }

    #[test]
    fn model_response_projects_to_the_exact_public_outcome() {
        let response = RuntimeWhitelistMutationResponseModel {
            outcome: RuntimeWhitelistMutationOutcomeModel::Removed,
            principal: Principal::from_slice(&[4; 29]),
            revision: 7,
            membership_digest: [5; 32],
        };

        assert_eq!(
            RuntimeWhitelistOps::response_to_dto(response),
            RuntimeWhitelistMutationResponse {
                outcome: RuntimeWhitelistMutationOutcome::Removed,
                principal: Principal::from_slice(&[4; 29]),
                revision: 7,
                membership_digest: [5; 32],
            }
        );
    }
}
