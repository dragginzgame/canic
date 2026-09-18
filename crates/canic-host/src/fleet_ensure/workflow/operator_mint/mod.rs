//! Module: fleet_ensure::workflow::operator_mint
//!
//! Responsibility: retain conversion approval, exact requests and reply observations.
//! Boundary: execution owns payment calls; ops authenticates receipts; pending minting fences effects.

pub mod execution;
mod replies;

use crate::fleet_ensure::{
    model::{
        FleetEnsureCompletion, FleetEnsureJournalRecord, FleetEnsurePlan, FundingReviewRecord,
        operator_mint::{
            OperatorMintAuthority, OperatorMintIntentRecord, OperatorMintReviewRecord,
        },
    },
    ops::{
        EnsurePaths, lock_operation,
        operator_mint::{
            journal as records,
            receipts::{VerifiedCyclesDeposit, icp::VerifiedIcpTransfer},
            transfer_argument,
        },
        read_journal, read_plan, read_state, write_journal,
    },
    policy::operator_mint::{admit_receipt, operator_source, progress},
    workflow::{EnsureWorkflowError, verified_plan, verify_journal_integrity},
};
use std::convert::Infallible;

pub use replies::{prepare_notification, record_notification_reply, record_transfer_reply};

/// Select a read-only fresh quote without hiding an active or retained conversion.
pub fn fresh_quote_available(paths: &EnsurePaths) -> Result<bool, EnsureWorkflowError<Infallible>> {
    let _lock = lock_operation(paths)?;
    if let Some(plan) = crate::fleet_ensure::ops::reinstall::adoption::review(paths)? {
        verified_plan::<Infallible>(plan)?;
        return Ok(true);
    }
    let plan = verified_plan(read_plan(paths)?.ok_or(EnsureWorkflowError::PlanMissing)?)?;
    let Some(journal) = read_journal(paths)? else {
        return Ok(true);
    };
    if journal.completion == FleetEnsureCompletion::InProgress {
        return Ok(false);
    }
    if journal.plan_sha256 != plan.plan_sha256 {
        return Ok(true);
    }
    let state = read_state(paths, &plan.fleet)?;
    verify_journal_integrity(&journal, &plan, &plan.fleet, &state)?;
    Ok(journal
        .funding_reviews
        .last()
        .is_none_or(|funding| funding.operator_mint.is_none()))
}

/// Quote a fresh selected plan without retaining a journal or payment intent.
pub fn quote_fresh(
    paths: &EnsurePaths,
    transport: &crate::fleet_ensure::ops::operator_mint::transport::OperatorMintTransport,
    icp_ledger: candid::Principal,
    cmc: candid::Principal,
) -> Result<
    crate::fleet_ensure::view::operator_mint::FreshOperatorFundingQuote,
    execution::OperatorMintExecutionError,
> {
    let _lock = lock_operation(paths).map_err(EnsureWorkflowError::from)?;
    let staged = crate::fleet_ensure::ops::reinstall::adoption::review(paths)
        .map_err(EnsureWorkflowError::from)?;
    let plan = if let Some(plan) = staged {
        verified_plan::<Infallible>(plan)?
    } else {
        if read_journal(paths)
            .map_err(EnsureWorkflowError::from)?
            .is_some_and(|journal| journal.completion == FleetEnsureCompletion::InProgress)
        {
            return Err(EnsureWorkflowError::OperatorMintReviewConflict.into());
        }
        verified_plan::<Infallible>(
            read_plan(paths)
                .map_err(EnsureWorkflowError::from)?
                .ok_or(EnsureWorkflowError::PlanMissing)?,
        )?
    };
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?
        .desired();
    if transport.operator()?.to_text() != desired.operator {
        return Err(EnsureWorkflowError::OperatorMintReviewConflict.into());
    }
    let ledger = candid::Principal::from_text(&desired.cycles_ledger)
        .map_err(|_| EnsureWorkflowError::PlanIntegrity)?;
    let available = transport.operator_balance(ledger)?;
    let rate = transport.quote_blocking(icp_ledger, cmc, ledger)?;
    let shortfall = plan
        .conservation
        .maximum_operator_debit_cycles
        .saturating_sub(available);
    let amount = if shortfall == 0 {
        None
    } else {
        Some(
            crate::fleet_ensure::policy::operator_mint::quote::amount_e8s(
                shortfall,
                rate.estimated_deposit_fee_cycles,
                rate.xdr_permyriad_per_icp,
                rate.transfer_fee_e8s,
            )
            .ok_or(EnsureWorkflowError::OperatorMintReviewConflict)?,
        )
    };
    records::fresh_quote(&plan, available, rate, amount)
        .ok_or_else(|| EnsureWorkflowError::PlanIntegrity.into())
}

