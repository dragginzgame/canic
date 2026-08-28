//! Module: fleet_ensure::workflow
//!
//! Responsibility: plan and advance one durable idempotent Fleet convergence operation.
//! Does not own: policy decisions, IC transport, or storage mechanics.
//! Boundary: persists exact intent before each ops effect and reconciles it before any retry.

use crate::fleet_ensure::{
    inventory::project_current_fleet_inventory,
    model::{
        ActualCycleConservation, CanisterDisposition, CycleConservation, EffectRecord, EffectState,
        EnsureAction, FLEET_ENSURE_SCHEMA_VERSION, FleetEnsureCompletion, FleetEnsureJournalRecord,
        FleetEnsurePlan, FleetEnsureReport, FleetEnsureStateRecord, FleetObservation,
    },
    ops::{
        EnsurePaths, EnsurePlatform, EnsureStateError, action_sha256, compact_inline_plan,
        lock_operation, read_journal, read_plan, read_state, resolve_desired_artifacts,
        write_journal, write_plan, write_state,
    },
    policy::{
        EnsurePolicyError, compile_plan, expected_plan_sha256, operation_id,
        validate_path_identity, validate_path_labels,
    },
};
use std::{collections::BTreeMap, path::Path};
use thiserror::Error as ThisError;

/// Fleet ensure planning or convergence failure.

