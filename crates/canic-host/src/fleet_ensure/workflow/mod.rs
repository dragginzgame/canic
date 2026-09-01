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
        FleetEnsurePlan, FleetEnsurePlanScope, FleetEnsureReport, FleetEnsureStateRecord,
        FleetObservation, RootManagementObservation,
    },
    ops::{
        EffectRetry, EnsurePaths, EnsurePlatform, EnsureStateError, action_sha256,
        compact_inline_plan, lock_operation, read_journal, read_plan, read_root_start_authority,
        read_state, resolve_desired_artifacts, verify_root_start_release_authority, write_journal,
        write_plan, write_state,
    },
    policy::{
        EnsurePolicyError, RootStartPlanInput, compile_plan, compile_root_start_prerequisite_plan,
        expected_plan_sha256, operation_id, recompile_root_start_prerequisite_plan,
        validate_path_identity, validate_path_labels,
    },
};
use canic_core::cdk::types::Cycles;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
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

    #[error(
        "replan-required Fleet operation owns completed reinstall evidence for desired input {expected}; refusing alternate desired input {actual}"
    )]
    RetainedReinstallDesiredConflict { actual: String, expected: String },

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

    #[error(
        "retained Fleet prerequisite was synchronously rejected before mutation ({evidence}); the immutable operation is now replan-required"
    )]
    ReplanRequiredAfterRejectedPrerequisite { evidence: String },

    #[error(
        "created canister {canister} retained one exact Ledger creation receipt, but its first live balance is {actual_cycles} cycles instead of requested target {requested_cycles} (deficit {deficit_cycles}; configured management creation fee {configured_fee_cycles}); no controller or protocol action followed, and the immutable operation is now replan-required"
    )]
    ReplanRequiredAfterCreateBalanceDrift {
        actual_cycles: u128,
        canister: String,
        configured_fee_cycles: u128,
        deficit_cycles: u128,
        requested_cycles: u128,
    },

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
    let prior_journal = read_journal(&paths)?;
    let matching_prior = reconcile_retained_planning_state(
        &mut state,
        prior_plan.as_ref(),
        prior_journal.as_ref(),
        requested_fleet,
        &desired.environment,
        desired_sha256,
    )?;
    if let Some(prior) = matching_prior {
        retain_plan_cycles(&mut state, prior);
        if let Some(journal) = &prior_journal {
            retain_completed_reinstalls(&mut state, prior, journal);
        }
    }
    let operation_id = operation_id(desired_sha256, &desired.environment, requested_fleet);
    if let Some(management) = platform
        .observe_root_management(&state, &BTreeSet::new())
        .map_err(EnsureWorkflowError::Platform)?
    {
        let root_start_authority = read_root_start_authority(&paths)?;
        if let Some(authority) = &root_start_authority {
            verify_root_start_release_authority(root, authority)?;
        }
        if let Some(plan) = compile_root_start_prerequisite_plan(RootStartPlanInput {
            authority: root_start_authority.as_ref(),
            created_at_time,
            desired,
            desired_sha256,
            observation: &management,
            requested_fleet,
        })? {
            write_plan(&paths, &plan)?;
            return Ok(FleetEnsureReport {
                actual_conservation: None,
                effects_applied: 0,
                plan,
                terminal: false,
            });
        }
    }
    let artifacts = resolve_desired_artifacts(root, desired)?;
    let mut observation = platform
        .observe(&operation_id, &state)
        .map_err(EnsureWorkflowError::Platform)?;
    retain_observed_cycles(&mut state, &observation);
    write_state(&paths, &state)?;
    let terminal_inventory_operation_id = current_terminal_inventory_operation(
        &state,
        prior_plan.as_ref(),
        prior_journal.as_ref(),
        requested_fleet,
    )?;
    if let Some(operation_id) = terminal_inventory_operation_id.as_deref() {
        attach_terminal_inventory_cycles(operation_id, &state, platform, &mut observation)?;
    }
    let protocol_actions = platform
        .protocol_actions(&operation_id, &state)
        .map_err(EnsureWorkflowError::Platform)?;
    let mut plan = compile_plan(
        desired,
        &artifacts,
        &protocol_actions,
        desired_sha256,
        requested_fleet,
        &observation,
        created_at_time,
    )?;
    bind_terminal_inventory_operation(&mut plan, terminal_inventory_operation_id);
    write_plan(&paths, &plan)?;
    Ok(FleetEnsureReport {
        actual_conservation: None,
        effects_applied: 0,
        plan,
        terminal: false,
    })
}

fn current_terminal_inventory_operation<E>(
    state: &FleetEnsureStateRecord,
    prior_plan: Option<&FleetEnsurePlan>,
    prior_journal: Option<&FleetEnsureJournalRecord>,
    requested_fleet: &str,
) -> Result<Option<String>, EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    if state.active_registry.is_none() {
        return Ok(None);
    }
    let prior_plan = prior_plan.ok_or(EnsureWorkflowError::PlanMissing)?;
    let prior_journal = prior_journal.ok_or(EnsureWorkflowError::JournalIntegrity)?;
    verify_journal(prior_journal, prior_plan, requested_fleet)?;
    if prior_journal.completion != FleetEnsureCompletion::Converged
        || prior_plan.scope != FleetEnsurePlanScope::Full
    {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    let operation_id = prior_plan
        .terminal_inventory_operation_id
        .as_ref()
        .unwrap_or(&prior_plan.operation_id);
    if operation_id.is_empty() {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    Ok(Some(operation_id.clone()))
}

fn bind_terminal_inventory_operation(plan: &mut FleetEnsurePlan, operation_id: Option<String>) {
    plan.terminal_inventory_operation_id = operation_id;
    plan.plan_sha256 = expected_plan_sha256(plan);
}

fn reconcile_retained_planning_state<'a, E>(
    state: &mut FleetEnsureStateRecord,
    prior_plan: Option<&'a FleetEnsurePlan>,
    prior_journal: Option<&FleetEnsureJournalRecord>,
    requested_fleet: &str,
    desired_environment: &str,
    desired_sha256: &str,
) -> Result<Option<&'a FleetEnsurePlan>, EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    if let Some((prior, journal)) = prior_plan.zip(prior_journal)
        && prior.fleet == requested_fleet
        && prior.environment == desired_environment
        && retained_reinstall_desired_conflict(
            state,
            &prior.fleet,
            &prior.operation_id,
            &prior.desired_sha256,
            desired_sha256,
            journal,
        )
    {
        return Err(EnsureWorkflowError::RetainedReinstallDesiredConflict {
            actual: desired_sha256.to_string(),
            expected: prior.desired_sha256.clone(),
        });
    }
    let matching_prior = prior_plan.filter(|prior| {
        prior.fleet == requested_fleet
            && prior.environment == desired_environment
            && prior.desired_sha256 == desired_sha256
    });
    let retained_operation_matches =
        matching_prior
            .zip(prior_journal)
            .is_some_and(|(prior, journal)| {
                retained_reinstall_operation_matches(
                    state,
                    &prior.fleet,
                    &prior.operation_id,
                    journal,
                )
            });
    if !retained_operation_matches {
        state.completed_reinstall_action_sha256.clear();
        state.completed_reinstall_operation_id = None;
        state.completed_reinstalls.clear();
    }
    Ok(matching_prior)
}

