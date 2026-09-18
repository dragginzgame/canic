//! Module: fleet_ensure::workflow::funding::native
//!
//! Responsibility: review one Root credit within an issued provisioning operation.
//! Boundary: existing Ledger withdrawal adapter owns effects; starting balances stay fixed.

use crate::fleet_ensure::{
    model::{
        CurrentFleetProtocolAction, DesiredCanisterKind, DesiredPresence, EffectState,
        EnsureAction, FleetEnsureCompletion, FleetEnsureJournalRecord, FleetEnsurePlan,
        FleetEnsureStateRecord, FundingPauseRecord, FundingReviewRecord, NativeFundingObservation,
        NativeFundingRequiredRecord,
    },
    ops::{EnsurePaths, EnsurePlatform, action_sha256, funding as records, write_journal},
    policy::startup_funding::recovery_minimum_cycles,
    workflow::{EnsureWorkflowError, continuation, funding, reviewed_estate_root_principal},
};
use canic_core::cdk::types::Cycles;
use std::collections::BTreeSet;

pub(super) fn applicable(plan: &FleetEnsurePlan, journal: &FleetEnsureJournalRecord) -> bool {
    issued_provisioning(plan, journal).is_some()
        || journal
            .funding_reviews
            .iter()
            .any(|review| matches!(review.pause, FundingPauseRecord::Native(_)))
}

fn issued_provisioning<'a>(
    plan: &'a FleetEnsurePlan,
    journal: &'a FleetEnsureJournalRecord,
) -> Option<&'a EnsureAction> {
    if journal.completion != FleetEnsureCompletion::InProgress {
        return None;
    }
    let index = journal.effects.len().checked_sub(1)?;
    if journal.effects[index].state != EffectState::Issued {
        return None;
    }
    let action = *continuation::actions(plan, journal).get(index)?;
    is_provisioning(action).then_some(action)
}

fn is_provisioning(action: &EnsureAction) -> bool {
    matches!(action, EnsureAction::FleetProtocol { action, .. }
        if matches!(action.as_ref(), CurrentFleetProtocolAction::ProvisionComponents { .. }))
}

fn selects_root(action: &EnsureAction, root: &str) -> bool {
    let EnsureAction::FleetProtocol { action, .. } = action else {
        return false;
    };
    let CurrentFleetProtocolAction::ProvisionComponents { request, .. } = action.as_ref() else {
        return false;
    };
    request
        .plan
        .batches
        .iter()
        .any(|batch| batch.root.fleet_subnet_root.to_text() == root)
        || request
            .plan
            .directory_confirmation_roots
            .iter()
            .any(|principal| principal.to_text() == root)
}

pub(super) fn verify<E: std::error::Error + 'static>(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
) -> Result<(), EnsureWorkflowError<E>> {
    let actions = continuation::actions(plan, journal);
    let mut identities = BTreeSet::new();
    for review in &journal.funding_reviews {
        let FundingPauseRecord::Native(pause) = &review.pause else {
            continue;
        };
        let Some((action, effect)) =
            actions
                .iter()
                .zip(&journal.effects)
                .find(|(action, effect)| {
                    is_provisioning(action)
                        && effect.action_sha256 == pause.provisioning_action_sha256
                        && action_sha256(action) == pause.provisioning_action_sha256
                })
        else {
            return Err(EnsureWorkflowError::JournalIntegrity);
        };
        let (minimum, margin, ledger, fee) = bounds(plan, &pause.root)?;
        let root = plan
            .conservation
            .estate_funding_domains
            .iter()
            .find(|domain| domain.root == pause.root)
            .and_then(|domain| reviewed_estate_root_principal(domain, state));
        let amount = minimum
            .checked_sub(pause.available_cycles)
            .filter(|n| *n > 0)
            .and_then(|deficit| deficit.checked_add(margin));
        let expected = NativeFundingAuthority {
            cycles_ledger: ledger,
            funding_margin_cycles: margin,
            ledger_fee_cycles: fee,
            minimum_cycles: minimum,
            operation_id: &plan.operation_id,
            plan_sha256: &plan.plan_sha256,
            root_principal: root,
        };
        let exact = NativeFundingAuthority::from(pause) == expected
            && selects_root(action, &pause.root_principal)
            && amount == Some(pause.shortfall_cycles)
            && minimum.checked_add(margin).is_some();
        let EnsureAction::Fund {
            created_at_time, ..
        } = review.action
        else {
            return Err(EnsureWorkflowError::JournalIntegrity);
        };
        if !exact
            || created_at_time == 0
            || effect.state == EffectState::Intent
            || !identities.insert((&pause.root, &pause.provisioning_action_sha256))
        {
            return Err(EnsureWorkflowError::JournalIntegrity);
        }
        let expected = records::native_review(pause.clone(), created_at_time);
        if review.action != expected.action || review.review_sha256 != expected.review_sha256 {
            return Err(EnsureWorkflowError::JournalIntegrity);
        }
        verify_receipt::<E>(review, pause)?;
        if review
            .effect
            .as_ref()
            .is_none_or(|effect| effect.state != EffectState::Applied)
            && (journal.completion != FleetEnsureCompletion::InProgress
                || effect.state != EffectState::Issued)
        {
            return Err(EnsureWorkflowError::JournalIntegrity);
        }
    }
    Ok(())
}