#[derive(Debug, ThisError)]
pub enum EnsureWorkflowError<E>
where
    E: std::error::Error + 'static,
{
    #[error("reviewed Fleet plan digest changed before its first effect")]
    DriftedBeforeApply,

    #[error(
        "in-progress Fleet plan requires desired input {expected}, but the retained plan predates desired-input retention and the supplied input is {actual}; supply the exact reviewed desired document"
    )]
    RetainedDesiredUnavailable { actual: String, expected: String },

    #[error("reviewed Fleet plan {expected} does not match retained plan {actual}")]
    PlanDigestMismatch { actual: String, expected: String },

    #[error("reviewed Fleet plan is missing; run `canic fleet ensure <fleet>` first")]
    PlanMissing,

    #[error("retained Fleet plan failed its content digest")]
    PlanIntegrity,

    #[error("retained Fleet journal does not match the reviewed Fleet operation")]
    JournalIntegrity,

    #[error(
        "Fleet ensure made no progress for {observations} consecutive observations; operation remains resumable"
    )]
    Stalled { observations: u32 },

    #[error(
        "selected Cycles Ledger account has {actual} cycles, below reviewed maximum debit {required}"
    )]
    InsufficientOperatorCycles { actual: u128, required: u128 },

    #[error("terminal Fleet cycle conservation failed: {0}")]
    Conservation(String),

    #[error("terminal Fleet inventory failed exact current-authority validation: {0}")]
    TerminalInventory(String),

    #[error(
        "reviewed effects completed, but live Fleet state still differs from desired state; review a new plan"
    )]
    ConvergenceDrift,

    #[error(transparent)]
    Policy(#[from] EnsurePolicyError),

    #[error(transparent)]
    Inventory(#[from] crate::fleet_ensure::CurrentFleetInventoryError),

    #[error("Fleet platform operation failed: {0}")]
    Platform(#[source] E),

    #[error(transparent)]
    State(#[from] EnsureStateError),
}

/// Load and verify the sole immutable in-progress operation for one exact Fleet path.
pub fn retained_in_progress_plan<E>(
    root: &Path,
    environment: &str,
    requested_fleet: &str,
) -> Result<Option<FleetEnsurePlan>, EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    validate_path_labels(environment, requested_fleet)?;
    let paths = EnsurePaths::under(root, environment, requested_fleet);
    let _lock = lock_operation(&paths)?;
    let Some(journal) = read_journal(&paths)? else {
        return Ok(None);
    };
    if journal.completion != FleetEnsureCompletion::InProgress {
        return Ok(None);
    }
    let retained = verified_plan(read_plan(&paths)?.ok_or(EnsureWorkflowError::PlanMissing)?)?;
    if retained.environment != environment || retained.fleet != requested_fleet {
        return Err(EnsureWorkflowError::PlanIntegrity);
    }
    verify_journal(&journal, &retained, requested_fleet)?;
    Ok(Some(retained))
}

/// Build and retain one read-only plan from current desired state plus live observation.
pub fn plan<P>(
    root: &Path,
    desired: &crate::fleet_ensure::model::DesiredFleet,
    desired_sha256: &str,
    requested_fleet: &str,
    created_at_time: u64,
    platform: &mut P,
) -> Result<FleetEnsureReport, EnsureWorkflowError<P::Error>>
where
    P: EnsurePlatform,
{
    validate_path_identity(desired, requested_fleet)?;
    let paths = EnsurePaths::under(root, &desired.environment, requested_fleet);
    let _lock = lock_operation(&paths)?;
    if let Some(journal) = read_journal(&paths)?
        && journal.completion == FleetEnsureCompletion::InProgress
    {
        let retained = verified_plan(read_plan(&paths)?.ok_or(EnsureWorkflowError::PlanMissing)?)?;
        verify_journal(&journal, &retained, requested_fleet)?;
        return Ok(FleetEnsureReport {
            actual_conservation: None,
            effects_applied: applied_count(&journal),
            plan: retained,
            terminal: false,
        });
    }
    let mut state = read_state(&paths, requested_fleet)?;
    let prior_plan = read_plan(&paths)?.map(verified_plan).transpose()?;
    if let Some(prior) = prior_plan.as_ref().filter(|prior| {
        prior.fleet == requested_fleet
            && prior.environment == desired.environment
            && prior.desired_sha256 == desired_sha256
    }) {
        retain_plan_cycles(&mut state, prior);
        if let Some(journal) = read_journal(&paths)? {
            retain_completed_reinstalls(&mut state, prior, &journal);
        }
    }
    let operation_id = operation_id(desired_sha256, &desired.environment, requested_fleet);
    let mut observation = platform
        .observe(&operation_id, &state)
        .map_err(EnsureWorkflowError::Platform)?;
    retain_observed_cycles(&mut state, &observation);
    write_state(&paths, &state)?;
    if state.active_registry.is_some() {
        let prior = verified_plan(read_plan(&paths)?.ok_or(EnsureWorkflowError::PlanMissing)?)?;
        let inventory = platform
            .terminal_inventory(&prior.operation_id, &state)
            .map_err(EnsureWorkflowError::Platform)?;
        attach_terminal_cycles(&mut observation, inventory.controlled_cycles_by_principal)?;
    }
    let protocol_actions = platform
        .protocol_actions(&operation_id, &state)
        .map_err(EnsureWorkflowError::Platform)?;
    let artifacts = resolve_desired_artifacts(root, desired)?;
    let plan = compile_plan(
        desired,
        &artifacts,
        &protocol_actions,
        desired_sha256,
        requested_fleet,
        &observation,
        created_at_time,
    )?;
    write_plan(&paths, &plan)?;
    Ok(FleetEnsureReport {
        actual_conservation: None,
        effects_applied: 0,
        plan,
        terminal: false,
    })
}

fn retain_completed_reinstalls(
    state: &mut FleetEnsureStateRecord,
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
) {
    for (action, effect) in ordered_actions(plan).into_iter().zip(&journal.effects) {
        let EnsureAction::Install {
            mode: crate::fleet_ensure::model::InstallMode::Reinstall,
            name,
            ..
        } = action
        else {
            continue;
        };
        if effect.state == EffectState::Applied
            && let Some(pre_canister_version) = effect.pre_canister_version
        {
            state
                .completed_reinstalls
                .insert(name.clone(), pre_canister_version);
        }
    }
}

fn retain_plan_cycles(state: &mut FleetEnsureStateRecord, plan: &FleetEnsurePlan) {
    for canister in &plan.canisters {
        if let Some(principal) = &canister.principal {
            state
                .retained_cycles_by_principal
                .entry(principal.clone())
                .or_insert(canister.observed_cycles);
        }
    }
}

fn retain_observed_cycles(state: &mut FleetEnsureStateRecord, observation: &FleetObservation) {
    for live in observation.canisters.values().filter_map(Option::as_ref) {
        state
            .retained_cycles_by_principal
            .insert(live.principal.clone(), live.cycles);
    }
    state
        .retained_cycles_by_principal
        .extend(observation.additional_controlled_cycles.clone());
}

/// Apply or resume exactly one reviewed plan until it converges or returns a typed blocker.
#[expect(
    clippy::too_many_lines,
    reason = "the sole effect driver keeps intent, reconciliation and terminal publication visibly ordered"
)]
pub fn apply<P>(
    root: &Path,
    desired: &crate::fleet_ensure::model::DesiredFleet,
    desired_sha256: &str,
    requested_fleet: &str,
    reviewed_plan_sha256: &str,
    platform: &mut P,
) -> Result<FleetEnsureReport, EnsureWorkflowError<P::Error>>
where
    P: EnsurePlatform,
{
    validate_path_identity(desired, requested_fleet)?;
    let paths = EnsurePaths::under(root, &desired.environment, requested_fleet);
    let _lock = lock_operation(&paths)?;
    let retained_plan = verified_plan(read_plan(&paths)?.ok_or(EnsureWorkflowError::PlanMissing)?)?;
    if retained_plan.plan_sha256 != reviewed_plan_sha256 {
        return Err(EnsureWorkflowError::PlanDigestMismatch {
            actual: retained_plan.plan_sha256,
            expected: reviewed_plan_sha256.to_string(),
        });
    }
    let mut state = read_state(&paths, requested_fleet)?;
    let retained_journal = read_journal(&paths)?;
    let in_progress = retained_journal
        .as_ref()
        .is_some_and(|journal| journal.completion == FleetEnsureCompletion::InProgress);
    if let Some(journal) = retained_journal.as_ref().filter(|_| in_progress) {
        verify_journal(journal, &retained_plan, requested_fleet)?;
        compact_inline_plan(&paths, &retained_plan)?;
    }
    let mut issued_observation_resume = false;
    let operation_desired = if in_progress {
        if let Some(reviewed) = retained_plan.reviewed_desired.as_deref() {
            let reviewed = reviewed.desired();
            validate_path_identity(reviewed, requested_fleet)?;
            if reviewed.environment != retained_plan.environment {
                return Err(EnsureWorkflowError::PlanIntegrity);
            }
            reviewed
        } else if retained_plan.desired_sha256 == desired_sha256 {
            desired
        } else if retained_journal
            .as_ref()
            .is_some_and(|journal| safe_issued_observation_resume(&retained_plan, journal, desired))
        {
            issued_observation_resume = true;
            desired
        } else {
            return Err(EnsureWorkflowError::RetainedDesiredUnavailable {
                actual: desired_sha256.to_string(),
                expected: retained_plan.desired_sha256.clone(),
            });
        }
    } else {
        if retained_plan.desired_sha256 != desired_sha256 {
            return Err(EnsureWorkflowError::DriftedBeforeApply);
        }
        desired
    };
    platform
        .bind_reviewed_desired(operation_desired)
        .map_err(EnsureWorkflowError::Platform)?;
    let operation_desired_sha256 = retained_plan.desired_sha256.as_str();
    let mut journal = match retained_journal {
        Some(journal) if journal.completion == FleetEnsureCompletion::InProgress => {
            verify_journal(&journal, &retained_plan, requested_fleet)?;
            journal
        }
        _ => {
            let observation = verify_fresh_plan(
                root,
                operation_desired,
                operation_desired_sha256,
                requested_fleet,
                &retained_plan,
                platform,
                &state,
            )?;
            let journal = FleetEnsureJournalRecord {
                completion: FleetEnsureCompletion::InProgress,
                effects: Vec::new(),
                fleet: requested_fleet.to_string(),
                initial_controlled_cycles: retained_plan.conservation.observed_controlled_cycles,
                initial_operator_cycles: observation.operator_cycles,
                operation_id: retained_plan.operation_id.clone(),
                plan_sha256: retained_plan.plan_sha256.clone(),
                schema_version: FLEET_ENSURE_SCHEMA_VERSION,
                stalled_observations: 0,
            };
            write_journal(&paths, &journal)?;
            journal
        }
    };

    let actions = ordered_actions(&retained_plan);
    for (index, action) in actions.iter().enumerate() {
        let action_hash = action_sha256(action);
        if journal.effects.len() <= index {
            let pre_cycles = platform
                .action_cycles(action, &state)
                .map_err(EnsureWorkflowError::Platform)?;
            let destination_pre_cycles = platform
                .action_destination_cycles(action, &state)
                .map_err(EnsureWorkflowError::Platform)?;
            let pre_canister_version = platform
                .action_canister_version(action, &state)
                .map_err(EnsureWorkflowError::Platform)?;
            journal.effects.push(EffectRecord {
                action_sha256: action_hash.clone(),
                created_principal: None,
                destination_post_cycles: destination_pre_cycles,
                destination_pre_cycles,
                post_cycles: None,
                pre_cycles,
                pre_canister_version,
                progress_identity: None,
                receipt: None,
                state: EffectState::Intent,
            });
            write_journal(&paths, &journal)?;
        }
        loop {
            let record = journal
                .effects
                .get(index)
                .ok_or(EnsureWorkflowError::JournalIntegrity)?;
            if record.action_sha256 != action_hash {
                return Err(EnsureWorkflowError::DriftedBeforeApply);
            }
            if matches!(record.state, EffectState::Applied) {
                break;
            }

            let observed = platform
                .observe_effect(&journal.operation_id, action, record, &state)
                .map_err(EnsureWorkflowError::Platform)?;
            let source_cycles = platform
                .action_cycles(action, &state)
                .map_err(EnsureWorkflowError::Platform)?;
            let destination_cycles = platform
                .action_destination_cycles(action, &state)
                .map_err(EnsureWorkflowError::Platform)?;
            let record = journal
                .effects
                .get_mut(index)
                .ok_or(EnsureWorkflowError::JournalIntegrity)?;
            record.post_cycles = source_cycles;
            record.destination_post_cycles = destination_cycles;
            if observed.applied {
                record.progress_identity = Some(observed.progress_identity);
                record.state = EffectState::Applied;
                journal.stalled_observations = 0;
                write_journal(&paths, &journal)?;
                break;
            }

            if matches!(record.state, EffectState::Intent) {
                let outcome = match platform.apply(&journal.operation_id, action, record, &state) {
                    Ok(outcome) => outcome,
                    Err(source) => {
                        journal.stalled_observations =
                            journal.stalled_observations.saturating_add(1);
                        write_journal(&paths, &journal)?;
                        if journal.stalled_observations
                            >= operation_desired.maximum_stalled_observations
                        {
                            return Err(EnsureWorkflowError::Stalled {
                                observations: journal.stalled_observations,
                            });
                        }
                        return Err(EnsureWorkflowError::Platform(source));
                    }
                };
                if let Some(created) = &outcome.created_principal {
                    state
                        .pending_principals
                        .insert(action.name().to_string(), created.clone());
                    write_state(&paths, &state)?;
                }
                let record = journal
                    .effects
                    .get_mut(index)
                    .ok_or(EnsureWorkflowError::JournalIntegrity)?;
                record.created_principal = outcome.created_principal;
                record.receipt = outcome.receipt;
                record.post_cycles = outcome.post_cycles;
                record.progress_identity = Some(observed.progress_identity);
                record.state = EffectState::Issued;
                journal.stalled_observations = 0;
                write_journal(&paths, &journal)?;
                continue;
            }

            if record.progress_identity.as_deref() == Some(&observed.progress_identity) {
                journal.stalled_observations = journal.stalled_observations.saturating_add(1);
            } else {
                record.progress_identity = Some(observed.progress_identity);
                journal.stalled_observations = 0;
            }
            write_journal(&paths, &journal)?;
            if journal.stalled_observations >= operation_desired.maximum_stalled_observations {
                return Err(EnsureWorkflowError::Stalled {
                    observations: journal.stalled_observations,
                });
            }
        }
    }

    let mut terminal_state = state.clone();
    publish_terminal_state(operation_desired, &retained_plan, &mut terminal_state);
    project_current_fleet_inventory(&terminal_state)?;
    let terminal_observation = platform
        .observe(&retained_plan.operation_id, &terminal_state)
        .map_err(EnsureWorkflowError::Platform)?;
    if !issued_observation_resume {
        let artifacts = resolve_desired_artifacts(root, operation_desired)?;
        let protocol_actions = platform
            .protocol_actions(&retained_plan.operation_id, &terminal_state)
            .map_err(EnsureWorkflowError::Platform)?;
        let converged = compile_plan(
            operation_desired,
            &artifacts,
            &protocol_actions,
            operation_desired_sha256,
            requested_fleet,
            &terminal_observation,
            retained_plan.planned_at_time,
        )?;
        if converged
            .canisters
            .iter()
            .any(|canister| !canister.actions.is_empty())
            || !converged.protocol_actions.is_empty()
        {
            journal.completion = FleetEnsureCompletion::ReplanRequired;
            journal.stalled_observations = 0;
            write_journal(&paths, &journal)?;
            return Err(EnsureWorkflowError::ConvergenceDrift);
        }
    }
    let terminal_inventory = platform
        .terminal_inventory(&retained_plan.operation_id, &terminal_state)
        .map_err(EnsureWorkflowError::Platform)?;
    let terminal_cycles = terminal_inventory.controlled_cycles_by_principal.clone();
    merge_terminal_inventory(&mut terminal_state, terminal_inventory)?;
    project_current_fleet_inventory(&terminal_state)?;
    let mut final_observation = platform
        .observe(&retained_plan.operation_id, &terminal_state)
        .map_err(EnsureWorkflowError::Platform)?;
    attach_terminal_cycles(&mut final_observation, terminal_cycles)?;
    let actual_conservation =
        verify_terminal_conservation(&retained_plan, &journal, &final_observation)?;
    write_state(&paths, &terminal_state)?;
    journal.completion = FleetEnsureCompletion::Converged;
    journal.stalled_observations = 0;
    write_journal(&paths, &journal)?;
    Ok(FleetEnsureReport {
        actual_conservation: Some(actual_conservation),
        effects_applied: applied_count(&journal),
        plan: retained_plan,
        terminal: true,
    })
}

