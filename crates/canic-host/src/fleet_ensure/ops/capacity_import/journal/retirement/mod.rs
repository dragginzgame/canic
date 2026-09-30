//! Reconcile retired ingress against exact custody without changing its original paid authority.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    model::{
        EffectState,
        capacity_import::{
            CapacityImportHandoffRecord, CapacityImportHandoffRequestRecord,
            CapacityImportJournalRecord, CapacityImportPlanRecord,
            retirement::CapacityImportHandoffRetirementReason,
        },
    },
    ops::capacity_import::{
        journal::{self, CapacityImportJournalError, source_index},
        transport::{RetiredHandoff, validate_request},
    },
    policy::capacity_import::{
        admit_source_handoff, expired_absence_proves_retirement, source_total,
    },
    view::capacity_import::CapacityImportSourceView,
};
use candid::Principal;
use std::collections::BTreeSet;

/// Distinct envelopes consume the same finite submission allowance as resubmissions.
pub(in crate::fleet_ensure::ops::capacity_import) const MAXIMUM_REQUESTS: usize = 2;

/// Return whether the current ingress has retained terminal evidence.
#[must_use]
pub fn pending(handoff: &CapacityImportHandoffRecord) -> bool {
    handoff
        .retirements
        .last()
        .is_some_and(|retired| Some(&retired.request) == handoff.request.as_ref())
}

/// Retain authenticated terminal evidence while preserving Issued intent and cycle baselines.
pub fn retain(
    journal: &CapacityImportJournalRecord,
    canister: Principal,
    rejected: &RetiredHandoff,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    journal::validate(journal)?;
    let index = source_index(journal, canister)?;
    let handoff = &journal.handoffs[index];
    let request = handoff
        .request
        .as_ref()
        .ok_or(CapacityImportJournalError::Integrity)?;
    if handoff
        .effect
        .as_ref()
        .is_none_or(|effect| effect.state != EffectState::Issued)
        || !rejected.matches(&journal.plan, canister, request)
    {
        return Err(CapacityImportJournalError::Integrity);
    }
    if pending(handoff) {
        return Ok(journal.clone());
    }
    if handoff.retirements.len() >= MAXIMUM_REQUESTS {
        return Err(exhausted());
    }
    let mut result = journal.clone();
    result.handoffs[index]
        .retirements
        .push(rejected.receipt.clone());
    journal::validate(&result)?;
    Ok(result)
}

/// Finish a pruned request from exact custody, or renew only an unchanged original source.
pub fn reconcile(
    journal: &CapacityImportJournalRecord,
    observed: &CapacityImportSourceView,
    request: CapacityImportHandoffRequestRecord,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    journal::validate(journal)?;
    let index = source_index(journal, observed.binding.canister_id)?;
    let handoff = &journal.handoffs[index];
    if !pending(handoff) {
        return Err(CapacityImportJournalError::Unresolved);
    }
    let retired = handoff
        .retirements
        .last()
        .ok_or(CapacityImportJournalError::Integrity)?;
    if !matches!(
        retired.reason,
        CapacityImportHandoffRetirementReason::Rejected { .. }
    ) && observed.binding != journal.plan.sources[index].binding
    {
        return journal::apply_observed_handoff(journal, observed);
    }
    renew(journal, observed, request)
}

/// Admit fresh original custody after retirement; neither terminal status nor controllers alone suffice.
pub fn renew(
    journal: &CapacityImportJournalRecord,
    observed: &CapacityImportSourceView,
    request: CapacityImportHandoffRequestRecord,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    journal::validate(journal)?;
    let index = source_index(journal, observed.binding.canister_id)?;
    let handoff = &journal.handoffs[index];
    if !pending(handoff) {
        return Err(CapacityImportJournalError::Unresolved);
    }
    if handoff.retirements.len() >= MAXIMUM_REQUESTS {
        return Err(exhausted());
    }
    let effect = handoff
        .effect
        .as_ref()
        .ok_or(CapacityImportJournalError::Integrity)?;
    if effect.state != EffectState::Issued {
        return Err(CapacityImportJournalError::Integrity);
    }
    admit_source_handoff(&journal.plan.sources[index], observed)?;
    let original = source_total(
        effect
            .pre_cycles
            .ok_or(CapacityImportJournalError::Integrity)?,
        handoff
            .before_reserved_cycles
            .ok_or(CapacityImportJournalError::Integrity)?,
    )?;
    if source_total(observed.cycles, observed.reserved_cycles)? > original {
        return Err(CapacityImportJournalError::Unresolved);
    }
    validate_request(&journal.plan, handoff.canister_id, &request)?;
    if handoff
        .retirements
        .iter()
        .any(|rejected| rejected.request.request_id == request.request_id)
    {
        return Err(CapacityImportJournalError::RequestInvalid);
    }
    let mut result = journal.clone();
    result.handoffs[index].request = Some(request);
    result.handoffs[index]
        .effect
        .as_mut()
        .ok_or(CapacityImportJournalError::Integrity)?
        .state = EffectState::Intent;
    journal::validate(&result)?;
    Ok(result)
}

