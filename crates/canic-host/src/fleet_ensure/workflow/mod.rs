//! Module: fleet_ensure::workflow
//!
//! Responsibility: plan and advance one durable idempotent Fleet convergence operation.
//! Does not own: policy decisions, IC transport, or storage mechanics.
//! Boundary: persists exact intent before each ops effect and reconciles it before any retry.

use crate::fleet_ensure::{
    inventory::project_current_fleet_inventory,
    model::{
        ActualCycleConservation, CanisterDisposition, CycleConservation, EffectRecord, EffectState,
        EnsureAction, EstateFundingDomainPlan, EstateFundingRequiredRecord,
        FLEET_ENSURE_SCHEMA_VERSION, FleetEnsureCompletion, FleetEnsureJournalRecord,
        FleetEnsurePlan, FleetEnsurePlanScope, FleetEnsureReport, FleetEnsureStateRecord,
        FleetObservation, RootManagementObservation, create_balance_is_terminal,
    },
    ops::{
        EffectRetry, EnsurePaths, EnsurePlatform, EnsureStateError, action_sha256,
        compact_inline_plan, lock_operation, read_journal, read_plan, read_root_start_authority,
        read_state, resolve_desired_artifacts, verify_root_start_release_authority, write_journal,
        write_plan, write_state,
    },
    policy::{
        EnsurePolicyError, RootStartPlanInput, compile_plan, compile_root_start_prerequisite_plan,
        effect_observation_policy, expected_plan_sha256, operation_id,
        recompile_root_start_prerequisite_plan, validate_path_identity, validate_path_labels,
    },
};
use canic_core::cdk::types::Cycles;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use thiserror::Error as ThisError;

/// Exact no-effect evidence that a fresh pool cannot retain its readiness floor.

#[derive(Debug, ThisError)]
#[error(
    "fresh pool canister {canister} requested {requested_creation_funding_cycles} creation cycles for readiness floor {readiness_floor_cycles}; its first live balance is {live_balance_cycles} cycles (readiness shortfall {readiness_shortfall_cycles}) and must retain {remaining_controller_burn_cycles} cycles for controller finalization (required pre-finalization balance {required_pre_finalization_balance_cycles}, pre-finalization shortfall {pre_finalization_shortfall_cycles}; maximum first-observation burn {maximum_observation_burn_cycles}); no controller or protocol action followed"
)]
pub struct FreshPoolCreationUnderfundedError {
    pub canister: String,
    pub live_balance_cycles: u128,
    pub maximum_observation_burn_cycles: u128,
    pub pre_finalization_shortfall_cycles: u128,
    pub readiness_floor_cycles: u128,
    pub readiness_shortfall_cycles: u128,
    pub remaining_controller_burn_cycles: u128,
    pub requested_creation_funding_cycles: u128,
    pub required_pre_finalization_balance_cycles: u128,
}

/// Exact reviewed Root-account funding prerequisite that blocks protocol mutation.
#[derive(Debug, ThisError)]
#[error(
    "Root {root} Cycles Ledger account {root_principal} on {cycles_ledger} has {available_cycles} cycles; {required_creation_count} autonomous creations require at most {maximum_creation_debit_cycles} cycles ({creation_amount_cycles} gross amount containing {readiness_floor_cycles} readiness floor, {management_creation_fee_cycles} management creation fee and {creation_execution_margin_cycles} execution margin, plus {ledger_fee_cycles} Ledger fee each; total reviewed fees {maximum_creation_fee_cycles}), shortfall {shortfall_cycles}; pending_creation={pending_creation_operation_id:?} attempts={attempt_count:?} last_attempt_at_ns={last_attempt_at_ns:?} retry_at_ns={retry_at_ns:?}; preserve this operation and obtain a newly reviewed Fleet ensure plan for the exact shortfall; do not fund the account out of band"
)]
pub struct EstateFundingRequiredError {
    pub attempt_count: Option<u32>,
    pub available_cycles: u128,
    pub creation_amount_cycles: u128,
    pub cycles_ledger: String,
    pub creation_execution_margin_cycles: u128,
    pub readiness_floor_cycles: u128,
    pub ledger_fee_cycles: u128,
    pub management_creation_fee_cycles: u128,
    pub maximum_creation_debit_cycles: u128,
    pub maximum_creation_fee_cycles: u128,
    pub pending_creation_operation_id: Option<String>,
    pub required_creation_count: u32,
    pub root: String,
    pub root_principal: String,
    pub last_attempt_at_ns: Option<u64>,
    pub retry_at_ns: Option<u64>,
    pub shortfall_cycles: u128,
}

/// Exact terminal-inventory merge or cycle-observation invariant failure.
#[derive(Debug, ThisError)]
pub enum TerminalInventoryError {
    #[error("one Principal is retained under more than one desired role")]
    DuplicateRetainedPrincipal,

    #[error("verified canister {canister} lost its Principal binding")]
    LostPrincipalBinding { canister: String },

    #[error("canister {canister} names unknown parent {parent}")]
    UnknownParent { canister: String, parent: String },

    #[error(
        "canister {canister} retained parent {retained_parent} differs from terminal parent {terminal_parent}"
    )]
    RetainedParentConflict {
        canister: String,
        retained_parent: String,
        terminal_parent: String,
    },

    #[error(
        "terminal Root-owned Idle canister {canister} cycle observations differ: expected {expected}, observed {observed}"
    )]
    IdleCycleObservationConflict {
        canister: String,
        expected: u128,
        observed: u128,
    },

    #[error(
        "terminal cycle observation duplicates canister {canister} outside its exact Root-owned Idle or Workload lifecycle"
    )]
    CycleObservationLifecycleConflict { canister: String },
}

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
        "Fleet ensure action {action} made no progress for {observations} consecutive observations (last progress: {progress_identity}); operation remains resumable"
    )]
    Stalled {
        action: String,
        observations: u32,
        progress_identity: String,
    },

    #[error(
        "Root-owned canister {target} remained {last_lifecycle} for {observations} consecutive passive observations; no command was reissued and the operation remains resumable"
    )]
    RootOwnedObservationStalled {
        target: String,
        observations: u32,
        last_lifecycle: String,
    },

    #[error(
        "selected Cycles Ledger account has {actual} cycles, below reviewed maximum debit {required}"
    )]
    InsufficientOperatorCycles { actual: u128, required: u128 },

    #[error("terminal Fleet cycle conservation failed: {0}")]
    Conservation(String),

    #[error("terminal Fleet inventory failed exact current-authority validation: {0}")]
    TerminalInventory(#[source] TerminalInventoryError),

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
    FreshPoolCreationUnderfunded(Box<FreshPoolCreationUnderfundedError>),

    #[error(transparent)]
    EstateFundingRequired(Box<EstateFundingRequiredError>),

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
    let state = read_state(&paths, requested_fleet)?;
    verify_journal(&journal, &retained, requested_fleet, &state)?;
    Ok(Some(retained))
}

/// Build and retain one read-only plan from current desired state plus live observation.
#[expect(
    clippy::too_many_lines,
    reason = "one locked read-only transaction reconciles retained authority, observation and the immutable plan"
)]
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
        let state = read_state(&paths, requested_fleet)?;
        verify_journal(&journal, &retained, requested_fleet, &state)?;
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
    // The prior Converged journal can remain visible until apply. Planning may refresh only
    // backup-inert observation and prior-operation evidence, never successor topology authority.
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
    let mut protocol_actions = platform
        .protocol_actions(&operation_id, &state)
        .map_err(EnsureWorkflowError::Platform)?;
    let mut retained_observations = 0_u32;
    let mut plan = loop {
        match compile_plan(
            desired,
            &artifacts,
            &protocol_actions,
            desired_sha256,
            requested_fleet,
            &observation,
            created_at_time,
        ) {
            Ok(plan) => break plan,
            Err(EnsurePolicyError::PendingRootOwnedBalance { name }) => {
                retained_observations = retained_observations.saturating_add(1);
                if retained_observations >= desired.maximum_stalled_observations {
                    return Err(EnsureWorkflowError::RootOwnedObservationStalled {
                        last_lifecycle: root_owned_lifecycle_label(&observation, &name).to_string(),
                        observations: retained_observations,
                        target: name,
                    });
                }
                platform.pace_root_owned_observation(&name, retained_observations);
                observation = platform
                    .observe(&operation_id, &state)
                    .map_err(EnsureWorkflowError::Platform)?;
                retain_observed_cycles(&mut state, &observation);
                write_state(&paths, &state)?;
                if let Some(operation_id) = terminal_inventory_operation_id.as_deref() {
                    attach_terminal_inventory_cycles(
                        operation_id,
                        &state,
                        platform,
                        &mut observation,
                    )?;
                }
                protocol_actions = platform
                    .protocol_actions(&operation_id, &state)
                    .map_err(EnsureWorkflowError::Platform)?;
            }
            Err(error) => return Err(error.into()),
        }
    };
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
    verify_journal(prior_journal, prior_plan, requested_fleet, state)?;
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

