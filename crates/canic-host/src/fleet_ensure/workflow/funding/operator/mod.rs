//! Module: fleet_ensure::workflow::funding::operator
//!
//! Responsibility: bind conversion recovery to the original first withdrawal.
//! Boundary: never resets its intent, proves nonpayment or submits another withdrawal.

use crate::fleet_ensure::{
    model::{
        EffectState, EnsureAction, FleetEnsureCompletion, FleetEnsureJournalRecord,
        FleetEnsurePlan, FundingPauseRecord, FundingReviewRecord,
    },
    ops::{EnsurePaths, EnsurePlatform, action_sha256, funding as records, write_journal},
    policy::operator_mint::operator_source,
    workflow::{EnsureWorkflowError, ordered_actions},
};
use canic_core::cdk::types::Cycles;

pub(super) fn applicable(plan: &FleetEnsurePlan, journal: &FleetEnsureJournalRecord) -> bool {
    let Some(effect) = journal.effects.first() else {
        return false;
    };
    journal.completion == FleetEnsureCompletion::InProgress
        && journal.effects.len() == 1
        && effect.state == EffectState::Intent
        && effect.receipt.is_none()
        && journal.initial_operator_cycles < plan.conservation.maximum_operator_debit_cycles
        && ordered_actions(plan).first().is_some_and(|action| {
            matches!(action, EnsureAction::Fund { .. })
                && action_sha256(action) == effect.action_sha256
        })
}

pub(super) fn verify<E: std::error::Error + 'static>(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
) -> Result<(), EnsureWorkflowError<E>> {
    let Some(desired) = plan
        .reviewed_desired
        .as_ref()
        .map(|reviewed| reviewed.desired())
    else {
        if journal
            .funding_reviews
            .iter()
            .any(|review| matches!(review.pause, FundingPauseRecord::Operator(_)))
        {
            return Err(EnsureWorkflowError::JournalIntegrity);
        }
        return Ok(());
    };
    for review in &journal.funding_reviews {
        let FundingPauseRecord::Operator(pause) = &review.pause else {
            continue;
        };
        let action = ordered_actions(plan)
            .first()
            .copied()
            .ok_or(EnsureWorkflowError::JournalIntegrity)?;
        let fee = desired
            .ledger_fee_cycles
            .parse::<Cycles>()
            .map_err(|_| EnsureWorkflowError::PlanIntegrity)?
            .to_u128();
        let expected = records::operator_review(
            action,
            &desired.operator,
            pause.available_cycles,
            plan.conservation.maximum_operator_debit_cycles,
            fee,
        )
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
        let exact = review.action == expected.action
            && review.pause == expected.pause
            && review.review_sha256 == expected.review_sha256
            && review.effect.is_none()
            && pause.cycles_ledger == desired.cycles_ledger
            && journal.initial_operator_cycles < plan.conservation.maximum_operator_debit_cycles;
        let retained = journal
            .effects
            .first()
            .is_some_and(|effect| effect.action_sha256 == pause.action_sha256);
        if !exact || !retained {
            return Err(EnsureWorkflowError::JournalIntegrity);
        }
    }
    Ok(())
}

pub(super) fn prepare<P: EnsurePlatform>(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    journal: &mut FleetEnsureJournalRecord,
    platform: &mut P,
) -> Result<Option<FundingReviewRecord>, EnsureWorkflowError<P::Error>> {
    if !applicable(plan, journal) {
        return Ok(None);
    }
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?
        .desired();
    let observed = platform
        .observe_operator_funding()
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::DriftedBeforeApply)?;
    let fee = desired
        .ledger_fee_cycles
        .parse::<Cycles>()
        .map_err(|_| EnsureWorkflowError::PlanIntegrity)?
        .to_u128();
    let source = operator_source(journal).ok_or(EnsureWorkflowError::JournalIntegrity)?;
    // The original receipt-less request remains ambiguous. A debit may already have
    // happened; unexplained credits must never become a new starting balance.
    if observed.cycles_ledger != desired.cycles_ledger
        || observed.ledger_fee_cycles != fee
        || source
            .checked_sub(observed.operator_cycles)
            .is_none_or(|debit| debit > plan.conservation.maximum_operator_debit_cycles)
    {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    if observed.operator_cycles >= plan.conservation.maximum_operator_debit_cycles {
        return Ok(None);
    }
    let action = ordered_actions(plan)
        .first()
        .copied()
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    let review = records::operator_review(
        action,
        &desired.operator,
        observed.operator_cycles,
        plan.conservation.maximum_operator_debit_cycles,
        fee,
    )
    .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    journal.funding_reviews.push(review.clone());
    verify(plan, journal)?;
    write_journal(paths, journal)?;
    Ok(Some(review))
}
