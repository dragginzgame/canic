//! Module: fleet_ensure::ops::funding_observation
//!
//! Responsibility: construct sealed observation reviews and mutate their retained attempt prefix.
//! Boundary: workflow owns the operation lock and writes each intent before a platform call.

mod identity;
pub(in crate::fleet_ensure) mod transport;

pub(in crate::fleet_ensure) use identity::{resolved, resolved_from_state};
pub(in crate::fleet_ensure) mod validation;

use crate::fleet_ensure::{
    model::{FleetEnsurePlan, funding_observation::*},
    view::startup_funding::{StartupChildFundingBinding, StartupRelayQuote},
};
use canic_core::cdk::utils::hash::sha256_hex;

/// Convert qualified preview evidence into the exact immutable review body.
pub(in crate::fleet_ensure) fn review(
    plan: &FleetEnsurePlan,
    root: &str,
    authority: FundingObservationAuthorityRecord,
    configuration_source: String,
    quote: StartupRelayQuote,
) -> Result<FundingObservationReviewRecord, FundingObservationError> {
    let body = FundingObservationReviewBodyRecord {
        configuration_source,
        operation_id: plan.operation_id.clone(),
        plan_sha256: plan.plan_sha256.clone(),
        root: root.into(),
        authority,
        requests: quote
            .requests
            .into_iter()
            .map(|binding| {
                Ok(FundingObservationRequestRecord {
                    component: binding.component,
                    release_set: binding.release_set,
                    parent: binding.parent,
                    parent_role: binding.parent_role,
                    child: binding.canister_id,
                    role: binding.role,
                })
            })
            .collect::<Result<_, FundingObservationError>>()?,
        per_attempt_cycles: quote.per_attempt_burn_allowance_cycles,
        maximum_cycles: quote.proposed_burn_allowance_cycles,
        recovery_floor_cycles: quote.minimum_recovery_cycles,
    };
    Ok(FundingObservationReviewRecord {
        final_root_cycles: None,
        review_sha256: digest(&body),
        body,
        approved: false,
        attempts: Vec::new(),
    })
}

pub(in crate::fleet_ensure) fn digest(body: &FundingObservationReviewBodyRecord) -> String {
    sha256_hex(
        &serde_json::to_vec(&("canic:funding-observation:v1", body))
            .expect("bounded serializable review"),
    )
}

pub(in crate::fleet_ensure) fn binding(
    request: &FundingObservationRequestRecord,
) -> StartupChildFundingBinding {
    StartupChildFundingBinding {
        component: request.component.clone(),
        release_set: request.release_set,
        parent: request.parent,
        parent_role: request.parent_role.clone(),
        canister_id: request.child,
        role: request.role.clone(),
    }
}

pub(in crate::fleet_ensure) const fn approve(record: &mut FundingObservationReviewRecord) {
    record.approved = true;
}

pub(in crate::fleet_ensure) fn consume(record: &mut FundingObservationReviewRecord) -> usize {
    let index = record.attempts.len();
    record.attempts.push(FundingObservationAttemptRecord {
        request_index: index,
        outcome: None,
    });
    index
}

pub(in crate::fleet_ensure) fn finish(
    record: &mut FundingObservationReviewRecord,
    outcome: FundingObservationOutcomeRecord,
) {
    record
        .attempts
        .last_mut()
        .expect("validated pending attempt")
        .outcome = Some(outcome);
}

pub(in crate::fleet_ensure) fn finish_root(
    record: &mut FundingObservationReviewRecord,
    cycles: Option<u128>,
) {
    let complete = record.attempts.len() == record.body.requests.len()
        && record.attempts.iter().all(|attempt| {
            matches!(
                attempt.outcome,
                Some(FundingObservationOutcomeRecord::Observed(_))
            )
        });
    if complete {
        record.final_root_cycles = cycles;
    }
}