fn verify_fresh_plan<P>(
    root: &Path,
    desired: &crate::fleet_ensure::model::DesiredFleet,
    desired_sha256: &str,
    requested_fleet: &str,
    retained_plan: &FleetEnsurePlan,
    platform: &mut P,
    state: &FleetEnsureStateRecord,
) -> Result<FleetObservation, EnsureWorkflowError<P::Error>>
where
    P: EnsurePlatform,
{
    let observation = platform
        .observe(&retained_plan.operation_id, state)
        .map_err(EnsureWorkflowError::Platform)?;
    if observation.operator_cycles < retained_plan.conservation.maximum_operator_debit_cycles {
        return Err(EnsureWorkflowError::InsufficientOperatorCycles {
            actual: observation.operator_cycles,
            required: retained_plan.conservation.maximum_operator_debit_cycles,
        });
    }
    let artifacts = resolve_desired_artifacts(root, desired)?;
    let protocol_actions = platform
        .protocol_actions(&retained_plan.operation_id, state)
        .map_err(EnsureWorkflowError::Platform)?;
    let current = compile_plan(
        desired,
        &artifacts,
        &protocol_actions,
        desired_sha256,
        requested_fleet,
        &observation,
        retained_plan.planned_at_time,
    )?;
    if current.plan_sha256 != retained_plan.plan_sha256
        && !compatible_after_bounded_observation(retained_plan, &current, desired, &observation)
    {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    Ok(observation)
}

fn compatible_after_bounded_observation(
    retained: &FleetEnsurePlan,
    current: &FleetEnsurePlan,
    desired: &crate::fleet_ensure::model::DesiredFleet,
    observation: &FleetObservation,
) -> bool {
    let Ok(maximum_observation_burn) = desired.maximum_observation_burn_cycles.parse::<u128>()
    else {
        return false;
    };
    if retained.canisters.len() != current.canisters.len() {
        return false;
    }
    for (retained_canister, current_canister) in retained.canisters.iter().zip(&current.canisters) {
        let Some(decrease) = retained_canister
            .observed_cycles
            .checked_sub(current_canister.observed_cycles)
        else {
            return false;
        };
        if decrease > maximum_observation_burn {
            return false;
        }
    }
    if !retained_funding_remains_sufficient(retained, desired, observation) {
        return false;
    }
    normalized_plan(retained) == normalized_plan(current)
}

fn retained_funding_remains_sufficient(
    retained: &FleetEnsurePlan,
    desired: &crate::fleet_ensure::model::DesiredFleet,
    observation: &FleetObservation,
) -> bool {
    retained
        .canisters
        .iter()
        .flat_map(|canister| &canister.actions)
        .filter_map(|action| match action {
            EnsureAction::Fund { amount, name, .. } => Some((amount, name)),
            _ => None,
        })
        .all(|(amount, name)| {
            let Some(live) = observation.canisters.get(name).and_then(Option::as_ref) else {
                return false;
            };
            let Some(configured) = desired
                .canisters
                .iter()
                .find(|configured| configured.name == *name)
            else {
                return false;
            };
            let Ok(minimum) = configured.minimum_cycles.parse::<u128>() else {
                return false;
            };
            live.cycles
                .checked_add(*amount)
                .is_some_and(|funded| funded >= minimum)
        })
}

fn normalized_plan(plan: &FleetEnsurePlan) -> FleetEnsurePlan {
    let mut normalized = plan.clone();
    normalized.plan_sha256.clear();
    normalized.reviewed_desired = None;
    normalized.conservation = CycleConservation {
        expected_post_operation_cycles: 0,
        maximum_execution_burn_cycles: 0,
        maximum_new_funding_cycles: 0,
        maximum_operator_debit_cycles: 0,
        maximum_unavoidable_fee_cycles: 0,
        observed_controlled_cycles: 0,
        retained_in_reused_canisters_cycles: 0,
        scheduled_transfer_cycles: 0,
    };
    for canister in &mut normalized.canisters {
        canister.observed_cycles = 0;
        for action in &mut canister.actions {
            if let EnsureAction::Fund {
                amount,
                funding_deficit_cycles,
                ..
            } = action
            {
                *amount = 0;
                *funding_deficit_cycles = 0;
            }
        }
    }
    normalized
}

fn safe_issued_observation_resume(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    supplied: &crate::fleet_ensure::model::DesiredFleet,
) -> bool {
    if plan.conservation.maximum_new_funding_cycles != 0
        || plan.conservation.maximum_operator_debit_cycles != 0
        || plan.conservation.maximum_unavoidable_fee_cycles != 0
        || plan.canisters.iter().any(|canister| {
            canister.disposition != CanisterDisposition::Reuse
                || !canister.actions.is_empty()
                || canister.principal.is_none()
        })
    {
        return false;
    }
    let supplied_principals = supplied
        .canisters
        .iter()
        .filter(|canister| {
            canister.presence == crate::fleet_ensure::model::DesiredPresence::Present
                && !canister.replace
        })
        .filter_map(|canister| {
            canister
                .principal
                .as_ref()
                .map(|principal| (canister.name.as_str(), principal.as_str()))
        })
        .collect::<BTreeMap<_, _>>();
    if supplied_principals.len() != plan.canisters.len()
        || plan.canisters.iter().any(|canister| {
            canister.principal.as_deref().is_none_or(|principal| {
                supplied_principals.get(canister.name.as_str()).copied() != Some(principal)
            })
        })
    {
        return false;
    }
    let actions = ordered_actions(plan);
    if actions.len() != journal.effects.len() || actions.is_empty() {
        return false;
    }
    if actions
        .iter()
        .zip(&journal.effects)
        .any(|(action, effect)| action_sha256(action) != effect.action_sha256)
    {
        return false;
    }
    let Some((last_action, last_effect)) = actions.last().zip(journal.effects.last()) else {
        return false;
    };
    journal.effects[..journal.effects.len() - 1]
        .iter()
        .all(|effect| effect.state == EffectState::Applied)
        && matches!(
            last_effect.state,
            EffectState::Issued | EffectState::Applied
        )
        && matches!(
            last_action,
            EnsureAction::FleetProtocol { action, .. }
                if matches!(
                    action.as_ref(),
                    crate::fleet_ensure::model::CurrentFleetProtocolAction::ProvisionComponents { .. }
                )
        )
}

fn attach_terminal_cycles<E>(
    observation: &mut FleetObservation,
    additional: BTreeMap<String, u128>,
) -> Result<(), EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    let mut retained = BTreeMap::new();
    for (principal, cycles) in additional {
        let Some(configured) = observation
            .canisters
            .values_mut()
            .filter_map(Option::as_mut)
            .find(|canister| canister.principal == principal)
        else {
            retained.insert(principal, cycles);
            continue;
        };
        if configured.root_owned_lifecycle
            != Some(crate::fleet_ensure::model::RootOwnedCanisterLifecycle::Workload)
        {
            return Err(EnsureWorkflowError::TerminalInventory(
                "terminal cycle observation duplicates a configured canister outside its exact Root-owned workload lifecycle"
                    .to_string(),
            ));
        }
        // Both protected observations identify the same Root-owned workload, but execution
        // may burn cycles between them. Retaining the lower balance is conservative in both
        // planning and terminal conservation regardless of observation order.
        configured.cycles = configured.cycles.min(cycles);
    }
    observation.additional_controlled_cycles = retained;
    Ok(())
}

