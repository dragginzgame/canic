//! Compile only explicitly selected infrastructure effects from original supplied custody.
//!
//! No creation is inferred from a missing supplied canister. Pool handoff is a later phase.

#[cfg(test)]
mod tests;

pub(in crate::fleet_ensure) mod registration_recovery;

use crate::fleet_ensure::{
    model::{
        DesiredCanister, DesiredCanisterKind, DesiredFleet, DesiredFleetArtifacts, DesiredPresence,
        EnsureAction, FleetEnsurePlan, FleetEnsurePlanScope, FleetObservation,
        infrastructure_bootstrap::{
            BOOTSTRAP_EFFECT_INSPECTION_ROUNDS, BOOTSTRAP_EFFECT_OBSERVATIONS_PER_ROUND,
            BOOTSTRAP_PHASE_INSPECTION_ROUNDS, BootstrapCoordinatorSelection,
            InfrastructureBootstrapRecord,
        },
    },
    policy::{
        CanisterCyclePolicy, CanisterRuntimeStatus, CycleConservation, EnsurePolicyError,
        FLEET_ENSURE_SCHEMA_VERSION, PlanAccumulator, append_target_funding, canister_cycle_policy,
        checked_add, compile_canister, cycle_bounds, expected_plan_sha256, operation_id,
        resolved_controllers, validate_authority, validate_creation_fee_scope,
        validate_observation_authority, wasm_sha256,
    },
};
use candid::Principal;
use thiserror::Error;

/// A bootstrap review cannot relax physical custody, creation selection or conservation.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum InfrastructureBootstrapError {
    #[error("infrastructure bootstrap authority or Coordinator selection is incomplete")]
    Authority,
    #[error("supplied bootstrap source {name} differs from its exact reviewed custody")]
    Source { name: String },
    #[error(
        "bootstrap target {name} needs {required} cycles before its reviewed effects; available {available}"
    )]
    Headroom {
        name: String,
        required: u128,
        available: u128,
    },
}

/// Compile a finite Coordinator-first initialization plan; no pool or workload effect is admitted.
pub(in crate::fleet_ensure) fn compile(
    desired: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
    record: &InfrastructureBootstrapRecord,
    observation: &FleetObservation,
    desired_sha256: &str,
    time: u64,
) -> Result<FleetEnsurePlan, EnsurePolicyError> {
    validate_authority(desired, &desired.fleet)?;
    validate_observation_authority(desired, observation)?;
    validate_sources(desired, artifacts, record, observation)?;
    let bounds = cycle_bounds(desired)?;
    if bounds.ledger_fee != record.ledger_fee_cycles {
        return Err(EnsurePolicyError::LedgerFeeDrift {
            actual: record.ledger_fee_cycles,
            expected: bounds.ledger_fee,
        });
    }
    let protocol_reserve = protocol_reserve(artifacts, bounds)?;
    let mut accumulator = PlanAccumulator::new();
    accumulator.add_burn(protocol_reserve)?;
    for (index, configured) in ordered_targets(desired).into_iter().enumerate() {
        if configured.kind == DesiredCanisterKind::Pool {
            continue;
        }
        let live = observation.canisters[&configured.name].as_ref();
        let initialize = configured.kind != DesiredCanisterKind::Coordinator
            || record.coordinator != BootstrapCoordinatorSelection::Ready;
        let mut cycle_policy = canister_cycle_policy(configured)?;
        let minimum_cycles = cycle_policy.minimum_cycles;
        cycle_policy.minimum_cycles = 0;
        let action_time =
            time.checked_add(index as u64)
                .ok_or(EnsurePolicyError::ArithmeticOverflow {
                    field: "bootstrap action time",
                })?;
        let mut canister = compile_canister(
            desired,
            artifacts,
            configured,
            live,
            observation,
            cycle_policy,
            bounds,
            action_time,
            initialize && live.is_some_and(|live| live.module_sha256.is_some()),
            &mut accumulator,
        )?;
        include_lifecycle(
            configured,
            live,
            &mut canister,
            initialize,
            bounds.update_burn,
            &mut accumulator,
        )?;
        let reserve = local_reserve(canister.actions.len(), minimum_cycles, bounds)?;
        // Continuation is a shared debit ceiling, not a deposit on every owner.
        // Further writes must pass fresh target-local headroom admission.
        let required = funding_floor(canister.actions.len(), minimum_cycles, bounds)?;
        accumulator.add_burn(reserve.observation_burn)?;
        if let Some(live) = live {
            append_target_funding(
                desired,
                configured,
                live,
                CanisterCyclePolicy {
                    minimum_cycles: required,
                    ..cycle_policy
                },
                bounds,
                action_time,
                false,
                &mut canister.actions,
                &mut accumulator,
            )?;
        } else if cycle_policy.initial_cycles < required {
            return Err(InfrastructureBootstrapError::Headroom {
                name: configured.name.clone(),
                required,
                available: cycle_policy.initial_cycles,
            }
            .into());
        }
        canister.actions.sort_by_key(effect_order);
        accumulator.canisters.push(canister);
    }
    finish(
        desired,
        artifacts,
        record,
        desired_sha256,
        time,
        accumulator,
    )
}

