//! Module: fleet_ensure::workflow::independent_effects
//!
//! Responsibility: retain and reconcile a bounded group of independent Store uploads or pool imports.
//! Does not own: transport, preparation admission or retry of retained effects.
//! Boundary: persist every intent before submission and every result before returning failure.

use crate::fleet_ensure::{
    model::{
        EffectState, EnsureAction, FleetEnsureJournalRecord, FleetEnsurePlan, FleetEnsurePlanScope,
        FleetEnsureStateRecord,
    },
    ops::{
        EnsurePaths, EnsurePlatform,
        effect_preparation::prepare_effect,
        independent_effects::{IndependentEffect, retain_observation, retain_outcome},
        write_journal,
    },
    policy::independent_effects::batch_len,
    workflow::{EnsureWorkflowError, action_progress_phase, report_progress},
};

#[expect(
    clippy::too_many_arguments,
    reason = "one batch shares the reviewed operation, journal and existing stall bound"
)]
pub(super) fn apply_batch<P: EnsurePlatform>(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
    actions: &[&EnsureAction],
    index: usize,
    journal: &mut FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
    maximum_stalled_observations: u32,
) -> Result<(), EnsureWorkflowError<P::Error>> {
    if plan.scope != FleetEnsurePlanScope::Full {
        return Ok(());
    }
    let count = batch_len(actions, &journal.effects, index);
    if count < 2 {
        return Ok(());
    }
    let batch = &actions[index..index + count];
    for action in batch {
        let prepared = prepare_effect(platform, &journal.operation_id, action, state)
            .map_err(EnsureWorkflowError::Platform)?;
        journal.effects.push(prepared.record);
    }
    write_journal(paths, journal)?;
    // Exact live observation still precedes every fresh submission. Already completed
    // effects become terminal without another update.
    for (offset, action) in batch.iter().enumerate() {
        let record = &mut journal.effects[index + offset];
        reconcile(platform, &journal.operation_id, action, record, state)?;
    }
    write_journal(paths, journal)?;
    let pending = batch
        .iter()
        .enumerate()
        .filter(|(offset, _)| journal.effects[index + offset].state != EffectState::Applied)
        .map(|(offset, action)| (index + offset, *action))
        .collect::<Vec<_>>();
    let uploads = pending
        .iter()
        .map(|(index, action)| IndependentEffect {
            action,
            record: &journal.effects[*index],
        })
        .collect::<Vec<_>>();
    if uploads.is_empty() {
        return Ok(());
    }
    let outcomes = platform
        .apply_independent_effects(&journal.operation_id, &uploads, state)
        .map_err(EnsureWorkflowError::Platform)?;
    if outcomes.len() != pending.len() {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    let mut first_error = None;
    // All workers have joined. Persist successful receipts even if an earlier
    // sibling failed, then observe every submitted effect including lost replies.
    for ((effect_index, action), outcome) in pending.into_iter().zip(outcomes) {
        let record = &mut journal.effects[effect_index];
        match outcome {
            Ok(outcome) => {
                if !retain_outcome(record, outcome) {
                    first_error
                        .get_or_insert((effect_index, EnsureWorkflowError::JournalIntegrity));
                }
            }
            Err(error) => {
                first_error.get_or_insert((effect_index, EnsureWorkflowError::Platform(error)));
            }
        }
        write_journal(paths, journal)?;
        let record = &mut journal.effects[effect_index];
        if let Err(error) = reconcile(platform, &journal.operation_id, action, record, state) {
            first_error.get_or_insert((effect_index, error));
        }
        let applied = journal.effects[effect_index].state == EffectState::Applied;
        if applied {
            journal.stalled_observations = 0;
        }
        write_journal(paths, journal)?;
        if applied {
            report_progress(platform, plan, journal, action_progress_phase(action));
        }
    }
    if let Some((failed_index, error)) = first_error {
        journal.stalled_observations = journal.stalled_observations.saturating_add(1);
        write_journal(paths, journal)?;
        if journal.stalled_observations >= maximum_stalled_observations {
            return Err(EnsureWorkflowError::Stalled {
                action: actions[failed_index].name().to_string(),
                observations: journal.stalled_observations,
                progress_identity: journal.effects[failed_index]
                    .progress_identity
                    .clone()
                    .unwrap_or_else(|| "effect-call-failed".into()),
            });
        }
        return Err(error);
    }
    Ok(())
}

fn reconcile<P: EnsurePlatform>(
    platform: &mut P,
    operation_id: &str,
    action: &EnsureAction,
    record: &mut crate::fleet_ensure::model::EffectRecord,
    state: &FleetEnsureStateRecord,
) -> Result<(), EnsureWorkflowError<P::Error>> {
    let observed = platform
        .observe_effect(operation_id, action, record, state)
        .map_err(EnsureWorkflowError::Platform)?;
    let source = match observed.post_cycles {
        Some(cycles) => Some(cycles),
        None => platform
            .action_cycles(action, state)
            .map_err(EnsureWorkflowError::Platform)?,
    };
    let destination = platform
        .action_destination_cycles(action, state)
        .map_err(EnsureWorkflowError::Platform)?;
    retain_observation(record, observed, source, destination);
    Ok(())
}