/// Read the retained conversion without triggering another transfer or refreshing it.
pub fn status(
    paths: &EnsurePaths,
) -> Result<Option<OperatorMintReviewRecord>, EnsureWorkflowError<Infallible>> {
    let _lock = lock_operation(paths)?;
    let (_, journal) = load(paths)?;
    Ok(journal
        .funding_reviews
        .last()
        .and_then(|funding| funding.operator_mint.clone()))
}

/// Retain a conversion with an exact reviewed amount, fee and original timestamp.
pub fn review_conversion(
    paths: &EnsurePaths,
    authority: OperatorMintAuthority,
    amount_e8s: u64,
    transfer_fee_e8s: u64,
    created_at_time_ns: u64,
) -> Result<OperatorMintReviewRecord, EnsureWorkflowError<Infallible>> {
    let intent = {
        let _lock = lock_operation(paths)?;
        let (plan, journal) = load(paths)?;
        let funding = journal
            .funding_reviews
            .last()
            .ok_or(EnsureWorkflowError::OperatorMintReviewConflict)?;
        let timestamp = journal
            .funding_reviews
            .iter()
            .filter_map(|review| review.operator_mint.as_ref())
            .map(|review| review.intent.created_at_time_ns)
            .max();
        if timestamp.is_some_and(|previous| created_at_time_ns <= previous)
            && funding.operator_mint.is_none()
        {
            return Err(EnsureWorkflowError::OperatorMintReviewConflict);
        }
        records::reviewed_intent(
            &plan,
            funding,
            authority,
            amount_e8s,
            transfer_fee_e8s,
            created_at_time_ns,
        )?
    };
    review(paths, &intent)
}

/// Retain an effect-free conversion review against the paused funding review.
/// The caller supplies authority derived from its selected network and signer.
pub fn review(
    paths: &EnsurePaths,
    intent: &OperatorMintIntentRecord,
) -> Result<OperatorMintReviewRecord, EnsureWorkflowError<Infallible>> {
    let _lock = lock_operation(paths)?;
    let (plan, mut journal) = load(paths)?;
    let funding = journal
        .funding_reviews
        .last()
        .ok_or(EnsureWorkflowError::OperatorMintReviewConflict)?;
    if journal.completion != FleetEnsureCompletion::InProgress || funding.effect.is_some() {
        return Err(EnsureWorkflowError::OperatorMintReviewConflict);
    }
    validate_binding(&plan, funding, intent)?;
    if let Some(retained) = &funding.operator_mint {
        if retained.intent != *intent {
            return Err(EnsureWorkflowError::OperatorMintReviewConflict);
        }
        return Ok(retained.clone());
    }
    let review = records::review(intent.clone())?;
    records::retain(&mut journal, Some(review.clone()));
    verify(&plan, &journal)?;
    write_journal(paths, &journal)?;
    Ok(review)
}

/// Atomically admit both authenticated transactions once; replay does not rewrite history.
pub fn admit_credit(
    paths: &EnsurePaths,
    authority: &OperatorMintAuthority,
    reviewed_sha256: &str,
    transfer: &VerifiedIcpTransfer,
    deposit: &VerifiedCyclesDeposit,
) -> Result<OperatorMintReviewRecord, EnsureWorkflowError<Infallible>> {
    let _lock = lock_operation(paths)?;
    let (plan, mut journal) = load(paths)?;
    let retained = selected(&journal, authority, reviewed_sha256)?;
    let credited = records::credit(retained, transfer, deposit)
        .ok_or(EnsureWorkflowError::OperatorMintReplyConflict)?;
    if retained.receipt.is_some() {
        return if retained == &credited {
            Ok(credited)
        } else {
            Err(EnsureWorkflowError::OperatorMintReplyConflict)
        };
    }
    if journal.completion != FleetEnsureCompletion::InProgress {
        return Err(EnsureWorkflowError::OperatorMintReviewConflict);
    }
    records::retain(&mut journal, Some(credited.clone()));
    verify(&plan, &journal)?;
    write_journal(paths, &journal)?;
    Ok(credited)
}

