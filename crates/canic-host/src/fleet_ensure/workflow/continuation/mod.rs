//! Module: fleet_ensure::workflow::continuation
//!
//! Responsibility: advance bounded fresh phases and verify completed-plan replay.
//! Does not own: durable record construction, artifact identity, or remote effects.
//! Boundary: composes the retained plan, policy decisions, and platform observations.

#[cfg(test)]
mod tests;

use super::{
    EnsureWorkflowError, attach_terminal_cycles, completed_inventory_operation,
    merge_terminal_inventory, ordered_actions, verify_terminal_conservation,
};
use crate::fleet_ensure::{
    model::{
        ActualCycleConservation, CanisterDisposition, DesiredFleet, EnsureAction,
        FleetEnsureCompletion, FleetEnsureJournalRecord, FleetEnsurePlan, FleetEnsurePlanScope,
        FleetEnsureStateRecord, FleetEnsureSuccessorReviewReason, FleetObservation,
        ReviewedDesiredFleetRecord,
    },
    ops::{
        EnsurePaths, EnsurePlatform, action_sha256,
        continuation::{candidate_journal, retain_phase},
        resolve_desired_artifacts, write_journal,
    },
    policy::{compile_plan, expected_plan_sha256, successor_phase_burn},
};
use std::{collections::BTreeSet, path::Path};

pub(super) fn actions<'a>(
    plan: &'a FleetEnsurePlan,
    journal: &'a FleetEnsureJournalRecord,
) -> Vec<&'a EnsureAction> {
    let mut actions = ordered_actions(plan);
    for phase in &journal.successor_phases {
        if let Some(plan) = &phase.plan {
            actions.extend(plan.protocol_actions.iter());
        }
    }
    actions
}

pub(super) fn verify_records<E>(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
) -> Result<(), EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    if journal.successor_phases.is_empty() {
        return Ok(());
    }
    let authority = plan
        .continuation
        .as_ref()
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or(EnsureWorkflowError::PlanIntegrity)?
        .desired();
    let mut hashes = BTreeSet::new();
    let mut count = 0_usize;
    let mut previous_burn = 0;
    for record in &journal.successor_phases {
        let phase = record
            .plan
            .as_deref()
            .ok_or(EnsureWorkflowError::JournalIntegrity)?;
        if phase.plan_sha256 != record.plan_sha256
            || expected_plan_sha256(phase) != record.plan_sha256
            || !phase_binding_matches(plan, phase)
            || !phase_effects_are_protocol_only(plan, phase)
        {
            return Err(EnsureWorkflowError::JournalIntegrity);
        }
        if record.execution_burn_before_phase < previous_burn {
            return Err(EnsureWorkflowError::JournalIntegrity);
        }
        previous_burn = record.execution_burn_before_phase;
        let burn = successor_phase_burn(desired, phase)?;
        if record
            .execution_burn_before_phase
            .checked_add(burn)
            .is_none_or(|total| total > plan.conservation.maximum_execution_burn_cycles)
        {
            return Err(EnsureWorkflowError::JournalIntegrity);
        }
        for action in &phase.protocol_actions {
            if !hashes.insert(action_sha256(action)) {
                return Err(EnsureWorkflowError::JournalIntegrity);
            }
            count += 1;
        }
    }
    if count > authority.maximum_successor_actions as usize {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    Ok(())
}

pub(super) fn verify_inputs<E>(
    root: &Path,
    desired: &DesiredFleet,
    plan: &FleetEnsurePlan,
) -> Result<(), EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    if plan.continuation.is_some()
        && resolve_desired_artifacts(root, desired)?.continuation != plan.continuation
    {
        return Err(review(FleetEnsureSuccessorReviewReason::ChangedInputs));
    }
    Ok(())
}