#[derive(Eq, PartialEq)]
struct NativeFundingAuthority<'a> {
    cycles_ledger: &'a str,
    funding_margin_cycles: u128,
    ledger_fee_cycles: u128,
    minimum_cycles: u128,
    operation_id: &'a str,
    plan_sha256: &'a str,
    root_principal: Option<&'a str>,
}

impl<'a> From<&'a NativeFundingRequiredRecord> for NativeFundingAuthority<'a> {
    fn from(pause: &'a NativeFundingRequiredRecord) -> Self {
        Self {
            cycles_ledger: &pause.cycles_ledger,
            funding_margin_cycles: pause.funding_margin_cycles,
            ledger_fee_cycles: pause.ledger_fee_cycles,
            minimum_cycles: pause.minimum_cycles,
            operation_id: &pause.operation_id,
            plan_sha256: &pause.plan_sha256,
            root_principal: Some(&pause.root_principal),
        }
    }
}

fn verify_receipt<E: std::error::Error + 'static>(
    review: &FundingReviewRecord,
    pause: &NativeFundingRequiredRecord,
) -> Result<(), EnsureWorkflowError<E>> {
    if let Some(effect) = &review.effect {
        if effect.action_sha256 != action_sha256(&review.action)
            || effect.destination_pre_cycles.is_none_or(|before| {
                pause.available_cycles.saturating_sub(before) > pause.funding_margin_cycles
            })
            || effect.pre_cycles.is_none()
            || effect.created_principal.is_some()
            || effect.pre_canister_version.is_some()
            || effect.progress_identity.is_some()
            || effect.publication_attempts != 0
            || effect.maintenance_attempts != 0
        {
            return Err(EnsureWorkflowError::JournalIntegrity);
        }
        let valid_receipt = effect
            .receipt
            .as_ref()
            .is_some_and(|receipt| receipt.parse::<candid::Nat>().is_ok());
        let shape_is_exact = match effect.state {
            EffectState::Intent => {
                effect.receipt.is_none()
                    && effect.post_cycles.is_none()
                    && effect.destination_post_cycles.is_none()
            }
            EffectState::Issued => {
                valid_receipt
                    && effect.post_cycles.is_none()
                    && effect.destination_post_cycles.is_none()
            }
            EffectState::Applied => {
                valid_receipt
                    && effect.post_cycles.is_some()
                    && effect.destination_post_cycles.is_some()
            }
        };
        if !shape_is_exact {
            return Err(EnsureWorkflowError::JournalIntegrity);
        }
        if effect.state == EffectState::Applied {
            funding::applied::<E>(review)?;
        }
    }
    Ok(())
}

fn bounds<'a, E: std::error::Error + 'static>(
    plan: &'a FleetEnsurePlan,
    root: &str,
) -> Result<(u128, u128, &'a str, u128), EnsureWorkflowError<E>> {
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?
        .desired();
    let configured = desired
        .canisters
        .iter()
        .find(|configured| {
            configured.name == root
                && configured.kind == DesiredCanisterKind::Root
                && configured.presence == DesiredPresence::Present
        })
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let parse = |value: &str| {
        value
            .parse::<Cycles>()
            .map(|n| n.to_u128())
            .map_err(|_| EnsureWorkflowError::PlanIntegrity)
    };
    let minimum = recovery_minimum_cycles(desired, root, parse(&configured.minimum_cycles)?)?;
    let margin = parse(&desired.maximum_update_burn_cycles)?
        .checked_add(parse(&desired.maximum_observation_burn_cycles)?)
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    Ok((
        minimum,
        margin,
        &desired.cycles_ledger,
        parse(&desired.ledger_fee_cycles)?,
    ))
}