fn verify_terminal_conservation<E>(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    terminal: &FleetObservation,
) -> Result<ActualCycleConservation, EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    let final_controlled_cycles = controlled_cycles(terminal)?;
    let operator_debit_cycles = journal
        .initial_operator_cycles
        .checked_sub(terminal.operator_cycles)
        .ok_or_else(|| {
            EnsureWorkflowError::Conservation(
                "operator balance increased during apply; review a new plan".to_string(),
            )
        })?;
    if operator_debit_cycles > plan.conservation.maximum_operator_debit_cycles {
        return Err(EnsureWorkflowError::Conservation(format!(
            "operator debit {operator_debit_cycles} exceeded reviewed maximum {}",
            plan.conservation.maximum_operator_debit_cycles
        )));
    }
    let received_new_funding_cycles = operator_debit_cycles
        .checked_sub(plan.conservation.maximum_unavoidable_fee_cycles)
        .ok_or_else(|| {
            EnsureWorkflowError::Conservation(
                "operator debit was below the exact reviewed fee total".to_string(),
            )
        })?;
    if received_new_funding_cycles > plan.conservation.maximum_new_funding_cycles {
        return Err(EnsureWorkflowError::Conservation(format!(
            "received funding {received_new_funding_cycles} exceeded reviewed maximum {}",
            plan.conservation.maximum_new_funding_cycles
        )));
    }
    let available = journal
        .initial_controlled_cycles
        .checked_add(received_new_funding_cycles)
        .ok_or_else(|| {
            EnsureWorkflowError::Conservation(
                "terminal controlled-cycle arithmetic overflowed".to_string(),
            )
        })?;
    let measured_execution_burn_cycles = available
        .checked_sub(final_controlled_cycles)
        .ok_or_else(|| {
            EnsureWorkflowError::Conservation(
                "terminal estate contains more cycles than starting estate plus received funding"
                    .to_string(),
            )
        })?;
    if measured_execution_burn_cycles > plan.conservation.maximum_execution_burn_cycles {
        return Err(EnsureWorkflowError::Conservation(format!(
            "measured execution burn {measured_execution_burn_cycles} exceeded reviewed maximum {}",
            plan.conservation.maximum_execution_burn_cycles
        )));
    }
    Ok(ActualCycleConservation {
        exact_unavoidable_fee_cycles: plan.conservation.maximum_unavoidable_fee_cycles,
        final_controlled_cycles,
        measured_execution_burn_cycles,
        observed_starting_cycles: journal.initial_controlled_cycles,
        operator_debit_cycles,
        received_new_funding_cycles,
    })
}

