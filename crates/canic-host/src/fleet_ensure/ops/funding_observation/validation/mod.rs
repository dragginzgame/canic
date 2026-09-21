//! Module: fleet_ensure::ops::funding_observation::validation
//!
//! Responsibility: validate retained observation identity, bounded progress and selected policy.
//! Boundary: restore never trusts a digest alone or invents a reply for a consumed attempt.

use crate::fleet_ensure::{
    model::{
        FleetEnsureJournalRecord, FleetEnsurePlan, MAX_FLEET_ENSURE_CANISTERS,
        funding_observation::*,
    },
    ops::funding_observation as records,
    policy::startup_funding::{live_binding, relay_quote::observation_bounds},
};
use std::collections::BTreeSet;

pub(in crate::fleet_ensure) fn verify(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
) -> Result<(), FundingObservationError> {
    if journal.funding_observations.is_empty() {
        return Ok(());
    }
    let desired = records::resolved(plan, journal)?;
    let mut total = 0u128;
    for (root, review) in &journal.funding_observations {
        if root != &review.body.root {
            return Err(FundingObservationError::AuthorityMismatch);
        }
        record(plan, &desired, review)?;
        total = total
            .checked_add(consumed(review)?)
            .ok_or(FundingObservationError::ArithmeticOverflow)?;
    }
    plan.conservation
        .maximum_execution_burn_cycles
        .checked_add(total)
        .ok_or(FundingObservationError::ArithmeticOverflow)?;
    Ok(())
}

pub(in crate::fleet_ensure) fn consumed(
    review: &FundingObservationReviewRecord,
) -> Result<u128, FundingObservationError> {
    review
        .body
        .per_attempt_cycles
        .checked_mul(review.attempts.len() as u128)
        .ok_or(FundingObservationError::ArithmeticOverflow)
}

pub(in crate::fleet_ensure) fn total(
    journal: &FleetEnsureJournalRecord,
) -> Result<u128, FundingObservationError> {
    journal
        .funding_observations
        .values()
        .try_fold(0u128, |total, review| {
            total
                .checked_add(consumed(review)?)
                .ok_or(FundingObservationError::ArithmeticOverflow)
        })
}

pub(in crate::fleet_ensure) fn record(
    plan: &FleetEnsurePlan,
    desired: &crate::fleet_ensure::model::DesiredFleet,
    review: &FundingObservationReviewRecord,
) -> Result<(), FundingObservationError> {
    record_authority(desired, &plan.operation_id, &plan.plan_sha256, review)
}

fn record_authority(
    desired: &crate::fleet_ensure::model::DesiredFleet,
    operation: &str,
    plan_digest: &str,
    review: &FundingObservationReviewRecord,
) -> Result<(), FundingObservationError> {
    let invalid = FundingObservationError::AuthorityMismatch;
    let body = &review.body;
    records::configuration(desired, review)?;
    if body.operation_id != operation
        || body.plan_sha256 != plan_digest
        || review.review_sha256 != records::digest(body)
    {
        return Err(invalid);
    }
    let bootstrap = desired.bootstrap.as_ref().ok_or(invalid)?;
    let bounds =
        observation_bounds(desired, &body.root, !body.requests.is_empty()).map_err(|_| invalid)?;
    let minimum = bounds.recovery_floor_cycles;
    let attempt = bounds.per_attempt_cycles;
    if body.per_attempt_cycles != attempt
        || body.recovery_floor_cycles != minimum
        || body.maximum_cycles
            != attempt
                .checked_mul(body.requests.len() as u128)
                .ok_or(invalid)?
        || body.requests.len() > MAX_FLEET_ENSURE_CANISTERS
        || body.authority.revision == 0
        || body.authority.content_hash == [0; 32]
        || body.authority.components.len() > MAX_FLEET_ENSURE_CANISTERS
        || body
            .authority
            .components
            .values()
            .any(|head| head.revision == 0 || head.content_hash == [0; 32])
    {
        return Err(invalid);
    }
    // The full request pass contains every descendant, including each intermediate parent.
    let mut bindings = Vec::new();
    let mut components = BTreeSet::new();
    let mut previous = None;
    for request in &body.requests {
        let order = (request.parent, request.child);
        if previous.is_some_and(|prior| prior >= order)
            || request.component.authority != body.authority.registry
            || !body
                .authority
                .components
                .contains_key(&request.component.component)
        {
            return Err(invalid);
        }
        previous = Some(order);
        let binding = records::binding(request);
        live_binding::selected(desired, &body.root, &binding).map_err(|_| invalid)?;
        components.insert(binding.component.component);
        bindings.push(binding);
    }
    // Every top-level member now has its own paid native inspection too.
    if components != body.authority.components.keys().copied().collect() {
        return Err(invalid);
    }
    if live_binding::graph(
        &bootstrap
            .component_deployment_configuration
            .component_topology,
        bootstrap.release_build_id,
        bindings.iter(),
    )
    .values()
    .any(Result::is_err)
    {
        return Err(invalid);
    }
    attempts(review)
}

fn attempts(review: &FundingObservationReviewRecord) -> Result<(), FundingObservationError> {
    let invalid = FundingObservationError::AuthorityMismatch;
    let body = &review.body;
    let mut seen = BTreeSet::new();
    if body
        .requests
        .iter()
        .any(|request| !seen.insert(request.child))
        || review.attempts.len() > body.requests.len()
        || (!review.approved && !review.attempts.is_empty())
    {
        return Err(invalid);
    }
    if review.final_root_cycles.is_some()
        && (review.attempts.len() != body.requests.len()
            || !review.attempts.iter().all(|attempt| {
                matches!(
                    attempt.outcome,
                    Some(FundingObservationOutcomeRecord::Observed(_))
                )
            }))
    {
        return Err(invalid);
    }
    for (index, attempt) in review.attempts.iter().enumerate() {
        if attempt.request_index != index
            || (attempt.outcome.is_none() && index + 1 != review.attempts.len())
        {
            return Err(invalid);
        }
        if let Some(FundingObservationOutcomeRecord::Observed(value)) = &attempt.outcome
            && !usage(&body.requests[index], value)
        {
            return Err(invalid);
        }
    }
    Ok(())
}

pub(in crate::fleet_ensure) fn usage(
    request: &FundingObservationRequestRecord,
    value: &FundingChildAccountingRecord,
) -> bool {
    value.parent == request.parent
        && value.child == request.child
        && value.last_accounted_at_secs <= value.observed_at_ns / 1_000_000_000
        && (value.pending_operations > 0 || value.reserved_cycles == Some(0))
}

/// Validate non-executable source records and retain every consumed observation allowance.
pub(in crate::fleet_ensure) fn source_allowance(
    desired: &crate::fleet_ensure::model::DesiredFleet,
    operation: &str,
    plan_digest: &str,
    reviews: &std::collections::BTreeMap<String, FundingObservationReviewRecord>,
) -> Result<u128, FundingObservationError> {
    reviews.iter().try_fold(0u128, |total, (root, review)| {
        if root != &review.body.root {
            return Err(FundingObservationError::AuthorityMismatch);
        }
        record_authority(desired, operation, plan_digest, review)?;
        total
            .checked_add(consumed(review)?)
            .ok_or(FundingObservationError::ArithmeticOverflow)
    })
}
