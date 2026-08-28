//! Module: fleet_ensure::policy
//!
//! Responsibility: validate current desired state and compile one conservative convergence plan.
//! Does not own: storage, clocks, transport, live observation, or effects.
//! Boundary: workflow supplies exact desired/live inputs and persists the returned immutable plan.

use crate::fleet_ensure::model::{
    CanisterCyclePolicy, CanisterDisposition, CanisterPlan, CanisterRuntimeStatus,
    CurrentFleetProtocolAction, CycleConservation, DesiredCanisterKind, DesiredFleet,
    DesiredFleetArtifacts, DesiredPresence, EnsureAction, FLEET_ENSURE_SCHEMA_VERSION,
    FleetEnsurePlan, FleetObservation, InstallMode, LiveCanister, MAX_FLEET_ENSURE_CANISTERS,
    MAX_FLEET_ENSURE_PROTOCOL_STEPS, RootOwnedCanisterLifecycle,
};
use candid::Principal;
use canic_core::ids::FleetName;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use thiserror::Error as ThisError;

/// Pure current-contract plan compilation failure.

#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum EnsurePolicyError {
    #[error("cycle arithmetic overflow while compiling {field}")]
    ArithmeticOverflow { field: &'static str },

    #[error("controlled canister {name} has duplicate name or principal authority")]
    DuplicateAuthority { name: String },

    #[error("desired Fleet has invalid or anonymous Principal for {field}: {value}")]
    InvalidPrincipal { field: String, value: String },

    #[error("desired Fleet contains an empty canister or drain-method name")]
    EmptyName,

    #[error("desired Fleet has unsafe {field} path label: {value}")]
    UnsafePathLabel { field: &'static str, value: String },

    #[error("controlled canister {name} has invalid {field} cycle value: {value}")]
    InvalidCanisterCycles {
        field: &'static str,
        name: String,
        value: String,
    },

    #[error("desired Fleet has invalid {field} cycle value: {value}")]
    InvalidFleetCycles { field: &'static str, value: String },

    #[error("live Cycles Ledger fee is {actual}, but reviewed desired fee is {expected}")]
    LedgerFeeDrift { actual: u128, expected: u128 },

    #[error("desired Fleet has {actual} controlled canisters, above limit {maximum}")]
    TooManyCanisters { actual: usize, maximum: usize },

    #[error("desired Fleet has {actual} protocol steps, above limit {maximum}")]
    TooManyProtocolSteps { actual: usize, maximum: usize },

    #[error("desired Fleet maximum_stalled_observations must be greater than zero")]
    InvalidStallBound,

    #[error("controlled canister {name} initial cycles are below its minimum cycles")]
    InitialCyclesBelowMinimum { name: String },

    #[error(
        "live observation for controlled canister {name} returned Principal {actual}, expected {expected}"
    )]
    ObservationAuthorityMismatch {
        actual: String,
        expected: String,
        name: String,
    },

    #[error("desired Fleet name {actual} does not match requested Fleet {expected}")]
    FleetMismatch { actual: String, expected: String },

    #[error("controlled canister {name} is missing exact live observation authority")]
    MissingObservation { name: String },

    #[error("Fleet treasury {treasury} is not an explicitly controlled present canister")]
    MissingTreasury { treasury: String },

    #[error("Fleet treasury {treasury} must be reused and cannot be replaced")]
    TreasuryReplacement { treasury: String },

    #[error(
        "present controlled canister {name} does not retain the configured operator as controller"
    )]
    MissingOperatorController { name: String },

    #[error(
        "controlled canister {name} retains {cycles} transferable cycles but has no exact treasury-bound drain authority; canister left untouched"
    )]
    NoSafeDrain { name: String, cycles: u128 },

    #[error("controlled canister {name} names a drain destination other than Fleet treasury")]
    WrongDrainDestination { name: String },

    #[error("desired Fleet schema {actual} is unsupported; current schema is {expected}")]
    WrongSchema { actual: u16, expected: u16 },

    #[error("controlled canister {name} has no resolved current Wasm identity")]
    MissingWasmIdentity { name: String },

    #[error("controlled canister {name} has no resolved {kind} identity")]
    MissingArtifactIdentity { kind: &'static str, name: String },

    #[error("protocol step {step} targets unknown or absent controlled canister {canister}")]
    InvalidProtocolTarget { canister: String, step: String },

    #[error("protocol step {0} has duplicate or empty authority")]
    InvalidProtocolStep(String),

    #[error("protocol step {0} is missing an exact live terminal observation")]
    MissingProtocolObservation(String),

    #[error("controlled canister {0} must configure init_arg and init_candid together")]
    IncompleteInitTemplate(String),

    #[error("Canic infrastructure canister {name} requires one exact typed bootstrap initializer")]
    MissingCanicInitializer { name: String },

    #[error("Canic infrastructure canister {name} has conflicting typed and generic initializers")]
    ConflictingCanicInitializer { name: String },

    #[error("retained pool asset {name} is missing; no replacement was created")]
    MissingPoolAsset { name: String },

    #[error("retained pool asset {name} has no exact Root-owned lifecycle observation")]
    MissingPoolLifecycle { name: String },

    #[error("retained pool evidence cannot be fenced by an exact reinstall of {name}")]
    RecoveryReinstallUnavailable { name: String },

    #[error("Root-owned canister {name} is still awaiting exact current balance observation")]
    PendingRootOwnedBalance { name: String },

    #[error("desired Fleet topology is invalid for {name}: {reason}")]
    InvalidTopology { name: String, reason: &'static str },
}

#[derive(Clone, Copy)]
struct CycleBounds {
    ledger_fee: u128,
    management_creation_fee: u128,
    material_threshold: u128,
    observation_burn: u128,
    update_burn: u128,
}

struct PlanAccumulator {
    canisters: Vec<CanisterPlan>,
    execution_burn: u128,
    fees: u128,
    new_funding: u128,
    retained: u128,
    transfers: u128,
}

impl PlanAccumulator {
    const fn new() -> Self {
        Self {
            canisters: Vec::new(),
            execution_burn: 0,
            fees: 0,
            new_funding: 0,
            retained: 0,
            transfers: 0,
        }
    }

    fn add_burn(&mut self, value: u128) -> Result<(), EnsurePolicyError> {
        self.execution_burn = checked_add(self.execution_burn, value, "execution burn")?;
        Ok(())
    }

    fn add_fee(&mut self, value: u128) -> Result<(), EnsurePolicyError> {
        self.fees = checked_add(self.fees, value, "unavoidable fees")?;
        Ok(())
    }

    fn add_funding(&mut self, value: u128) -> Result<(), EnsurePolicyError> {
        self.new_funding = checked_add(self.new_funding, value, "new funding")?;
        Ok(())
    }
}

/// Compile one immutable reviewed plan from current desired state and exact live observation.
#[expect(
    clippy::too_many_lines,
    reason = "one pure compiler keeps the complete reviewed conservation and action authority visible"
)]
pub fn compile_plan(
    desired: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
    compiled_protocol_actions: &[EnsureAction],
    desired_sha256: &str,
    requested_fleet: &str,
    observation: &FleetObservation,
    created_at_time: u64,
) -> Result<FleetEnsurePlan, EnsurePolicyError> {
    validate_authority(desired, requested_fleet)?;
    validate_observation_authority(desired, observation)?;
    let bounds = cycle_bounds(desired)?;
    if observation.ledger_fee_cycles != bounds.ledger_fee {
        return Err(EnsurePolicyError::LedgerFeeDrift {
            actual: observation.ledger_fee_cycles,
            expected: bounds.ledger_fee,
        });
    }
    let operation_id = operation_id(desired_sha256, &desired.environment, requested_fleet);
    let mut accumulator = PlanAccumulator::new();
    let recovery_reinstalls = recovery_reinstall_canisters(desired, observation)?;

    for (index, configured) in desired.canisters.iter().enumerate() {
        let observed = observation
            .canisters
            .get(&configured.name)
            .ok_or_else(|| EnsurePolicyError::MissingObservation {
                name: configured.name.clone(),
            })?
            .as_ref();
        let cycle_policy = canister_cycle_policy(configured)?;
        let action_time = created_at_time
            .checked_add(u64::try_from(index).unwrap_or(u64::MAX))
            .ok_or(EnsurePolicyError::ArithmeticOverflow {
                field: "action timestamp",
            })?;
        let plan = compile_canister(
            desired,
            artifacts,
            configured,
            observed,
            cycle_policy,
            bounds,
            action_time,
            recovery_reinstalls.contains(&configured.name),
            &mut accumulator,
        )?;
        accumulator.canisters.push(plan);
    }
    for name in &recovery_reinstalls {
        let has_exact_reinstall = accumulator.canisters.iter().any(|canister| {
            canister.name == *name
                && canister.actions.iter().any(|action| {
                    matches!(
                        action,
                        EnsureAction::Install {
                            mode: InstallMode::Reinstall,
                            ..
                        }
                    )
                })
        });
        if !has_exact_reinstall {
            return Err(EnsurePolicyError::RecoveryReinstallUnavailable { name: name.clone() });
        }
    }

    let mut protocol_actions = Vec::new();
    let mut protocol_names = BTreeSet::new();
    for step in &desired.protocol_steps {
        let ready = observation
            .protocol_ready
            .get(&step.name)
            .copied()
            .ok_or_else(|| EnsurePolicyError::MissingProtocolObservation(step.name.clone()))?;
        if ready {
            continue;
        }
        if !protocol_names.insert(step.name.as_str()) {
            return Err(EnsurePolicyError::InvalidProtocolStep(step.name.clone()));
        }
        let configured = desired
            .canisters
            .iter()
            .find(|canister| canister.name == step.canister)
            .ok_or_else(|| EnsurePolicyError::InvalidProtocolTarget {
                canister: step.canister.clone(),
                step: step.name.clone(),
            })?;
        let principal = observation
            .canisters
            .get(&step.canister)
            .and_then(Option::as_ref)
            .map_or_else(
                || format!("created:{}", step.canister),
                |live| live.principal.clone(),
            );
        if configured.presence != DesiredPresence::Present {
            return Err(EnsurePolicyError::InvalidProtocolTarget {
                canister: step.canister.clone(),
                step: step.name.clone(),
            });
        }
        let identities = artifacts.protocol_by_step.get(&step.name).ok_or_else(|| {
            EnsurePolicyError::MissingArtifactIdentity {
                kind: "protocol contract",
                name: step.name.clone(),
            }
        })?;
        let maximum_execution_burn_cycles = parse_canister_cycles(
            &step.canister,
            "protocol.maximum_execution_burn_cycles",
            &step.maximum_execution_burn_cycles,
        )?;
        accumulator.add_burn(maximum_execution_burn_cycles)?;
        protocol_actions.push(EnsureAction::Protocol {
            candid: step.candid.clone(),
            candid_sha256: identities.candid_sha256.clone(),
            command_args: step.command_args.clone(),
            command_args_sha256: identities.command_args_sha256.clone(),
            command_method: step.command_method.clone(),
            expected_status: step.expected_status.clone(),
            expected_status_sha256: identities.expected_status_sha256.clone(),
            maximum_execution_burn_cycles,
            name: step.name.clone(),
            principal,
            status_args: step.status_args.clone(),
            status_args_sha256: identities.status_args_sha256.clone(),
            status_method: step.status_method.clone(),
        });
    }
    let expected_typed_protocol = desired.protocol.as_ref();
    for action in compiled_protocol_actions {
        let EnsureAction::FleetProtocol {
            action: current_action,
            candid,
            maximum_execution_burn_cycles,
            name,
            principal,
            ..
        } = action
        else {
            return Err(EnsurePolicyError::InvalidProtocolStep(
                action.name().to_string(),
            ));
        };
        let target_kind = current_action.target_kind();
        let target_is_exact = desired.canisters.iter().any(|canister| {
            canister.kind == target_kind
                && canister.presence == DesiredPresence::Present
                && observation
                    .canisters
                    .get(&canister.name)
                    .and_then(Option::as_ref)
                    .is_some_and(|live| live.principal == *principal)
        });
        let operation_matches = match current_action.as_ref() {
            CurrentFleetProtocolAction::ProvisionComponents { request, .. } => {
                canic_core::cdk::utils::hash::decode_hex(&operation_id)
                    .ok()
                    .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
                    .is_some_and(|operation_id| request.operation_id == operation_id)
            }
            current_action => current_action
                .operation_id()
                .is_none_or(|identity| identity != [0; 32]),
        };
        let protocol_matches = expected_typed_protocol.is_some_and(|protocol| {
            let expected_candid = match target_kind {
                DesiredCanisterKind::Coordinator => protocol.coordinator_candid.as_str(),
                DesiredCanisterKind::Root => protocol.root_candid.as_str(),
                DesiredCanisterKind::Store => protocol.store_candid.as_str(),
                DesiredCanisterKind::Auxiliary
                | DesiredCanisterKind::Component
                | DesiredCanisterKind::Pool => return false,
            };
            candid == expected_candid
        });
        if name.is_empty()
            || !protocol_names.insert(name.as_str())
            || !target_is_exact
            || !operation_matches
            || !protocol_matches
        {
            return Err(EnsurePolicyError::InvalidProtocolStep(name.clone()));
        }
        accumulator.add_burn(*maximum_execution_burn_cycles)?;
        protocol_actions.push(action.clone());
    }
    if protocol_actions.len() > MAX_FLEET_ENSURE_PROTOCOL_STEPS {
        return Err(EnsurePolicyError::TooManyProtocolSteps {
            actual: protocol_actions.len(),
            maximum: MAX_FLEET_ENSURE_PROTOCOL_STEPS,
        });
    }
    if recovery_reinstalls.is_empty()
        && protocol_actions.is_empty()
        && let Some(name) = desired.canisters.iter().find_map(|configured| {
            observation
                .canisters
                .get(&configured.name)
                .and_then(Option::as_ref)
                .is_some_and(|live| {
                    live.root_owned_lifecycle == Some(RootOwnedCanisterLifecycle::Retained)
                })
                .then(|| configured.name.clone())
        })
    {
        return Err(EnsurePolicyError::PendingRootOwnedBalance { name });
    }

    let observation_count = maximum_observation_count(
        desired,
        &accumulator.canisters,
        &protocol_actions,
        observation.additional_controlled_cycles.len(),
    )?;
    accumulator.add_burn(
        bounds
            .observation_burn
            .checked_mul(observation_count)
            .ok_or(EnsurePolicyError::ArithmeticOverflow {
                field: "observation burn",
            })?,
    )?;

    let observed_controlled_cycles = observation
        .canisters
        .values()
        .filter_map(Option::as_ref)
        .try_fold(0_u128, |total, live| {
            checked_add(total, live.cycles, "observed controlled cycles")
        })?;
    let observed_controlled_cycles = observation
        .additional_controlled_cycles
        .values()
        .try_fold(observed_controlled_cycles, |total, cycles| {
            checked_add(total, *cycles, "observed controlled cycles")
        })?;
    let maximum_operator_debit_cycles = checked_add(
        accumulator.new_funding,
        accumulator.fees,
        "maximum operator debit",
    )?;
    if observation.operator_cycles < maximum_operator_debit_cycles {
        // Funding sufficiency is deliberately enforced by apply after a reviewed plan is loaded.
        // The plan remains useful and truthful even when the current account is insufficient.
    }
    let expected_post_operation_cycles = observed_controlled_cycles
        .checked_add(maximum_operator_debit_cycles)
        .and_then(|value| value.checked_sub(accumulator.fees))
        .and_then(|value| value.checked_sub(accumulator.execution_burn))
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "cycle conservation equation",
        })?;
    let conservation = CycleConservation {
        expected_post_operation_cycles,
        maximum_execution_burn_cycles: accumulator.execution_burn,
        maximum_new_funding_cycles: accumulator.new_funding,
        maximum_operator_debit_cycles,
        maximum_unavoidable_fee_cycles: accumulator.fees,
        observed_controlled_cycles,
        retained_in_reused_canisters_cycles: accumulator.retained,
        scheduled_transfer_cycles: accumulator.transfers,
    };
    let mut plan = FleetEnsurePlan {
        canisters: accumulator.canisters,
        conservation,
        desired_sha256: desired_sha256.to_string(),
        environment: desired.environment.clone(),
        fleet: requested_fleet.to_string(),
        operation_id,
        plan_sha256: String::new(),
        planned_at_time: created_at_time,
        protocol_actions,
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
    };
    plan.plan_sha256 = expected_plan_sha256(&plan);
    Ok(plan)
}