fn protocol_reserve(
    artifacts: &DesiredFleetArtifacts,
    bounds: crate::fleet_ensure::policy::CycleBounds,
) -> Result<u128, EnsurePolicyError> {
    let continuation = artifacts
        .continuation
        .as_ref()
        .ok_or(InfrastructureBootstrapError::Authority)?;
    let protocol_steps = usize::try_from(
        u64::from(continuation.maximum_successor_actions)
            + u64::from(continuation.fixture_publication_retry_attempts),
    )
    .map_err(|_| InfrastructureBootstrapError::Authority)?;
    Ok(local_reserve(protocol_steps, 0, bounds)?.required)
}

fn ordered_targets(desired: &DesiredFleet) -> Vec<&DesiredCanister> {
    let mut targets = desired.canisters.iter().collect::<Vec<_>>();
    targets.sort_by_key(|configured| match configured.kind {
        DesiredCanisterKind::Coordinator => 0,
        DesiredCanisterKind::Root => 1,
        DesiredCanisterKind::Store => 2,
        _ => 3,
    });
    targets
}

fn include_lifecycle(
    configured: &DesiredCanister,
    live: Option<&crate::fleet_ensure::model::LiveCanister>,
    canister: &mut crate::fleet_ensure::model::CanisterPlan,
    initialize: bool,
    update_burn: u128,
    accumulator: &mut PlanAccumulator,
) -> Result<(), EnsurePolicyError> {
    if initialize && let Some(live) = live {
        if live.module_sha256.is_some() {
            canister.actions.push(EnsureAction::Uninstall {
                name: configured.name.clone(),
                principal: live.principal.clone(),
            });
            accumulator.add_burn(update_burn)?;
            for action in &mut canister.actions {
                if let EnsureAction::Install { mode, .. } = action {
                    *mode = crate::fleet_ensure::model::InstallMode::Install;
                }
            }
        }
        if live.status != CanisterRuntimeStatus::Stopped {
            canister.actions.insert(
                0,
                EnsureAction::Stop {
                    name: configured.name.clone(),
                    principal: live.principal.clone(),
                },
            );
            accumulator.add_burn(update_burn)?;
        }
        if !canister
            .actions
            .iter()
            .any(|action| matches!(action, EnsureAction::Start { .. }))
        {
            canister.actions.push(EnsureAction::Start {
                name: configured.name.clone(),
                principal: live.principal.clone(),
            });
            accumulator.add_burn(update_burn)?;
        }
    }
    Ok(())
}

/// Native headroom includes finite inspection retries before funding or completion.
struct BootstrapReserve {
    required: u128,
    observation_burn: u128,
}

/// Fund installation and one observation window; unused retry ceilings are not payments.
fn funding_floor(
    actions: usize,
    minimum: u128,
    bounds: crate::fleet_ensure::policy::CycleBounds,
) -> Result<u128, EnsurePolicyError> {
    let updates = bounds.update_burn.checked_mul(actions as u128 + 1).ok_or(
        EnsurePolicyError::ArithmeticOverflow {
            field: "bootstrap funded updates",
        },
    )?;
    let observations = observation_window(bounds)?;
    checked_add(
        minimum,
        checked_add(updates, observations, "bootstrap funded work")?,
        "bootstrap funding floor",
    )
}

