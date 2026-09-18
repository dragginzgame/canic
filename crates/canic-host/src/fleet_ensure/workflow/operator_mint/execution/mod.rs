//! Module: fleet_ensure::workflow::operator_mint::execution
//!
//! Responsibility: resume one approved conversion through authenticated receipt admission.
//! Boundary: never resumes Fleet withdrawals or substitutes a new payment identity.

use crate::fleet_ensure::{
    model::operator_mint::{
        OperatorMintAuthority, OperatorMintNotificationOutcomeRecord, OperatorMintReviewRecord,
    },
    ops::{
        EnsurePaths, lock_operation,
        operator_mint::transport::{OperatorMintTransport, OperatorMintTransportError},
    },
    policy::operator_mint::progress,
    workflow::{
        EnsureWorkflowError,
        operator_mint::{self, load, selected},
    },
};
use std::convert::Infallible;
use thiserror::Error;

///
/// OperatorMintExecutionError
///
/// Workflow-owned retained conversion failure; an unknown outcome never permits re-payment.
///
#[derive(Debug, Error)]
pub enum OperatorMintExecutionError {
    #[error("operator mint async runtime could not start")]
    Runtime(#[from] std::io::Error),
    #[error(transparent)]
    Transport(Box<OperatorMintTransportError>),
    #[error(transparent)]
    Workflow(Box<EnsureWorkflowError<Infallible>>),
}

impl From<OperatorMintTransportError> for OperatorMintExecutionError {
    fn from(error: OperatorMintTransportError) -> Self {
        Self::Transport(Box::new(error))
    }
}

impl From<EnsureWorkflowError<Infallible>> for OperatorMintExecutionError {
    fn from(error: EnsureWorkflowError<Infallible>) -> Self {
        Self::Workflow(Box::new(error))
    }
}

/// Synchronous command adapter for the same retained conversion workflow.
pub fn apply_blocking(
    paths: &EnsurePaths,
    authority: &OperatorMintAuthority,
    digest: &str,
    transport: &OperatorMintTransport,
) -> Result<OperatorMintReviewRecord, OperatorMintExecutionError> {
    tokio::runtime::Runtime::new()?.block_on(apply(paths, authority, digest, transport))
}

/// Approve and resume exactly one conversion.
///
/// A returned review without a receipt
/// remains unresolved; callers must display its typed transfer/notification outcomes.
/// A credited replay returns before any network call or journal write.
pub async fn apply(
    paths: &EnsurePaths,
    authority: &OperatorMintAuthority,
    digest: &str,
    transport: &OperatorMintTransport,
) -> Result<OperatorMintReviewRecord, OperatorMintExecutionError> {
    let (retained, source, maximum_debit) = {
        let _lock = lock_operation(paths).map_err(EnsureWorkflowError::from)?;
        let (plan, journal) = load(paths)?;
        let source = crate::fleet_ensure::policy::operator_mint::operator_source(&journal)
            .ok_or(EnsureWorkflowError::JournalIntegrity)?;
        let maximum = crate::fleet_ensure::workflow::funding_plan::<Infallible>(&plan, &journal)?
            .conservation
            .maximum_operator_debit_cycles;
        (
            selected(&journal, authority, digest)?.clone(),
            source,
            maximum,
        )
    };
    if retained.receipt.is_some() {
        return Ok(retained);
    }
    if transport.operator()? != authority.operator
        || transport.network_identity() != authority.network_identity_sha256
    {
        return Err(OperatorMintTransportError::ReaderMismatch.into());
    }
    if retained.transfer_argument.is_none() {
        let observed = transport
            .read_operator_balance(authority.cycles_ledger)
            .await?;
        if source
            .checked_sub(observed)
            .is_none_or(|debit| debit > maximum_debit)
        {
            return Err(EnsureWorkflowError::DriftedBeforeApply.into());
        }
    }
    let mut retained = operator_mint::approve(paths, authority, digest)?;
    if retained
        .transfer_outcome
        .as_ref()
        .and_then(progress::transfer_block)
        .is_none()
    {
        let argument = retained
            .transfer_argument
            .as_ref()
            .ok_or(EnsureWorkflowError::OperatorMintReviewConflict)?;
        let reply = transport.transfer(&retained.intent, argument).await?;
        retained = operator_mint::record_transfer_reply(paths, authority, digest, &reply)?;
    }
    let Some(block) = retained
        .transfer_outcome
        .as_ref()
        .and_then(progress::transfer_block)
    else {
        return Ok(retained);
    };
    let transfer = transport.read_transfer(&retained.intent, block).await?;
    retained = operator_mint::prepare_notification(paths, authority, digest)?;
    let notification = retained
        .notification
        .as_ref()
        .ok_or(EnsureWorkflowError::OperatorMintReviewConflict)?;
    if !matches!(
        notification.outcome,
        Some(
            OperatorMintNotificationOutcomeRecord::Minted { .. }
                | OperatorMintNotificationOutcomeRecord::Refunded { .. }
                | OperatorMintNotificationOutcomeRecord::TransactionTooOld { .. }
                | OperatorMintNotificationOutcomeRecord::InvalidTransaction { .. }
        )
    ) {
        let reply = transport.notify(&transfer, &notification.argument).await?;
        retained = operator_mint::record_notification_reply(paths, authority, digest, &reply)?;
    }
    let Some(outcome @ OperatorMintNotificationOutcomeRecord::Minted { .. }) = retained
        .notification
        .as_ref()
        .and_then(|n| n.outcome.as_ref())
    else {
        return Ok(retained);
    };
    let deposit = transport.read_deposit(&retained.intent, outcome).await?;
    Ok(operator_mint::admit_credit(
        paths, authority, digest, &transfer, &deposit,
    )?)
}