/// Validate identity labels before they are used to construct operator-state paths.
pub(crate) fn validate_path_identity(
    desired: &DesiredFleet,
    requested_fleet: &str,
) -> Result<(), EnsurePolicyError> {
    if desired.schema_version != FLEET_ENSURE_SCHEMA_VERSION {
        return Err(EnsurePolicyError::WrongSchema {
            actual: desired.schema_version,
            expected: FLEET_ENSURE_SCHEMA_VERSION,
        });
    }
    if desired.fleet != requested_fleet {
        return Err(EnsurePolicyError::FleetMismatch {
            actual: desired.fleet.clone(),
            expected: requested_fleet.to_string(),
        });
    }
    requested_fleet
        .parse::<FleetName>()
        .map_err(|_| EnsurePolicyError::UnsafePathLabel {
            field: "fleet",
            value: requested_fleet.to_string(),
        })?;
    if desired.environment.is_empty()
        || desired.environment.len() > 64
        || !desired
            .environment
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(EnsurePolicyError::UnsafePathLabel {
            field: "environment",
            value: desired.environment.clone(),
        });
    }
    Ok(())
}

#[expect(
    clippy::too_many_arguments,
    reason = "pure compiler receives one complete authority tuple"
)]
fn compile_canister(
    desired: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
    configured: &crate::fleet_ensure::model::DesiredCanister,
    observed: Option<&LiveCanister>,
    cycle_policy: CanisterCyclePolicy,
    bounds: CycleBounds,
    created_at_time: u64,
    force_reinstall: bool,
    accumulator: &mut PlanAccumulator,
) -> Result<CanisterPlan, EnsurePolicyError> {
    match (configured.presence, observed) {
        (DesiredPresence::Absent, None) => Ok(CanisterPlan {
            actions: Vec::new(),
            disposition: CanisterDisposition::Delete,
            name: configured.name.clone(),
            observed_cycles: 0,
            principal: configured.principal.clone(),
        }),
        (DesiredPresence::Absent, Some(live)) => retire_plan(
            desired,
            artifacts,
            configured,
            live,
            bounds,
            CanisterDisposition::Delete,
            accumulator,
        ),
        (DesiredPresence::Present, None) => {
            if configured.kind == DesiredCanisterKind::Pool {
                return Err(EnsurePolicyError::MissingPoolAsset {
                    name: configured.name.clone(),
                });
            }
            if configured.principal.is_some() {
                return Err(EnsurePolicyError::MissingObservation {
                    name: configured.name.clone(),
                });
            }
            create_plan(
                desired,
                artifacts,
                configured,
                cycle_policy,
                bounds,
                created_at_time,
                CanisterDisposition::Create,
                accumulator,
                None,
            )
        }
        (DesiredPresence::Present, Some(live))
            if configured.replace
                && configured.principal.as_deref() == Some(live.principal.as_str()) =>
        {
            create_plan(
                desired,
                artifacts,
                configured,
                cycle_policy,
                bounds,
                created_at_time,
                CanisterDisposition::Replace,
                accumulator,
                Some(live),
            )
        }
        (DesiredPresence::Present, Some(live)) => reuse_plan(
            desired,
            artifacts,
            configured,
            live,
            cycle_policy,
            bounds,
            created_at_time,
            force_reinstall,
            accumulator,
        ),
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the pure create compiler receives the complete reviewed authority tuple"
)]
fn create_plan(
    desired: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
    configured: &crate::fleet_ensure::model::DesiredCanister,
    cycle_policy: CanisterCyclePolicy,
    bounds: CycleBounds,
    created_at_time: u64,
    disposition: CanisterDisposition,
    accumulator: &mut PlanAccumulator,
    replaced: Option<&LiveCanister>,
) -> Result<CanisterPlan, EnsurePolicyError> {
    accumulator.add_funding(cycle_policy.initial_cycles)?;
    accumulator.add_fee(bounds.ledger_fee)?;
    accumulator.add_fee(bounds.management_creation_fee)?;
    let symbolic = format!("created:{}", configured.name);
    let mut actions = vec![EnsureAction::Create {
        controllers: configured.controllers.clone(),
        created_at_time,
        ledger: desired.cycles_ledger.clone(),
        name: configured.name.clone(),
        requested_initial_cycles: cycle_policy.initial_cycles,
        subnet: configured.subnet.clone(),
    }];
    if let Some(wasm) = &configured.wasm {
        require_install_initializer(desired, configured)?;
        let wasm_sha256 = wasm_sha256(artifacts, &configured.name)?;
        actions.push(EnsureAction::Install {
            canic_init: configured.canic_init.clone(),
            init_arg: configured.init_arg.clone(),
            init_arg_sha256: optional_init_arg_sha256(artifacts, configured)?,
            init_candid: configured.init_candid.clone(),
            init_candid_sha256: optional_init_candid_sha256(artifacts, configured)?,
            mode: InstallMode::Install,
            name: configured.name.clone(),
            principal: symbolic,
            wasm: wasm.clone(),
            wasm_sha256,
        });
        accumulator.add_burn(bounds.update_burn)?;
    }
    if let Some(live) = replaced {
        append_retirement_actions(
            desired,
            artifacts,
            configured,
            live,
            bounds,
            &mut actions,
            accumulator,
        )?;
    }
    Ok(CanisterPlan {
        actions,
        disposition,
        name: configured.name.clone(),
        observed_cycles: replaced.map_or(0, |live| live.cycles),
        principal: replaced
            .map(|live| live.principal.clone())
            .or_else(|| configured.principal.clone()),
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "the pure reuse compiler receives the complete reviewed authority tuple"
)]
fn reuse_plan(
    desired: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
    configured: &crate::fleet_ensure::model::DesiredCanister,
    live: &LiveCanister,
    cycle_policy: CanisterCyclePolicy,
    bounds: CycleBounds,
    created_at_time: u64,
    force_reinstall: bool,
    accumulator: &mut PlanAccumulator,
) -> Result<CanisterPlan, EnsurePolicyError> {
    accumulator.retained = checked_add(accumulator.retained, live.cycles, "retained cycles")?;
    let mut actions = Vec::new();
    if configured.kind == DesiredCanisterKind::Pool && live.root_owned_lifecycle.is_none() {
        return Err(EnsurePolicyError::MissingPoolLifecycle {
            name: configured.name.clone(),
        });
    }
    let active_pool_asset = configured.kind == DesiredCanisterKind::Pool
        && matches!(
            live.root_owned_lifecycle,
            Some(RootOwnedCanisterLifecycle::Claimed | RootOwnedCanisterLifecycle::Workload)
        );
    let retained_balance_evidence =
        live.root_owned_lifecycle == Some(RootOwnedCanisterLifecycle::Retained);
    let mut disposition = CanisterDisposition::Reuse;
    if let Some(wasm) = &configured.wasm {
        let wasm_sha256 = wasm_sha256(artifacts, &configured.name)?;
        if force_reinstall || live.module_sha256.as_deref() != Some(wasm_sha256.as_str()) {
            require_install_initializer(desired, configured)?;
            actions.push(EnsureAction::Install {
                canic_init: configured.canic_init.clone(),
                init_arg: configured.init_arg.clone(),
                init_arg_sha256: optional_init_arg_sha256(artifacts, configured)?,
                init_candid: configured.init_candid.clone(),
                init_candid_sha256: optional_init_candid_sha256(artifacts, configured)?,
                mode: if force_reinstall || live.module_sha256.is_some() {
                    InstallMode::Reinstall
                } else {
                    InstallMode::Install
                },
                name: configured.name.clone(),
                principal: live.principal.clone(),
                wasm: wasm.clone(),
                wasm_sha256,
            });
            accumulator.add_burn(bounds.update_burn)?;
            disposition = CanisterDisposition::Reinstall;
        }
    }
    if configured.kind != DesiredCanisterKind::Pool && live.status != CanisterRuntimeStatus::Running
    {
        actions.push(EnsureAction::Start {
            name: configured.name.clone(),
            principal: live.principal.clone(),
        });
        accumulator.add_burn(bounds.update_burn)?;
    }
    let mut actual_controllers = live.controllers.clone();
    actual_controllers.sort();
    let mut desired_controllers = configured.controllers.clone();
    desired_controllers.sort();
    if configured.kind != DesiredCanisterKind::Pool && actual_controllers != desired_controllers {
        actions.push(EnsureAction::SetControllers {
            controllers: desired_controllers,
            name: configured.name.clone(),
            principal: live.principal.clone(),
        });
        accumulator.add_burn(bounds.update_burn)?;
    }
    append_target_funding(
        desired,
        configured,
        live,
        cycle_policy,
        bounds,
        created_at_time,
        active_pool_asset || retained_balance_evidence,
        &mut actions,
        accumulator,
    )?;
    Ok(CanisterPlan {
        actions,
        disposition,
        name: configured.name.clone(),
        observed_cycles: live.cycles,
        principal: Some(live.principal.clone()),
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "the funding step receives one complete target-local economic authority tuple"
)]
fn append_target_funding(
    desired: &DesiredFleet,
    configured: &crate::fleet_ensure::model::DesiredCanister,
    live: &LiveCanister,
    cycle_policy: CanisterCyclePolicy,
    bounds: CycleBounds,
    created_at_time: u64,
    funding_fenced: bool,
    actions: &mut Vec<EnsureAction>,
    accumulator: &mut PlanAccumulator,
) -> Result<(), EnsurePolicyError> {
    if funding_fenced || live.cycles >= cycle_policy.minimum_cycles {
        return Ok(());
    }
    let target_updates =
        u128::try_from(actions.len()).map_err(|_| EnsurePolicyError::ArithmeticOverflow {
            field: "target update count",
        })?;
    let update_margin = bounds.update_burn.checked_mul(target_updates).ok_or(
        EnsurePolicyError::ArithmeticOverflow {
            field: "target update margin",
        },
    )?;
    let target_margin = bounds.observation_burn.checked_add(update_margin).ok_or(
        EnsurePolicyError::ArithmeticOverflow {
            field: "target funding margin",
        },
    )?;
    let funding_deficit_cycles = cycle_policy.minimum_cycles - live.cycles;
    let amount = funding_deficit_cycles.checked_add(target_margin).ok_or(
        EnsurePolicyError::ArithmeticOverflow {
            field: "funding observation margin",
        },
    )?;
    let expected_post_cycles =
        live.cycles
            .checked_add(amount)
            .ok_or(EnsurePolicyError::ArithmeticOverflow {
                field: "funding post-balance",
            })?;
    actions.insert(
        0,
        EnsureAction::Fund {
            amount,
            created_at_time,
            expected_post_cycles,
            funding_deficit_cycles,
            funding_margin_cycles: target_margin,
            ledger: desired.cycles_ledger.clone(),
            name: configured.name.clone(),
            principal: live.principal.clone(),
        },
    );
    accumulator.add_funding(amount)?;
    accumulator.add_fee(bounds.ledger_fee)
}

fn recovery_reinstall_canisters(
    desired: &DesiredFleet,
    observation: &FleetObservation,
) -> Result<BTreeSet<String>, EnsurePolicyError> {
    let mut reinstalls = BTreeSet::new();
    for configured in &desired.canisters {
        let required = observation
            .canisters
            .get(&configured.name)
            .and_then(Option::as_ref)
            .is_some_and(|live| live.reinstall_required);
        if !required {
            continue;
        }
        let exact_infrastructure = matches!(
            configured.kind,
            DesiredCanisterKind::Coordinator
                | DesiredCanisterKind::Root
                | DesiredCanisterKind::Store
        ) && configured.presence == DesiredPresence::Present;
        if !exact_infrastructure {
            return Err(EnsurePolicyError::RecoveryReinstallUnavailable {
                name: configured.name.clone(),
            });
        }
        reinstalls.insert(configured.name.clone());
    }
    Ok(reinstalls)
}

fn retire_plan(
    desired: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
    configured: &crate::fleet_ensure::model::DesiredCanister,
    live: &LiveCanister,
    bounds: CycleBounds,
    disposition: CanisterDisposition,
    accumulator: &mut PlanAccumulator,
) -> Result<CanisterPlan, EnsurePolicyError> {
    let mut actions = Vec::new();
    append_retirement_actions(
        desired,
        artifacts,
        configured,
        live,
        bounds,
        &mut actions,
        accumulator,
    )?;
    Ok(CanisterPlan {
        actions,
        disposition,
        name: configured.name.clone(),
        observed_cycles: live.cycles,
        principal: Some(live.principal.clone()),
    })
}

fn append_retirement_actions(
    desired: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
    configured: &crate::fleet_ensure::model::DesiredCanister,
    live: &LiveCanister,
    bounds: CycleBounds,
    actions: &mut Vec<EnsureAction>,
    accumulator: &mut PlanAccumulator,
) -> Result<(), EnsurePolicyError> {
    if live.cycles > bounds.material_threshold {
        let drain = configured
            .drain
            .as_ref()
            .ok_or_else(|| EnsurePolicyError::NoSafeDrain {
                name: configured.name.clone(),
                cycles: live.cycles,
            })?;
        if drain.destination != desired.treasury {
            return Err(EnsurePolicyError::WrongDrainDestination {
                name: configured.name.clone(),
            });
        }
        let maximum_execution_burn_cycles = parse_canister_cycles(
            &configured.name,
            "drain.maximum_execution_burn_cycles",
            &drain.maximum_execution_burn_cycles,
        )?;
        let reserved = checked_add(
            bounds.material_threshold,
            maximum_execution_burn_cycles,
            "retirement reserve",
        )?;
        let amount =
            live.cycles
                .checked_sub(reserved)
                .ok_or_else(|| EnsurePolicyError::NoSafeDrain {
                    name: configured.name.clone(),
                    cycles: live.cycles,
                })?;
        if amount == 0 {
            return Err(EnsurePolicyError::NoSafeDrain {
                name: configured.name.clone(),
                cycles: live.cycles,
            });
        }
        actions.push(EnsureAction::Transfer {
            amount,
            candid: drain.candid.clone(),
            candid_sha256: artifacts
                .drain_candid_sha256_by_canister
                .get(&configured.name)
                .cloned()
                .ok_or_else(|| EnsurePolicyError::MissingArtifactIdentity {
                    kind: "drain Candid",
                    name: configured.name.clone(),
                })?,
            destination: drain.destination.clone(),
            maximum_execution_burn_cycles,
            method: drain.method.clone(),
            name: configured.name.clone(),
            principal: live.principal.clone(),
        });
        accumulator.transfers = checked_add(accumulator.transfers, amount, "scheduled transfer")?;
        accumulator.add_burn(maximum_execution_burn_cycles)?;
    }
    actions.push(EnsureAction::Stop {
        name: configured.name.clone(),
        principal: live.principal.clone(),
    });
    actions.push(EnsureAction::Delete {
        maximum_remaining_cycles: bounds.material_threshold,
        name: configured.name.clone(),
        principal: live.principal.clone(),
    });
    accumulator.add_burn(bounds.material_threshold)?;
    accumulator.add_burn(bounds.update_burn)?;
    accumulator.add_burn(bounds.update_burn)?;
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "one pure validation boundary checks the complete desired Fleet authority"
)]
fn validate_authority(
    desired: &DesiredFleet,
    requested_fleet: &str,
) -> Result<(), EnsurePolicyError> {
    validate_path_identity(desired, requested_fleet)?;
    if desired.canisters.len() > MAX_FLEET_ENSURE_CANISTERS {
        return Err(EnsurePolicyError::TooManyCanisters {
            actual: desired.canisters.len(),
            maximum: MAX_FLEET_ENSURE_CANISTERS,
        });
    }
    if desired.protocol_steps.len() > MAX_FLEET_ENSURE_PROTOCOL_STEPS {
        return Err(EnsurePolicyError::TooManyProtocolSteps {
            actual: desired.protocol_steps.len(),
            maximum: MAX_FLEET_ENSURE_PROTOCOL_STEPS,
        });
    }
    if desired.maximum_stalled_observations == 0 {
        return Err(EnsurePolicyError::InvalidStallBound);
    }
    validate_principal("operator", &desired.operator)?;
    validate_principal("treasury", &desired.treasury)?;
    validate_principal("cycles_ledger", &desired.cycles_ledger)?;
    let mut names = BTreeSet::new();
    let mut principals = BTreeSet::new();
    for configured in &desired.canisters {
        if configured.canic_init.is_some()
            && (configured.init_arg.is_some() || configured.init_candid.is_some())
        {
            return Err(EnsurePolicyError::ConflictingCanicInitializer {
                name: configured.name.clone(),
            });
        }
        if configured.init_arg.is_some() != configured.init_candid.is_some() {
            return Err(EnsurePolicyError::IncompleteInitTemplate(
                configured.name.clone(),
            ));
        }
        if configured.name.is_empty()
            || configured
                .drain
                .as_ref()
                .is_some_and(|drain| drain.method.is_empty())
        {
            return Err(EnsurePolicyError::EmptyName);
        }
        validate_principal(
            &format!("canisters.{}.subnet", configured.name),
            &configured.subnet,
        )?;
        for controller in &configured.controllers {
            validate_principal(
                &format!("canisters.{}.controllers", configured.name),
                controller,
            )?;
        }
        if let Some(principal) = &configured.principal {
            validate_principal(
                &format!("canisters.{}.principal", configured.name),
                principal,
            )?;
        }
        if let Some(drain) = &configured.drain {
            validate_principal(
                &format!("canisters.{}.drain.destination", configured.name),
                &drain.destination,
            )?;
        }
        let unique_name = names.insert(configured.name.as_str());
        let unique_principal = configured
            .principal
            .as_deref()
            .is_none_or(|principal| principals.insert(principal));
        if !unique_name || !unique_principal {
            return Err(EnsurePolicyError::DuplicateAuthority {
                name: configured.name.clone(),
            });
        }
        if configured.presence == DesiredPresence::Present
            && !configured.controllers.contains(&desired.operator)
            && configured.kind != DesiredCanisterKind::Pool
        {
            return Err(EnsurePolicyError::MissingOperatorController {
                name: configured.name.clone(),
            });
        }
        validate_canic_initializer(desired, configured)?;
        if configured.kind == DesiredCanisterKind::Pool
            && (configured.presence != DesiredPresence::Present
                || configured.principal.is_none()
                || configured.replace
                || configured.wasm.is_some()
                || configured.drain.is_some()
                || configured.init_arg.is_some()
                || configured.init_candid.is_some()
                || configured.canic_init.is_some())
        {
            return Err(EnsurePolicyError::InvalidTopology {
                name: configured.name.clone(),
                reason: "pool assets must be exact present non-replaceable identities without independent runtime authority",
            });
        }
    }
    validate_typed_topology(desired)?;
    let mut protocol_names = BTreeSet::new();
    for step in &desired.protocol_steps {
        if step.name.is_empty()
            || step.command_method.is_empty()
            || step.status_method.is_empty()
            || !protocol_names.insert(step.name.as_str())
            || !desired.canisters.iter().any(|canister| {
                canister.name == step.canister && canister.presence == DesiredPresence::Present
            })
        {
            return Err(EnsurePolicyError::InvalidProtocolStep(step.name.clone()));
        }
    }
    if !desired.canisters.iter().any(|configured| {
        configured.presence == DesiredPresence::Present
            && configured.principal.as_deref() == Some(desired.treasury.as_str())
    }) {
        return Err(EnsurePolicyError::MissingTreasury {
            treasury: desired.treasury.clone(),
        });
    }
    if desired.canisters.iter().any(|configured| {
        configured.principal.as_deref() == Some(desired.treasury.as_str()) && configured.replace
    }) {
        return Err(EnsurePolicyError::TreasuryReplacement {
            treasury: desired.treasury.clone(),
        });
    }
    Ok(())
}

fn validate_canic_initializer(
    desired: &DesiredFleet,
    configured: &crate::fleet_ensure::model::DesiredCanister,
) -> Result<(), EnsurePolicyError> {
    use crate::fleet_ensure::model::DesiredCanisterInit;

    let matches_kind = matches!(
        (&configured.kind, &configured.canic_init),
        (_, None)
            | (
                DesiredCanisterKind::Coordinator,
                Some(DesiredCanisterInit::Coordinator)
            )
            | (
                DesiredCanisterKind::Root,
                Some(DesiredCanisterInit::Root { .. })
            )
            | (
                DesiredCanisterKind::Store,
                Some(DesiredCanisterInit::Store { .. })
            )
    );
    let infrastructure_artifact = desired.protocol.is_some()
        && configured.wasm.is_some()
        && matches!(
            configured.kind,
            DesiredCanisterKind::Coordinator
                | DesiredCanisterKind::Root
                | DesiredCanisterKind::Store
        );
    if !matches_kind
        || (configured.canic_init.is_some() && desired.bootstrap.is_none())
        || (infrastructure_artifact && configured.canic_init.is_none())
    {
        return Err(EnsurePolicyError::MissingCanicInitializer {
            name: configured.name.clone(),
        });
    }
    if let (Some(bootstrap), Some(initializer)) = (&desired.bootstrap, &configured.canic_init) {
        let bound = match initializer {
            DesiredCanisterInit::Coordinator => configured.name == bootstrap.coordinator,
            DesiredCanisterInit::Root { root } => {
                root == &configured.name && bootstrap.roots.iter().any(|entry| entry.root == *root)
            }
            DesiredCanisterInit::Store { root } => bootstrap
                .roots
                .iter()
                .any(|entry| entry.root == *root && entry.store == configured.name),
        };
        if !bound {
            return Err(EnsurePolicyError::MissingCanicInitializer {
                name: configured.name.clone(),
            });
        }
    }
    Ok(())
}

fn require_install_initializer(
    desired: &DesiredFleet,
    configured: &crate::fleet_ensure::model::DesiredCanister,
) -> Result<(), EnsurePolicyError> {
    if desired.protocol.is_some()
        && matches!(
            configured.kind,
            DesiredCanisterKind::Coordinator
                | DesiredCanisterKind::Root
                | DesiredCanisterKind::Store
        )
        && configured.canic_init.is_none()
    {
        return Err(EnsurePolicyError::MissingCanicInitializer {
            name: configured.name.clone(),
        });
    }
    Ok(())
}

fn validate_typed_topology(desired: &DesiredFleet) -> Result<(), EnsurePolicyError> {
    let coordinators = desired
        .canisters
        .iter()
        .filter(|canister| {
            canister.presence == DesiredPresence::Present
                && canister.kind == DesiredCanisterKind::Coordinator
        })
        .collect::<Vec<_>>();
    let [coordinator] = coordinators.as_slice() else {
        return Err(EnsurePolicyError::InvalidTopology {
            name: desired.fleet.clone(),
            reason: "exactly one present Coordinator role is required",
        });
    };
    if coordinator.parent.is_some() {
        return Err(EnsurePolicyError::InvalidTopology {
            name: coordinator.name.clone(),
            reason: "Coordinator cannot have a topology parent",
        });
    }
    let has_managed_topology = desired.canisters.iter().any(|canister| {
        canister.presence == DesiredPresence::Present
            && matches!(
                canister.kind,
                DesiredCanisterKind::Root
                    | DesiredCanisterKind::Store
                    | DesiredCanisterKind::Component
            )
    });
    if has_managed_topology && desired.protocol.is_none() && desired.protocol_steps.is_empty() {
        return Err(EnsurePolicyError::InvalidTopology {
            name: desired.fleet.clone(),
            reason: "managed Fleet roles require complete typed protocol intent",
        });
    }
    for canister in desired
        .canisters
        .iter()
        .filter(|canister| canister.presence == DesiredPresence::Present)
    {
        let required_parent_kind = match canister.kind {
            DesiredCanisterKind::Root => Some(DesiredCanisterKind::Coordinator),
            DesiredCanisterKind::Store | DesiredCanisterKind::Component => {
                Some(DesiredCanisterKind::Root)
            }
            DesiredCanisterKind::Pool => Some(DesiredCanisterKind::Root),
            DesiredCanisterKind::Auxiliary | DesiredCanisterKind::Coordinator => None,
        };
        let Some(required_parent_kind) = required_parent_kind else {
            continue;
        };
        let Some(parent) = canister.parent.as_deref().and_then(|parent| {
            desired.canisters.iter().find(|candidate| {
                candidate.name == parent && candidate.presence == DesiredPresence::Present
            })
        }) else {
            return Err(EnsurePolicyError::InvalidTopology {
                name: canister.name.clone(),
                reason: "required present topology parent is missing",
            });
        };
        if parent.kind != required_parent_kind
            || (canister.kind == DesiredCanisterKind::Root && parent.name != coordinator.name)
        {
            return Err(EnsurePolicyError::InvalidTopology {
                name: canister.name.clone(),
                reason: "topology parent has the wrong Canic role",
            });
        }
    }
    Ok(())
}

fn validate_principal(field: &str, value: &str) -> Result<(), EnsurePolicyError> {
    let principal =
        Principal::from_text(value).map_err(|_| EnsurePolicyError::InvalidPrincipal {
            field: field.to_string(),
            value: value.to_string(),
        })?;
    if principal == Principal::anonymous() {
        return Err(EnsurePolicyError::InvalidPrincipal {
            field: field.to_string(),
            value: value.to_string(),
        });
    }
    Ok(())
}

fn validate_observation_authority(
    desired: &DesiredFleet,
    observation: &FleetObservation,
) -> Result<(), EnsurePolicyError> {
    let mut principals = BTreeSet::new();
    for configured in &desired.canisters {
        let Some(live) = observation
            .canisters
            .get(&configured.name)
            .ok_or_else(|| EnsurePolicyError::MissingObservation {
                name: configured.name.clone(),
            })?
            .as_ref()
        else {
            continue;
        };
        if !configured.replace
            && let Some(expected) = &configured.principal
            && expected != &live.principal
        {
            return Err(EnsurePolicyError::ObservationAuthorityMismatch {
                actual: live.principal.clone(),
                expected: expected.clone(),
                name: configured.name.clone(),
            });
        }
        if !principals.insert(live.principal.as_str()) {
            return Err(EnsurePolicyError::DuplicateAuthority {
                name: configured.name.clone(),
            });
        }
    }
    for principal in observation.additional_controlled_cycles.keys() {
        validate_principal("additional_controlled_cycles", principal)?;
        if !principals.insert(principal.as_str()) {
            return Err(EnsurePolicyError::DuplicateAuthority {
                name: principal.clone(),
            });
        }
    }
    Ok(())
}

fn cycle_bounds(desired: &DesiredFleet) -> Result<CycleBounds, EnsurePolicyError> {
    Ok(CycleBounds {
        ledger_fee: parse_fleet_cycles("ledger_fee_cycles", &desired.ledger_fee_cycles)?,
        management_creation_fee: parse_fleet_cycles(
            "management_creation_fee_cycles",
            &desired.management_creation_fee_cycles,
        )?,
        material_threshold: parse_fleet_cycles(
            "material_cycle_threshold",
            &desired.material_cycle_threshold,
        )?,
        observation_burn: parse_fleet_cycles(
            "maximum_observation_burn_cycles",
            &desired.maximum_observation_burn_cycles,
        )?,
        update_burn: parse_fleet_cycles(
            "maximum_update_burn_cycles",
            &desired.maximum_update_burn_cycles,
        )?,
    })
}

fn maximum_observation_count(
    desired: &DesiredFleet,
    canisters: &[CanisterPlan],
    protocol_actions: &[EnsureAction],
    additional_controlled_canisters: usize,
) -> Result<u128, EnsurePolicyError> {
    let initially_present = u128::try_from(
        canisters
            .iter()
            .filter(|canister| canister.principal.is_some())
            .count(),
    )
    .map_err(|_| EnsurePolicyError::ArithmeticOverflow {
        field: "initial observation count",
    })?;
    let terminal_present = u128::try_from(
        desired
            .canisters
            .iter()
            .filter(|canister| canister.presence == DesiredPresence::Present)
            .count(),
    )
    .map_err(|_| EnsurePolicyError::ArithmeticOverflow {
        field: "terminal observation count",
    })?;
    let action_observations = canisters
        .iter()
        .flat_map(|canister| &canister.actions)
        .try_fold(0_u128, |total, action| {
            let count = match action {
                EnsureAction::Create { .. } => 1,
                EnsureAction::Delete { .. } => 4,
                EnsureAction::Fund { .. } | EnsureAction::Transfer { .. } => 2,
                EnsureAction::Install { .. }
                | EnsureAction::FleetProtocol { .. }
                | EnsureAction::Protocol { .. }
                | EnsureAction::SetControllers { .. }
                | EnsureAction::Start { .. }
                | EnsureAction::Stop { .. } => 3,
            };
            checked_add(total, count, "effect observation count")
        })?;
    let protocol_observations = u128::try_from(protocol_actions.len())
        .map_err(|_| EnsurePolicyError::ArithmeticOverflow {
            field: "protocol observation count",
        })?
        .checked_mul(3)
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "protocol observation count",
        })?;
    let terminal_protocol_observations = terminal_protocol_observation_bound(
        desired,
        protocol_actions,
        additional_controlled_canisters,
    )?;
    initially_present
        .checked_mul(2)
        .and_then(|value| value.checked_add(terminal_present))
        .and_then(|value| value.checked_add(action_observations))
        .and_then(|value| value.checked_add(protocol_observations))
        .and_then(|value| value.checked_add(terminal_protocol_observations))
        .and_then(|value| value.checked_add(u128::from(desired.maximum_stalled_observations)))
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "maximum observation count",
        })
}