pub(super) fn verify_canonical<P: EnsurePlatform>(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<(), EnsureWorkflowError<P::Error>> {
    if journal.fleet != plan.fleet {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    if journal.plan_sha256 != plan.plan_sha256 {
        // Planning retains the previous inactive journal until the newly reviewed
        // plan crosses its own intent boundary. Its phases belong to that prior plan.
        return match journal.completion {
            FleetEnsureCompletion::InProgress => Err(EnsureWorkflowError::JournalIntegrity),
            FleetEnsureCompletion::Converged | FleetEnsureCompletion::ReplanRequired => Ok(()),
        };
    }
    if journal.operation_id != plan.operation_id {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    verify_records(plan, journal)?;
    if journal.successor_phases.is_empty() {
        return Ok(());
    }
    let allowed = platform
        .fresh_protocol_actions(&plan.operation_id, state)
        .map_err(EnsureWorkflowError::Platform)?
        .iter()
        .map(action_sha256)
        .collect::<BTreeSet<_>>();
    for phase in &journal.successor_phases {
        let phase = phase
            .plan
            .as_deref()
            .ok_or(EnsureWorkflowError::JournalIntegrity)?;
        let exact_principals = phase.canisters.iter().all(|canister| {
            let retained = state
                .pending_principals
                .get(&canister.name)
                .or_else(|| state.principals.get(&canister.name));
            retained.is_some() && canister.principal.as_ref() == retained
        });
        if !exact_principals
            || phase
                .protocol_actions
                .iter()
                .any(|action| !allowed.contains(&action_sha256(action)))
        {
            return Err(review(FleetEnsureSuccessorReviewReason::ProtocolAuthority));
        }
    }
    Ok(())
}

pub(super) fn replay<P: EnsurePlatform>(
    root: &Path,
    desired: &DesiredFleet,
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
) -> Result<ActualCycleConservation, EnsureWorkflowError<P::Error>> {
    let observation = platform
        .observe(&plan.operation_id, state)
        .map_err(EnsureWorkflowError::Platform)?;
    let protocol = platform
        .protocol_actions(&plan.operation_id, state)
        .map_err(EnsureWorkflowError::Platform)?;
    let current = compile_plan(
        desired,
        &resolve_desired_artifacts(root, desired)?,
        &protocol,
        &plan.desired_sha256,
        &plan.fleet,
        &observation,
        plan.planned_at_time,
    )?;
    if !ordered_actions(&current).is_empty() {
        return Err(review(FleetEnsureSuccessorReviewReason::AdditionalEffect));
    }
    let inventory = platform
        .terminal_inventory(completed_inventory_operation(plan, journal, state)?, state)
        .map_err(EnsureWorkflowError::Platform)?;
    let cycles = inventory.controlled_cycles_by_principal.clone();
    let mut verified_state = state.clone();
    merge_terminal_inventory(&mut verified_state, inventory)?;
    let mut final_observation = platform
        .observe(&plan.operation_id, &verified_state)
        .map_err(EnsureWorkflowError::Platform)?;
    attach_terminal_cycles(&mut final_observation, cycles)?;
    verify_terminal_conservation(plan, journal, &verified_state, &final_observation)
}

#[expect(
    clippy::too_many_arguments,
    reason = "the phase boundary binds the original plan, journal, observed state and effect owner explicitly"
)]
pub(super) fn append<P: EnsurePlatform>(
    paths: &EnsurePaths,
    desired: &DesiredFleet,
    plan: &FleetEnsurePlan,
    journal: &mut FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    observation: &FleetObservation,
    phase: FleetEnsurePlan,
    platform: &mut P,
) -> Result<(), EnsureWorkflowError<P::Error>> {
    if !phase_binding_matches(plan, &phase) || !phase_effects_are_protocol_only(plan, &phase) {
        return Err(review(FleetEnsureSuccessorReviewReason::AdditionalEffect));
    }
    let authority = plan
        .continuation
        .as_ref()
        .ok_or_else(|| review(FleetEnsureSuccessorReviewReason::PhaseBound))?;
    let retained_actions = journal
        .successor_phases
        .iter()
        .filter_map(|record| record.plan.as_deref())
        .flat_map(|phase| phase.protocol_actions.iter())
        .map(action_sha256)
        .collect::<BTreeSet<_>>();
    if retained_actions
        .len()
        .saturating_add(phase.protocol_actions.len())
        > authority.maximum_successor_actions as usize
    {
        return Err(review(FleetEnsureSuccessorReviewReason::PhaseBound));
    }
    if phase
        .protocol_actions
        .iter()
        .any(|action| retained_actions.contains(&action_sha256(action)))
    {
        return Err(review(FleetEnsureSuccessorReviewReason::ProtocolAuthority));
    }
    let actual = verify_terminal_conservation(plan, journal, state, observation)?;
    let burn = successor_phase_burn(desired, &phase)?;
    if actual
        .measured_execution_burn_cycles
        .checked_add(burn)
        .is_none_or(|total| total > plan.conservation.maximum_execution_burn_cycles)
    {
        return Err(review(FleetEnsureSuccessorReviewReason::BudgetExceeded));
    }
    let candidate = candidate_journal(journal, &phase, actual.measured_execution_burn_cycles);
    verify_records(plan, &candidate)?;
    verify_canonical(plan, &candidate, state, platform)?;
    // The immutable phase must exist before the journal can expose authority to execute it.
    retain_phase(paths, &phase)?;
    write_journal(paths, &candidate)?;
    *journal = candidate;
    Ok(())
}

/// Exact immutable input identity shared by the original plan and each expansion.
#[derive(Eq, PartialEq)]
struct PhaseInputAuthority<'a> {
    desired_sha256: &'a str,
    environment: &'a str,
    fleet: &'a str,
    operation_id: &'a str,
    planned_at_time: u64,
    reviewed_desired: Option<&'a ReviewedDesiredFleetRecord>,
    schema_version: u16,
}

