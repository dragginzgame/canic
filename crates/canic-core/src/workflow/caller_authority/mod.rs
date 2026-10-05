//! Module: workflow::caller_authority
//!
//! Protected receiver publication orchestration after endpoint authentication.

pub mod admission;

use crate::{
    InternalError,
    config::caller_authority::CompiledCallerPolicy,
    domain::policy::pure::caller_authority::matches_permission,
    dto::caller_authority::{
        CallerAuthorityCommand, CallerAuthorityReceipt, CallerAuthorityStatus,
    },
    model::caller_authority::{
        CallerChangeRecord, CallerPublicationError, CallerPublicationRecord, CallerReceiptRecord,
    },
    ops::{caller_authority::CallerAuthorityOps, config::ConfigOps},
};

pub fn apply(command: CallerAuthorityCommand) -> Result<CallerAuthorityReceipt, InternalError> {
    let receipt = match command {
        CallerAuthorityCommand::Prepare(publication) => ConfigOps::with_caller_policy(|policy| {
            prepare(
                CallerAuthorityOps::publication_from_dto(publication),
                policy,
            )
        })?,
        CallerAuthorityCommand::Commit(publication) => {
            CallerAuthorityOps::commit(&CallerAuthorityOps::publication_from_dto(publication))
        }
        CallerAuthorityCommand::Complete(publication) => {
            CallerAuthorityOps::complete(&CallerAuthorityOps::publication_from_dto(publication))
        }
    }
    .map_err(publication_error)?;
    Ok(CallerAuthorityOps::receipt_to_dto(receipt))
}

/// Decide source eligibility before the storage owner reserves and persists a publication.
pub fn prepare(
    publication: CallerPublicationRecord,
    policy: &CompiledCallerPolicy,
) -> Result<CallerReceiptRecord, CallerPublicationError> {
    let source_permitted = match &publication.change {
        CallerChangeRecord::StageSource(source) | CallerChangeRecord::Grant(source) => {
            policy.configuration.as_ref().is_some_and(|config| {
                config.permissions.keys().any(|permission| {
                    matches_permission(policy, permission, &publication.authority.receiver, source)
                })
            })
        }
        _ => true,
    };
    CallerAuthorityOps::prepare(publication, policy, source_permitted)
}

pub fn publication(
    authority: crate::ids::CallerReceiverAuthority,
    operation_id: [u8; 32],
    previous_generation: u64,
    change: crate::dto::caller_authority::CallerAuthorityChange,
) -> Result<crate::dto::caller_authority::CallerAuthorityPublication, InternalError> {
    CallerAuthorityOps::build_publication(authority, operation_id, previous_generation, change)
        .map_err(publication_error)
}

pub fn status(operation_id: [u8; 32]) -> Result<CallerAuthorityStatus, InternalError> {
    let receiver = CallerAuthorityOps::receiver().ok_or_else(InternalError::unavailable)?;
    let mut status =
        CallerAuthorityOps::status_to_dto(receiver, CallerAuthorityOps::receipt(operation_id));
    if status.readiness == crate::dto::caller_authority::CallerAuthorityReadiness::ApplicationReady
        && !crate::workflow::fixture_provisioning::is_ready(
            &crate::workflow::fixture_provisioning::status(),
        )
    {
        status.readiness = crate::dto::caller_authority::CallerAuthorityReadiness::FrameworkReady;
    }
    Ok(status)
}

pub(in crate::workflow) const fn publication_error(error: CallerPublicationError) -> InternalError {
    use crate::diagnostics::codes;
    InternalError::public(match error {
        CallerPublicationError::AuthorityConflict => codes::AUTHORITY_CONFLICT,
        CallerPublicationError::Capacity => codes::STATE_CAPACITY,
        CallerPublicationError::InProgress => codes::STATE_UNAVAILABLE,
        CallerPublicationError::InvalidPublication => codes::STATE_INVALID,
        CallerPublicationError::Retired => codes::AUTHORITY_INACTIVE,
        CallerPublicationError::GenerationConflict
        | CallerPublicationError::PhaseConflict
        | CallerPublicationError::ReplayConflict => codes::STATE_CONFLICT,
    })
}