fn root_owned_lifecycle_label(observation: &FleetObservation, target: &str) -> &'static str {
    observation
        .canisters
        .get(target)
        .and_then(Option::as_ref)
        .and_then(|live| live.root_owned_lifecycle)
        .map_or("unavailable", |lifecycle| lifecycle.label())
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
        verify_journal(journal, &retained_plan, requested_fleet, &state)?;
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
            verify_journal(&journal, &retained_plan, requested_fleet, &state)?;
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
                estate_funding_required: None,
                effects: Vec::new(),
                fleet: requested_fleet.to_string(),
                initial_controlled_cycles,
                initial_estate_funding_cycles_by_root: retained_plan
                    .conservation
                    .estate_funding_domains
                    .iter()
                    .map(|domain| {
                        (
                            domain.root.clone(),
                            domain.available_cycles.unwrap_or_default(),
                        )
                    })
                    .collect(),
                initial_operator_cycles: observation.operator_cycles,
                operation_id: retained_plan.operation_id.clone(),
                plan_sha256: retained_plan.plan_sha256.clone(),
                schema_version: FLEET_ENSURE_SCHEMA_VERSION,
                stalled_observations: 0,
            };
            // Cross the nonterminal boundary before any effect-owned state can be retained.
            write_journal(&paths, &journal)?;
            journal
        }
    };

    let actions = ordered_actions(&retained_plan);
    let mut replayed_issued_commands = BTreeSet::new();
    loop {
        let mut deferred_controller_observation = false;
        for (index, action) in actions.iter().enumerate() {
            let action_hash = action_sha256(action);
            let observation_policy = effect_observation_policy(operation_desired, action)?;
            let retained_effect = journal.effects.get(index);
            if matches!(action, EnsureAction::FleetProtocol { .. })
                && retained_effect.is_none_or(|effect| effect.state == EffectState::Intent)
                && !prior_fleet_protocol_effect_started(&actions, &journal, index)
            {
                let funding_observation = platform
                    .observe(&journal.operation_id, &state)
                    .map_err(EnsureWorkflowError::Platform)?;
                let required =
                    estate_funding_requirement(&retained_plan, &state, &funding_observation)?;
                retain_estate_funding_pause(&paths, &mut journal, required.as_ref())?;
                if let Some(required) = required {
                    return Err(EnsureWorkflowError::EstateFundingRequired(Box::new(
                        estate_funding_error(&required),
                    )));
                }
            }
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
                    let retained_create = journal
                        .effects
                        .get(index)
                        .ok_or(EnsureWorkflowError::JournalIntegrity)
                        .and_then(|record| {
                            retain_applied_create_authority(
                                operation_desired,
                                action,
                                record,
                                &mut state,
                            )
                        })?;
                    let retained_funding = journal.effects.get(index).is_some_and(|record| {
                        retain_applied_funding_cycles(&mut state, action, record)
                    });
                    if retained_create || retained_funding {
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
                let protocol_funding_required = if record.state == EffectState::Issued {
                    observed
                        .estate_funding_required
                        .as_ref()
                        .map(|funding| {
                            protocol_estate_funding_requirement(&retained_plan, &state, funding)
                        })
                        .transpose()?
                } else {
                    None
                };
                if let Some(required) = protocol_funding_required.as_ref() {
                    retain_estate_funding_pause(&paths, &mut journal, Some(required))?;
                    return Err(EnsureWorkflowError::EstateFundingRequired(Box::new(
                        estate_funding_error(required),
                    )));
                }
                if journal
                    .estate_funding_required
                    .as_ref()
                    .is_some_and(|required| required.pending_creation_operation_id.is_some())
                {
                    retain_estate_funding_pause(&paths, &mut journal, None)?;
                }
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
                    retain_created_canister_for_replan(
                        operation_desired,
                        name,
                        record,
                        &mut state,
                    )?;
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
                    write_journal(&paths, &journal)?;
                    let record = journal
                        .effects
                        .get(index)
                        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
                    let retained_create = retain_applied_create_authority(
                        operation_desired,
                        action,
                        record,
                        &mut state,
                    )?;
                    let retained_funding =
                        retain_applied_funding_cycles(&mut state, action, record);
                    if retained_create || retained_funding {
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

                if observed.retry == EffectRetry::DeferUntilControllerObservation {
                    if !deferred_create_observation_is_exact(
                        operation_desired,
                        &actions,
                        index,
                        action,
                        record,
                        &state,
                    ) {
                        return Err(EnsureWorkflowError::JournalIntegrity);
                    }
                    let progress_identity = observed.progress_identity.clone();
                    if record.progress_identity.as_deref() == Some(&observed.progress_identity) {
                        journal.stalled_observations =
                            journal.stalled_observations.saturating_add(1);
                    } else {
                        record.progress_identity = Some(observed.progress_identity);
                        journal.stalled_observations = 0;
                    }
                    write_journal(&paths, &journal)?;
                    if journal.stalled_observations
                        >= observation_policy.maximum_stalled_observations
                    {
                        return Err(EnsureWorkflowError::Stalled {
                            action: action.name().to_string(),
                            observations: journal.stalled_observations,
                            progress_identity,
                        });
                    }
                    deferred_controller_observation = true;
                    break;
                }

                if matches!(record.state, EffectState::Intent) {
                    let outcome =
                        match platform.apply(&journal.operation_id, action, record, &state) {
                            Ok(outcome) => outcome,
                            Err(source) => {
                                let progress_identity = record
                                    .progress_identity
                                    .clone()
                                    .unwrap_or_else(|| "effect-call-failed".to_string());
                                journal.stalled_observations =
                                    journal.stalled_observations.saturating_add(1);
                                write_journal(&paths, &journal)?;
                                if journal.stalled_observations
                                    >= observation_policy.maximum_stalled_observations
                                {
                                    return Err(EnsureWorkflowError::Stalled {
                                        action: action.name().to_string(),
                                        observations: journal.stalled_observations,
                                        progress_identity,
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
                let progress_identity = record
                    .progress_identity
                    .clone()
                    .ok_or(EnsureWorkflowError::JournalIntegrity)?;
                write_journal(&paths, &journal)?;
                if journal.stalled_observations >= observation_policy.maximum_stalled_observations {
                    return Err(EnsureWorkflowError::Stalled {
                        action: action.name().to_string(),
                        observations: journal.stalled_observations,
                        progress_identity,
                    });
                }
                if observation_policy.paced {
                    platform.pace_effect_observation(action, journal.stalled_observations);
                }
            }
        }
        if journal.effects.len() == actions.len()
            && journal
                .effects
                .iter()
                .all(|record| record.state == EffectState::Applied)
        {
            break;
        }
        if !deferred_controller_observation {
            return Err(EnsureWorkflowError::JournalIntegrity);
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
    let mut terminal_observation = platform
        .observe(&retained_plan.operation_id, &terminal_state)
        .map_err(EnsureWorkflowError::Platform)?;
    if !issued_observation_resume {
        let artifacts = resolve_desired_artifacts(root, operation_desired)?;
        let converged = loop {
            let protocol_actions = platform
                .protocol_actions(&retained_plan.operation_id, &terminal_state)
                .map_err(EnsureWorkflowError::Platform)?;
            match compile_plan(
                operation_desired,
                &artifacts,
                &protocol_actions,
                operation_desired_sha256,
                requested_fleet,
                &terminal_observation,
                retained_plan.planned_at_time,
            ) {
                Ok(plan) => {
                    if journal.stalled_observations != 0 {
                        journal.stalled_observations = 0;
                        write_journal(&paths, &journal)?;
                    }
                    break plan;
                }
                Err(EnsurePolicyError::PendingRootOwnedBalance { name }) => {
                    journal.stalled_observations = journal.stalled_observations.saturating_add(1);
                    retain_observed_cycles(&mut state, &terminal_observation);
                    write_state(&paths, &state)?;
                    write_journal(&paths, &journal)?;
                    if journal.stalled_observations
                        >= operation_desired.maximum_stalled_observations
                    {
                        return Err(EnsureWorkflowError::RootOwnedObservationStalled {
                            last_lifecycle: root_owned_lifecycle_label(
                                &terminal_observation,
                                &name,
                            )
                            .to_string(),
                            observations: journal.stalled_observations,
                            target: name,
                        });
                    }
                    platform.pace_root_owned_observation(&name, journal.stalled_observations);
                    terminal_observation = platform
                        .observe(&retained_plan.operation_id, &terminal_state)
                        .map_err(EnsureWorkflowError::Platform)?;
                }
                Err(error) => return Err(error.into()),
            }
        };
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
    let actual_conservation = verify_terminal_conservation(
        &retained_plan,
        &journal,
        &terminal_state,
        &final_observation,
    )?;
    terminal_state.completed_reinstall_action_sha256.clear();
    terminal_state.completed_reinstall_operation_id = None;
    terminal_state.completed_reinstalls.clear();
    // Publish the fully validated topology before making its terminal journal visible to backup.
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

pub(super) fn retain_applied_create_authority<E>(
    desired: &crate::fleet_ensure::model::DesiredFleet,
    action: &EnsureAction,
    record: &EffectRecord,
    state: &mut FleetEnsureStateRecord,
) -> Result<bool, EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    let EnsureAction::Create {
        name,
        requested_initial_cycles,
        ..
    } = action
    else {
        return Ok(false);
    };
    if record.state != EffectState::Applied {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    let configured = desired
        .canisters
        .iter()
        .find(|canister| {
            canister.name == *name
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
    let maximum_observation_burn_cycles = desired
        .maximum_observation_burn_cycles
        .parse::<Cycles>()
        .map(|cycles| cycles.to_u128())
        .map_err(|_| EnsureWorkflowError::JournalIntegrity)?;
    if !create_identity_is_exact(name, record, state)
        || !create_balance_is_terminal(
            Some(cycles),
            *requested_initial_cycles,
            maximum_observation_burn_cycles,
        )
        || state
            .retained_cycles_by_principal
            .get(principal)
            .is_some_and(|retained| *retained != cycles)
    {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    validate_fresh_pool_creation_balance(
        desired,
        configured,
        name,
        cycles,
        maximum_observation_burn_cycles,
        *requested_initial_cycles,
    )?;

    let prior_topology = state.topology.get(name);
    let topology_identity_matches = prior_topology.is_some_and(|topology| {
        topology.kind == configured.kind
            && topology.parent == configured.parent
            && topology.protocol_binding == configured.protocol_binding
            && topology.role
                == configured
                    .protocol_binding
                    .as_ref()
                    .map(|binding| binding.role.to_string())
    });
    let topology = crate::fleet_ensure::model::FleetEnsureTopologyRecord {
        kind: configured.kind,
        module_hash: if topology_identity_matches {
            prior_topology.and_then(|topology| topology.module_hash.clone())
        } else {
            None
        },
        parent: configured.parent.clone(),
        protocol_binding: configured.protocol_binding.clone(),
        role: configured
            .protocol_binding
            .as_ref()
            .map(|binding| binding.role.to_string()),
    };
    let changed = state.retained_cycles_by_principal.get(principal) != Some(&cycles)
        || state.topology.get(name) != Some(&topology);
    state
        .retained_cycles_by_principal
        .insert(principal.to_string(), cycles);
    state.topology.insert(name.clone(), topology);
    Ok(changed)
}

fn validate_fresh_pool_creation_balance<E>(
    desired: &crate::fleet_ensure::model::DesiredFleet,
    configured: &crate::fleet_ensure::model::DesiredCanister,
    name: &str,
    cycles: u128,
    maximum_observation_burn_cycles: u128,
    requested_initial_cycles: u128,
) -> Result<(), EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    if configured.kind == crate::fleet_ensure::model::DesiredCanisterKind::Pool
        && desired
            .bootstrap
            .as_ref()
            .is_some_and(|bootstrap| bootstrap.fresh_estate)
        && configured.principal.is_none()
        && configured.controllers.is_empty()
        && configured.parent.as_ref().is_some_and(|parent| {
            configured.controller_canisters.as_slice() == std::slice::from_ref(parent)
        })
    {
        let readiness_floor_cycles = configured
            .minimum_cycles
            .parse::<Cycles>()
            .map(|cycles| cycles.to_u128())
            .map_err(|_| EnsureWorkflowError::JournalIntegrity)?;
        let remaining_controller_burn_cycles = desired
            .maximum_update_burn_cycles
            .parse::<Cycles>()
            .map(|cycles| cycles.to_u128())
            .map_err(|_| EnsureWorkflowError::JournalIntegrity)?;
        let required_pre_finalization_balance_cycles = readiness_floor_cycles
            .checked_add(remaining_controller_burn_cycles)
            .ok_or(EnsureWorkflowError::JournalIntegrity)?;
        if cycles < required_pre_finalization_balance_cycles {
            return Err(EnsureWorkflowError::FreshPoolCreationUnderfunded(Box::new(
                FreshPoolCreationUnderfundedError {
                    canister: name.to_string(),
                    live_balance_cycles: cycles,
                    maximum_observation_burn_cycles,
                    pre_finalization_shortfall_cycles: required_pre_finalization_balance_cycles
                        - cycles,
                    readiness_floor_cycles,
                    readiness_shortfall_cycles: readiness_floor_cycles.saturating_sub(cycles),
                    remaining_controller_burn_cycles,
                    requested_creation_funding_cycles: requested_initial_cycles,
                    required_pre_finalization_balance_cycles,
                },
            )));
        }
    }
    Ok(())
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
    if matches!(action, EnsureAction::FundEstate { .. }) {
        return outcome.created_principal.is_none()
            && outcome.post_cycles.is_none()
            && outcome
                .receipt
                .as_deref()
                .is_some_and(|receipt| !receipt.is_empty());
    }
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
    let artifacts = resolve_desired_artifacts(root, desired)?;
    let mut retained_observations = 0_u32;
    let mut current = loop {
        let protocol_actions = platform
            .protocol_actions(&retained_plan.operation_id, state)
            .map_err(EnsureWorkflowError::Platform)?;
        match compile_plan(
            desired,
            &artifacts,
            &protocol_actions,
            desired_sha256,
            requested_fleet,
            &observation,
            retained_plan.planned_at_time,
        ) {
            Ok(plan) => break plan,
            Err(EnsurePolicyError::PendingRootOwnedBalance { name }) => {
                retained_observations = retained_observations.saturating_add(1);
                if retained_observations >= desired.maximum_stalled_observations {
                    return Err(EnsureWorkflowError::RootOwnedObservationStalled {
                        last_lifecycle: root_owned_lifecycle_label(&observation, &name).to_string(),
                        observations: retained_observations,
                        target: name,
                    });
                }
                platform.pace_root_owned_observation(&name, retained_observations);
                observation = platform
                    .observe(&retained_plan.operation_id, state)
                    .map_err(EnsureWorkflowError::Platform)?;
                if let Some(operation_id) = terminal_inventory_operation_id {
                    attach_terminal_inventory_cycles(
                        operation_id,
                        state,
                        platform,
                        &mut observation,
                    )?;
                }
            }
            Err(error) => return Err(error.into()),
        }
    };
    if observation.operator_cycles < retained_plan.conservation.maximum_operator_debit_cycles {
        return Err(EnsureWorkflowError::InsufficientOperatorCycles {
            actual: observation.operator_cycles,
            required: retained_plan.conservation.maximum_operator_debit_cycles,
        });
    }
    bind_terminal_inventory_operation(
        &mut current,
        retained_plan.terminal_inventory_operation_id.clone(),
    );
    if current.plan_sha256 != retained_plan.plan_sha256
        && !compatible_after_bounded_observation(retained_plan, &current, desired, &observation)
    {
        return Err(EnsureWorkflowError::DriftedBeforeApply);
    }
    Ok((
        observation,
        retained_plan.conservation.observed_controlled_cycles,
    ))
}

fn estate_funding_requirement<E>(
    plan: &FleetEnsurePlan,
    state: &FleetEnsureStateRecord,
    observation: &FleetObservation,
) -> Result<Option<EstateFundingRequiredRecord>, EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    for reviewed in &plan.conservation.estate_funding_domains {
        let observed = observation
            .estate_funding_domains
            .get(&reviewed.root)
            .ok_or(EnsureWorkflowError::PlanIntegrity)?;
        if observed.cycles_ledger != reviewed.cycles_ledger {
            return Err(EnsureWorkflowError::PlanIntegrity);
        }
        let root_principal = observed
            .root_principal
            .as_deref()
            .ok_or(EnsureWorkflowError::PlanIntegrity)?;
        if reviewed_estate_root_principal(reviewed, state) != Some(root_principal) {
            return Err(EnsureWorkflowError::PlanIntegrity);
        }
        let available_cycles = observed
            .balance_cycles
            .ok_or(EnsureWorkflowError::PlanIntegrity)?;
        let reviewed_available = reviewed.available_cycles.unwrap_or_default();
        let maximum_reviewed_balance = reviewed_available
            .checked_add(reviewed.maximum_funding_cycles)
            .ok_or(EnsureWorkflowError::PlanIntegrity)?;
        if available_cycles < reviewed_available || available_cycles > maximum_reviewed_balance {
            return Err(EnsureWorkflowError::DriftedBeforeApply);
        }
        if reviewed.required_creation_count == 0 {
            if available_cycles != reviewed_available {
                return Err(EnsureWorkflowError::DriftedBeforeApply);
            }
            continue;
        }
        if available_cycles < reviewed.maximum_creation_debit_cycles {
            return Ok(Some(EstateFundingRequiredRecord {
                attempt_count: None,
                available_cycles,
                creation_amount_cycles: reviewed.creation_amount_cycles,
                creation_execution_margin_cycles: reviewed.creation_execution_margin_cycles,
                readiness_floor_cycles: reviewed.readiness_floor_cycles,
                cycles_ledger: reviewed.cycles_ledger.clone(),
                ledger_fee_cycles: reviewed.ledger_fee_cycles,
                management_creation_fee_cycles: reviewed.management_creation_fee_cycles,
                maximum_creation_debit_cycles: reviewed.maximum_creation_debit_cycles,
                maximum_creation_fee_cycles: reviewed.maximum_creation_fee_cycles,
                operation_id: plan.operation_id.clone(),
                pending_creation_operation_id: None,
                plan_sha256: plan.plan_sha256.clone(),
                required_creation_count: reviewed.required_creation_count,
                root: reviewed.root.clone(),
                root_principal: root_principal.to_string(),
                last_attempt_at_ns: None,
                retry_at_ns: None,
                shortfall_cycles: reviewed
                    .maximum_creation_debit_cycles
                    .saturating_sub(available_cycles),
            }));
        }
    }
    Ok(None)
}

fn protocol_estate_funding_requirement<E>(
    plan: &FleetEnsurePlan,
    state: &FleetEnsureStateRecord,
    funding: &canic_core::dto::component_provisioning::RootEstateFundingRequired,
) -> Result<EstateFundingRequiredRecord, EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    let root_principal = funding.root.to_text();
    let reviewed = plan
        .conservation
        .estate_funding_domains
        .iter()
        .find(|domain| reviewed_estate_root_principal(domain, state) == Some(&root_principal))
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let creation_amount_cycles = funding.creation_amount.to_u128();
    let ledger_fee_cycles = funding.ledger_fee.to_u128();
    let management_creation_fee_cycles = funding.management_creation_fee.to_u128();
    let required_cycles = creation_amount_cycles
        .checked_add(ledger_fee_cycles)
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let maximum_creation_fee_cycles = management_creation_fee_cycles
        .checked_add(ledger_fee_cycles)
        .ok_or(EnsureWorkflowError::PlanIntegrity)?;
    let exact_reviewed_authority = reviewed.required_creation_count > 0
        && reviewed.cycles_ledger == funding.cycles_ledger.to_text()
        && reviewed.creation_amount_cycles == creation_amount_cycles
        && reviewed.creation_execution_margin_cycles == funding.execution_margin.to_u128()
        && reviewed.readiness_floor_cycles == funding.readiness_floor.to_u128()
        && reviewed.ledger_fee_cycles == ledger_fee_cycles
        && reviewed.management_creation_fee_cycles == management_creation_fee_cycles;
    let exact_runtime_arithmetic = funding.required.to_u128() == required_cycles
        && funding.available < funding.required
        && funding.shortfall.to_u128()
            == required_cycles.saturating_sub(funding.available.to_u128());
    if !exact_reviewed_authority || !exact_runtime_arithmetic {
        return Err(EnsureWorkflowError::PlanIntegrity);
    }
    Ok(EstateFundingRequiredRecord {
        attempt_count: Some(funding.attempt_count),
        available_cycles: funding.available.to_u128(),
        creation_amount_cycles,
        cycles_ledger: funding.cycles_ledger.to_text(),
        creation_execution_margin_cycles: funding.execution_margin.to_u128(),
        readiness_floor_cycles: funding.readiness_floor.to_u128(),
        ledger_fee_cycles,
        management_creation_fee_cycles,
        maximum_creation_debit_cycles: required_cycles,
        maximum_creation_fee_cycles,
        operation_id: plan.operation_id.clone(),
        pending_creation_operation_id: Some(canic_core::cdk::utils::hash::hex_bytes(
            funding.operation_id,
        )),
        plan_sha256: plan.plan_sha256.clone(),
        required_creation_count: 1,
        root: reviewed.root.clone(),
        root_principal,
        last_attempt_at_ns: funding.last_attempt_at_ns,
        retry_at_ns: Some(funding.retry_at_ns),
        shortfall_cycles: funding.shortfall.to_u128(),
    })
}

fn estate_funding_error(required: &EstateFundingRequiredRecord) -> EstateFundingRequiredError {
    EstateFundingRequiredError {
        attempt_count: required.attempt_count,
        available_cycles: required.available_cycles,
        creation_amount_cycles: required.creation_amount_cycles,
        creation_execution_margin_cycles: required.creation_execution_margin_cycles,
        readiness_floor_cycles: required.readiness_floor_cycles,
        cycles_ledger: required.cycles_ledger.clone(),
        ledger_fee_cycles: required.ledger_fee_cycles,
        management_creation_fee_cycles: required.management_creation_fee_cycles,
        maximum_creation_debit_cycles: required.maximum_creation_debit_cycles,
        maximum_creation_fee_cycles: required.maximum_creation_fee_cycles,
        pending_creation_operation_id: required.pending_creation_operation_id.clone(),
        required_creation_count: required.required_creation_count,
        root: required.root.clone(),
        root_principal: required.root_principal.clone(),
        last_attempt_at_ns: required.last_attempt_at_ns,
        retry_at_ns: required.retry_at_ns,
        shortfall_cycles: required.shortfall_cycles,
    }
}

fn retain_estate_funding_pause<E>(
    paths: &EnsurePaths,
    journal: &mut FleetEnsureJournalRecord,
    required: Option<&EstateFundingRequiredRecord>,
) -> Result<bool, EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    if journal.estate_funding_required.as_ref() == required {
        return Ok(false);
    }
    journal.estate_funding_required = required.cloned();
    write_journal(paths, journal)?;
    Ok(true)
}

fn reviewed_estate_root_principal<'a>(
    domain: &'a crate::fleet_ensure::model::EstateFundingDomainPlan,
    state: &'a FleetEnsureStateRecord,
) -> Option<&'a str> {
    domain
        .root_principal
        .as_deref()
        .or_else(|| {
            state
                .pending_principals
                .get(&domain.root)
                .map(String::as_str)
        })
        .or_else(|| state.principals.get(&domain.root).map(String::as_str))
}

fn prior_fleet_protocol_effect_started(
    actions: &[&EnsureAction],
    journal: &FleetEnsureJournalRecord,
    current_index: usize,
) -> bool {
    actions
        .iter()
        .take(current_index)
        .zip(&journal.effects)
        .any(|(action, effect)| {
            matches!(action, EnsureAction::FleetProtocol { .. })
                && matches!(effect.state, EffectState::Issued | EffectState::Applied)
        })
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
    verify_terminal_conservation(retained_plan, journal, state, &terminal)
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
    verify_journal(journal, plan, requested_fleet, state)?;
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
        || !plan.conservation.estate_funding_domains.is_empty()
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
        estate_funding_domains: BTreeMap::new(),
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
        estate_funding_domains: plan
            .conservation
            .estate_funding_domains
            .iter()
            .cloned()
            .map(|mut domain| {
                domain.available_cycles = None;
                domain.maximum_funding_cycles = 0;
                domain.root_principal = None;
                domain.shortfall_cycles = 0;
                domain
            })
            .collect(),
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
        || !plan.conservation.estate_funding_domains.is_empty()
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
                    TerminalInventoryError::IdleCycleObservationConflict {
                        canister: principal,
                        expected: configured.cycles,
                        observed: cycles,
                    },
                ));
            }
            _ => {
                return Err(EnsureWorkflowError::TerminalInventory(
                    TerminalInventoryError::CycleObservationLifecycleConflict {
                        canister: principal,
                    },
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
    state: &FleetEnsureStateRecord,
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
    let (estate_funding_cycles, exact_estate_creation_fee_cycles) =
        reconcile_estate_funding(plan, journal, state, terminal)?;
    let available = journal
        .initial_controlled_cycles
        .checked_add(received_new_funding_cycles)
        .ok_or_else(|| {
            EnsureWorkflowError::Conservation(
                "terminal controlled-cycle arithmetic overflowed".to_string(),
            )
        })?;
    let after_estate_fees = available
        .checked_sub(exact_estate_creation_fee_cycles)
        .ok_or_else(|| {
            EnsureWorkflowError::Conservation(
                "terminal controlled cycles cannot cover reviewed estate creation fees".to_string(),
            )
        })?;
    let measured_execution_burn_cycles = after_estate_fees
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
        estate_funding_cycles,
        exact_estate_creation_fee_cycles,
        exact_unavoidable_fee_cycles: plan.conservation.maximum_unavoidable_fee_cycles,
        final_controlled_cycles,
        measured_execution_burn_cycles,
        observed_starting_cycles: journal.initial_controlled_cycles,
        operator_debit_cycles,
        received_new_funding_cycles,
    })
}

fn component_provisioning_is_applied(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
) -> bool {
    plan.protocol_actions.iter().any(|action| {
        let is_component_provisioning = matches!(
            action,
            EnsureAction::FleetProtocol { action, .. }
                if matches!(
                    action.as_ref(),
                    crate::fleet_ensure::model::CurrentFleetProtocolAction::ProvisionComponents { .. }
                )
        );
        let has_applied_effect = journal.effects.iter().any(|effect| {
            effect.action_sha256 == action_sha256(action)
                && effect.state == EffectState::Applied
                && effect.receipt.is_some()
        });
        is_component_provisioning && has_applied_effect
    })
}

fn resolved_initial_pool_assets<E>(
    domain: &crate::fleet_ensure::model::EstateFundingDomainPlan,
    state: &FleetEnsureStateRecord,
) -> Result<BTreeSet<String>, EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    domain
        .initial_pool_assets
        .iter()
        .map(|identity| {
            identity.strip_prefix("created:").map_or_else(
                || Ok(identity.clone()),
                |name| {
                    state
                        .pending_principals
                        .get(name)
                        .or_else(|| state.principals.get(name))
                        .cloned()
                        .ok_or(EnsureWorkflowError::JournalIntegrity)
                },
            )
        })
        .collect()
}

fn exact_estate_creation_costs<E>(
    component_provisioning_applied: bool,
    state: &FleetEnsureStateRecord,
    terminal: &FleetObservation,
    domain: &crate::fleet_ensure::model::EstateFundingDomainPlan,
) -> Result<(u128, u128), EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    let observed = terminal
        .estate_funding_domains
        .get(&domain.root)
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    let Some(pool) = observed.pool.as_ref() else {
        return if domain.required_creation_count == 0 {
            Ok((0, 0))
        } else {
            Err(EnsureWorkflowError::JournalIntegrity)
        };
    };
    if pool.pending_creation.is_some() {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }

    let initial = resolved_initial_pool_assets(domain, state)?;
    let terminal_principals = pool
        .assets
        .iter()
        .map(|asset| asset.principal.clone())
        .collect::<BTreeSet<_>>();
    if terminal_principals.len() != pool.assets.len() || !initial.is_subset(&terminal_principals) {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    let created = pool
        .assets
        .iter()
        .filter(|asset| !initial.contains(&asset.principal))
        .collect::<Vec<_>>();
    let creation_count_is_bounded = created.len()
        <= usize::try_from(domain.required_creation_count)
            .map_err(|_| EnsureWorkflowError::JournalIntegrity)?;
    let creation_phase_is_valid = created.is_empty() || component_provisioning_applied;
    let pool_policy_is_exact = created.is_empty() || terminal_pool_policy_is_exact(pool, domain);
    if !creation_count_is_bounded || !creation_phase_is_valid || !pool_policy_is_exact {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }

    let mut operation_ids = BTreeSet::new();
    let mut block_indexes = BTreeSet::new();
    let mut creation_debit = 0_u128;
    let mut creation_fee = 0_u128;
    for asset in created {
        let (asset_debit, asset_fee) = exact_estate_creation_receipt_costs(
            asset,
            domain,
            &mut operation_ids,
            &mut block_indexes,
        )?;
        creation_debit = creation_debit.checked_add(asset_debit).ok_or_else(|| {
            EnsureWorkflowError::Conservation(
                "estate creation-debit arithmetic overflowed".to_string(),
            )
        })?;
        creation_fee = creation_fee.checked_add(asset_fee).ok_or_else(|| {
            EnsureWorkflowError::Conservation(
                "estate creation-fee arithmetic overflowed".to_string(),
            )
        })?;
    }
    if creation_debit > domain.maximum_creation_debit_cycles
        || creation_fee > domain.maximum_creation_fee_cycles
    {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    Ok((creation_debit, creation_fee))
}

const fn terminal_pool_policy_is_exact(
    pool: &crate::fleet_ensure::model::EstatePoolInventoryObservation,
    domain: &crate::fleet_ensure::model::EstateFundingDomainPlan,
) -> bool {
    let expected_maximum_size = domain.pool_maximum_size;
    let expected_readiness_floor = domain.readiness_floor_cycles;
    let expected_execution_margin = domain.creation_execution_margin_cycles;
    pool.maximum_size == expected_maximum_size
        && pool.readiness_floor_cycles == expected_readiness_floor
        && pool.creation_execution_margin_cycles == expected_execution_margin
}

#[derive(Eq, PartialEq)]
struct EstateCreationReceiptAuthority<'a> {
    creation_execution_margin_cycles: u128,
    cycles_ledger: &'a str,
    ledger_amount_cycles: u128,
    ledger_fee_cycles: u128,
    management_creation_fee_cycles: u128,
    readiness_floor_cycles: u128,
}

fn observed_estate_creation_authority(
    receipt: &crate::fleet_ensure::model::EstatePoolCreationReceiptObservation,
) -> EstateCreationReceiptAuthority<'_> {
    EstateCreationReceiptAuthority {
        creation_execution_margin_cycles: receipt.creation_execution_margin_cycles,
        cycles_ledger: &receipt.cycles_ledger,
        ledger_amount_cycles: receipt.ledger_amount_cycles,
        ledger_fee_cycles: receipt.ledger_fee_cycles,
        management_creation_fee_cycles: receipt.management_creation_fee_cycles,
        readiness_floor_cycles: receipt.readiness_floor_cycles,
    }
}

fn reviewed_estate_creation_authority(
    domain: &crate::fleet_ensure::model::EstateFundingDomainPlan,
) -> EstateCreationReceiptAuthority<'_> {
    EstateCreationReceiptAuthority {
        creation_execution_margin_cycles: domain.creation_execution_margin_cycles,
        cycles_ledger: &domain.cycles_ledger,
        ledger_amount_cycles: domain.creation_amount_cycles,
        ledger_fee_cycles: domain.ledger_fee_cycles,
        management_creation_fee_cycles: domain.management_creation_fee_cycles,
        readiness_floor_cycles: domain.readiness_floor_cycles,
    }
}

fn first_estate_creation_observation_is_exact(
    receipt: &crate::fleet_ensure::model::EstatePoolCreationReceiptObservation,
) -> bool {
    receipt
        .ledger_amount_cycles
        .checked_sub(receipt.management_creation_fee_cycles)
        .zip(receipt.first_observed_cycles)
        .is_some_and(|(funded_native_cycles, observed_cycles)| {
            observed_cycles >= receipt.readiness_floor_cycles
                && observed_cycles <= funded_native_cycles
                && funded_native_cycles - observed_cycles
                    <= receipt.creation_execution_margin_cycles
        })
}

fn exact_estate_creation_receipt_costs<E>(
    asset: &crate::fleet_ensure::model::EstatePoolAssetObservation,
    domain: &crate::fleet_ensure::model::EstateFundingDomainPlan,
    operation_ids: &mut BTreeSet<String>,
    block_indexes: &mut BTreeSet<u64>,
) -> Result<(u128, u128), EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    let receipt = asset
        .creation_receipt
        .as_ref()
        .filter(|_| asset.origin == crate::fleet_ensure::model::EstatePoolAssetOrigin::Created)
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    let authority_is_exact =
        observed_estate_creation_authority(receipt) == reviewed_estate_creation_authority(domain);
    if !is_sha256_hex(&receipt.operation_id)
        || !authority_is_exact
        || !first_estate_creation_observation_is_exact(receipt)
        || !operation_ids.insert(receipt.operation_id.clone())
        || !block_indexes.insert(receipt.block_index)
    {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    let debit = receipt
        .ledger_amount_cycles
        .checked_add(receipt.ledger_fee_cycles)
        .ok_or_else(|| {
            EnsureWorkflowError::Conservation(
                "estate creation-debit arithmetic overflowed".to_string(),
            )
        })?;
    let fee = receipt
        .management_creation_fee_cycles
        .checked_add(receipt.ledger_fee_cycles)
        .ok_or_else(|| {
            EnsureWorkflowError::Conservation(
                "estate creation-fee arithmetic overflowed".to_string(),
            )
        })?;
    Ok((debit, fee))
}

fn reconcile_estate_funding<E>(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    terminal: &FleetObservation,
) -> Result<(u128, u128), EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    let component_provisioning_applied = component_provisioning_is_applied(plan, journal);
    plan.conservation
        .estate_funding_domains
        .iter()
        .try_fold((0_u128, 0_u128), |(funding_total, fee_total), domain| {
            let initial = journal
                .initial_estate_funding_cycles_by_root
                .get(&domain.root)
                .copied()
                .ok_or(EnsureWorkflowError::JournalIntegrity)?;
            let observed = terminal
                .estate_funding_domains
                .get(&domain.root)
                .ok_or(EnsureWorkflowError::JournalIntegrity)?;
            if observed.cycles_ledger != domain.cycles_ledger
                || reviewed_estate_root_principal(domain, state)
                    != observed.root_principal.as_deref()
            {
                return Err(EnsureWorkflowError::JournalIntegrity);
            }
            let terminal_balance = observed
                .balance_cycles
                .ok_or(EnsureWorkflowError::JournalIntegrity)?;
            let (creation_debit, creation_fee) = exact_estate_creation_costs(
                component_provisioning_applied,
                state,
                terminal,
                domain,
            )?;
            let funding = applied_estate_funding_for_domain(plan, journal, state, domain)?;
            let expected_terminal_balance = initial
                .checked_add(funding)
                .and_then(|available| available.checked_sub(creation_debit))
                .ok_or_else(|| {
                    EnsureWorkflowError::Conservation(format!(
                        "Root {} funding-account balance cannot reconcile its reviewed funding and creation debit",
                        domain.root
                    ))
                })?;
            if terminal_balance != expected_terminal_balance {
                return Err(EnsureWorkflowError::Conservation(format!(
                    "Root {} funding-account balance {terminal_balance} differs from reviewed terminal balance {expected_terminal_balance}",
                    domain.root
                )));
            }
            Ok((
                funding_total.checked_add(funding).ok_or_else(|| {
                    EnsureWorkflowError::Conservation(
                        "estate funding arithmetic overflowed".to_string(),
                    )
                })?,
                fee_total
                    .checked_add(creation_fee)
                    .ok_or_else(|| {
                        EnsureWorkflowError::Conservation(
                            "estate creation-fee arithmetic overflowed".to_string(),
                        )
                    })?,
            ))
        })
}

fn applied_estate_funding_for_domain<E>(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    state: &FleetEnsureStateRecord,
    domain: &crate::fleet_ensure::model::EstateFundingDomainPlan,
) -> Result<u128, EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    let action = plan
        .canisters
        .iter()
        .find(|canister| canister.name == domain.root)
        .into_iter()
        .flat_map(|canister| &canister.actions)
        .find(|action| matches!(action, EnsureAction::FundEstate { .. }));
    let Some(action) = action else {
        return if domain.shortfall_cycles == 0 {
            Ok(0)
        } else {
            Err(EnsureWorkflowError::JournalIntegrity)
        };
    };
    let EnsureAction::FundEstate {
        amount,
        expected_post_cycles,
        ledger,
        ledger_fee_cycles,
        principal,
        ..
    } = action
    else {
        unreachable!("filtered estate-funding action")
    };
    let root_principal = principal.strip_prefix("created:").map_or_else(
        || Some(principal.as_str()),
        |name| {
            state
                .pending_principals
                .get(name)
                .or_else(|| state.principals.get(name))
                .map(String::as_str)
        },
    );
    let expected_destination = domain
        .available_cycles
        .unwrap_or_default()
        .checked_add(domain.shortfall_cycles)
        .ok_or(EnsureWorkflowError::JournalIntegrity)?;
    let action_authority_is_exact = *amount == domain.shortfall_cycles
        && *expected_post_cycles == expected_destination
        && ledger == &domain.cycles_ledger
        && *ledger_fee_cycles == domain.ledger_fee_cycles
        && root_principal == reviewed_estate_root_principal(domain, state);
    let effect = journal
        .effects
        .iter()
        .find(|effect| effect.action_sha256 == action_sha256(action));
    let Some(effect) = effect.filter(|effect| {
        effect.state == EffectState::Applied
            && effect
                .receipt
                .as_deref()
                .is_some_and(|receipt| !receipt.is_empty())
    }) else {
        return Err(EnsureWorkflowError::JournalIntegrity);
    };
    let balance_evidence_is_exact = crate::fleet_ensure::ops::estate_funding_applied(
        crate::fleet_ensure::ops::EstateFundingObservation {
            amount: *amount,
            destination_after: effect.destination_post_cycles.unwrap_or_default(),
            destination_before: effect.destination_pre_cycles,
            expected_destination_after: *expected_post_cycles,
            ledger_fee_cycles: *ledger_fee_cycles,
            source_after: effect.post_cycles.unwrap_or_default(),
            source_before: effect.pre_cycles,
        },
    );
    if !action_authority_is_exact || !balance_evidence_is_exact {
        return Err(EnsureWorkflowError::JournalIntegrity);
    }
    Ok(*amount)
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
        .and_then(|total| {
            observation
                .estate_funding_domains
                .values()
                .filter_map(|domain| domain.balance_cycles)
                .try_fold(total, u128::checked_add)
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
    state: &FleetEnsureStateRecord,
) -> Result<(), EnsureWorkflowError<E>>
where
    E: std::error::Error + 'static,
{
    if journal.fleet != requested_fleet
        || journal.operation_id != plan.operation_id
        || journal.plan_sha256 != plan.plan_sha256
        || !initial_estate_funding_is_exact(journal, plan)
        || !estate_funding_pause_is_exact(journal, plan, state)
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

fn initial_estate_funding_is_exact(
    journal: &FleetEnsureJournalRecord,
    plan: &FleetEnsurePlan,
) -> bool {
    journal.initial_estate_funding_cycles_by_root.len()
        == plan.conservation.estate_funding_domains.len()
        && plan
            .conservation
            .estate_funding_domains
            .iter()
            .all(|domain| {
                journal
                    .initial_estate_funding_cycles_by_root
                    .get(&domain.root)
                    .copied()
                    == Some(domain.available_cycles.unwrap_or_default())
            })
}

fn estate_funding_pause_is_exact(
    journal: &FleetEnsureJournalRecord,
    plan: &FleetEnsurePlan,
    state: &FleetEnsureStateRecord,
) -> bool {
    let Some(required) = journal.estate_funding_required.as_ref() else {
        return true;
    };
    let Some(reviewed) = plan
        .conservation
        .estate_funding_domains
        .iter()
        .find(|domain| domain.root == required.root)
    else {
        return false;
    };
    let operation_authority_is_exact = journal.completion == FleetEnsureCompletion::InProgress
        && required.operation_id == plan.operation_id
        && required.plan_sha256 == plan.plan_sha256;
    let funding_authority_is_exact =
        EstateFundingPauseAuthority::from(required) == EstateFundingPauseAuthority::from(reviewed);
    let root_authority_is_exact =
        reviewed_estate_root_principal(reviewed, state) == Some(required.root_principal.as_str());
    let shortfall_is_exact = required.available_cycles < required.maximum_creation_debit_cycles
        && required.shortfall_cycles
            == required
                .maximum_creation_debit_cycles
                .saturating_sub(required.available_cycles);
    if !(operation_authority_is_exact
        && funding_authority_is_exact
        && root_authority_is_exact
        && shortfall_is_exact)
    {
        return false;
    }
    if required.pending_creation_operation_id.is_some() {
        current_creation_funding_pause_is_exact(required, reviewed)
    } else {
        planned_estate_funding_pause_is_exact(required, reviewed)
    }
}

#[derive(Eq, PartialEq)]
struct EstateFundingPauseAuthority<'a> {
    creation_amount_cycles: u128,
    creation_execution_margin_cycles: u128,
    cycles_ledger: &'a str,
    ledger_fee_cycles: u128,
    management_creation_fee_cycles: u128,
    readiness_floor_cycles: u128,
    root: &'a str,
}

impl<'a> From<&'a EstateFundingRequiredRecord> for EstateFundingPauseAuthority<'a> {
    fn from(required: &'a EstateFundingRequiredRecord) -> Self {
        Self {
            creation_amount_cycles: required.creation_amount_cycles,
            creation_execution_margin_cycles: required.creation_execution_margin_cycles,
            cycles_ledger: &required.cycles_ledger,
            ledger_fee_cycles: required.ledger_fee_cycles,
            management_creation_fee_cycles: required.management_creation_fee_cycles,
            readiness_floor_cycles: required.readiness_floor_cycles,
            root: &required.root,
        }
    }
}

impl<'a> From<&'a EstateFundingDomainPlan> for EstateFundingPauseAuthority<'a> {
    fn from(reviewed: &'a EstateFundingDomainPlan) -> Self {
        Self {
            creation_amount_cycles: reviewed.creation_amount_cycles,
            creation_execution_margin_cycles: reviewed.creation_execution_margin_cycles,
            cycles_ledger: &reviewed.cycles_ledger,
            ledger_fee_cycles: reviewed.ledger_fee_cycles,
            management_creation_fee_cycles: reviewed.management_creation_fee_cycles,
            readiness_floor_cycles: reviewed.readiness_floor_cycles,
            root: &reviewed.root,
        }
    }
}