impl<'a> From<&'a FleetEnsurePlan> for PhaseInputAuthority<'a> {
    fn from(plan: &'a FleetEnsurePlan) -> Self {
        Self {
            desired_sha256: &plan.desired_sha256,
            environment: &plan.environment,
            fleet: &plan.fleet,
            operation_id: &plan.operation_id,
            planned_at_time: plan.planned_at_time,
            reviewed_desired: plan.reviewed_desired.as_deref(),
            schema_version: plan.schema_version,
        }
    }
}

fn phase_binding_matches(original: &FleetEnsurePlan, phase: &FleetEnsurePlan) -> bool {
    let fresh_scope = phase.continuation.is_none()
        && phase.scope == FleetEnsurePlanScope::Full
        && phase.root_start_authority.is_none()
        && phase.root_reinstall_bindings.is_empty()
        && phase.terminal_inventory_operation_id.is_none();
    fresh_scope && PhaseInputAuthority::from(original) == PhaseInputAuthority::from(phase)
}

fn phase_effects_are_protocol_only(original: &FleetEnsurePlan, phase: &FleetEnsurePlan) -> bool {
    let names = original
        .canisters
        .iter()
        .map(|canister| canister.name.as_str())
        .collect::<BTreeSet<_>>();
    let phase_names = phase
        .canisters
        .iter()
        .map(|canister| canister.name.as_str())
        .collect::<BTreeSet<_>>();
    let same_canisters = phase.canisters.len() == names.len()
        && phase_names == names
        && phase.canisters.iter().all(|canister| {
            canister.disposition == CanisterDisposition::Reuse && canister.actions.is_empty()
        });
    let no_new_debit = phase.conservation.maximum_operator_debit_cycles == 0
        && phase.conservation.maximum_new_funding_cycles == 0
        && phase.conservation.maximum_unavoidable_fee_cycles == 0
        && phase.conservation.scheduled_transfer_cycles == 0
        && phase
            .conservation
            .estate_funding_domains
            .iter()
            .all(|domain| {
                domain.maximum_creation_debit_cycles == 0 && domain.maximum_funding_cycles == 0
            });
    same_canisters
        && no_new_debit
        && !phase.protocol_actions.is_empty()
        && phase
            .protocol_actions
            .iter()
            .all(|action| matches!(action, EnsureAction::FleetProtocol { .. }))
}

const fn review<E: std::error::Error + 'static>(
    reason: FleetEnsureSuccessorReviewReason,
) -> EnsureWorkflowError<E> {
    EnsureWorkflowError::SuccessorReviewRequired { reason }
}