/// Recompile retained source so role funding inputs remain bound to the approved Spec hashes.
pub(in crate::fleet_ensure) fn configuration(
    desired: &crate::fleet_ensure::model::DesiredFleet,
    record: &FundingObservationReviewRecord,
) -> Result<canic_core::bootstrap::compiled::ConfigModel, FundingObservationError> {
    let config: canic_core::bootstrap::compiled::ConfigModel =
        toml::from_str(&record.body.configuration_source)
            .map_err(|_| FundingObservationError::AuthorityMismatch)?;
    let bootstrap = desired
        .bootstrap
        .as_ref()
        .ok_or(FundingObservationError::AuthorityMismatch)?;
    if config
        .compile_component_deployment_configuration()
        .map_err(|_| FundingObservationError::AuthorityMismatch)?
        != bootstrap.component_deployment_configuration
    {
        return Err(FundingObservationError::AuthorityMismatch);
    }
    Ok(config)
}

/// Recompute a diagnostic recovery quote without remote calls, including on effect-free replay.
pub(in crate::fleet_ensure) fn demand(
    plan: &FleetEnsurePlan,
    journal: &crate::fleet_ensure::model::FleetEnsureJournalRecord,
    record: &FundingObservationReviewRecord,
) -> Result<
    crate::fleet_ensure::view::startup_funding::StartupRecoveryDemand,
    FundingObservationError,
> {
    use crate::fleet_ensure::{policy::startup_funding::recursive, view::startup_funding::*};
    let desired = resolved(plan, journal)?;
    let config = configuration(&desired, record)?;
    if record.attempts.len() != record.body.requests.len() {
        return Err(FundingObservationError::Unavailable);
    }
    let child_usage = record
        .body
        .requests
        .iter()
        .zip(&record.attempts)
        .map(|(request, attempt)| {
            let Some(FundingObservationOutcomeRecord::Observed(usage)) = &attempt.outcome else {
                return Err(FundingObservationError::Unavailable);
            };
            Ok(StartupChildFundingUsage {
                allowance: Err(StartupUsageUnavailable::NotObserved),
                local_demand: Err(StartupDemandUnavailable::BalanceNotObserved),
                binding: Some(binding(request)),
                observed_balance_cycles: Some(usage.native_cycles),
                name: request.child.to_text(),
                child: request.child.to_text(),
                usage: Ok(StartupChildAccounting {
                    observed_at_ns: usage.observed_at_ns,
                    accounted_cycles: usage.accounted_cycles,
                    last_accounted_at_secs: usage.last_accounted_at_secs,
                    pending_operations: usage.pending_operations,
                    reserved_cycles: usage.reserved_cycles,
                }),
            })
        })
        .collect::<Result<Vec<_>, FundingObservationError>>()?;
    let root = desired
        .bootstrap
        .as_ref()
        .and_then(|bootstrap| {
            bootstrap
                .roots
                .iter()
                .find(|root| root.root == record.body.root)
        })
        .ok_or(FundingObservationError::AuthorityMismatch)?;
    let components = record.body.authority.components.len();
    let descendants = record
        .body
        .requests
        .len()
        .checked_sub(components)
        .ok_or(FundingObservationError::AuthorityMismatch)?;
    let root = StartupRootFunding {
        root: record.body.root.clone(),
        child_usage,
        balance: StartupNativeBalance::Observed(
            record
                .final_root_cycles
                .ok_or(FundingObservationError::Unavailable)?,
        ),
        inventory: Ok(StartupInventoryCoverage {
            components,
            descendants,
        }),
        relay_quote: Err(StartupDemandUnavailable::BalanceNotObserved),
        recovery_demand: Err(StartupDemandUnavailable::BalanceNotObserved),
        child_grants_cycles: 0,
        components: Vec::new(),
        funding_budget: root.limits.cycles_funding.clone(),
        exceeds_window_budget: false,
        minimum_native_cycles: 0,
        request_threshold_cycles: 0,
        shortfall_cycles: 0,
    };
    recursive::project(&config, &desired, &root).map_err(|_| FundingObservationError::Unavailable)
}

/// Add the first review for a Root after workflow rejects an existing entry.
pub(in crate::fleet_ensure) fn retain(
    journal: &mut crate::fleet_ensure::model::FleetEnsureJournalRecord,
    root: &str,
    record: FundingObservationReviewRecord,
) {
    journal.funding_observations.insert(root.into(), record);
}