fn current_creation_funding_pause_is_exact(
    required: &EstateFundingRequiredRecord,
    reviewed: &EstateFundingDomainPlan,
) -> bool {
    let operation_id_is_exact = required
        .pending_creation_operation_id
        .as_deref()
        .is_some_and(is_sha256_hex);
    let attempt_evidence_is_exact = required.attempt_count.is_some()
        && required
            .retry_at_ns
            .is_some_and(|retry_at_ns| retry_at_ns > 0)
        && required
            .last_attempt_at_ns
            .is_none_or(|attempted_at_ns| attempted_at_ns > 0);
    let single_creation_debit = required
        .creation_amount_cycles
        .checked_add(required.ledger_fee_cycles);
    let single_creation_fees = required
        .management_creation_fee_cycles
        .checked_add(required.ledger_fee_cycles);
    let debit_authority_is_exact = Some(required.maximum_creation_debit_cycles)
        == single_creation_debit
        && Some(required.maximum_creation_fee_cycles) == single_creation_fees;
    reviewed.required_creation_count > 0
        && required.required_creation_count == 1
        && operation_id_is_exact
        && attempt_evidence_is_exact
        && debit_authority_is_exact
}

const fn planned_estate_funding_pause_is_exact(
    required: &EstateFundingRequiredRecord,
    reviewed: &EstateFundingDomainPlan,
) -> bool {
    let runtime_attempt_is_absent = required.attempt_count.is_none()
        && required.last_attempt_at_ns.is_none()
        && required.retry_at_ns.is_none();
    let reviewed_debit_is_exact = required.maximum_creation_debit_cycles
        == reviewed.maximum_creation_debit_cycles
        && required.maximum_creation_fee_cycles == reviewed.maximum_creation_fee_cycles
        && required.required_creation_count == reviewed.required_creation_count;
    runtime_attempt_is_absent && reviewed_debit_is_exact
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
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
    let maximum_observation_burn_cycles = desired
        .maximum_observation_burn_cycles
        .parse::<Cycles>()
        .map(|cycles| cycles.to_u128())
        .map_err(|_| TerminalStatePublicationError::JournalIntegrity)?;
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
        if !create_balance_is_terminal(
            Some(post_cycles),
            *requested_initial_cycles,
            maximum_observation_burn_cycles,
        ) || !create_identity_is_exact(name, record, state)
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
            TerminalInventoryError::DuplicateRetainedPrincipal,
        ));
    }
    for entry in &entries {
        names_by_principal
            .entry(entry.pid.clone())
            .or_insert_with(|| format!("observed:{}", entry.pid));
    }
    for entry in entries {
        let name = names_by_principal.get(&entry.pid).cloned().ok_or_else(|| {
            EnsureWorkflowError::TerminalInventory(TerminalInventoryError::LostPrincipalBinding {
                canister: entry.pid.clone(),
            })
        })?;
        let parent = entry
            .parent_pid
            .as_ref()
            .map(|principal| {
                names_by_principal.get(principal).cloned().ok_or_else(|| {
                    EnsureWorkflowError::TerminalInventory(TerminalInventoryError::UnknownParent {
                        canister: entry.pid.clone(),
                        parent: principal.clone(),
                    })
                })
            })
            .transpose()?;
        let existing = state.topology.get(&name);
        let kind = terminal_inventory_kind(existing, &entry);
        let parent_conflict = existing
            .and_then(|topology| topology.parent.as_ref())
            .zip(parent.as_ref())
            .filter(|(expected, actual)| expected != actual);
        if let Some((retained_parent, terminal_parent)) = parent_conflict
            && !verified_pool_to_component_parent_transition(existing, kind, &entry)
        {
            return Err(EnsureWorkflowError::TerminalInventory(
                TerminalInventoryError::RetainedParentConflict {
                    canister: entry.pid,
                    retained_parent: retained_parent.clone(),
                    terminal_parent: terminal_parent.clone(),
                },
            ));
        }
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

fn terminal_inventory_kind(
    existing: Option<&crate::fleet_ensure::model::FleetEnsureTopologyRecord>,
    entry: &crate::registry::RegistryEntry,
) -> crate::fleet_ensure::model::DesiredCanisterKind {
    existing.map_or_else(
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
    )
}

fn verified_pool_to_component_parent_transition(
    existing: Option<&crate::fleet_ensure::model::FleetEnsureTopologyRecord>,
    terminal_kind: crate::fleet_ensure::model::DesiredCanisterKind,
    entry: &crate::registry::RegistryEntry,
) -> bool {
    existing.is_some_and(|topology| {
        topology.kind == crate::fleet_ensure::model::DesiredCanisterKind::Pool
            && terminal_kind == crate::fleet_ensure::model::DesiredCanisterKind::Component
            && entry.module_hash.is_some()
            && entry.protocol_binding.is_some()
    })
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
    let temporary_pool_observation_finalizations = plan
        .reviewed_desired
        .as_deref()
        .map(|reviewed| {
            let desired = reviewed.desired();
            plan.canisters
                .iter()
                .filter_map(|canister_plan| {
                    let configured = desired.canisters.iter().find(|configured| {
                        configured.name == canister_plan.name
                            && configured.kind
                                == crate::fleet_ensure::model::DesiredCanisterKind::Pool
                    })?;
                    let temporary_create = desired
                        .bootstrap
                        .as_ref()
                        .is_some_and(|bootstrap| bootstrap.fresh_estate)
                        && configured.principal.is_none()
                        && configured.controllers.is_empty()
                        && configured.controller_canisters.len() == 1
                        && canister_plan.actions.iter().any(|action| {
                            matches!(
                                action,
                                EnsureAction::Create {
                                    controller_canisters,
                                    controllers,
                                    ..
                                } if controller_canisters == &configured.controller_canisters
                                    && controllers == std::slice::from_ref(&desired.operator)
                            )
                        });
                    temporary_create.then_some(canister_plan.name.as_str())
                })
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
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
        } else if matches!(action, EnsureAction::SetControllers { name, .. } if temporary_pool_observation_finalizations.contains(name.as_str()))
        {
            6
        } else {
            action_order(action)
        }
    });
    actions
}

