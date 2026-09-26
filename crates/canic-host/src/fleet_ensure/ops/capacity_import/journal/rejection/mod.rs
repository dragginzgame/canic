//! Retain certified failures before bounded renewal of an unchanged source's controller request.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::{
    model::{
        EffectState,
        capacity_import::{
            CapacityImportHandoffRecord, CapacityImportHandoffRequestRecord,
            CapacityImportJournalRecord, CapacityImportPlanRecord,
        },
    },
    ops::capacity_import::{
        journal::{self, CapacityImportJournalError, source_index},
        transport::{RejectedHandoff, validate_request},
    },
    policy::capacity_import::{admit_source_handoff, source_total},
    view::capacity_import::CapacityImportSourceView,
};
use candid::Principal;
use std::collections::BTreeSet;

/// Distinct envelopes consume the same finite submission allowance as resubmissions.
pub(in crate::fleet_ensure::ops::capacity_import) const MAXIMUM_REQUESTS: usize = 2;

/// Return whether the current ingress has an already retained terminal rejection.
#[must_use]
pub fn pending(handoff: &CapacityImportHandoffRecord) -> bool {
    handoff
        .rejections
        .last()
        .is_some_and(|rejection| Some(&rejection.request) == handoff.request.as_ref())
}

/// Record authenticated terminal rejection while preserving Issued intent and cycle baselines.
pub fn retain(
    journal: &CapacityImportJournalRecord,
    canister: Principal,
    rejected: &RejectedHandoff,
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
    if handoff.rejections.len() >= MAXIMUM_REQUESTS {
        return Err(exhausted());
    }
    let mut result = journal.clone();
    result.handoffs[index]
        .rejections
        .push(rejected.receipt.clone());
    journal::validate(&result)?;
    Ok(result)
}

/// Admit fresh original custody after rejection; neither rejection nor matching controllers alone suffice.
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
    if handoff.rejections.len() >= MAXIMUM_REQUESTS {
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
        .rejections
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

pub(super) fn validate(
    plan: &CapacityImportPlanRecord,
    handoff: &CapacityImportHandoffRecord,
) -> Result<(), CapacityImportJournalError> {
    if handoff.rejections.len() > MAXIMUM_REQUESTS {
        return Err(CapacityImportJournalError::Integrity);
    }
    let mut seen = BTreeSet::new();
    for rejected in &handoff.rejections {
        validate_request(plan, handoff.canister_id, &rejected.request)?;
        if !seen.insert(rejected.request.request_id)
            || !(1..=5).contains(&rejected.reject_code)
            || rejected.reject_message_sha256 == [0; 32]
            || rejected.certificate_sha256 == [0; 32]
        {
            return Err(CapacityImportJournalError::Integrity);
        }
    }
    if let Some(request) = &handoff.request {
        let current_rejected = pending(handoff);
        if seen.contains(&request.request_id) != current_rejected
            || (!current_rejected && handoff.rejections.len() == MAXIMUM_REQUESTS)
            || (current_rejected
                && handoff
                    .effect
                    .as_ref()
                    .is_none_or(|effect| effect.state != EffectState::Issued))
        {
            return Err(CapacityImportJournalError::Integrity);
        }
    }
    Ok(())
}

/// Only append exact rejection evidence or replace its current request with one new intent.
pub(super) fn monotonic(
    before: &CapacityImportHandoffRecord,
    after: &CapacityImportHandoffRecord,
) -> bool {
    if before.request == after.request {
        let append = after.rejections.len() == before.rejections.len() + 1
            && after.rejections.starts_with(&before.rejections)
            && pending(after)
            && !pending(before);
        before.rejections == after.rejections || append
    } else {
        let first_intent = before.request.is_none() && after.rejections.is_empty();
        first_intent || is_renewal(before, after)
    }
}

/// The only permitted return from Issued to Intent retains a rejected request as evidence.
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
        && before.rejections == after.rejections
}

fn exhausted() -> CapacityImportJournalError {
    CapacityImportJournalError::BudgetExhausted {
        step: "distinct controller handoff requests".into(),
    }
}