/// Persist the exact transfer bytes before a future caller may submit them.
/// Approval repeats bind the same authority and digest and do not rewrite state.
pub fn approve(
    paths: &EnsurePaths,
    authority: &OperatorMintAuthority,
    reviewed_sha256: &str,
) -> Result<OperatorMintReviewRecord, EnsureWorkflowError<Infallible>> {
    let _lock = lock_operation(paths)?;
    let (plan, mut journal) = load(paths)?;
    let retained = selected(&journal, authority, reviewed_sha256)?;
    if retained.transfer_argument.is_some() {
        return Ok(retained.clone());
    }
    let approved = records::approve(retained)?;
    records::retain(&mut journal, Some(approved.clone()));
    verify(&plan, &journal)?;
    write_journal(paths, &journal)?;
    Ok(approved)
}

/// Remove only an unapproved review. A retained transfer intent cannot be cancelled.
pub fn cancel_review(
    paths: &EnsurePaths,
    authority: &OperatorMintAuthority,
    reviewed_sha256: &str,
) -> Result<(), EnsureWorkflowError<Infallible>> {
    let _lock = lock_operation(paths)?;
    let (_, mut journal) = load(paths)?;
    let retained = selected(&journal, authority, reviewed_sha256)?;
    if retained.transfer_argument.is_some() {
        return Err(EnsureWorkflowError::OperatorMintReviewConflict);
    }
    records::retain(&mut journal, None);
    write_journal(paths, &journal)?;
    Ok(())
}

fn selected<'a>(
    journal: &'a FleetEnsureJournalRecord,
    authority: &OperatorMintAuthority,
    reviewed_sha256: &str,
) -> Result<&'a OperatorMintReviewRecord, EnsureWorkflowError<Infallible>> {
    let retained = journal
        .funding_reviews
        .last()
        .and_then(|r| r.operator_mint.as_ref())
        .ok_or(EnsureWorkflowError::OperatorMintReviewConflict)?;
    if retained.intent.authority != *authority || retained.review_sha256 != reviewed_sha256 {
        return Err(EnsureWorkflowError::OperatorMintReviewConflict);
    }
    Ok(retained)
}

fn load(
    paths: &EnsurePaths,
) -> Result<(FleetEnsurePlan, FleetEnsureJournalRecord), EnsureWorkflowError<Infallible>> {
    let plan = verified_plan(read_plan(paths)?.ok_or(EnsureWorkflowError::PlanMissing)?)?;
    let journal = read_journal(paths)?.ok_or(EnsureWorkflowError::OperatorMintReviewConflict)?;
    let state = read_state(paths, &plan.fleet)?;
    verify_journal_integrity(&journal, &plan, &plan.fleet, &state)?;
    Ok((plan, journal))
}

pub(super) fn verify<E: std::error::Error + 'static>(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
) -> Result<(), EnsureWorkflowError<E>> {
    let mut admitted = Vec::new();
    for (index, funding) in journal.funding_reviews.iter().enumerate() {
        let Some(mint) = &funding.operator_mint else {
            continue;
        };
        if mint.receipt.is_none()
            && (index + 1 != journal.funding_reviews.len()
                || funding.effect.is_some()
                || journal.completion != FleetEnsureCompletion::InProgress)
        {
            return Err(EnsureWorkflowError::JournalIntegrity);
        }
        validate_binding(plan, funding, &mint.intent)?;
        let expected = records::review(mint.intent.clone())?;
        if !progress::valid(mint)
            || !progress::receipt_matches(mint)
            || !records::notification_matches(mint)
        {
            return Err(EnsureWorkflowError::JournalIntegrity);
        }
        if expected.review_sha256 != mint.review_sha256
            || mint.transfer_argument.as_ref().is_some_and(|bytes| {
                transfer_argument(&mint.intent).map_or(true, |expected| expected != *bytes)
            })
        {
            return Err(EnsureWorkflowError::JournalIntegrity);
        }
        if let Some(receipt) = &mint.receipt {
            admit_receipt(&mint.intent, receipt, &admitted)
                .map_err(|_| EnsureWorkflowError::JournalIntegrity)?;
            admitted.push(receipt.clone());
        }
    }
    operator_source(journal).ok_or(EnsureWorkflowError::JournalIntegrity)?;
    Ok(())
}

fn validate_binding<E: std::error::Error + 'static>(
    plan: &FleetEnsurePlan,
    funding: &FundingReviewRecord,
    intent: &OperatorMintIntentRecord,
) -> Result<(), EnsureWorkflowError<E>> {
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?
        .desired();
    let authority_matches = records::binds_plan(intent, plan, &funding.review_sha256)
        && intent.authority.operator.to_text() == desired.operator
        && intent.authority.cycles_ledger.to_text() == funding.pause.cycles_ledger();
    if !authority_matches {
        return Err(EnsureWorkflowError::OperatorMintReviewConflict);
    }
    Ok(())
}