fn deferred_create_observation_is_exact(
    desired: &crate::fleet_ensure::model::DesiredFleet,
    actions: &[&EnsureAction],
    index: usize,
    action: &EnsureAction,
    record: &EffectRecord,
    state: &FleetEnsureStateRecord,
) -> bool {
    let EnsureAction::Create {
        controller_canisters,
        controllers,
        name,
        ..
    } = action
    else {
        return false;
    };
    let Some(configured) = desired.canisters.iter().find(|canister| {
        canister.name == *name
            && canister.kind == crate::fleet_ensure::model::DesiredCanisterKind::Pool
    }) else {
        return false;
    };
    let Some(parent) = configured.parent.as_deref() else {
        return false;
    };
    let fresh_root_only_create = desired
        .bootstrap
        .as_ref()
        .is_some_and(|bootstrap| bootstrap.fresh_estate)
        && configured.principal.is_none()
        && configured.controllers.is_empty()
        && configured.controller_canisters.as_slice() == [parent]
        && controller_canisters == &configured.controller_canisters
        && controllers.is_empty();
    let exact_issued_create = record.state == EffectState::Issued
        && create_identity_is_exact(name, record, state)
        && record.post_cycles.is_some();
    let later_root_install = actions[index.saturating_add(1)..].iter().any(|candidate| {
        matches!(
            candidate,
            EnsureAction::Install {
                canic_init: Some(crate::fleet_ensure::model::DesiredCanisterInit::Root { root }),
                name: install_name,
                ..
            } if root == parent && install_name == parent
        )
    });
    let infrastructure_only = actions.iter().all(|candidate| {
        matches!(
            candidate,
            EnsureAction::Create { .. }
                | EnsureAction::FundEstate { .. }
                | EnsureAction::Install { .. }
                | EnsureAction::SetControllers { .. }
        )
    });
    fresh_root_only_create && exact_issued_create && later_root_install && infrastructure_only
}