pub(super) fn refresh<P: EnsurePlatform>(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    journal: &mut FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    created_at_time: u64,
    platform: &mut P,
) -> Result<Option<FundingReviewRecord>, EnsureWorkflowError<P::Error>> {
    let review = journal
        .funding_reviews
        .last()
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    let FundingPauseRecord::Native(pause) = &review.pause else {
        return Err(EnsureWorkflowError::JournalIntegrity);
    };
    if review.effect.is_some() {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    let live = platform
        .observe_native_funding(&pause.root, state)
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::DriftedBeforeApply)?;
    funding::verify_root_authority(plan, state, &live.live, &pause.root, &pause.root_principal)?;
    let within_margin = live.live.cycles <= pause.available_cycles
        && pause.available_cycles - live.live.cycles <= pause.funding_margin_cycles;
    if within_margin {
        return Ok(Some(review.clone()));
    }
    let EnsureAction::Fund {
        created_at_time: previous_time,
        ..
    } = review.action
    else {
        return Err(EnsureWorkflowError::JournalIntegrity);
    };
    let timestamp = created_at_time.max(
        previous_time
            .checked_add(1)
            .ok_or(EnsureWorkflowError::JournalIntegrity)?,
    );
    // A quote has no approval or effect. Replace only a disposable copy until its
    // replacement passes review; an approved intent never enters this path.
    let mut replacement = journal.clone();
    replacement.funding_reviews.pop();
    let refreshed = prepare(paths, plan, &mut replacement, state, timestamp, platform)?;
    if refreshed.is_none() {
        write_journal(paths, &replacement)?;
    }
    *journal = replacement;
    Ok(refreshed)
}

pub(super) fn prepare<P: EnsurePlatform>(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    journal: &mut FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    created_at_time: u64,
    platform: &mut P,
) -> Result<Option<FundingReviewRecord>, EnsureWorkflowError<P::Error>> {
    let Some(action) = issued_provisioning(plan, journal) else {
        return Ok(None);
    };
    let action_hash = action_sha256(action);
    for domain in &plan.conservation.estate_funding_domains {
        if journal.funding_reviews.iter().any(|review| {
            matches!(&review.pause, FundingPauseRecord::Native(pause)
            if pause.root == domain.root && pause.provisioning_action_sha256 == action_hash)
        }) {
            continue;
        }
        let Some(principal) = reviewed_estate_root_principal(domain, state) else {
            continue;
        };
        if !selects_root(action, principal) {
            continue;
        }
        let (minimum, margin, ledger, fee) = bounds(plan, &domain.root)?;
        let Some(observation) = platform
            .observe_native_funding(&domain.root, state)
            .map_err(EnsureWorkflowError::Platform)?
        else {
            return Ok(None);
        };
        if observation.live.cycles >= minimum {
            continue;
        }
        let amount = minimum
            .checked_sub(observation.live.cycles)
            .and_then(|n| n.checked_add(margin))
            .ok_or(EnsureWorkflowError::JournalIntegrity)?;
        minimum
            .checked_add(margin)
            .ok_or(EnsureWorkflowError::JournalIntegrity)?;
        let pause = records::native_pause(records::NativeFundingQuote {
            plan,
            root: &domain.root,
            root_principal: principal,
            provisioning_action_sha256: action_hash,
            available_cycles: observation.live.cycles,
            minimum_cycles: minimum,
            funding_margin_cycles: margin,
            cycles_ledger: ledger,
            ledger_fee_cycles: fee,
            shortfall_cycles: amount,
        });
        validate_observation(plan, journal, state, &pause, &observation, false)?;
        let latest = continuation::actions(plan, journal)
            .into_iter()
            .chain(journal.funding_reviews.iter().map(|review| &review.action))
            .filter_map(|action| match action {
                EnsureAction::Fund {
                    created_at_time, ..
                }
                | EnsureAction::FundEstate {
                    created_at_time, ..
                } => Some(*created_at_time),
                _ => None,
            })
            .max()
            .unwrap_or(plan.planned_at_time);
        let timestamp = created_at_time.max(
            latest
                .checked_add(1)
                .ok_or(EnsureWorkflowError::JournalIntegrity)?,
        );
        let candidate = records::native_review(pause, timestamp);
        journal.funding_reviews.push(candidate.clone());
        funding::verify(plan, journal, state)?;
        write_journal(paths, journal)?;
        return Ok(Some(candidate));
    }
    Ok(None)
}

