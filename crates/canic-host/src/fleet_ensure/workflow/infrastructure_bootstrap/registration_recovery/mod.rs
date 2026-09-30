//! Review and approve supplementary registration authority inside the retained bootstrap operation.

use crate::fleet_ensure::{
    model::{
        FleetEnsureJournalRecord, FleetEnsurePlan, FleetEnsureStateRecord,
        infrastructure_bootstrap::registration_recovery::BootstrapRegistrationReviewRecord,
    },
    ops::{
        self, EnsurePaths, EnsurePlatform,
        infrastructure_bootstrap::{
            self as bootstrap, InfrastructureBootstrapError, registration_recovery as records,
        },
    },
    workflow::{self, EnsureWorkflowError, infrastructure_bootstrap},
};
use std::path::Path;

fn load<P: EnsurePlatform>(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    digest: &str,
    platform: &mut P,
) -> Result<
    (
        FleetEnsurePlan,
        FleetEnsureJournalRecord,
        FleetEnsureStateRecord,
    ),
    EnsureWorkflowError<P::Error>,
> {
    let plan = infrastructure_bootstrap::retained(workspace, environment, fleet, digest)?;
    let paths = EnsurePaths::under(workspace, environment, fleet);
    let journal = ops::read_journal(&paths)?.ok_or(EnsureWorkflowError::JournalIntegrity)?;
    let mut state = ops::read_state(&paths, fleet)?;
    workflow::verify_journal(&journal, &plan, fleet, &state)?;
    bootstrap::verify_plan(workspace, &plan)?;
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?
        .desired();
    workflow::continuation::verify_inputs(workspace, desired, &plan)?;
    platform
        .bind_reviewed_desired(desired)
        .map_err(EnsureWorkflowError::Platform)?;
    platform
        .bind_infrastructure_bootstrap(
            plan.infrastructure_bootstrap
                .as_ref()
                .ok_or(EnsureWorkflowError::PlanIntegrity)?,
        )
        .map_err(EnsureWorkflowError::Platform)?;
    if records::approved(&journal).is_none() {
        records::boundary(&plan, &journal)?;
        workflow::publish_terminal_state(desired, &plan, &journal, &mut state)
            .map_err(|_| EnsureWorkflowError::JournalIntegrity)?;
        ops::write_state(&paths, &state)?;
    }
    Ok((plan, journal, state))
}

/// Review the exact successor and target deposits. Repeating a completed review performs no reads.
pub fn review<P: EnsurePlatform>(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    plan_sha256: &str,
    created_at_time: u64,
    platform: &mut P,
) -> Result<BootstrapRegistrationReviewRecord, EnsureWorkflowError<P::Error>> {
    let paths = EnsurePaths::under(workspace, environment, fleet);
    let _lock = ops::lock_operation(&paths)?;
    let (plan, mut journal, state) = load(workspace, environment, fleet, plan_sha256, platform)?;
    if let Some(review) = journal
        .bootstrap_registration_recovery
        .as_ref()
        .and_then(|record| record.review.as_ref())
    {
        return Ok(review.clone());
    }
    records::reserve(&paths, &mut journal, false)?;
    let observed = platform
        .infrastructure_bootstrap_observation(
            plan.infrastructure_bootstrap
                .as_ref()
                .ok_or(EnsureWorkflowError::PlanIntegrity)?,
            &state,
        )
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let (observation, total) =
        bootstrap::terminal_observation(workspace, &plan, &state, &observed)?;
    let actual = workflow::verify_terminal_conservation_with_total(
        &plan,
        &journal,
        &state,
        &observation,
        total,
    )?;
    let phase = bootstrap::registration::compile(workspace, &plan, &state, total)?;
    let review = records::candidate(
        &plan,
        &journal,
        &phase,
        &observed,
        actual.observed_net_cycle_debit_cycles,
        created_at_time,
    )?;
    let required = review
        .additional_funding_cycles
        .checked_add(review.additional_ledger_fee_cycles)
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    if observed.operator_cycles < required {
        return Err(EnsureWorkflowError::InsufficientOperatorCycles {
            actual: observed.operator_cycles,
            required,
        });
    }
    records::retain(&paths, &plan, &state, &mut journal, review.clone())?;
    records::verify(&plan, &journal, &state)?;
    Ok(review)
}

/// Requalify current custody and retain exact approval before the ordinary effect driver resumes.
pub fn approve<P: EnsurePlatform>(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    plan_sha256: &str,
    review_sha256: &str,
    platform: &mut P,
) -> Result<(), EnsureWorkflowError<P::Error>> {
    let paths = EnsurePaths::under(workspace, environment, fleet);
    let _lock = ops::lock_operation(&paths)?;
    let (plan, mut journal, state) = load(workspace, environment, fleet, plan_sha256, platform)?;
    let review = journal
        .bootstrap_registration_recovery
        .as_ref()
        .and_then(|record| record.review.clone())
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    if review.review_sha256 != review_sha256 {
        return Err(InfrastructureBootstrapError::RegistrationApproval {
            review_sha256: review.review_sha256,
        }
        .into());
    }
    if records::approved(&journal).is_some() {
        return Ok(());
    }
    records::reserve(&paths, &mut journal, true)?;
    let observed = platform
        .infrastructure_bootstrap_observation(
            plan.infrastructure_bootstrap
                .as_ref()
                .ok_or(EnsureWorkflowError::PlanIntegrity)?,
            &state,
        )
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    bootstrap::terminal_observation(workspace, &plan, &state, &observed)?;
    let phase = bootstrap::registration::compile(workspace, &plan, &state, 0)?;
    if phase.protocol_actions != review.protocol_actions
        || observed.operator_cycles != review.operator_cycles
        || observed.ledger_fee_cycles
            != plan
                .infrastructure_bootstrap
                .as_ref()
                .ok_or(EnsureWorkflowError::PlanIntegrity)?
                .ledger_fee_cycles
    {
        return Err(InfrastructureBootstrapError::Integrity.into());
    }
    // Each native balance may consume only its reviewed recovery allowance before funding.
    let per_owner = review.recovery_burn_cycles / review.canisters.len() as u128;
    for (name, before) in &review.canisters {
        let current = observed
            .canisters
            .get(name)
            .and_then(Option::as_ref)
            .ok_or(EnsureWorkflowError::PlanIntegrity)?;
        let mut binding = before.binding.clone();
        // Running canisters can advance their version without changing installed authority.
        if current.binding.canister_version >= binding.canister_version {
            binding.canister_version = current.binding.canister_version;
        }
        if binding != current.binding
            || before.cycles.saturating_sub(current.cycles) > per_owner
            || current.reserved_cycles != before.reserved_cycles
        {
            return Err(InfrastructureBootstrapError::Integrity.into());
        }
    }
    records::approve(&paths, &mut journal)?;
    records::verify(&plan, &journal, &state)?;
    Ok(())
}