pub(super) const fn action_order(action: &EnsureAction) -> u8 {
    match action {
        EnsureAction::Create { .. } => 0,
        EnsureAction::Fund { .. } | EnsureAction::FundEstate { .. } => 1,
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

    fn terminal_component_entry(canister: &str, parent: &str) -> crate::registry::RegistryEntry {
        crate::registry::RegistryEntry {
            pid: canister.to_string(),
            role: Some("managed_component".to_string()),
            parent_pid: Some(parent.to_string()),
            module_hash: Some("11".repeat(32)),
            protocol_binding: Some(crate::protocol_binding::RegistryProtocolBinding {
                release_identity: "0.110.test".to_string(),
                role: canic_core::ids::CanisterRole::from("managed_component"),
                capabilities: BTreeSet::new(),
                candid_sha256: [1; 32],
                protocol_profile_digest:
                    canic_core::role_contract::ProtocolProfileDigest::from_bytes([2; 32]),
            }),
        }
    }

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
            estate_funding_required: None,
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
            initial_estate_funding_cycles_by_root: BTreeMap::new(),
            initial_operator_cycles: 0,
            operation_id: "operation".to_string(),
            plan_sha256: "plan".to_string(),
            schema_version: crate::fleet_ensure::model::FLEET_ENSURE_SCHEMA_VERSION,
            stalled_observations: 0,
        };
        (state, journal)
    }

    fn estate_funding_plan() -> FleetEnsurePlan {
        FleetEnsurePlan {
            canisters: Vec::new(),
            conservation: CycleConservation {
                estate_funding_domains: vec![crate::fleet_ensure::model::EstateFundingDomainPlan {
                    allocated_workloads: 0,
                    available_cycles: Some(40),
                    available_pool_slots: 2,
                    creation_amount_cycles: 45,
                    creation_execution_margin_cycles: 5,
                    readiness_floor_cycles: 35,
                    cycles_ledger: "ledger".to_string(),
                    eligible_ready_pool_assets: 0,
                    initial_pool_assets: Vec::new(),
                    ledger_fee_cycles: 5,
                    management_creation_fee_cycles: 5,
                    maximum_creation_debit_cycles: 100,
                    maximum_creation_fee_cycles: 20,
                    maximum_funding_cycles: 60,
                    occupied_pool_assets: 0,
                    pending_creation_count: 0,
                    pending_creation: None,
                    planned_initial_workloads: 2,
                    pool_maximum_size: 2,
                    required_creation_count: 2,
                    root: "root".to_string(),
                    root_principal: Some("rrkah-fqaaa-aaaaa-aaaaq-cai".to_string()),
                    shortfall_cycles: 60,
                }],
                expected_post_operation_cycles: 80,
                maximum_execution_burn_cycles: 0,
                maximum_new_funding_cycles: 0,
                maximum_operator_debit_cycles: 0,
                maximum_unavoidable_fee_cycles: 0,
                observed_controlled_cycles: 40,
                retained_in_reused_canisters_cycles: 0,
                scheduled_transfer_cycles: 0,
            },
            desired_sha256: "desired".to_string(),
            environment: "local".to_string(),
            fleet: "fleet".to_string(),
            operation_id: "operation".to_string(),
            plan_sha256: "plan".to_string(),
            planned_at_time: 1,
            protocol_actions: Vec::new(),
            root_start_authority: None,
            reviewed_desired: None,
            schema_version: FLEET_ENSURE_SCHEMA_VERSION,
            scope: FleetEnsurePlanScope::Full,
            terminal_inventory_operation_id: None,
        }
    }

    fn estate_funding_observation(balance_cycles: Option<u128>) -> FleetObservation {
        FleetObservation {
            additional_controlled_cycles: BTreeMap::new(),
            canisters: BTreeMap::new(),
            estate_funding_domains: BTreeMap::from([(
                "root".to_string(),
                crate::fleet_ensure::model::EstateFundingDomainObservation {
                    balance_cycles,
                    cycles_ledger: "ledger".to_string(),
                    pool: None,
                    root_principal: Some("rrkah-fqaaa-aaaaa-aaaaq-cai".to_string()),
                },
            )]),
            ledger_fee_cycles: 0,
            operator_cycles: 0,
            protocol_ready: BTreeMap::new(),
        }
    }

    fn terminal_estate_creation_observation() -> FleetObservation {
        let mut observation = estate_funding_observation(Some(0));
        let receipt =
            |byte, block_index| crate::fleet_ensure::model::EstatePoolCreationReceiptObservation {
                block_index,
                operation_id: format!("{byte:02x}").repeat(32),
                cycles_ledger: "ledger".to_string(),
                ledger_amount_cycles: 45,
                ledger_fee_cycles: 5,
                readiness_floor_cycles: 35,
                creation_execution_margin_cycles: 5,
                management_creation_fee_cycles: 5,
                first_observed_cycles: Some(39),
            };
        observation
            .estate_funding_domains
            .get_mut("root")
            .expect("Root funding domain")
            .pool = Some(crate::fleet_ensure::model::EstatePoolInventoryObservation {
            assets: vec![
                crate::fleet_ensure::model::EstatePoolAssetObservation {
                    creation_receipt: Some(receipt(1, 10)),
                    cycles: 35,
                    lifecycle: crate::fleet_ensure::model::EstatePoolAssetLifecycle::Ready,
                    origin: crate::fleet_ensure::model::EstatePoolAssetOrigin::Created,
                    principal: "created-one".to_string(),
                },
                crate::fleet_ensure::model::EstatePoolAssetObservation {
                    creation_receipt: Some(receipt(2, 11)),
                    cycles: 35,
                    lifecycle: crate::fleet_ensure::model::EstatePoolAssetLifecycle::Workload,
                    origin: crate::fleet_ensure::model::EstatePoolAssetOrigin::Created,
                    principal: "created-two".to_string(),
                },
            ],
            maximum_size: 2,
            minimum_size: 0,
            pending_creation: None,
            readiness_floor_cycles: 35,
            creation_execution_margin_cycles: 5,
        });
        observation
    }

    fn estate_creation_receipt(
        observation: &mut FleetObservation,
        asset_index: usize,
    ) -> &mut crate::fleet_ensure::model::EstatePoolCreationReceiptObservation {
        observation
            .estate_funding_domains
            .get_mut("root")
            .expect("Root domain")
            .pool
            .as_mut()
            .expect("pool")
            .assets[asset_index]
            .creation_receipt
            .as_mut()
            .expect("receipt")
    }

    #[test]
    fn terminal_estate_creation_uses_exact_protected_receipts() {
        let plan = estate_funding_plan();
        let domain = &plan.conservation.estate_funding_domains[0];
        let (state, _) = retained_evidence();
        let exact = terminal_estate_creation_observation();
        assert_eq!(
            exact_estate_creation_costs::<std::io::Error>(true, &state, &exact, domain)
                .expect("reconcile exact creation receipts"),
            (100, 20)
        );
        assert!(matches!(
            exact_estate_creation_costs::<std::io::Error>(false, &state, &exact, domain),
            Err(EnsureWorkflowError::JournalIntegrity)
        ));

        let mut drifted = Vec::new();
        let mut changed = exact.clone();
        estate_creation_receipt(&mut changed, 0).ledger_amount_cycles = 44;
        drifted.push(changed);
        let mut changed = exact.clone();
        estate_creation_receipt(&mut changed, 1).operation_id = "01".repeat(32);
        drifted.push(changed);
        let mut changed = exact.clone();
        estate_creation_receipt(&mut changed, 1).block_index = 10;
        drifted.push(changed);
        let mut changed = exact.clone();
        estate_creation_receipt(&mut changed, 0).first_observed_cycles = None;
        drifted.push(changed);
        let mut changed = exact.clone();
        estate_creation_receipt(&mut changed, 0).first_observed_cycles = Some(34);
        drifted.push(changed);
        let mut changed = exact.clone();
        estate_creation_receipt(&mut changed, 0).first_observed_cycles = Some(41);
        drifted.push(changed);
        let mut changed = exact;
        changed
            .estate_funding_domains
            .get_mut("root")
            .expect("Root domain")
            .pool
            .as_mut()
            .expect("pool")
            .assets[0]
            .origin = crate::fleet_ensure::model::EstatePoolAssetOrigin::Imported;
        drifted.push(changed);
        for observation in drifted {
            assert!(matches!(
                exact_estate_creation_costs::<std::io::Error>(true, &state, &observation, domain,),
                Err(EnsureWorkflowError::JournalIntegrity)
            ));
        }
    }

    #[test]
    fn estate_funding_pause_is_exact_and_clears_only_after_sufficient_balance() {
        let plan = estate_funding_plan();
        let (state, mut journal) = retained_evidence();
        let required = estate_funding_requirement::<std::io::Error>(
            &plan,
            &state,
            &estate_funding_observation(Some(40)),
        )
        .expect("validate funding observation")
        .expect("underfunded estate pauses");
        assert_eq!(required.available_cycles, 40);
        assert_eq!(required.maximum_creation_debit_cycles, 100);
        assert_eq!(required.shortfall_cycles, 60);
        assert!(
            estate_funding_requirement::<std::io::Error>(
                &plan,
                &state,
                &estate_funding_observation(Some(100)),
            )
            .expect("validate sufficient funding observation")
            .is_none()
        );

        journal.completion = FleetEnsureCompletion::InProgress;
        journal.effects.clear();
        journal.estate_funding_required = Some(required);
        journal.initial_estate_funding_cycles_by_root = BTreeMap::from([("root".to_string(), 40)]);
        assert!(initial_estate_funding_is_exact(&journal, &plan));
        assert!(estate_funding_pause_is_exact(&journal, &plan, &state));

        journal
            .estate_funding_required
            .as_mut()
            .expect("funding pause")
            .shortfall_cycles = 59;
        assert!(!estate_funding_pause_is_exact(&journal, &plan, &state));

        *journal
            .initial_estate_funding_cycles_by_root
            .get_mut("root")
            .expect("initial Root funding balance") = 41;
        assert!(!initial_estate_funding_is_exact(&journal, &plan));
    }

    #[test]
    fn current_creation_funding_pause_is_bound_to_exact_runtime_evidence() {
        let mut plan = estate_funding_plan();
        plan.conservation.estate_funding_domains[0].cycles_ledger = "aaaaa-aa".to_string();
        let (state, mut journal) = retained_evidence();
        let funding = canic_core::dto::component_provisioning::RootEstateFundingRequired {
            available: Cycles::new(40),
            attempt_count: 2,
            creation_amount: Cycles::new(45),
            cycles_ledger: candid::Principal::from_text("aaaaa-aa").expect("Ledger Principal"),
            execution_margin: Cycles::new(5),
            last_attempt_at_ns: Some(10),
            ledger_fee: Cycles::new(5),
            management_creation_fee: Cycles::new(5),
            operation_id: [17; 32],
            readiness_floor: Cycles::new(35),
            required: Cycles::new(50),
            retry_at_ns: 20,
            root: candid::Principal::from_text("rrkah-fqaaa-aaaaa-aaaaq-cai")
                .expect("Root Principal"),
            shortfall: Cycles::new(10),
        };
        let required =
            protocol_estate_funding_requirement::<std::io::Error>(&plan, &state, &funding)
                .expect("bind runtime funding evidence");
        journal.completion = FleetEnsureCompletion::InProgress;
        journal.effects.clear();
        journal.estate_funding_required = Some(required);
        assert!(estate_funding_pause_is_exact(&journal, &plan, &state));

        journal
            .estate_funding_required
            .as_mut()
            .expect("funding pause")
            .pending_creation_operation_id = Some("AA".repeat(32));
        assert!(!estate_funding_pause_is_exact(&journal, &plan, &state));

        let mut drifted = funding;
        drifted.execution_margin = Cycles::new(6);
        assert!(matches!(
            protocol_estate_funding_requirement::<std::io::Error>(&plan, &state, &drifted),
            Err(EnsureWorkflowError::PlanIntegrity)
        ));
    }

    #[test]
    fn estate_funding_observation_rejects_missing_or_mismatched_authority() {
        let plan = estate_funding_plan();
        let (state, _) = retained_evidence();
        let mut missing_balance = estate_funding_observation(None);
        assert!(matches!(
            estate_funding_requirement::<std::io::Error>(&plan, &state, &missing_balance),
            Err(EnsureWorkflowError::PlanIntegrity)
        ));

        let observed = missing_balance
            .estate_funding_domains
            .get_mut("root")
            .expect("Root funding domain");
        observed.balance_cycles = Some(40);
        observed.root_principal = Some("aaaaa-aa".to_string());
        assert!(matches!(
            estate_funding_requirement::<std::io::Error>(&plan, &state, &missing_balance),
            Err(EnsureWorkflowError::PlanIntegrity)
        ));

        for changed_balance in [39, 101] {
            assert!(matches!(
                estate_funding_requirement::<std::io::Error>(
                    &plan,
                    &state,
                    &estate_funding_observation(Some(changed_balance)),
                ),
                Err(EnsureWorkflowError::DriftedBeforeApply)
            ));
        }
    }

    #[test]
    fn non_creation_protocol_completion_does_not_consume_estate_creation_authority() {
        let mut plan = estate_funding_plan();
        plan.conservation.estate_funding_domains[0].maximum_creation_debit_cycles = 0;
        plan.conservation.estate_funding_domains[0].maximum_creation_fee_cycles = 0;
        plan.conservation.estate_funding_domains[0].maximum_funding_cycles = 0;
        plan.conservation.estate_funding_domains[0].required_creation_count = 0;
        plan.conservation.estate_funding_domains[0].shortfall_cycles = 0;
        let action = EnsureAction::FleetProtocol {
            action: Box::new(
                crate::fleet_ensure::model::CurrentFleetProtocolAction::PrepareStoreChunkSet {
                    request: canic_control_plane::dto::template::TemplateChunkSetPrepareInput {
                        template_id: canic_control_plane::ids::TemplateId::from("root"),
                        version: canic_control_plane::ids::TemplateVersion::from("current"),
                        payload_hash: vec![1; 32],
                        payload_size_bytes: 1,
                        chunk_hashes: vec![vec![2; 32]],
                    },
                },
            ),
            candid: "store.did".to_string(),
            candid_sha256: "11".repeat(32),
            maximum_execution_burn_cycles: 0,
            name: "prepare-store-chunks".to_string(),
            principal: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
        };
        let action_hash = action_sha256(&action);
        plan.protocol_actions.push(action);
        let (state, mut journal) = retained_evidence();
        journal.initial_estate_funding_cycles_by_root = BTreeMap::from([("root".to_string(), 40)]);
        journal.effects = vec![EffectRecord {
            action_sha256: action_hash,
            created_principal: None,
            destination_post_cycles: None,
            destination_pre_cycles: None,
            post_cycles: None,
            pre_cycles: None,
            pre_canister_version: None,
            progress_identity: Some("store chunks prepared".to_string()),
            receipt: Some("receipt".to_string()),
            state: EffectState::Applied,
        }];

        assert!(!component_provisioning_is_applied(&plan, &journal));
        assert_eq!(
            reconcile_estate_funding::<std::io::Error>(
                &plan,
                &journal,
                &state,
                &estate_funding_observation(Some(40)),
            )
            .expect("non-creation protocol preserves the Root Ledger balance"),
            (0, 0),
        );
    }

    #[test]
    fn fresh_estate_funding_binds_the_created_root_from_durable_state() {
        let mut plan = estate_funding_plan();
        plan.conservation.estate_funding_domains[0].available_cycles = None;
        plan.conservation.estate_funding_domains[0].maximum_funding_cycles = 100;
        plan.conservation.estate_funding_domains[0].root_principal = None;
        let (mut state, _) = retained_evidence();
        state.pending_principals.insert(
            "root".to_string(),
            "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
        );

        let required = estate_funding_requirement::<std::io::Error>(
            &plan,
            &state,
            &estate_funding_observation(Some(0)),
        )
        .expect("validate created Root funding authority")
        .expect("fresh underfunded estate pauses");
        assert_eq!(required.shortfall_cycles, 100);
        assert_eq!(required.root_principal, "rrkah-fqaaa-aaaaa-aaaaq-cai");

        state
            .pending_principals
            .insert("root".to_string(), "aaaaa-aa".to_string());
        assert!(matches!(
            estate_funding_requirement::<std::io::Error>(
                &plan,
                &state,
                &estate_funding_observation(Some(0)),
            ),
            Err(EnsureWorkflowError::PlanIntegrity)
        ));
    }

    #[test]
    fn unchanged_estate_funding_pause_does_not_rewrite_its_journal() {
        let root = crate::test_support::temp_dir("estate-funding-pause");
        let paths = EnsurePaths::under(&root, "local", "fleet");
        let plan = estate_funding_plan();
        let (state, mut journal) = retained_evidence();
        journal.completion = FleetEnsureCompletion::InProgress;
        journal.effects.clear();
        journal.initial_estate_funding_cycles_by_root = BTreeMap::from([("root".to_string(), 40)]);
        let required = estate_funding_requirement::<std::io::Error>(
            &plan,
            &state,
            &estate_funding_observation(Some(40)),
        )
        .expect("validate funding observation")
        .expect("underfunded estate pauses");

        assert!(
            retain_estate_funding_pause::<std::io::Error>(&paths, &mut journal, Some(&required),)
                .expect("retain first funding pause")
        );
        let retained = std::fs::read(&paths.journal).expect("read first retained pause");
        assert!(
            !retain_estate_funding_pause::<std::io::Error>(&paths, &mut journal, Some(&required),)
                .expect("replay unchanged funding pause")
        );
        assert_eq!(
            std::fs::read(&paths.journal).expect("reread unchanged retained pause"),
            retained
        );
        assert!(
            retain_estate_funding_pause::<std::io::Error>(&paths, &mut journal, None)
                .expect("clear funded pause")
        );
        assert!(journal.estate_funding_required.is_none());

        std::fs::remove_dir_all(root).expect("remove funding-pause fixture");
    }

    #[test]
    fn estate_funding_is_rechecked_until_the_first_protocol_effect_starts() {
        let protocol_action = || EnsureAction::FleetProtocol {
            action: Box::new(
                crate::fleet_ensure::model::CurrentFleetProtocolAction::PrepareStoreChunkSet {
                    request: canic_control_plane::dto::template::TemplateChunkSetPrepareInput {
                        template_id: canic_control_plane::ids::TemplateId::from("root"),
                        version: canic_control_plane::ids::TemplateVersion::from("current"),
                        payload_hash: vec![1; 32],
                        payload_size_bytes: 1,
                        chunk_hashes: vec![vec![2; 32]],
                    },
                },
            ),
            candid: "store.did".to_string(),
            candid_sha256: "11".repeat(32),
            maximum_execution_burn_cycles: 1,
            name: "prepare".to_string(),
            principal: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
        };
        let first = protocol_action();
        let second = protocol_action();
        let actions = [&first, &second];
        let (_, mut journal) = retained_evidence();
        journal.effects = vec![EffectRecord {
            action_sha256: action_sha256(&first),
            created_principal: None,
            destination_post_cycles: None,
            destination_pre_cycles: None,
            post_cycles: None,
            pre_cycles: None,
            pre_canister_version: None,
            progress_identity: None,
            receipt: None,
            state: EffectState::Intent,
        }];
        assert!(!prior_fleet_protocol_effect_started(&actions, &journal, 1));

        journal.effects[0].state = EffectState::Issued;
        assert!(prior_fleet_protocol_effect_started(&actions, &journal, 1));
    }

    #[test]
    fn terminal_inventory_allows_only_verified_pool_to_component_parent_transition() {
        let asset = "rrkah-fqaaa-aaaaa-aaaaq-cai";
        let old_root = "r7inp-6aaaa-aaaaa-aaabq-cai";
        let hub = "ryjl3-tyaaa-aaaaa-aaaba-cai";
        let state = || {
            let (mut state, _) = retained_evidence();
            state.principals = BTreeMap::from([
                ("asset".to_string(), asset.to_string()),
                ("hub".to_string(), hub.to_string()),
                ("root".to_string(), old_root.to_string()),
            ]);
            state.topology.insert(
                "asset".to_string(),
                crate::fleet_ensure::model::FleetEnsureTopologyRecord {
                    kind: crate::fleet_ensure::model::DesiredCanisterKind::Pool,
                    module_hash: None,
                    parent: Some("root".to_string()),
                    protocol_binding: None,
                    role: Some("canister_pool_asset".to_string()),
                },
            );
            state
        };

        let mut promoted = state();
        merge_terminal_inventory::<std::io::Error>(
            &mut promoted,
            crate::fleet_ensure::ops::TerminalFleetInventory {
                active_registry: None,
                controlled_cycles_by_principal: BTreeMap::new(),
                entries: vec![terminal_component_entry(asset, hub)],
            },
        )
        .expect("verified pool workload assumes its terminal Component parent");
        let promoted = promoted.topology.get("asset").expect("promoted asset");
        assert_eq!(
            promoted.kind,
            crate::fleet_ensure::model::DesiredCanisterKind::Component
        );
        assert_eq!(promoted.parent.as_deref(), Some("hub"));

        let mut already_component = state();
        already_component
            .topology
            .get_mut("asset")
            .expect("retained asset")
            .kind = crate::fleet_ensure::model::DesiredCanisterKind::Component;
        assert!(matches!(
            merge_terminal_inventory::<std::io::Error>(
                &mut already_component,
                crate::fleet_ensure::ops::TerminalFleetInventory {
                    active_registry: None,
                    controlled_cycles_by_principal: BTreeMap::new(),
                    entries: vec![terminal_component_entry(asset, hub)],
                },
            ),
            Err(EnsureWorkflowError::TerminalInventory(
                TerminalInventoryError::RetainedParentConflict {
                    canister,
                    retained_parent,
                    terminal_parent,
                }
            )) if canister == asset
                && retained_parent == "root"
                && terminal_parent == "hub"
        ));

        for incomplete_entry in [
            crate::registry::RegistryEntry {
                module_hash: None,
                ..terminal_component_entry(asset, hub)
            },
            crate::registry::RegistryEntry {
                protocol_binding: None,
                ..terminal_component_entry(asset, hub)
            },
        ] {
            assert!(matches!(
                merge_terminal_inventory::<std::io::Error>(
                    &mut state(),
                    crate::fleet_ensure::ops::TerminalFleetInventory {
                        active_registry: None,
                        controlled_cycles_by_principal: BTreeMap::new(),
                        entries: vec![incomplete_entry],
                    },
                ),
                Err(EnsureWorkflowError::TerminalInventory(
                    TerminalInventoryError::RetainedParentConflict {
                        canister,
                        retained_parent,
                        terminal_parent,
                    }
                )) if canister == asset
                    && retained_parent == "root"
                    && terminal_parent == "hub"
            ));
        }
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
            estate_funding_domains: BTreeMap::new(),
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
            Err(EnsureWorkflowError::TerminalInventory(
                TerminalInventoryError::IdleCycleObservationConflict {
                    canister,
                    expected: 99,
                    observed: 100,
                }
            )) if canister == "controlled-principal"
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