/// Replace a never-issued envelope without consuming an effect or losing previous evidence.
pub fn refresh_unissued(
    journal: &CapacityImportJournalRecord,
    canister: Principal,
    request: CapacityImportHandoffRequestRecord,
) -> Result<CapacityImportJournalRecord, CapacityImportJournalError> {
    journal::validate(journal)?;
    let index = source_index(journal, canister)?;
    if journal.handoffs[index]
        .effect
        .as_ref()
        .is_none_or(|effect| effect.state != EffectState::Intent)
    {
        return Err(CapacityImportJournalError::Unresolved);
    }
    validate_request(&journal.plan, canister, &request)?;
    let mut result = journal.clone();
    result.handoffs[index].request = Some(request);
    journal::validate(&result)?;
    Ok(result)
}

pub(super) fn validate(
    plan: &CapacityImportPlanRecord,
    handoff: &CapacityImportHandoffRecord,
) -> Result<(), CapacityImportJournalError> {
    if handoff.retirements.len() > MAXIMUM_REQUESTS {
        return Err(CapacityImportJournalError::Integrity);
    }
    let mut seen = BTreeSet::new();
    for retired in &handoff.retirements {
        validate_request(plan, handoff.canister_id, &retired.request)?;
        let reason_valid = match &retired.reason {
            CapacityImportHandoffRetirementReason::Rejected {
                reject_code,
                reject_message_sha256,
            } => (1..=5).contains(reject_code) && *reject_message_sha256 != [0; 32],
            CapacityImportHandoffRetirementReason::Done => true,
            CapacityImportHandoffRetirementReason::Absent { certified_at_ns } => {
                expired_absence_proves_retirement(retired.request.ingress_expiry, *certified_at_ns)
            }
        };
        if !seen.insert(retired.request.request_id)
            || !reason_valid
            || retired.certificate_sha256 == [0; 32]
        {
            return Err(CapacityImportJournalError::Integrity);
        }
    }
    if let Some(request) = &handoff.request {
        let current_retired = pending(handoff);
        let state_valid = handoff.effect.as_ref().is_some_and(|effect| {
            effect.state == EffectState::Issued
                || (effect.state == EffectState::Applied
                    && handoff.retirements.last().is_some_and(|retired| {
                        !matches!(
                            retired.reason,
                            CapacityImportHandoffRetirementReason::Rejected { .. }
                        )
                    }))
        });
        if seen.contains(&request.request_id) != current_retired
            || (!current_retired && handoff.retirements.len() == MAXIMUM_REQUESTS)
            || (current_retired && !state_valid)
        {
            return Err(CapacityImportJournalError::Integrity);
        }
    }
    Ok(())
}

/// Only append terminal evidence, refresh unissued bytes or renew a retired request.
pub(super) fn monotonic(
    before: &CapacityImportHandoffRecord,
    after: &CapacityImportHandoffRecord,
) -> bool {
    if before.request == after.request {
        let append = after.retirements.len() == before.retirements.len() + 1
            && after.retirements.starts_with(&before.retirements)
            && pending(after)
            && !pending(before);
        before.retirements == after.retirements || append
    } else {
        let first_intent = before.request.is_none() && after.retirements.is_empty();
        let unissued_refresh = before
            .effect
            .as_ref()
            .zip(after.effect.as_ref())
            .is_some_and(|(old, new)| {
                old.state == EffectState::Intent
                    && matches!(new.state, EffectState::Intent | EffectState::Issued)
            })
            && before.retirements == after.retirements
            && !pending(after);
        first_intent || is_renewal(before, after) || unissued_refresh
    }
}

/// The only permitted return from Issued to Intent retains terminal request evidence.
pub(super) fn is_renewal(
    before: &CapacityImportHandoffRecord,
    after: &CapacityImportHandoffRecord,
) -> bool {
    let states = before
        .effect
        .as_ref()
        .zip(after.effect.as_ref())
        .is_some_and(|(old, new)| {
            old.state == EffectState::Issued && new.state == EffectState::Intent
        });
    states
        && pending(before)
        && !pending(after)
        && before.request != after.request
        && before.retirements == after.retirements
}

fn exhausted() -> CapacityImportJournalError {
    CapacityImportJournalError::BudgetExhausted {
        step: "distinct controller handoff requests".into(),
    }
}