fn controlled_cycles<E>(observation: &FleetObservation) -> Result<u128, EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    observation
        .canisters
        .values()
        .filter_map(Option::as_ref)
        .try_fold(0_u128, |total, live| total.checked_add(live.cycles))
        .and_then(|total| {
            observation
                .additional_controlled_cycles
                .values()
                .try_fold(total, |subtotal, cycles| subtotal.checked_add(*cycles))
        })
        .ok_or_else(|| {
            EnsureWorkflowError::Conservation("controlled-cycle total overflowed".to_string())
        })
}

fn verified_plan<E>(plan: FleetEnsurePlan) -> Result<FleetEnsurePlan, EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    if expected_plan_sha256(&plan) != plan.plan_sha256 {
        return Err(EnsureWorkflowError::PlanIntegrity);
    }
    Ok(plan)
}

fn verify_journal<E>(
    journal: &FleetEnsureJournalRecord,
    plan: &FleetEnsurePlan,
    requested_fleet: &str,
) -> Result<(), EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    if journal.fleet != requested_fleet
        || journal.operation_id != plan.operation_id
        || journal.plan_sha256 != plan.plan_sha256
    {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    Ok(())
}

fn publish_terminal_state(
    desired: &crate::fleet_ensure::model::DesiredFleet,
    plan: &FleetEnsurePlan,
    state: &mut FleetEnsureStateRecord,
) {
    for canister in &plan.canisters {
        match canister.disposition {
            CanisterDisposition::Create | CanisterDisposition::Replace => {
                if let Some(principal) = state.pending_principals.remove(&canister.name) {
                    state.principals.insert(canister.name.clone(), principal);
                }
            }
            CanisterDisposition::Reinstall | CanisterDisposition::Reuse => {
                if let Some(principal) = state
                    .pending_principals
                    .remove(&canister.name)
                    .or_else(|| canister.principal.clone())
                {
                    state.principals.insert(canister.name.clone(), principal);
                }
            }
            CanisterDisposition::Delete => {
                state.principals.remove(&canister.name);
                state.pending_principals.remove(&canister.name);
            }
        }
    }
    state.topology = desired
        .canisters
        .iter()
        .filter(|canister| {
            canister.presence == crate::fleet_ensure::model::DesiredPresence::Present
        })
        .map(|canister| {
            (
                canister.name.clone(),
                crate::fleet_ensure::model::FleetEnsureTopologyRecord {
                    kind: canister.kind,
                    module_hash: None,
                    parent: canister.parent.clone(),
                    protocol_binding: canister.protocol_binding.clone(),
                    role: canister
                        .protocol_binding
                        .as_ref()
                        .map(|binding| binding.role.to_string()),
                },
            )
        })
        .collect();
}