fn terminal_protocol_observation_bound(
    desired: &DesiredFleet,
    protocol_actions: &[EnsureAction],
    additional_controlled_canisters: usize,
) -> Result<u128, EnsurePolicyError> {
    if desired.protocol.is_none() {
        return Ok(0);
    }
    let configured = u128::try_from(desired.canisters.len()).map_err(|_| {
        EnsurePolicyError::ArithmeticOverflow {
            field: "terminal protocol observation count",
        }
    })?;
    let additional = u128::try_from(additional_controlled_canisters).map_err(|_| {
        EnsurePolicyError::ArithmeticOverflow {
            field: "terminal protocol observation count",
        }
    })?;
    let observed_bound = configured
        .checked_mul(4)
        .and_then(|count| {
            additional
                .checked_mul(3)
                .and_then(|extra| count.checked_add(extra))
        })
        .and_then(|count| count.checked_add(4))
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "terminal protocol observation count",
        })?;
    let planned_bound = protocol_actions.iter().try_fold(0_u128, |bound, action| {
        let EnsureAction::FleetProtocol {
            action: current, ..
        } = action
        else {
            return Ok(bound);
        };
        let crate::fleet_ensure::model::CurrentFleetProtocolAction::ProvisionComponents {
            request,
            ..
        } = current.as_ref()
        else {
            return Ok(bound);
        };
        request
            .plan
            .batches
            .iter()
            .try_fold(1_u128, |total, batch| {
                let root_base =
                    total
                        .checked_add(5)
                        .ok_or(EnsurePolicyError::ArithmeticOverflow {
                            field: "terminal protocol observation count",
                        })?;
                batch
                    .placements
                    .iter()
                    .try_fold(root_base, |subtotal, placement| {
                        placement.entries.iter().try_fold(subtotal, |sum, entry| {
                            sum.checked_add(3)
                                .and_then(|value| {
                                    value.checked_add(
                                        u128::from(entry.limits.maximum_descendants) * 2,
                                    )
                                })
                                .ok_or(EnsurePolicyError::ArithmeticOverflow {
                                    field: "terminal protocol observation count",
                                })
                        })
                    })
            })
            .map(|current_bound| bound.max(current_bound))
    })?;
    Ok(observed_bound.max(planned_bound))
}