fn observation_window(
    bounds: crate::fleet_ensure::policy::CycleBounds,
) -> Result<u128, EnsurePolicyError> {
    // Fund all permitted retries of the next effect, plus both rounds of
    // review, apply, registration and terminal inspection. Do not multiply
    // this complete window by every hypothetical future protocol action.
    let count = u128::from(BOOTSTRAP_EFFECT_INSPECTION_ROUNDS)
        * u128::from(BOOTSTRAP_EFFECT_OBSERVATIONS_PER_ROUND)
        + 4 * u128::from(BOOTSTRAP_PHASE_INSPECTION_ROUNDS);
    bounds
        .observation_burn
        .checked_mul(count)
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "bootstrap funded observations",
        })
}

/// Upper funding forecast for retained infrastructure before replacement bytes exist.
/// Five effects cover stop, uninstall, install, controller reconciliation and start.
pub(in crate::fleet_ensure) fn forecast_target_funding(
    minimum: u128,
    available: u128,
    observation_burn: u128,
    update_burn: u128,
) -> Result<u128, EnsurePolicyError> {
    let bounds = crate::fleet_ensure::policy::CycleBounds {
        ledger_fee: 0,
        management_creation_fee: 0,
        material_threshold: 0,
        observation_burn,
        update_burn,
    };
    let required = funding_floor(5, minimum, bounds)?;
    if available >= required {
        return Ok(0);
    }
    checked_add(
        required - available,
        super::target_funding_margin(5, bounds)?,
        "bootstrap funding forecast",
    )
}

/// Admit a write only against its own current native balance, preserving its configured floor.
pub(in crate::fleet_ensure) fn validate_effect_headroom(
    plan: &FleetEnsurePlan,
    action: &EnsureAction,
    available: Option<u128>,
) -> Result<(), EnsurePolicyError> {
    if plan.infrastructure_bootstrap.is_none()
        || matches!(
            action,
            EnsureAction::Create { .. } | EnsureAction::Fund { .. }
        )
    {
        return Ok(());
    }
    let desired = plan
        .reviewed_desired
        .as_ref()
        .ok_or(InfrastructureBootstrapError::Authority)?
        .desired();
    let configured = desired
        .canisters
        .iter()
        .find(|configured| match action {
            EnsureAction::FleetProtocol {
                principal, action, ..
            } => {
                configured.kind == action.target_kind()
                    && (configured.principal.as_deref() == Some(principal.as_str())
                        || (configured.kind == DesiredCanisterKind::Coordinator
                            && configured.principal.is_none()))
            }
            _ => configured.name == action.name(),
        })
        .ok_or(InfrastructureBootstrapError::Authority)?;
    let bounds = cycle_bounds(desired)?;
    let minimum = canister_cycle_policy(configured)?.minimum_cycles;
    let required = checked_add(
        minimum,
        checked_add(
            observation_window(bounds)?,
            bounds.update_burn,
            "bootstrap next effect window",
        )?,
        "bootstrap effect headroom",
    )?;
    let available = available.ok_or(InfrastructureBootstrapError::Authority)?;
    if available < required {
        return Err(InfrastructureBootstrapError::Headroom {
            name: configured.name.clone(),
            required,
            available,
        }
        .into());
    }
    Ok(())
}

fn local_reserve(
    actions: usize,
    minimum: u128,
    bounds: crate::fleet_ensure::policy::CycleBounds,
) -> Result<BootstrapReserve, EnsurePolicyError> {
    // Include a possible funding action. Eight effect rounds cover at most
    // three reads each, followed by two terminal inspections per effect.
    let count = actions as u128 + 1;
    let observations = count.checked_mul(26).and_then(|n| n.checked_add(8)).ok_or(
        EnsurePolicyError::ArithmeticOverflow {
            field: "bootstrap observations",
        },
    )?;
    let observation_burn = bounds.observation_burn.checked_mul(observations).ok_or(
        EnsurePolicyError::ArithmeticOverflow {
            field: "bootstrap observation debit",
        },
    )?;
    let updates =
        bounds
            .update_burn
            .checked_mul(count)
            .ok_or(EnsurePolicyError::ArithmeticOverflow {
                field: "bootstrap effect debit",
            })?;
    let reserve = checked_add(observation_burn, updates, "bootstrap local reserve")?;
    Ok(BootstrapReserve {
        required: checked_add(minimum, reserve, "bootstrap retained floor")?,
        observation_burn,
    })
}