fn merge_terminal_inventory<E>(
    state: &mut FleetEnsureStateRecord,
    inventory: crate::fleet_ensure::ops::TerminalFleetInventory,
) -> Result<(), EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    if inventory.entries.is_empty() {
        state.active_registry = inventory.active_registry;
        return Ok(());
    }
    let entries = inventory.entries;
    let mut names_by_principal = state
        .principals
        .iter()
        .map(|(name, principal)| (principal.clone(), name.clone()))
        .collect::<BTreeMap<_, _>>();
    if names_by_principal.len() != state.principals.len() {
        return Err(EnsureWorkflowError::TerminalInventory(
            "one Principal is retained under more than one desired role".to_string(),
        ));
    }
    for entry in &entries {
        names_by_principal
            .entry(entry.pid.clone())
            .or_insert_with(|| format!("observed:{}", entry.pid));
    }
    for entry in entries {
        let name = names_by_principal.get(&entry.pid).cloned().ok_or_else(|| {
            EnsureWorkflowError::TerminalInventory(
                "verified entry lost its Principal binding".to_string(),
            )
        })?;
        let parent = entry
            .parent_pid
            .as_ref()
            .map(|principal| {
                names_by_principal.get(principal).cloned().ok_or_else(|| {
                    EnsureWorkflowError::TerminalInventory(format!(
                        "Canister {} names unknown parent {principal}",
                        entry.pid
                    ))
                })
            })
            .transpose()?;
        let existing = state.topology.get(&name);
        if existing
            .and_then(|topology| topology.parent.as_ref())
            .zip(parent.as_ref())
            .is_some_and(|(expected, actual)| expected != actual)
        {
            return Err(EnsureWorkflowError::TerminalInventory(format!(
                "Canister {} conflicts with retained parent authority",
                entry.pid
            )));
        }
        let kind = existing.map_or_else(
            || {
                if entry.module_hash.is_some() {
                    crate::fleet_ensure::model::DesiredCanisterKind::Component
                } else {
                    crate::fleet_ensure::model::DesiredCanisterKind::Auxiliary
                }
            },
            |topology| {
                if topology.kind == crate::fleet_ensure::model::DesiredCanisterKind::Pool
                    && entry.module_hash.is_some()
                    && entry.protocol_binding.is_some()
                {
                    crate::fleet_ensure::model::DesiredCanisterKind::Component
                } else {
                    topology.kind
                }
            },
        );
        state.principals.insert(name.clone(), entry.pid);
        state.topology.insert(
            name,
            crate::fleet_ensure::model::FleetEnsureTopologyRecord {
                kind,
                module_hash: entry.module_hash,
                parent,
                protocol_binding: entry.protocol_binding,
                role: entry.role,
            },
        );
    }
    state.active_registry = inventory.active_registry;
    Ok(())
}