fn canister_cycle_policy(
    configured: &crate::fleet_ensure::model::DesiredCanister,
) -> Result<CanisterCyclePolicy, EnsurePolicyError> {
    let policy = CanisterCyclePolicy {
        initial_cycles: parse_canister_cycles(
            &configured.name,
            "initial_cycles",
            &configured.initial_cycles,
        )?,
        minimum_cycles: parse_canister_cycles(
            &configured.name,
            "minimum_cycles",
            &configured.minimum_cycles,
        )?,
    };
    if configured.presence == DesiredPresence::Present
        && policy.initial_cycles < policy.minimum_cycles
    {
        return Err(EnsurePolicyError::InitialCyclesBelowMinimum {
            name: configured.name.clone(),
        });
    }
    Ok(policy)
}

fn parse_fleet_cycles(field: &'static str, value: &str) -> Result<u128, EnsurePolicyError> {
    value
        .parse()
        .map_err(|_| EnsurePolicyError::InvalidFleetCycles {
            field,
            value: value.to_string(),
        })
}

fn parse_canister_cycles(
    name: &str,
    field: &'static str,
    value: &str,
) -> Result<u128, EnsurePolicyError> {
    value
        .parse()
        .map_err(|_| EnsurePolicyError::InvalidCanisterCycles {
            field,
            name: name.to_string(),
            value: value.to_string(),
        })
}