fn retain_completed_reinstalls(
    state: &mut FleetEnsureStateRecord,
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
) {
    if journal.fleet != plan.fleet || journal.operation_id != plan.operation_id {
        return;
    }
    for action in ordered_actions(plan) {
        let EnsureAction::Install {
            mode: crate::fleet_ensure::model::InstallMode::Reinstall,
            name,
            principal,
            wasm_sha256,
            ..
        } = action
        else {
            continue;
        };
        let action_sha256 = action_sha256(action);
        let Some(effect) = journal
            .effects
            .iter()
            .find(|effect| effect.action_sha256 == action_sha256)
        else {
            continue;
        };
        if effect.state == EffectState::Applied
            && let Some(pre_canister_version) = effect.pre_canister_version
            && state
                .principals
                .get(name)
                .is_some_and(|retained| retained == principal)
        {
            state.completed_reinstall_operation_id = Some(journal.operation_id.clone());
            state
                .completed_reinstall_action_sha256
                .insert(name.clone(), action_sha256);
            state
                .completed_reinstalls
                .insert(name.clone(), pre_canister_version);
            if let Some(topology) = state.topology.get_mut(name) {
                topology.module_hash = Some(wasm_sha256.clone());
            }
        }
    }
}

fn completed_reinstall_evidence_matches(
    state: &FleetEnsureStateRecord,
    journal: &FleetEnsureJournalRecord,
) -> bool {
    !state.completed_reinstalls.is_empty()
        && state
            .completed_reinstalls
            .iter()
            .all(|(name, pre_version)| {
                state
                    .completed_reinstall_action_sha256
                    .get(name)
                    .and_then(|action_sha256| {
                        journal
                            .effects
                            .iter()
                            .find(|effect| effect.action_sha256 == *action_sha256)
                    })
                    .is_some_and(|effect| {
                        effect.state == EffectState::Applied
                            && effect.pre_canister_version == Some(*pre_version)
                    })
            })
        && state.completed_reinstall_action_sha256.len() == state.completed_reinstalls.len()
}

fn retained_reinstall_operation_matches(
    state: &FleetEnsureStateRecord,
    prior_fleet: &str,
    prior_operation_id: &str,
    journal: &FleetEnsureJournalRecord,
) -> bool {
    journal.completion == FleetEnsureCompletion::ReplanRequired
        && journal.fleet == prior_fleet
        && journal.operation_id == prior_operation_id
        && state.completed_reinstall_operation_id.as_deref() == Some(journal.operation_id.as_str())
        && completed_reinstall_evidence_matches(state, journal)
}

