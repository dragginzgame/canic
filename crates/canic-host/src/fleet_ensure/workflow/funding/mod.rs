//! Module: fleet_ensure::workflow::funding
//!
//! Responsibility: review and resume a paused operation's exact Ledger shortfall.
//! Boundary: preserves protocol effects and delegates transfers to the existing adapter.

use crate::fleet_ensure::{
    model::{
        EffectState, EnsureAction, EstateFundingRequiredRecord, EstateFundingReviewRecord,
        FleetEnsureCompletion, FleetEnsureJournalRecord, FleetEnsurePlan, FleetEnsureStateRecord,
    },
    ops::{EnsurePaths, EnsurePlatform, action_sha256, funding as records, write_journal},
    workflow::{EnsureWorkflowError, estate_funding_error, estate_funding_record_is_exact},
};
use std::collections::BTreeSet;

pub(super) fn verify<E: std::error::Error + 'static>(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
) -> Result<(), EnsureWorkflowError<E>> {
    let mut identities = BTreeSet::new();
    let maximum_reviews = plan
        .conservation
        .estate_funding_domains
        .iter()
        .try_fold(0_usize, |count, domain| {
            count
                .checked_add(domain.required_creation_count as usize)?
                .checked_add(1)
        })
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    if journal.funding_reviews.len() > maximum_reviews {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    let mut incomplete = false;
    for review in &journal.funding_reviews {
        let EnsureAction::FundEstate {
            created_at_time, ..
        } = review.action
        else {
            return Err(EnsureWorkflowError::JournalIntegrity);
        };
        let expected = records::review(&review.pause, created_at_time);
        if incomplete
            || review.action != expected.action
            || review.review_sha256 != expected.review_sha256
            || created_at_time == 0
            || !estate_funding_record_is_exact(&review.pause, plan, state)
            || !identities.insert((
                &review.pause.root,
                &review.pause.pending_creation_operation_id,
            ))
        {
            return Err(EnsureWorkflowError::JournalIntegrity);
        }
        if let Some(effect) = &review.effect {
            if effect.action_sha256 != action_sha256(&review.action)
                || effect.destination_pre_cycles != Some(review.pause.available_cycles)
                || effect.pre_cycles.is_none()
                || effect.created_principal.is_some()
                || effect.pre_canister_version.is_some()
                || effect.maintenance_attempts != 0
            {
                return Err(EnsureWorkflowError::JournalIntegrity);
            }
            if effect.state == EffectState::Applied {
                applied(review)?;
            }
        }
        incomplete = review
            .effect
            .as_ref()
            .is_none_or(|effect| effect.state != EffectState::Applied);
        if incomplete
            && (journal.completion != FleetEnsureCompletion::InProgress
                || journal.estate_funding_required.as_ref() != Some(&review.pause))
        {
            return Err(EnsureWorkflowError::JournalIntegrity);
        }
    }
    Ok(())
}

pub(super) fn prepare<P: EnsurePlatform>(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    journal: &mut FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    created_at_time: u64,
    platform: &mut P,
) -> Result<Option<EstateFundingReviewRecord>, EnsureWorkflowError<P::Error>> {
    if let Some(review) = journal.funding_reviews.last()
        && review
            .effect
            .as_ref()
            .is_none_or(|effect| effect.state != EffectState::Applied)
    {
        return Ok(Some(review.clone()));
    }
    let Some(pause) = &journal.estate_funding_required else {
        return Ok(None);
    };
    if covered(journal, pause) {
        return Ok(None);
    }
    let latest = crate::fleet_ensure::workflow::continuation::actions(plan, journal)
        .into_iter()
        .chain(journal.funding_reviews.iter().map(|review| &review.action))
        .filter_map(|action| match action {
            EnsureAction::FundEstate {
                created_at_time, ..
            } => Some(*created_at_time),
            _ => None,
        })
        .max()
        .unwrap_or(plan.planned_at_time);
    let created_at_time = created_at_time.max(
        latest
            .checked_add(1)
            .ok_or(EnsureWorkflowError::JournalIntegrity)?,
    );
    let candidate = records::review(pause, created_at_time);
    verify_balance(plan, journal, state, &candidate, platform)?;
    journal.funding_reviews.push(candidate.clone());
    verify(plan, journal, state)?;
    write_journal(paths, journal)?;
    Ok(Some(candidate))
}

fn verify_balance<P: EnsurePlatform>(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    review: &EstateFundingReviewRecord,
    platform: &mut P,
) -> Result<u128, EnsureWorkflowError<P::Error>> {
    let observation = platform
        .observe(&plan.operation_id, state)
        .map_err(EnsureWorkflowError::Platform)?;
    let pause = &review.pause;
    verify_root_authority(plan, state, &observation, pause)?;
    let domain = observation
        .estate_funding_domains
        .get(&pause.root)
        .ok_or(EnsureWorkflowError::DriftedBeforeApply)?;
    let exact_account = domain.cycles_ledger == pause.cycles_ledger
        && domain.root_principal.as_deref() == Some(pause.root_principal.as_str())
        && domain.balance_cycles == Some(pause.available_cycles)
        && observation.ledger_fee_cycles == pause.ledger_fee_cycles;
    let pending_matches = pause
        .pending_creation_operation_id
        .as_ref()
        .is_none_or(|id| {
            domain
                .pool
                .as_ref()
                .and_then(|pool| pool.pending_creation.as_ref())
                .is_some_and(|pending| &pending.operation_id == id
                    && pending.creation_amount_cycles == pause.creation_amount_cycles
                    && pending.created_principal.is_none()
                    && !pending.uncertain_result
                    && pending.diagnostic == Some(crate::fleet_ensure::model::EstatePoolCreationDiagnostic::FundingRequired))
        });
    let debit = pause
        .shortfall_cycles
        .checked_add(pause.ledger_fee_cycles)
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    // New credits are an exact extension of the original operator debit bound.
    let (funding, fees) = totals::<P::Error>(journal, None)?;
    let maximum = plan
        .conservation
        .maximum_operator_debit_cycles
        .checked_add(funding)
        .and_then(|n| n.checked_add(fees))
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    let prior_debit = journal
        .initial_operator_cycles
        .checked_sub(observation.operator_cycles);
    if !exact_account
        || !pending_matches
        || observation.operator_cycles < debit
        || prior_debit.is_none_or(|spent| spent > maximum)
    {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    let reviewed = plan
        .conservation
        .estate_funding_domains
        .iter()
        .find(|domain| domain.root == pause.root)
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    let creation_debit = if domain.pool.is_some() {
        crate::fleet_ensure::workflow::estate_creation_costs(state, &observation, reviewed)?.0
    } else if reviewed.initial_pool_assets.is_empty()
        && pause.pending_creation_operation_id.is_none()
    {
        0
    } else {
        return Err(EnsureWorkflowError::JournalIntegrity);
    };
    let original_funding = crate::fleet_ensure::workflow::applied_estate_funding_for_domain(
        plan, journal, state, reviewed,
    )?;
    let expected_balance = journal
        .initial_estate_funding_cycles_by_root
        .get(&pause.root)
        .copied()
        .and_then(|n| n.checked_add(original_funding))
        .and_then(|n| n.checked_add(totals::<P::Error>(journal, Some(&pause.root)).ok()?.0))
        .and_then(|n| n.checked_sub(creation_debit));
    if expected_balance != Some(pause.available_cycles) {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    Ok(observation.operator_cycles)
}

#[derive(Eq, PartialEq)]
struct FundingRootAuthority<'a> {
    controllers: &'a [String],
    principal: &'a str,
}

fn verify_root_authority<E: std::error::Error + 'static>(
    plan: &FleetEnsurePlan,
    state: &FleetEnsureStateRecord,
    observation: &crate::fleet_ensure::model::FleetObservation,
    pause: &EstateFundingRequiredRecord,
) -> Result<(), EnsureWorkflowError<E>> {
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?
        .desired();
    let configured = desired
        .canisters
        .iter()
        .find(|configured| configured.name == pause.root)
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let live = observation
        .canisters
        .get(&pause.root)
        .and_then(Option::as_ref)
        .ok_or(EnsureWorkflowError::DriftedBeforeApply)?;
    let mut expected = configured.controllers.clone();
    for controller in &configured.controller_canisters {
        let principal = state
            .pending_principals
            .get(controller)
            .or_else(|| state.principals.get(controller))
            .or_else(|| {
                desired
                    .canisters
                    .iter()
                    .find(|configured| configured.name == *controller)
                    .and_then(|configured| configured.principal.as_ref())
            })
            .ok_or(EnsureWorkflowError::PlanIntegrity)?;
        expected.push(principal.clone());
    }
    expected.sort();
    expected.dedup();
    let mut actual = live.controllers.clone();
    actual.sort();
    let observed = FundingRootAuthority {
        controllers: &actual,
        principal: &live.principal,
    };
    let reviewed = FundingRootAuthority {
        controllers: &expected,
        principal: &pause.root_principal,
    };
    let running = live.status == crate::fleet_ensure::model::CanisterRuntimeStatus::Running
        && !live.reinstall_required;
    if observed != reviewed || !running {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    Ok(())
}

pub(super) fn accepts(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    digest: &str,
) -> bool {
    if journal.plan_sha256 != plan.plan_sha256 || journal.operation_id != plan.operation_id {
        return false;
    }
    journal
        .funding_reviews
        .last()
        .is_some_and(|review| review.review_sha256 == digest)
}

pub(super) fn resume<P: EnsurePlatform>(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    journal: &mut FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    digest: &str,
    platform: &mut P,
) -> Result<(), EnsureWorkflowError<P::Error>> {
    let Some(index) = journal.funding_reviews.len().checked_sub(1) else {
        return Ok(());
    };
    let review = &journal.funding_reviews[index];
    let retained_intent = review.effect.is_some();
    if review
        .effect
        .as_ref()
        .is_some_and(|effect| effect.state == EffectState::Applied)
    {
        return Ok(());
    }
    if review.effect.is_none() {
        if digest != review.review_sha256 {
            return Err(EnsureWorkflowError::EstateFundingRequired(Box::new(
                estate_funding_error(&review.pause),
            )));
        }
        let source = verify_balance(plan, journal, state, review, platform)?;
        let intent = records::intent(&review.action, source, review.pause.available_cycles);
        journal.funding_reviews[index].effect = Some(intent);
        write_journal(paths, journal)?;
    }
    let review = &journal.funding_reviews[index];
    let effect = review
        .effect
        .as_ref()
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    if effect.receipt.is_none() {
        if retained_intent {
            let observation = platform
                .observe(&plan.operation_id, state)
                .map_err(EnsureWorkflowError::Platform)?;
            verify_root_authority(plan, state, &observation, &review.pause)?;
        }
        // Retrying the exact timestamp/account/amount returns the same Ledger block.
        // A lost response retains this intent; no new transfer identity is allocated.
        let outcome = platform
            .apply(&plan.operation_id, &review.action, effect, state)
            .map_err(EnsureWorkflowError::Platform)?;
        if outcome.created_principal.is_some()
            || outcome.receipt.as_ref().is_none_or(String::is_empty)
        {
            return Err(EnsureWorkflowError::JournalIntegrity);
        }
        let effect = journal.funding_reviews[index]
            .effect
            .as_mut()
            .ok_or(EnsureWorkflowError::JournalIntegrity)?;
        effect.receipt = outcome.receipt;
        effect.state = EffectState::Issued;
        write_journal(paths, journal)?;
    }
    let source = platform
        .action_cycles(&journal.funding_reviews[index].action, state)
        .map_err(EnsureWorkflowError::Platform)?;
    let effect = journal.funding_reviews[index]
        .effect
        .as_mut()
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    effect.post_cycles = source;
    effect.state = EffectState::Applied;
    applied::<P::Error>(&journal.funding_reviews[index])?;
    // Root may already have spent the credit on its retained creation. Its Ledger
    // balance is reconciled against exact creation receipts at terminal verification.
    journal.stalled_observations = 0;
    write_journal(paths, journal)?;
    Ok(())
}

fn applied<E: std::error::Error + 'static>(
    review: &EstateFundingReviewRecord,
) -> Result<(), EnsureWorkflowError<E>> {
    let effect = review
        .effect
        .as_ref()
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    let debit = review
        .pause
        .shortfall_cycles
        .checked_add(review.pause.ledger_fee_cycles);
    if effect.state != EffectState::Applied
        || effect.receipt.as_ref().is_none_or(String::is_empty)
        || effect
            .pre_cycles
            .zip(effect.post_cycles)
            .and_then(|(before, after)| before.checked_sub(after))
            != debit
        || debit.is_none()
    {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    Ok(())
}

pub(super) fn covered(
    journal: &FleetEnsureJournalRecord,
    pause: &EstateFundingRequiredRecord,
) -> bool {
    journal.funding_reviews.iter().any(|review| {
        review.pause.root == pause.root
            && review.pause.pending_creation_operation_id == pause.pending_creation_operation_id
            && review
                .effect
                .as_ref()
                .is_some_and(|effect| effect.state == EffectState::Applied)
    })
}

pub(super) fn totals<E: std::error::Error + 'static>(
    journal: &FleetEnsureJournalRecord,
    root: Option<&str>,
) -> Result<(u128, u128), EnsureWorkflowError<E>> {
    journal
        .funding_reviews
        .iter()
        .filter(|review| {
            root.is_none_or(|root| root == review.pause.root) && review.effect.is_some()
        })
        .try_fold((0_u128, 0_u128), |(amount, fees), review| {
            Ok((
                amount
                    .checked_add(review.pause.shortfall_cycles)
                    .ok_or(EnsureWorkflowError::JournalIntegrity)?,
                fees.checked_add(review.pause.ledger_fee_cycles)
                    .ok_or(EnsureWorkflowError::JournalIntegrity)?,
            ))
        })
}