fn checked_add(left: u128, right: u128, field: &'static str) -> Result<u128, EnsurePolicyError> {
    left.checked_add(right)
        .ok_or(EnsurePolicyError::ArithmeticOverflow { field })
}

fn wasm_sha256(artifacts: &DesiredFleetArtifacts, name: &str) -> Result<String, EnsurePolicyError> {
    artifacts
        .wasm_sha256_by_canister
        .get(name)
        .cloned()
        .ok_or_else(|| EnsurePolicyError::MissingWasmIdentity {
            name: name.to_string(),
        })
}

fn optional_init_arg_sha256(
    artifacts: &DesiredFleetArtifacts,
    configured: &crate::fleet_ensure::model::DesiredCanister,
) -> Result<Option<String>, EnsurePolicyError> {
    configured
        .init_arg
        .as_ref()
        .map(|_| {
            artifacts
                .init_arg_sha256_by_canister
                .get(&configured.name)
                .cloned()
                .ok_or_else(|| EnsurePolicyError::MissingArtifactIdentity {
                    kind: "init argument",
                    name: configured.name.clone(),
                })
        })
        .transpose()
}

fn optional_init_candid_sha256(
    artifacts: &DesiredFleetArtifacts,
    configured: &crate::fleet_ensure::model::DesiredCanister,
) -> Result<Option<String>, EnsurePolicyError> {
    configured
        .init_candid
        .as_ref()
        .map(|_| {
            artifacts
                .init_candid_sha256_by_canister
                .get(&configured.name)
                .cloned()
                .ok_or_else(|| EnsurePolicyError::MissingArtifactIdentity {
                    kind: "init Candid",
                    name: configured.name.clone(),
                })
        })
        .transpose()
}

pub(crate) fn operation_id(desired_sha256: &str, environment: &str, fleet: &str) -> String {
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, b"canic:fleet-ensure:operation:v1");
    hash_field(&mut hasher, desired_sha256.as_bytes());
    hash_field(&mut hasher, environment.as_bytes());
    hash_field(&mut hasher, fleet.as_bytes());
    canic_core::cdk::utils::hash::hex_bytes(hasher.finalize())
}

pub(crate) fn expected_plan_sha256(plan: &FleetEnsurePlan) -> String {
    let mut canonical = plan.clone();
    canonical.plan_sha256.clear();
    let bytes = super::json::to_vec(&canonical).expect("Fleet ensure plan is JSON serializable");
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, b"canic:fleet-ensure:plan:v1");
    hash_field(&mut hasher, &bytes);
    canic_core::cdk::utils::hash::hex_bytes(hasher.finalize())
}

fn hash_field(hasher: &mut Sha256, value: &[u8]) {
    hasher.update(u64::try_from(value.len()).unwrap_or(u64::MAX).to_be_bytes());
    hasher.update(value);
}