const fn effect_order(action: &EnsureAction) -> u8 {
    match action {
        EnsureAction::Create { .. } | EnsureAction::Fund { .. } => 0,
        EnsureAction::Stop { .. } => 1,
        EnsureAction::Uninstall { .. } => 2,
        EnsureAction::Install { .. } => 3,
        EnsureAction::Start { .. } => 4,
        _ => 5,
    }
}

fn finish(
    desired: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
    record: &InfrastructureBootstrapRecord,
    desired_sha256: &str,
    time: u64,
    accumulator: PlanAccumulator,
) -> Result<FleetEnsurePlan, EnsurePolicyError> {
    validate_creation_fee_scope(desired, &accumulator.canisters, &[])?;
    let reserved = accumulator
        .canisters
        .iter()
        .try_fold(0, |total, canister| {
            checked_add(
                total,
                record
                    .sources
                    .get(&canister.name)
                    .map_or(0, |source| source.sample.reserved_cycles),
                "bootstrap reserved cycles",
            )
        })?;
    let controlled = checked_add(
        accumulator.retained,
        reserved,
        "bootstrap controlled cycles",
    )?;
    let available = checked_add(
        controlled,
        accumulator.new_funding,
        "bootstrap available cycles",
    )?;
    let floors = desired
        .canisters
        .iter()
        .filter(|target| target.kind != DesiredCanisterKind::Pool)
        .try_fold(reserved, |total, target| {
            checked_add(
                total,
                canister_cycle_policy(target)?.minimum_cycles,
                "bootstrap retained floors",
            )
        })?;
    let spendable = available
        .checked_sub(floors)
        .ok_or(InfrastructureBootstrapError::Authority)?;
    let execution_burn = accumulator.execution_burn.min(spendable);
    let retained = available - execution_burn;
    let mut plan = FleetEnsurePlan {
        infrastructure_bootstrap: Some(Box::new(record.clone())),
        continuation: artifacts.continuation.clone(),
        canisters: accumulator.canisters,
        conservation: CycleConservation {
            estate_funding_domains: Vec::new(),
            expected_post_operation_cycles: retained,
            maximum_execution_burn_cycles: execution_burn,
            maximum_new_funding_cycles: accumulator.new_funding,
            maximum_operator_debit_cycles: checked_add(
                accumulator.new_funding,
                accumulator.fees,
                "bootstrap operator debit",
            )?,
            maximum_unavoidable_fee_cycles: accumulator.fees,
            observed_controlled_cycles: controlled,
            retained_in_reused_canisters_cycles: controlled,
            scheduled_transfer_cycles: 0,
        },
        desired_sha256: desired_sha256.to_string(),
        environment: desired.environment.clone(),
        fleet: desired.fleet.clone(),
        operation_id: operation_id(desired_sha256, &desired.environment, &desired.fleet),
        plan_sha256: String::new(),
        planned_at_time: time,
        protocol_actions: Vec::new(),
        recovery_review: None,
        reinstall: None,
        root_reinstall_bindings: Vec::new(),
        root_start_authority: None,
        reviewed_desired: Some(Box::new(
            crate::fleet_ensure::model::ReviewedDesiredFleetRecord::capture(desired),
        )),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        scope: FleetEnsurePlanScope::InfrastructureBootstrap,
        terminal_inventory_operation_id: None,
    };
    plan.plan_sha256 = expected_plan_sha256(&plan);
    Ok(plan)
}