fn validate_observation<E: std::error::Error + 'static>(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    pause: &NativeFundingRequiredRecord,
    observation: &NativeFundingObservation,
    approved: bool,
) -> Result<(), EnsureWorkflowError<E>> {
    funding::verify_root_authority(
        plan,
        state,
        &observation.live,
        &pause.root,
        &pause.root_principal,
    )?;
    let (funds, fees) = funding::totals::<E>(journal, None)?;
    let maximum = plan
        .conservation
        .maximum_operator_debit_cycles
        .checked_add(funds)
        .and_then(|n| n.checked_add(fees))
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    let prior_debit = crate::fleet_ensure::policy::operator_mint::operator_source(journal)
        .ok_or(EnsureWorkflowError::JournalIntegrity)?
        .checked_sub(observation.operator_cycles);
    let balance_is_bounded = pause
        .available_cycles
        .saturating_sub(observation.live.cycles)
        <= pause.funding_margin_cycles;
    if observation.cycles_ledger != pause.cycles_ledger
        || observation.ledger_fee_cycles != pause.ledger_fee_cycles
        || prior_debit.is_none_or(|spent| spent > maximum)
        || (!approved && !balance_is_bounded)
    {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    Ok(())
}

pub(super) fn resume<P: EnsurePlatform>(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    journal: &mut FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    digest: &str,
    platform: &mut P,
) -> Result<(), EnsureWorkflowError<P::Error>> {
    let index = journal
        .funding_reviews
        .len()
        .checked_sub(1)
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    let review = &journal.funding_reviews[index];
    if review
        .effect
        .as_ref()
        .is_some_and(|effect| effect.state == EffectState::Applied)
    {
        return Ok(());
    }
    if review.effect.is_none() && digest != review.review_sha256 {
        return Err(EnsureWorkflowError::NativeFundingRequired {
            review_sha256: review.review_sha256.clone(),
        });
    }
    let FundingPauseRecord::Native(pause) = &review.pause else {
        return Err(EnsureWorkflowError::JournalIntegrity);
    };
    let observation = platform
        .observe_native_funding(&pause.root, state)
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::DriftedBeforeApply)?;
    validate_observation(
        plan,
        journal,
        state,
        pause,
        &observation,
        review.effect.is_some(),
    )?;
    if review.effect.is_none() {
        funding::require_operator_funds(review, observation.operator_cycles)?;
        journal.funding_reviews[index].effect = Some(records::intent(
            &review.action,
            observation.operator_cycles,
            observation.live.cycles,
        ));
        write_journal(paths, journal)?;
    }
    let review = &journal.funding_reviews[index];
    let effect = review
        .effect
        .as_ref()
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    let expected_after = effect
        .pre_cycles
        .and_then(|before| before.checked_sub(review.pause.shortfall_cycles()))
        .and_then(|n| n.checked_sub(review.pause.ledger_fee_cycles()))
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    if observation.operator_cycles != expected_after
        && Some(observation.operator_cycles) != effect.pre_cycles
    {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    if effect.receipt.is_none() {
        let outcome = platform
            .apply(&plan.operation_id, &review.action, effect, state)
            .map_err(EnsureWorkflowError::Platform)?;
        if outcome.created_principal.is_some()
            || outcome
                .receipt
                .as_ref()
                .is_none_or(|receipt| receipt.parse::<candid::Nat>().is_err())
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
    let root = journal.funding_reviews[index].pause.root();
    let after = platform
        .observe_native_funding(root, state)
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::DriftedBeforeApply)?;
    let FundingPauseRecord::Native(pause) = &journal.funding_reviews[index].pause else {
        return Err(EnsureWorkflowError::JournalIntegrity);
    };
    validate_observation(plan, journal, state, pause, &after, true)?;
    if after.operator_cycles != expected_after {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    let effect = journal.funding_reviews[index]
        .effect
        .as_mut()
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    effect.post_cycles = Some(after.operator_cycles);
    effect.destination_post_cycles = Some(after.live.cycles);
    effect.state = EffectState::Applied;
    funding::applied::<P::Error>(&journal.funding_reviews[index])?;
    journal.stalled_observations = 0;
    write_journal(paths, journal)?;
    Ok(())
}