fn retained_reinstall_desired_conflict(
    state: &FleetEnsureStateRecord,
    prior_fleet: &str,
    prior_operation_id: &str,
    prior_desired_sha256: &str,
    supplied_desired_sha256: &str,
    journal: &FleetEnsureJournalRecord,
) -> bool {
    prior_desired_sha256 != supplied_desired_sha256
        && retained_reinstall_operation_matches(state, prior_fleet, prior_operation_id, journal)
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
    if retained_plan.scope == FleetEnsurePlanScope::RootStartPrerequisite
        && let Some(journal) = retained_journal
            .as_ref()
            .filter(|journal| journal.completion == FleetEnsureCompletion::Converged)
    {
        verify_terminal_root_start_replay(
            root,
            operation_desired,
            &retained_plan,
            journal,
            requested_fleet,
            platform,
            &state,
        )?;
        return Ok(FleetEnsureReport {
            actual_conservation: None,
            effects_applied: applied_count(journal),
            plan: retained_plan,
            terminal: true,
        });
    }
    let mut journal = match retained_journal {
        Some(journal) if journal.completion == FleetEnsureCompletion::InProgress => {
            verify_journal(&journal, &retained_plan, requested_fleet)?;
            journal
        }
        _ => {
            let (observation, initial_controlled_cycles) = verify_fresh_plan(
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
                initial_controlled_cycles,
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
    let mut replayed_issued_commands = BTreeSet::new();
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
            if !create_record_is_exact(action, record, &state) {
                return Err(EnsureWorkflowError::JournalIntegrity);
            }
            if matches!(record.state, EffectState::Applied) {
                if applied_create_requires_exact_balance_recovery(action, record) {
                    let outcome = platform
                        .apply(&journal.operation_id, action, record, &state)
                        .map_err(EnsureWorkflowError::Platform)?;
                    if !create_outcome_is_exact(action, record, &state, &outcome) {
                        return Err(EnsureWorkflowError::JournalIntegrity);
                    }
                    let record = journal
                        .effects
                        .get_mut(index)
                        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
                    record.post_cycles = outcome.post_cycles;
                    journal.stalled_observations = 0;
                    write_journal(&paths, &journal)?;
                }
                let retained_funding = journal.effects.get(index).is_some_and(|record| {
                    retain_applied_funding_cycles(&mut state, action, record)
                });
                if retained_funding {
                    write_state(&paths, &state)?;
                }
                break;
            }

            let observed = platform
                .observe_effect(&journal.operation_id, action, record, &state)
                .map_err(EnsureWorkflowError::Platform)?;
            let source_cycles = if observed.post_cycles.is_some() {
                observed.post_cycles
            } else {
                platform
                    .action_cycles(action, &state)
                    .map_err(EnsureWorkflowError::Platform)?
            };
            let destination_cycles = platform
                .action_destination_cycles(action, &state)
                .map_err(EnsureWorkflowError::Platform)?;
            let replan_after_create_balance =
                observed.retry == EffectRetry::ReplanRequiredAfterCreateBalanceDrift;
            let replan_after_rejection =
                observed.retry == EffectRetry::ReplanRequiredAfterRejectedPrerequisite;
            let retained_create_balance_is_exact = !replan_after_create_balance
                || (journal.effects.len() == index + 1
                    && journal.effects[..index]
                        .iter()
                        .all(|effect| effect.state == EffectState::Applied)
                    && record.state == EffectState::Issued
                    && create_identity_is_exact(action.name(), record, &state)
                    && observed.post_cycles.is_some()
                    && observed.post_cycles == source_cycles);
            let retained_rejection_is_exact = !replan_after_rejection
                || (journal.effects.len() == index + 1
                    && journal.effects[..index]
                        .iter()
                        .all(|effect| effect.state == EffectState::Applied)
                    && record.state == EffectState::Intent
                    && record.created_principal.is_none()
                    && record.receipt.is_none());
            let record = journal
                .effects
                .get_mut(index)
                .ok_or(EnsureWorkflowError::JournalIntegrity)?;
            merge_observed_cycles(record, source_cycles, destination_cycles);
            if replan_after_create_balance {
                if !retained_create_balance_is_exact {
                    return Err(EnsureWorkflowError::JournalIntegrity);
                }
                let EnsureAction::Create {
                    name,
                    requested_initial_cycles,
                    ..
                } = action
                else {
                    return Err(EnsureWorkflowError::JournalIntegrity);
                };
                let actual_cycles = record
                    .post_cycles
                    .ok_or(EnsureWorkflowError::JournalIntegrity)?;
                record.progress_identity = Some(observed.progress_identity);
                record.state = EffectState::Applied;
                retain_created_canister_for_replan(operation_desired, name, record, &mut state)?;
                journal.completion = FleetEnsureCompletion::ReplanRequired;
                journal.stalled_observations = 0;
                write_journal(&paths, &journal)?;
                write_state(&paths, &state)?;
                let configured_fee_cycles = operation_desired
                    .management_creation_fee_cycles
                    .parse::<Cycles>()
                    .map(|cycles| cycles.to_u128())
                    .map_err(|_| EnsureWorkflowError::JournalIntegrity)?;
                return Err(EnsureWorkflowError::ReplanRequiredAfterCreateBalanceDrift {
                    actual_cycles,
                    canister: name.clone(),
                    configured_fee_cycles,
                    deficit_cycles: requested_initial_cycles.saturating_sub(actual_cycles),
                    requested_cycles: *requested_initial_cycles,
                });
            }
            if observed.applied {
                record.progress_identity = Some(observed.progress_identity);
                record.state = EffectState::Applied;
                journal.stalled_observations = 0;
                let retained_funding = retain_applied_funding_cycles(&mut state, action, record);
                write_journal(&paths, &journal)?;
                if retained_funding {
                    write_state(&paths, &state)?;
                }
                break;
            }

            if replan_after_rejection {
                if !retained_rejection_is_exact {
                    return Err(EnsureWorkflowError::JournalIntegrity);
                }
                record.progress_identity = Some(observed.progress_identity.clone());
                retain_completed_reinstalls(&mut state, &retained_plan, &journal);
                write_state(&paths, &state)?;
                journal.completion = FleetEnsureCompletion::ReplanRequired;
                journal.stalled_observations = 0;
                write_journal(&paths, &journal)?;
                return Err(
                    EnsureWorkflowError::ReplanRequiredAfterRejectedPrerequisite {
                        evidence: observed.progress_identity,
                    },
                );
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
                if !create_outcome_is_exact(action, record, &state, &outcome) {
                    return Err(EnsureWorkflowError::JournalIntegrity);
                }
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

            if observed.retry == EffectRetry::ReplayExactIssuedCommand
                && !replayed_issued_commands.contains(&index)
            {
                if !matches!(
                    action,
                    EnsureAction::FleetProtocol { action, .. }
                        if matches!(
                            action.as_ref(),
                            crate::fleet_ensure::model::CurrentFleetProtocolAction::ProvisionComponents { .. }
                        )
                ) {
                    return Err(EnsureWorkflowError::JournalIntegrity);
                }
                let retained_receipt = record.receipt.clone();
                let outcome = platform
                    .apply(&journal.operation_id, action, record, &state)
                    .map_err(EnsureWorkflowError::Platform)?;
                if outcome.created_principal.is_some()
                    || outcome.receipt != retained_receipt
                    || outcome.post_cycles.is_some()
                {
                    return Err(EnsureWorkflowError::JournalIntegrity);
                }
                let record = journal
                    .effects
                    .get_mut(index)
                    .ok_or(EnsureWorkflowError::JournalIntegrity)?;
                record.progress_identity = Some(observed.progress_identity);
                journal.stalled_observations = 0;
                write_journal(&paths, &journal)?;
                replayed_issued_commands.insert(index);
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

    if retained_plan.scope == FleetEnsurePlanScope::RootStartPrerequisite {
        let actual_conservation = complete_root_start_prerequisite(
            root,
            operation_desired,
            &retained_plan,
            platform,
            &state,
            &journal,
        )?;
        journal.completion = FleetEnsureCompletion::Converged;
        journal.stalled_observations = 0;
        write_journal(&paths, &journal)?;
        return Ok(FleetEnsureReport {
            actual_conservation: Some(actual_conservation),
            effects_applied: applied_count(&journal),
            plan: retained_plan,
            terminal: true,
        });
    }

    let mut terminal_state = state.clone();
    publish_terminal_state(
        operation_desired,
        &retained_plan,
        &journal,
        &mut terminal_state,
    )
    .map_err(|TerminalStatePublicationError::JournalIntegrity| {
        EnsureWorkflowError::JournalIntegrity
    })?;
    if completed_infrastructure_reinstall(operation_desired, &retained_plan, &journal) {
        terminal_state.active_registry = None;
    }
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
            retain_observed_cycles(&mut terminal_state, &terminal_observation);
            retain_completed_reinstalls(&mut terminal_state, &retained_plan, &journal);
            write_state(&paths, &terminal_state)?;
            journal.completion = FleetEnsureCompletion::ReplanRequired;
            journal.stalled_observations = 0;
            write_journal(&paths, &journal)?;
            return Err(EnsureWorkflowError::ConvergenceDrift);
        }
    }
    let terminal_inventory_operation_id =
        if retained_plan.protocol_actions.is_empty() && terminal_state.active_registry.is_some() {
            reviewed_terminal_inventory_operation(&retained_plan, &terminal_state)?
                .ok_or(EnsureWorkflowError::PlanIntegrity)?
        } else {
            retained_plan.operation_id.as_str()
        };
    let terminal_inventory = platform
        .terminal_inventory(terminal_inventory_operation_id, &terminal_state)
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
    terminal_state.completed_reinstall_action_sha256.clear();
    terminal_state.completed_reinstall_operation_id = None;
    terminal_state.completed_reinstalls.clear();
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

const fn merge_observed_cycles(
    record: &mut EffectRecord,
    source_cycles: Option<u128>,
    destination_cycles: Option<u128>,
) {
    if let Some(cycles) = source_cycles {
        record.post_cycles = Some(cycles);
    }
    if let Some(cycles) = destination_cycles {
        record.destination_post_cycles = Some(cycles);
    }
}

fn retain_applied_funding_cycles(
    state: &mut FleetEnsureStateRecord,
    action: &EnsureAction,
    record: &EffectRecord,
) -> bool {
    let EnsureAction::Fund { principal, .. } = action else {
        return false;
    };
    if record.state != EffectState::Applied || record.receipt.is_none() {
        return false;
    }
    let Some(cycles) = record.post_cycles else {
        return false;
    };
    let principal = principal.strip_prefix("created:").map_or_else(
        || Some(principal.as_str()),
        |name| state.pending_principals.get(name).map(String::as_str),
    );
    let Some(principal) = principal else {
        return false;
    };
    if state.retained_cycles_by_principal.get(principal) == Some(&cycles) {
        return false;
    }
    state
        .retained_cycles_by_principal
        .insert(principal.to_string(), cycles);
    true
}

fn retain_created_canister_for_replan<E>(
    desired: &crate::fleet_ensure::model::DesiredFleet,
    name: &str,
    record: &EffectRecord,
    state: &mut FleetEnsureStateRecord,
) -> Result<(), EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    let configured = desired
        .canisters
        .iter()
        .find(|canister| {
            canister.name == name
                && canister.presence == crate::fleet_ensure::model::DesiredPresence::Present
        })
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    let principal = record
        .created_principal
        .as_deref()
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    let cycles = record
        .post_cycles
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    if state.pending_principals.get(name).map(String::as_str) != Some(principal) {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    state.pending_principals.remove(name);
    state
        .principals
        .insert(name.to_string(), principal.to_string());
    state
        .retained_cycles_by_principal
        .insert(principal.to_string(), cycles);
    state.topology.insert(
        name.to_string(),
        crate::fleet_ensure::model::FleetEnsureTopologyRecord {
            kind: configured.kind,
            module_hash: None,
            parent: configured.parent.clone(),
            protocol_binding: configured.protocol_binding.clone(),
            role: configured
                .protocol_binding
                .as_ref()
                .map(|binding| binding.role.to_string()),
        },
    );
    Ok(())
}

fn create_record_is_exact(
    action: &EnsureAction,
    record: &EffectRecord,
    state: &FleetEnsureStateRecord,
) -> bool {
    let EnsureAction::Create {
        name,
        requested_initial_cycles,
        ..
    } = action
    else {
        return true;
    };
    match record.state {
        EffectState::Intent => {
            record.created_principal.is_none()
                && record.receipt.is_none()
                && record.post_cycles.is_none()
        }
        EffectState::Issued => {
            create_identity_is_exact(name, record, state)
                && record.post_cycles == Some(*requested_initial_cycles)
        }
        EffectState::Applied => create_identity_is_exact(name, record, state),
    }
}

fn create_identity_is_exact(
    name: &str,
    record: &EffectRecord,
    state: &FleetEnsureStateRecord,
) -> bool {
    let Some(created_principal) = record
        .created_principal
        .as_deref()
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    if record.receipt.as_deref().is_none_or(str::is_empty) {
        return false;
    }
    state_create_principal_matches(name, created_principal, state, true)
}

fn state_create_principal_matches(
    name: &str,
    created_principal: &str,
    state: &FleetEnsureStateRecord,
    require_retained: bool,
) -> bool {
    let pending = state.pending_principals.get(name).map(String::as_str);
    let terminal = state.principals.get(name).map(String::as_str);
    let retained = pending.is_some() || terminal.is_some();
    let pending_matches = pending.is_none_or(|principal| principal == created_principal);
    let terminal_matches = terminal.is_none_or(|principal| principal == created_principal);
    (!require_retained || retained) && pending_matches && terminal_matches
}

const fn applied_create_requires_exact_balance_recovery(
    action: &EnsureAction,
    record: &EffectRecord,
) -> bool {
    matches!(action, EnsureAction::Create { .. }) && record.post_cycles.is_none()
}

fn create_outcome_is_exact(
    action: &EnsureAction,
    record: &EffectRecord,
    state: &FleetEnsureStateRecord,
    outcome: &crate::fleet_ensure::ops::EffectOutcome,
) -> bool {
    let EnsureAction::Create {
        name,
        requested_initial_cycles,
        ..
    } = action
    else {
        return true;
    };
    let Some(created_principal) = outcome
        .created_principal
        .as_deref()
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    let Some(receipt) = outcome.receipt.as_deref().filter(|value| !value.is_empty()) else {
        return false;
    };
    if outcome.post_cycles != Some(*requested_initial_cycles) {
        return false;
    }
    let principal_matches = record
        .created_principal
        .as_deref()
        .is_none_or(|retained| retained == created_principal);
    let receipt_matches = record
        .receipt
        .as_deref()
        .is_none_or(|retained| retained == receipt);
    let state_matches = state_create_principal_matches(name, created_principal, state, false);
    principal_matches && receipt_matches && state_matches
}

fn verify_fresh_plan<P>(
    root: &Path,
    desired: &crate::fleet_ensure::model::DesiredFleet,
    desired_sha256: &str,
    requested_fleet: &str,
    retained_plan: &FleetEnsurePlan,
    platform: &mut P,
    state: &FleetEnsureStateRecord,
) -> Result<(FleetObservation, u128), EnsureWorkflowError<P::Error>>
where
    P: EnsurePlatform,
{
    if retained_plan.scope == FleetEnsurePlanScope::RootStartPrerequisite {
        if let Some(authority) = retained_plan.root_start_authority.as_deref() {
            verify_root_start_release_authority(root, authority)?;
        }
        let targets = reviewed_root_start_targets(retained_plan)?;
        let management = platform
            .observe_root_management(state, &targets)
            .map_err(EnsureWorkflowError::Platform)?
            .ok_or(EnsureWorkflowError::PlanIntegrity)?;
        let current = recompile_root_start_prerequisite_plan(
            RootStartPlanInput {
                authority: retained_plan.root_start_authority.as_deref(),
                created_at_time: retained_plan.planned_at_time,
                desired,
                desired_sha256,
                observation: &management,
                requested_fleet,
            },
            &targets,
        )?;
        if !compatible_root_start_prerequisite(retained_plan, &current, desired) {
            return Err(EnsureWorkflowError::DriftedBeforeApply);
        }
        let observation = root_management_fleet_observation(&management, &targets)?;
        return Ok((observation, current.conservation.observed_controlled_cycles));
    }
    let mut observation = platform
        .observe(&retained_plan.operation_id, state)
        .map_err(EnsureWorkflowError::Platform)?;
    let terminal_inventory_operation_id =
        reviewed_terminal_inventory_operation(retained_plan, state)?;
    if let Some(operation_id) = terminal_inventory_operation_id {
        attach_terminal_inventory_cycles(operation_id, state, platform, &mut observation)?;
    }
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
    let mut current = compile_plan(
        desired,
        &artifacts,
        &protocol_actions,
        desired_sha256,
        requested_fleet,
        &observation,
        retained_plan.planned_at_time,
    )?;
    bind_terminal_inventory_operation(
        &mut current,
        retained_plan.terminal_inventory_operation_id.clone(),
    );
    if current.plan_sha256 != retained_plan.plan_sha256
        && !compatible_after_bounded_observation(retained_plan, &current, desired, &observation)
    {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    Ok((observation, current.conservation.observed_controlled_cycles))
}

fn reviewed_terminal_inventory_operation<'a, E>(
    plan: &'a FleetEnsurePlan,
    state: &FleetEnsureStateRecord,
) -> Result<Option<&'a str>, EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    match (
        state.active_registry.is_some(),
        plan.terminal_inventory_operation_id.as_deref(),
    ) {
        (false, None) => Ok(None),
        (true, Some(operation_id)) if !operation_id.is_empty() => Ok(Some(operation_id)),
        _ => Err(EnsureWorkflowError::PlanIntegrity),
    }
}

fn complete_root_start_prerequisite<P>(
    root: &Path,
    desired: &crate::fleet_ensure::model::DesiredFleet,
    retained_plan: &FleetEnsurePlan,
    platform: &mut P,
    state: &FleetEnsureStateRecord,
    journal: &FleetEnsureJournalRecord,
) -> Result<ActualCycleConservation, EnsureWorkflowError<P::Error>>
where
    P: EnsurePlatform,
{
    if let Some(authority) = retained_plan.root_start_authority.as_deref() {
        verify_root_start_release_authority(root, authority)?;
    }
    let targets = reviewed_root_start_targets(retained_plan)?;
    let management = platform
        .observe_root_management(state, &targets)
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let current = recompile_root_start_prerequisite_plan(
        RootStartPlanInput {
            authority: retained_plan.root_start_authority.as_deref(),
            created_at_time: retained_plan.planned_at_time,
            desired,
            desired_sha256: &retained_plan.desired_sha256,
            observation: &management,
            requested_fleet: &retained_plan.fleet,
        },
        &targets,
    )?;
    if !compatible_root_start_prerequisite(retained_plan, &current, desired) {
        return Err(EnsureWorkflowError::ConvergenceDrift);
    }
    if management.roots.iter().any(|(name, observed)| {
        targets.contains(name)
            && observed.live.status != crate::fleet_ensure::model::CanisterRuntimeStatus::Running
    }) {
        return Err(EnsureWorkflowError::ConvergenceDrift);
    }
    let terminal = root_management_fleet_observation(&management, &targets)?;
    verify_terminal_conservation(retained_plan, journal, &terminal)
}

fn verify_terminal_root_start_replay<P>(
    root: &Path,
    desired: &crate::fleet_ensure::model::DesiredFleet,
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    requested_fleet: &str,
    platform: &mut P,
    state: &FleetEnsureStateRecord,
) -> Result<(), EnsureWorkflowError<P::Error>>
where
    P: EnsurePlatform,
{
    verify_journal(journal, plan, requested_fleet)?;
    if let Some(authority) = plan.root_start_authority.as_deref() {
        verify_root_start_release_authority(root, authority)?;
    }
    let actions = ordered_actions(plan);
    if actions.len() != journal.effects.len()
        || journal
            .effects
            .iter()
            .any(|effect| effect.state != EffectState::Applied)
    {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    for (action, record) in actions.into_iter().zip(&journal.effects) {
        if action_sha256(action) != record.action_sha256
            || !platform
                .observe_effect(&journal.operation_id, action, record, state)
                .map_err(EnsureWorkflowError::Platform)?
                .applied
        {
            return Err(EnsureWorkflowError::ConvergenceDrift);
        }
    }
    let targets = reviewed_root_start_targets(plan)?;
    let management = platform
        .observe_root_management(state, &targets)
        .map_err(EnsureWorkflowError::Platform)?
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let current = recompile_root_start_prerequisite_plan(
        RootStartPlanInput {
            authority: plan.root_start_authority.as_deref(),
            created_at_time: plan.planned_at_time,
            desired,
            desired_sha256: &plan.desired_sha256,
            observation: &management,
            requested_fleet: &plan.fleet,
        },
        &targets,
    )?;
    if normalized_plan(plan) != normalized_plan(&current)
        || management.roots.iter().any(|(name, observed)| {
            targets.contains(name)
                && observed.live.status
                    != crate::fleet_ensure::model::CanisterRuntimeStatus::Running
        })
    {
        return Err(EnsureWorkflowError::ConvergenceDrift);
    }
    Ok(())
}

fn reviewed_root_start_targets<E>(
    plan: &FleetEnsurePlan,
) -> Result<BTreeSet<String>, EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    if plan.scope != FleetEnsurePlanScope::RootStartPrerequisite
        || !plan.protocol_actions.is_empty()
        || plan.canisters.is_empty()
        || plan.conservation.maximum_new_funding_cycles != 0
        || plan.conservation.maximum_operator_debit_cycles != 0
        || plan.conservation.maximum_unavoidable_fee_cycles != 0
        || plan.conservation.scheduled_transfer_cycles != 0
        || plan.conservation.observed_controlled_cycles
            != plan.conservation.retained_in_reused_canisters_cycles
        || plan
            .conservation
            .observed_controlled_cycles
            .checked_sub(plan.conservation.maximum_execution_burn_cycles)
            != Some(plan.conservation.expected_post_operation_cycles)
    {
        return Err(EnsureWorkflowError::PlanIntegrity);
    }
    let mut targets = BTreeSet::new();
    for canister in &plan.canisters {
        if canister.disposition != CanisterDisposition::Reuse
            || canister.principal.is_none()
            || !matches!(
                canister.actions.as_slice(),
                [EnsureAction::Start { name, principal }]
                    if name == &canister.name
                        && Some(principal) == canister.principal.as_ref()
            )
            || !targets.insert(canister.name.clone())
        {
            return Err(EnsureWorkflowError::PlanIntegrity);
        }
    }
    Ok(targets)
}

fn root_management_fleet_observation<E>(
    management: &RootManagementObservation,
    targets: &BTreeSet<String>,
) -> Result<FleetObservation, EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    let canisters = targets
        .iter()
        .map(|name| {
            management
                .roots
                .get(name)
                .map(|observed| (name.clone(), Some(observed.live.clone())))
                .ok_or(EnsureWorkflowError::PlanIntegrity)
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    Ok(FleetObservation {
        additional_controlled_cycles: BTreeMap::new(),
        canisters,
        ledger_fee_cycles: 0,
        operator_cycles: management.operator_cycles,
        protocol_ready: BTreeMap::new(),
    })
}

fn compatible_root_start_prerequisite(
    retained: &FleetEnsurePlan,
    current: &FleetEnsurePlan,
    desired: &crate::fleet_ensure::model::DesiredFleet,
) -> bool {
    let Ok(maximum_movement) = desired
        .maximum_update_burn_cycles
        .parse::<Cycles>()
        .map(|cycles| cycles.to_u128())
    else {
        return false;
    };
    retained.canisters.len() == current.canisters.len()
        && retained
            .canisters
            .iter()
            .zip(&current.canisters)
            .all(|(retained, current)| {
                retained.observed_cycles.abs_diff(current.observed_cycles) <= maximum_movement
            })
        && normalized_plan(retained) == normalized_plan(current)
}

fn attach_terminal_inventory_cycles<P>(
    operation_id: &str,
    state: &FleetEnsureStateRecord,
    platform: &mut P,
    observation: &mut FleetObservation,
) -> Result<(), EnsureWorkflowError<P::Error>>
where
    P: EnsurePlatform,
{
    let inventory = platform
        .terminal_inventory(operation_id, state)
        .map_err(EnsureWorkflowError::Platform)?;
    attach_terminal_cycles(observation, inventory.controlled_cycles_by_principal)
}

pub(super) fn completed_infrastructure_reinstall(
    desired: &crate::fleet_ensure::model::DesiredFleet,
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
) -> bool {
    if desired.protocol.is_none() {
        return false;
    }
    let infrastructure = desired.canisters.iter().filter(|canister| {
        canister.presence == crate::fleet_ensure::model::DesiredPresence::Present
            && matches!(
                canister.kind,
                crate::fleet_ensure::model::DesiredCanisterKind::Coordinator
                    | crate::fleet_ensure::model::DesiredCanisterKind::Root
                    | crate::fleet_ensure::model::DesiredCanisterKind::Store
            )
    });
    let mut count = 0_usize;
    for configured in infrastructure {
        count += 1;
        let Some(action) = plan
            .canisters
            .iter()
            .find(|canister| {
                canister.name == configured.name
                    && canister.disposition == CanisterDisposition::Reinstall
            })
            .and_then(|canister| {
                canister.actions.iter().find(|action| {
                    matches!(
                        action,
                        EnsureAction::Install {
                            mode: crate::fleet_ensure::model::InstallMode::Reinstall,
                            name,
                            ..
                        } if name == &configured.name
                    )
                })
            })
        else {
            return false;
        };
        let action_sha256 = action_sha256(action);
        if !journal.effects.iter().any(|effect| {
            effect.action_sha256 == action_sha256 && effect.state == EffectState::Applied
        }) {
            return false;
        }
    }
    count > 0
}

fn compatible_after_bounded_observation(
    retained: &FleetEnsurePlan,
    current: &FleetEnsurePlan,
    desired: &crate::fleet_ensure::model::DesiredFleet,
    observation: &FleetObservation,
) -> bool {
    let Ok(maximum_observation_movement) = desired
        .maximum_observation_burn_cycles
        .parse::<Cycles>()
        .map(|cycles| cycles.to_u128())
    else {
        return false;
    };
    if retained.canisters.len() != current.canisters.len() {
        return false;
    }
    for (retained_canister, current_canister) in retained.canisters.iter().zip(&current.canisters) {
        let movement = retained_canister
            .observed_cycles
            .abs_diff(current_canister.observed_cycles);
        if movement > maximum_observation_movement {
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
            let Ok(minimum) = configured
                .minimum_cycles
                .parse::<Cycles>()
                .map(|cycles| cycles.to_u128())
            else {
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
        match configured.root_owned_lifecycle {
            Some(crate::fleet_ensure::model::RootOwnedCanisterLifecycle::Workload) => {
                // Both protected observations identify the same running workload, but
                // execution may burn cycles between them. Retain the conservative lower value.
                configured.cycles = configured.cycles.min(cycles);
            }
            Some(crate::fleet_ensure::model::RootOwnedCanisterLifecycle::Idle)
                if configured.cycles == cycles => {}
            Some(crate::fleet_ensure::model::RootOwnedCanisterLifecycle::Idle) => {
                return Err(EnsureWorkflowError::TerminalInventory(
                    "terminal Root-owned Idle canister has conflicting exact cycle observations"
                        .to_string(),
                ));
            }
            _ => {
                return Err(EnsureWorkflowError::TerminalInventory(
                    "terminal cycle observation duplicates a configured canister outside its exact Root-owned Idle or workload lifecycle"
                        .to_string(),
                ));
            }
        }
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
    let actions = ordered_actions(plan);
    let effect_count_matches = journal.effects.len() <= actions.len();
    let action_hashes_match = journal
        .effects
        .iter()
        .zip(actions)
        .all(|(effect, action)| effect.action_sha256 == action_sha256(action));
    if !(effect_count_matches && action_hashes_match) {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalStatePublicationError {
    JournalIntegrity,
}

fn publish_terminal_state(
    desired: &crate::fleet_ensure::model::DesiredFleet,
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &mut FleetEnsureStateRecord,
) -> Result<(), TerminalStatePublicationError> {
    let actions = ordered_actions(plan);
    if actions.len() != journal.effects.len() {
        return Err(TerminalStatePublicationError::JournalIntegrity);
    }
    for (action, record) in actions.into_iter().zip(&journal.effects) {
        if action_sha256(action) != record.action_sha256 || record.state != EffectState::Applied {
            return Err(TerminalStatePublicationError::JournalIntegrity);
        }
        let EnsureAction::Create {
            name,
            requested_initial_cycles,
            ..
        } = action
        else {
            continue;
        };
        let Some(created_principal) = record.created_principal.as_deref() else {
            return Err(TerminalStatePublicationError::JournalIntegrity);
        };
        let Some(post_cycles) = record.post_cycles else {
            return Err(TerminalStatePublicationError::JournalIntegrity);
        };
        if post_cycles != *requested_initial_cycles
            || !create_identity_is_exact(name, record, state)
            || state
                .retained_cycles_by_principal
                .get(created_principal)
                .is_some_and(|retained| *retained != post_cycles)
        {
            return Err(TerminalStatePublicationError::JournalIntegrity);
        }
        state
            .retained_cycles_by_principal
            .insert(created_principal.to_string(), post_cycles);
    }

    let prior_topology = state.topology.clone();
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
                    module_hash: projected_module_hash(plan, &prior_topology, canister),
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
    Ok(())
}

fn projected_module_hash(
    plan: &FleetEnsurePlan,
    prior_topology: &BTreeMap<String, crate::fleet_ensure::model::FleetEnsureTopologyRecord>,
    canister: &crate::fleet_ensure::model::DesiredCanister,
) -> Option<String> {
    let installed = plan
        .canisters
        .iter()
        .find(|planned| planned.name == canister.name)
        .and_then(|planned| {
            planned
                .actions
                .iter()
                .rev()
                .find_map(|action| match action {
                    EnsureAction::Install { wasm_sha256, .. } => Some(wasm_sha256.clone()),
                    _ => None,
                })
        });
    installed.or_else(|| {
        prior_topology
            .get(&canister.name)
            .filter(|retained| retained.kind == canister.kind && retained.parent == canister.parent)
            .and_then(|retained| retained.module_hash.clone())
    })
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
    // Current planning fences protocol work behind infrastructure convergence.
    // Therefore Store installation plus typed adoption can only be the exact
    // retained predecessor prerequisite that must stay ahead of installation.
    let retained_store_prerequisite = actions.iter().any(|action| {
        matches!(
            action,
            EnsureAction::Install {
                canic_init: Some(crate::fleet_ensure::model::DesiredCanisterInit::Store { .. }),
                ..
            }
        )
    }) && actions.iter().any(|action| {
        matches!(
            action,
            EnsureAction::Install {
                canic_init: Some(crate::fleet_ensure::model::DesiredCanisterInit::Root { .. }),
                ..
            }
        )
    });
    actions.sort_by_key(|action| {
        if retained_store_prerequisite
            && matches!(
                action,
                EnsureAction::FleetProtocol { action, .. }
                    if matches!(
                        action.as_ref(),
                        crate::fleet_ensure::model::CurrentFleetProtocolAction::AdoptStore { .. }
                    )
            )
        {
            3
        } else {
            action_order(action)
        }
    });
    actions
}

pub(super) const fn action_order(action: &EnsureAction) -> u8 {
    match action {
        EnsureAction::Create { .. } => 0,
        EnsureAction::Fund { .. } => 1,
        EnsureAction::Install {
            canic_init: Some(crate::fleet_ensure::model::DesiredCanisterInit::Coordinator),
            ..
        } => 2,
        EnsureAction::SetControllers { .. } => 3,
        EnsureAction::Install {
            canic_init: Some(crate::fleet_ensure::model::DesiredCanisterInit::Store { .. }),
            ..
        } => 4,
        EnsureAction::Install { .. } => 5,
        EnsureAction::Start { .. } => 6,
        EnsureAction::FleetProtocol { .. } | EnsureAction::Protocol { .. } => 7,
        EnsureAction::Transfer { .. } => 8,
        EnsureAction::Stop { .. } => 9,
        EnsureAction::Delete { .. } => 10,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn retained_evidence() -> (FleetEnsureStateRecord, FleetEnsureJournalRecord) {
        let state = FleetEnsureStateRecord {
            active_registry: None,
            completed_reinstall_action_sha256: BTreeMap::from([(
                "root".to_string(),
                "action".to_string(),
            )]),
            completed_reinstall_operation_id: Some("operation".to_string()),
            completed_reinstalls: BTreeMap::from([("root".to_string(), 7)]),
            fleet: "fleet".to_string(),
            pending_principals: BTreeMap::new(),
            principals: BTreeMap::new(),
            retained_cycles_by_principal: BTreeMap::new(),
            schema_version: crate::fleet_ensure::model::FLEET_ENSURE_SCHEMA_VERSION,
            topology: BTreeMap::new(),
        };
        let journal = FleetEnsureJournalRecord {
            completion: FleetEnsureCompletion::ReplanRequired,
            effects: vec![crate::fleet_ensure::model::EffectRecord {
                action_sha256: "action".to_string(),
                created_principal: None,
                destination_post_cycles: None,
                destination_pre_cycles: None,
                post_cycles: None,
                pre_cycles: None,
                pre_canister_version: Some(7),
                progress_identity: None,
                receipt: None,
                state: EffectState::Applied,
            }],
            fleet: "fleet".to_string(),
            initial_controlled_cycles: 0,
            initial_operator_cycles: 0,
            operation_id: "operation".to_string(),
            plan_sha256: "plan".to_string(),
            schema_version: crate::fleet_ensure::model::FLEET_ENSURE_SCHEMA_VERSION,
            stalled_observations: 0,
        };
        (state, journal)
    }

    #[test]
    fn completed_reinstall_evidence_requires_exact_operation_action_and_version() {
        let (state, journal) = retained_evidence();
        assert!(retained_reinstall_operation_matches(
            &state,
            "fleet",
            "operation",
            &journal,
        ));

        let mut wrong_action = journal.clone();
        wrong_action.effects[0].action_sha256 = "other".to_string();
        assert!(!retained_reinstall_operation_matches(
            &state,
            "fleet",
            "operation",
            &wrong_action,
        ));

        let mut wrong_version = journal.clone();
        wrong_version.effects[0].pre_canister_version = Some(8);
        assert!(!retained_reinstall_operation_matches(
            &state,
            "fleet",
            "operation",
            &wrong_version,
        ));

        for (fleet, operation) in [("other", "operation"), ("fleet", "other")] {
            assert!(!retained_reinstall_operation_matches(
                &state, fleet, operation, &journal,
            ));
        }

        let mut incomplete = journal;
        incomplete.effects[0].state = EffectState::Issued;
        assert!(!retained_reinstall_operation_matches(
            &state,
            "fleet",
            "operation",
            &incomplete,
        ));
    }

    #[test]
    fn toko_fresh_fleet_alternate_desired_cannot_clear_completed_reinstall_evidence() {
        let (state, journal) = retained_evidence();
        assert!(retained_reinstall_desired_conflict(
            &state,
            "fleet",
            "operation",
            "reviewed-desired",
            "alternate-desired",
            &journal,
        ));
        assert!(!retained_reinstall_desired_conflict(
            &state,
            "fleet",
            "operation",
            "reviewed-desired",
            "reviewed-desired",
            &journal,
        ));

        let mut terminal = journal;
        terminal.completion = FleetEnsureCompletion::Converged;
        assert!(!retained_reinstall_desired_conflict(
            &state,
            "fleet",
            "operation",
            "reviewed-desired",
            "alternate-desired",
            &terminal,
        ));
    }

    fn cycle_observation(
        lifecycle: crate::fleet_ensure::model::RootOwnedCanisterLifecycle,
        cycles: u128,
    ) -> FleetObservation {
        FleetObservation {
            additional_controlled_cycles: BTreeMap::new(),
            canisters: BTreeMap::from([(
                "asset".to_string(),
                Some(crate::fleet_ensure::model::LiveCanister {
                    canister_version: Some(1),
                    controllers: Vec::new(),
                    cycles,
                    module_sha256: None,
                    principal: "controlled-principal".to_string(),
                    reinstall_required: false,
                    root_owned_lifecycle: Some(lifecycle),
                    status: crate::fleet_ensure::model::CanisterRuntimeStatus::Stopped,
                }),
            )]),
            ledger_fee_cycles: 0,
            operator_cycles: 0,
            protocol_ready: BTreeMap::new(),
        }
    }

    #[test]
    fn toko_fresh_fleet_idle_cycle_duplicate_requires_exact_balance() {
        use crate::fleet_ensure::model::RootOwnedCanisterLifecycle;

        let additional = BTreeMap::from([("controlled-principal".to_string(), 100)]);
        let mut idle = cycle_observation(RootOwnedCanisterLifecycle::Idle, 100);
        attach_terminal_cycles::<std::io::Error>(&mut idle, additional.clone())
            .expect("merge exact Idle observation");
        assert!(idle.additional_controlled_cycles.is_empty());

        let mut conflicting_idle = cycle_observation(RootOwnedCanisterLifecycle::Idle, 99);
        assert!(matches!(
            attach_terminal_cycles::<std::io::Error>(&mut conflicting_idle, additional.clone()),
            Err(EnsureWorkflowError::TerminalInventory(reason))
                if reason.contains("Idle canister has conflicting exact cycle observations")
        ));

        let mut workload = cycle_observation(RootOwnedCanisterLifecycle::Workload, 99);
        attach_terminal_cycles::<std::io::Error>(&mut workload, additional)
            .expect("merge conservative workload observation");
        assert_eq!(
            workload
                .canisters
                .get("asset")
                .and_then(Option::as_ref)
                .map(|live| live.cycles),
            Some(99),
        );
    }
}