fn validate_sources(
    desired: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
    record: &InfrastructureBootstrapRecord,
    observation: &FleetObservation,
) -> Result<(), EnsurePolicyError> {
    let invalid = || InfrastructureBootstrapError::Authority;
    let bootstrap = desired.bootstrap.as_ref().ok_or_else(invalid)?;
    if record.schema_version != 1
        || record.source_sha256 == [0; 32]
        || record.network_root_key_sha256 == [0; 32]
        || Principal::from_text(&desired.operator).ok() != Some(record.operator)
        || bootstrap.fresh_estate
        || bootstrap.roots.is_empty()
        || record.sources.len()
            + record.held_sources.len()
            + usize::from(record.coordinator == BootstrapCoordinatorSelection::Create)
            != desired.canisters.len()
        || (record.coordinator == BootstrapCoordinatorSelection::Ready)
            != record
                .coordinator_registry_candid_hex
                .as_ref()
                .is_some_and(|text| !text.is_empty())
    {
        return Err(invalid().into());
    }
    for root in &bootstrap.roots {
        if root
            .capacity_import_bootstrap
            .as_ref()
            .is_none_or(|hold| hold.review_sha256 != record.source_sha256)
        {
            return Err(invalid().into());
        }
    }
    for configured in &desired.canisters {
        if configured.presence != DesiredPresence::Present
            || configured.replace
            || !matches!(
                configured.kind,
                DesiredCanisterKind::Coordinator
                    | DesiredCanisterKind::Root
                    | DesiredCanisterKind::Store
                    | DesiredCanisterKind::Pool
            )
        {
            return Err(invalid().into());
        }
        if configured.kind == DesiredCanisterKind::Coordinator
            && record.coordinator == BootstrapCoordinatorSelection::Create
        {
            if configured.principal.is_some()
                || observation.canisters[&configured.name].is_some()
                || record.sources.contains_key(&configured.name)
            {
                return Err(invalid().into());
            }
            continue;
        }
        if let Some(source) = record.held_sources.get(&configured.name) {
            validate_held_source(desired, configured, source)?;
            if record.sources.contains_key(&configured.name)
                || observation.canisters[&configured.name].is_some()
            {
                return Err(invalid().into());
            }
            continue;
        }
        validate_source(configured, record)?;
        if configured.kind == DesiredCanisterKind::Coordinator
            && record.coordinator == BootstrapCoordinatorSelection::Ready
        {
            let live = observation.canisters[&configured.name]
                .as_ref()
                .ok_or_else(invalid)?;
            let mut actual = live.controllers.clone();
            actual.sort();
            if live.module_sha256.as_ref() != Some(&wasm_sha256(artifacts, &configured.name)?)
                || actual != resolved_controllers(configured, observation)?
                || live.status != CanisterRuntimeStatus::Running
            {
                return Err(invalid().into());
            }
        }
    }
    Ok(())
}

fn validate_source(
    configured: &DesiredCanister,
    record: &InfrastructureBootstrapRecord,
) -> Result<(), InfrastructureBootstrapError> {
    let invalid = || InfrastructureBootstrapError::Source {
        name: configured.name.clone(),
    };
    let source = record.sources.get(&configured.name).ok_or_else(invalid)?;
    let binding = &source.sample.binding;
    let id = configured
        .principal
        .as_deref()
        .and_then(|id| Principal::from_text(id).ok());
    let subnet = Principal::from_text(&configured.subnet).ok();
    if id != Some(binding.canister_id)
        || binding.canister_id == Principal::anonymous()
        || binding.canister_id == Principal::management_canister()
        || subnet != Some(binding.subnet.into_principal())
        || !binding.controllers.contains(&record.operator)
        || binding.snapshots_size_bytes != 0
        || (configured.kind == DesiredCanisterKind::Pool
            && binding.module_sha256.is_some()
            && !binding.stopped)
        || !binding.controllers.windows(2).all(|pair| pair[0] < pair[1])
    {
        return Err(invalid());
    }
    Ok(())
}

fn validate_held_source(
    desired: &DesiredFleet,
    configured: &DesiredCanister,
    source: &crate::fleet_ensure::model::infrastructure_bootstrap::InfrastructureBootstrapHeldSourceRecord,
) -> Result<(), InfrastructureBootstrapError> {
    let binding = &source.custody;
    let parent = desired
        .canisters
        .iter()
        .find(|entry| Some(&entry.name) == configured.parent.as_ref());
    let parent_matches = parent.is_some_and(|entry| {
        entry.kind == DesiredCanisterKind::Root
            && entry
                .principal
                .as_deref()
                .and_then(|id| Principal::from_text(id).ok())
                == Some(source.root)
            && entry.subnet == configured.subnet
    });
    let custody_matches = configured
        .principal
        .as_deref()
        .and_then(|id| Principal::from_text(id).ok())
        == Some(binding.canister)
        && Principal::from_text(&configured.subnet).ok() == Some(binding.subnet.into_principal())
        && binding.controllers.contains(&source.root)
        && binding.controllers.len() <= 10
        && binding.controllers.windows(2).all(|pair| pair[0] < pair[1]);
    if configured.kind != DesiredCanisterKind::Pool || !parent_matches || !custody_matches {
        return Err(InfrastructureBootstrapError::Source {
            name: configured.name.clone(),
        });
    }
    Ok(())
}