fn applied_count(journal: &FleetEnsureJournalRecord) -> u32 {
    u32::try_from(
        journal
            .effects
            .iter()
            .filter(|effect| matches!(effect.state, EffectState::Applied))
            .count(),
    )
    .unwrap_or(u32::MAX)
}

pub(super) fn ordered_actions(plan: &FleetEnsurePlan) -> Vec<&EnsureAction> {
    let mut actions = plan
        .canisters
        .iter()
        .flat_map(|canister| canister.actions.iter())
        .chain(plan.protocol_actions.iter())
        .collect::<Vec<_>>();
    actions.sort_by_key(|action| action_order(action));
    actions
}

pub(super) fn action_order(action: &EnsureAction) -> u8 {
    match action {
        EnsureAction::Create { .. } => 0,
        EnsureAction::Fund { .. } => 1,
        EnsureAction::Install {
            canic_init: Some(crate::fleet_ensure::model::DesiredCanisterInit::Coordinator),
            ..
        } => 2,
        EnsureAction::FleetProtocol { action, .. }
            if matches!(
                action.as_ref(),
                crate::fleet_ensure::model::CurrentFleetProtocolAction::AdoptStore { .. }
            ) =>
        {
            3
        }
        EnsureAction::Install {
            canic_init: Some(crate::fleet_ensure::model::DesiredCanisterInit::Store { .. }),
            ..
        } => 4,
        EnsureAction::Install { .. } => 5,
        EnsureAction::SetControllers { .. } | EnsureAction::Start { .. } => 6,
        EnsureAction::FleetProtocol { .. } | EnsureAction::Protocol { .. } => 7,
        EnsureAction::Transfer { .. } => 8,
        EnsureAction::Stop { .. } => 9,
        EnsureAction::Delete { .. } => 10,
    }
}
