//! Module: fleet_ensure::policy
//!
//! Responsibility: validate current desired state and compile one conservative convergence plan.
//! Does not own: storage, clocks, transport, live observation, or effects.
//! Boundary: workflow supplies exact desired/live inputs and persists the returned immutable plan.

use crate::{
    component_topology::{
        RootPoolImportCapacityError, RootPoolImportCapacityInput,
        validate_root_pool_import_capacity,
    },
    fleet_ensure::model::{
        CanisterCyclePolicy, CanisterDisposition, CanisterPlan, CanisterRuntimeStatus,
        CurrentFleetProtocolAction, CycleConservation, DesiredCanisterKind, DesiredFleet,
        DesiredFleetArtifacts, DesiredFleetBootstrapRoot, DesiredPresence, EnsureAction,
        EstateFundingDomainPlan, EstatePoolAssetLifecycle, FLEET_ENSURE_SCHEMA_VERSION,
        FleetEnsurePlan, FleetEnsurePlanScope, FleetObservation, InstallMode, LiveCanister,
        MAX_FLEET_ENSURE_CANISTERS, MAX_FLEET_ENSURE_PROTOCOL_STEPS,
        RetainedRootStartAuthorityRecord, RetainedRootStartBinding, RootManagementObservation,
        RootOwnedCanisterLifecycle,
    },
};
use candid::Principal;
use canic_core::{
    cdk::types::Cycles,
    ids::{FleetName, FleetSubnetCanisterPoolConfig},
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error as ThisError;

/// Pure current-contract plan compilation failure.

#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum EnsurePolicyError {
    #[error("cycle arithmetic overflow while compiling {field}")]
    ArithmeticOverflow { field: &'static str },

    #[error(
        "Fleet cycle-conservation headroom is insufficient: available {available} cycles after reviewed funding, required maximum burn {required}, shortfall {shortfall}, actions {action_count}; no plan or effect was authorized"
    )]
    InsufficientCycleConservation {
        action_count: usize,
        available: u128,
        required: u128,
        shortfall: u128,
    },

    #[error("controlled canister {name} has duplicate name or principal authority")]
    DuplicateAuthority { name: String },

    #[error("controlled canister {name} references unavailable controller canister {controller}")]
    ControllerCanisterUnavailable { controller: String, name: String },

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

    #[error("desired Fleet estate-funding topology is invalid: {reason}")]
    EstateFundingTopology { reason: String },

    #[error(
        "Root {root} pool cannot satisfy current desired demand: occupied {occupied_assets}/{maximum_size}, eligible Ready {eligible_ready_assets}, allocated Workloads {allocated_workloads}, pending creations {pending_creations}, required new creations {required_creation_count}, available slots {available_slots}, capacity shortfall {capacity_shortfall}; repair or retire retained pool assets before funding"
    )]
    EstatePoolCapacity {
        allocated_workloads: u32,
        available_slots: u32,
        capacity_shortfall: u32,
        eligible_ready_assets: u32,
        maximum_size: u32,
        occupied_assets: u32,
        pending_creations: u32,
        required_creation_count: u32,
        root: String,
    },

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
        "fresh pool canister {name} requests {creation_funding_cycles} creation cycles for readiness floor {readiness_floor_cycles}, but its bounded pre-import burn is {admissible_burn_cycles} cycles; required funding {required_creation_funding_cycles}, shortfall {shortfall_cycles}; no effect was authorized"
    )]
    FreshPoolCreationFundingInsufficient {
        admissible_burn_cycles: u128,
        creation_funding_cycles: u128,
        name: String,
        readiness_floor_cycles: u128,
        required_creation_funding_cycles: u128,
        shortfall_cycles: u128,
    },

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

    #[error("management observation for retained Root {name} is missing")]
    MissingRootManagementObservation { name: String },

    #[error("management observation for retained Root {name} has invalid {field}")]
    RootManagementAuthorityMismatch { field: &'static str, name: String },

    #[error("retained Root {name} is stopping; wait for terminal Stopped state")]
    RootStopping { name: String },

    #[error(transparent)]
    PoolImportCapacity(#[from] RootPoolImportCapacityError),
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
            observation,
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
        protocol_actions.push(action.clone());
    }
    if protocol_actions.len() > MAX_FLEET_ENSURE_PROTOCOL_STEPS {
        return Err(EnsurePolicyError::TooManyProtocolSteps {
            actual: protocol_actions.len(),
            maximum: MAX_FLEET_ENSURE_PROTOCOL_STEPS,
        });
    }
    if recovery_reinstalls.is_empty() && protocol_actions.is_empty() {
        let pending = desired.canisters.iter().find_map(|configured| {
            observation
                .canisters
                .get(&configured.name)
                .and_then(Option::as_ref)
                .is_some_and(|live| {
                    live.root_owned_lifecycle == Some(RootOwnedCanisterLifecycle::Retained)
                })
                .then(|| configured.name.clone())
        });
        if let Some(name) = pending
            && !retained_root_observation_is_start_only(
                desired,
                &accumulator.canisters,
                observation,
            )
        {
            return Err(EnsurePolicyError::PendingRootOwnedBalance { name });
        }
    }

    let observed_native_cycles = observation
        .canisters
        .values()
        .filter_map(Option::as_ref)
        .try_fold(0_u128, |total, live| {
            checked_add(total, live.cycles, "observed controlled cycles")
        })?;
    let observed_native_cycles = observation
        .additional_controlled_cycles
        .values()
        .try_fold(observed_native_cycles, |total, cycles| {
            checked_add(total, *cycles, "observed controlled cycles")
        })?;
    tranche_protocol_actions(
        desired,
        observation,
        bounds,
        observed_native_cycles,
        &mut accumulator,
        &mut protocol_actions,
    )?;

    let estate_funding_domains = compile_estate_funding_domains(desired, observation, bounds)?;
    append_estate_funding_actions(
        &estate_funding_domains,
        created_at_time,
        desired.canisters.len(),
        &mut accumulator,
    )?;
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

    let observed_estate_funding_cycles =
        estate_funding_domains
            .iter()
            .try_fold(0_u128, |total, domain| {
                checked_add(
                    total,
                    domain.available_cycles.unwrap_or_default(),
                    "observed estate funding cycles",
                )
            })?;
    let maximum_estate_creation_fee_cycles =
        estate_funding_domains
            .iter()
            .try_fold(0_u128, |total, domain| {
                checked_add(
                    total,
                    domain.maximum_creation_fee_cycles,
                    "maximum estate creation fees",
                )
            })?;
    let observed_controlled_cycles = checked_add(
        observed_native_cycles,
        observed_estate_funding_cycles,
        "observed controlled cycles",
    )?;
    let maximum_operator_debit_cycles = checked_add(
        accumulator.new_funding,
        accumulator.fees,
        "maximum operator debit",
    )?;
    if observation.operator_cycles < maximum_operator_debit_cycles {
        // Funding sufficiency is deliberately enforced by apply after a reviewed plan is loaded.
        // The plan remains useful and truthful even when the current account is insufficient.
    }
    let available_after_operator_funding = checked_add(
        observed_controlled_cycles,
        accumulator.new_funding,
        "cycle conservation available balance",
    )?;
    let available_after_estate_fees = available_after_operator_funding
        .checked_sub(maximum_estate_creation_fee_cycles)
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "estate creation fees",
        })?;
    let expected_post_operation_cycles = available_after_estate_fees
        .checked_sub(accumulator.execution_burn)
        .ok_or_else(|| {
            insufficient_cycle_conservation(
                &accumulator,
                protocol_actions.len(),
                available_after_operator_funding,
                accumulator.execution_burn,
            )
        })?;
    let conservation = CycleConservation {
        estate_funding_domains,
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
        root_start_authority: None,
        reviewed_desired: Some(Box::new(
            crate::fleet_ensure::model::ReviewedDesiredFleetRecord::capture(desired),
        )),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        scope: FleetEnsurePlanScope::Full,
        terminal_inventory_operation_id: None,
    };
    plan.plan_sha256 = expected_plan_sha256(&plan);
    Ok(plan)
}

fn append_estate_funding_actions(
    domains: &[EstateFundingDomainPlan],
    created_at_time: u64,
    canister_count: usize,
    accumulator: &mut PlanAccumulator,
) -> Result<(), EnsurePolicyError> {
    for (index, domain) in domains.iter().enumerate() {
        if domain.shortfall_cycles == 0 {
            continue;
        }
        let offset = canister_count
            .checked_add(index)
            .and_then(|value| u64::try_from(value).ok())
            .ok_or(EnsurePolicyError::ArithmeticOverflow {
                field: "estate funding action timestamp",
            })?;
        let action_time =
            created_at_time
                .checked_add(offset)
                .ok_or(EnsurePolicyError::ArithmeticOverflow {
                    field: "estate funding action timestamp",
                })?;
        let expected_post_cycles = domain
            .available_cycles
            .unwrap_or_default()
            .checked_add(domain.shortfall_cycles)
            .ok_or(EnsurePolicyError::ArithmeticOverflow {
                field: "estate funding destination balance",
            })?;
        let principal = domain
            .root_principal
            .clone()
            .unwrap_or_else(|| format!("created:{}", domain.root));
        let root_plan = accumulator
            .canisters
            .iter_mut()
            .find(|canister| canister.name == domain.root)
            .ok_or_else(|| EnsurePolicyError::EstateFundingTopology {
                reason: format!("Root {} has no canister plan", domain.root),
            })?;
        root_plan.actions.push(EnsureAction::FundEstate {
            amount: domain.shortfall_cycles,
            created_at_time: action_time,
            expected_post_cycles,
            ledger: domain.cycles_ledger.clone(),
            ledger_fee_cycles: domain.ledger_fee_cycles,
            name: domain.root.clone(),
            principal,
        });
        accumulator.add_funding(domain.shortfall_cycles)?;
        accumulator.add_fee(domain.ledger_fee_cycles)?;
    }
    Ok(())
}

/// Complete pure authority used to compile one management-only Root Start prerequisite.
pub(crate) struct RootStartPlanInput<'a> {
    pub authority: Option<&'a RetainedRootStartAuthorityRecord>,
    pub created_at_time: u64,
    pub desired: &'a DesiredFleet,
    pub desired_sha256: &'a str,
    pub observation: &'a RootManagementObservation,
    pub requested_fleet: &'a str,
}

/// Compile the one management-only prerequisite allowed before protected Root observation.
pub(crate) fn compile_root_start_prerequisite_plan(
    input: RootStartPlanInput<'_>,
) -> Result<Option<FleetEnsurePlan>, EnsurePolicyError> {
    compile_root_start_plan(&input, None)
}

/// Recompile one reviewed Root-start prerequisite while accepting an already-started exact target.
pub(crate) fn recompile_root_start_prerequisite_plan(
    input: RootStartPlanInput<'_>,
    reviewed_targets: &BTreeSet<String>,
) -> Result<FleetEnsurePlan, EnsurePolicyError> {
    compile_root_start_plan(&input, Some(reviewed_targets))?.ok_or_else(|| {
        EnsurePolicyError::MissingRootManagementObservation {
            name: "reviewed Root-start target".to_string(),
        }
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "one pure boundary validates and compiles the complete Root-start authority"
)]
fn compile_root_start_plan(
    input: &RootStartPlanInput<'_>,
    reviewed_targets: Option<&BTreeSet<String>>,
) -> Result<Option<FleetEnsurePlan>, EnsurePolicyError> {
    let RootStartPlanInput {
        authority,
        created_at_time,
        desired,
        desired_sha256,
        observation,
        requested_fleet,
    } = *input;
    validate_authority(desired, requested_fleet)?;
    let bounds = cycle_bounds(desired)?;
    let configured_roots = desired
        .canisters
        .iter()
        .filter(|configured| {
            configured.kind == DesiredCanisterKind::Root
                && configured.presence == DesiredPresence::Present
        })
        .collect::<Vec<_>>();
    if observation.roots.len() != configured_roots.len() {
        return Err(EnsurePolicyError::MissingRootManagementObservation {
            name: "configured Root set".to_string(),
        });
    }

    let mut canisters = Vec::new();
    let mut root_start_bindings = Vec::new();
    let mut observed_controlled_cycles = 0_u128;
    for configured in configured_roots {
        let observed = observation.roots.get(&configured.name).ok_or_else(|| {
            EnsurePolicyError::MissingRootManagementObservation {
                name: configured.name.clone(),
            }
        })?;
        let expected_principal = configured.principal.as_deref().ok_or_else(|| {
            EnsurePolicyError::RootManagementAuthorityMismatch {
                field: "Principal",
                name: configured.name.clone(),
            }
        })?;
        if observed.name != configured.name || observed.live.principal != expected_principal {
            return Err(EnsurePolicyError::RootManagementAuthorityMismatch {
                field: "Principal",
                name: configured.name.clone(),
            });
        }
        if observed.subnet != configured.subnet {
            return Err(EnsurePolicyError::RootManagementAuthorityMismatch {
                field: "Subnet",
                name: configured.name.clone(),
            });
        }
        if observed.live.status == CanisterRuntimeStatus::Stopping {
            return Err(EnsurePolicyError::RootStopping {
                name: configured.name.clone(),
            });
        }
        let reviewed = reviewed_targets.is_some_and(|targets| targets.contains(&configured.name));
        let needs_start = observed.live.status == CanisterRuntimeStatus::Stopped;
        if reviewed_targets.is_some() && needs_start && !reviewed {
            return Err(EnsurePolicyError::RootManagementAuthorityMismatch {
                field: "reviewed runtime transition",
                name: configured.name.clone(),
            });
        }
        if !needs_start && !reviewed {
            continue;
        }
        if !configured.controller_canisters.is_empty() {
            return Err(EnsurePolicyError::RootManagementAuthorityMismatch {
                field: "controller authority",
                name: configured.name.clone(),
            });
        }
        let mut expected_controllers = configured.controllers.clone();
        expected_controllers.sort();
        let mut actual_controllers = observed.live.controllers.clone();
        actual_controllers.sort();
        if actual_controllers != expected_controllers {
            return Err(EnsurePolicyError::RootManagementAuthorityMismatch {
                field: "controllers",
                name: configured.name.clone(),
            });
        }
        let observed_module = observed.live.module_sha256.as_deref().ok_or_else(|| {
            EnsurePolicyError::RootManagementAuthorityMismatch {
                field: "module SHA-256",
                name: configured.name.clone(),
            }
        })?;
        root_start_bindings.push(RetainedRootStartBinding {
            controllers: actual_controllers.clone(),
            name: configured.name.clone(),
            predecessor_module_sha256: observed_module.to_string(),
            principal: observed.live.principal.clone(),
            subnet: observed.subnet.clone(),
        });
        if observed.live.root_owned_lifecycle.is_some() || observed.live.reinstall_required {
            return Err(EnsurePolicyError::RootManagementAuthorityMismatch {
                field: "runtime authority",
                name: configured.name.clone(),
            });
        }
        observed_controlled_cycles = checked_add(
            observed_controlled_cycles,
            observed.live.cycles,
            "Root-start observed cycles",
        )?;
        canisters.push(CanisterPlan {
            actions: vec![EnsureAction::Start {
                name: configured.name.clone(),
                principal: observed.live.principal.clone(),
            }],
            disposition: CanisterDisposition::Reuse,
            name: configured.name.clone(),
            observed_cycles: observed.live.cycles,
            principal: Some(observed.live.principal.clone()),
        });
    }
    if canisters.is_empty() {
        return Ok(None);
    }
    let retained_authority = validate_root_start_authority(
        desired,
        requested_fleet,
        authority,
        &root_start_bindings,
        true,
    )?;
    if let Some(reviewed_targets) = reviewed_targets
        && (reviewed_targets.len() != canisters.len()
            || canisters
                .iter()
                .any(|canister| !reviewed_targets.contains(&canister.name)))
    {
        return Err(EnsurePolicyError::RootManagementAuthorityMismatch {
            field: "reviewed Root set",
            name: "Root-start prerequisite".to_string(),
        });
    }
    let target_count =
        u128::try_from(canisters.len()).map_err(|_| EnsurePolicyError::ArithmeticOverflow {
            field: "Root-start target count",
        })?;
    let maximum_execution_burn_cycles = bounds.update_burn.checked_mul(target_count).ok_or(
        EnsurePolicyError::ArithmeticOverflow {
            field: "Root-start execution burn",
        },
    )?;
    let expected_post_operation_cycles = observed_controlled_cycles
        .checked_sub(maximum_execution_burn_cycles)
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "Root-start conservation equation",
        })?;
    let mut plan = FleetEnsurePlan {
        canisters,
        conservation: CycleConservation {
            estate_funding_domains: Vec::new(),
            expected_post_operation_cycles,
            maximum_execution_burn_cycles,
            maximum_new_funding_cycles: 0,
            maximum_operator_debit_cycles: 0,
            maximum_unavoidable_fee_cycles: 0,
            observed_controlled_cycles,
            retained_in_reused_canisters_cycles: observed_controlled_cycles,
            scheduled_transfer_cycles: 0,
        },
        desired_sha256: desired_sha256.to_string(),
        environment: desired.environment.clone(),
        fleet: requested_fleet.to_string(),
        operation_id: operation_id(desired_sha256, &desired.environment, requested_fleet),
        plan_sha256: String::new(),
        planned_at_time: created_at_time,
        protocol_actions: Vec::new(),
        root_start_authority: retained_authority.map(|authority| Box::new(authority.clone())),
        reviewed_desired: Some(Box::new(
            crate::fleet_ensure::model::ReviewedDesiredFleetRecord::capture(desired),
        )),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        scope: FleetEnsurePlanScope::RootStartPrerequisite,
        terminal_inventory_operation_id: None,
    };
    plan.plan_sha256 = expected_plan_sha256(&plan);
    Ok(Some(plan))
}

fn validate_root_start_authority<'a>(
    desired: &DesiredFleet,
    requested_fleet: &str,
    authority: Option<&'a RetainedRootStartAuthorityRecord>,
    root_start_bindings: &[RetainedRootStartBinding],
    authority_required: bool,
) -> Result<Option<&'a RetainedRootStartAuthorityRecord>, EnsurePolicyError> {
    if !authority_required {
        return Ok(None);
    }
    if root_start_bindings.is_empty() {
        return Err(EnsurePolicyError::RootManagementAuthorityMismatch {
            field: "retained module authority",
            name: "Root-start prerequisite".to_string(),
        });
    }
    let authority =
        authority.ok_or_else(|| EnsurePolicyError::RootManagementAuthorityMismatch {
            field: "retained module authority",
            name: root_start_bindings[0].name.clone(),
        })?;
    let bootstrap = desired.bootstrap.as_ref().ok_or_else(|| {
        EnsurePolicyError::RootManagementAuthorityMismatch {
            field: "Fleet bootstrap authority",
            name: root_start_bindings[0].name.clone(),
        }
    })?;
    let mut expected_bindings = root_start_bindings.to_vec();
    expected_bindings.sort_by(|left, right| left.name.cmp(&right.name));
    let mut actual_bindings = authority.roots.clone();
    actual_bindings.sort_by(|left, right| left.name.cmp(&right.name));
    let path_identity_matches =
        authority.environment == desired.environment && authority.fleet == requested_fleet;
    let fleet_identity_matches = authority.fleet_id == bootstrap.fleet_id;
    let record_identity_matches =
        authority.schema_version == FLEET_ENSURE_SCHEMA_VERSION && authority.has_valid_digest();
    let bindings_match = actual_bindings == expected_bindings;
    if !(path_identity_matches
        && fleet_identity_matches
        && record_identity_matches
        && bindings_match)
    {
        return Err(EnsurePolicyError::RootManagementAuthorityMismatch {
            field: "retained module authority",
            name: root_start_bindings[0].name.clone(),
        });
    }
    Ok(Some(authority))
}

fn retained_root_observation_is_start_only(
    desired: &DesiredFleet,
    plans: &[CanisterPlan],
    observation: &FleetObservation,
) -> bool {
    let only_start_effects = plans
        .iter()
        .flat_map(|plan| &plan.actions)
        .all(|action| matches!(action, EnsureAction::Start { .. }));
    only_start_effects
        && desired
            .canisters
            .iter()
            .filter(|configured| {
                observation
                    .canisters
                    .get(&configured.name)
                    .and_then(Option::as_ref)
                    .is_some_and(|live| {
                        live.root_owned_lifecycle == Some(RootOwnedCanisterLifecycle::Retained)
                    })
            })
            .all(|configured| {
                configured.parent.as_ref().is_some_and(|parent| {
                    plans.iter().any(|plan| {
                        plan.name == *parent
                            && plan.actions.iter().any(|action| {
                                matches!(action, EnsureAction::Start { name, .. } if name == parent)
                            })
                    })
                })
            })
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
    validate_path_labels(&desired.environment, requested_fleet)
}

pub(crate) fn validate_path_labels(
    environment: &str,
    requested_fleet: &str,
) -> Result<(), EnsurePolicyError> {
    requested_fleet
        .parse::<FleetName>()
        .map_err(|_| EnsurePolicyError::UnsafePathLabel {
            field: "fleet",
            value: requested_fleet.to_string(),
        })?;
    if environment.is_empty()
        || environment.len() > 64
        || !environment
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(EnsurePolicyError::UnsafePathLabel {
            field: "environment",
            value: environment.to_string(),
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
    observation: &FleetObservation,
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
            let fresh_pool = configured.kind == DesiredCanisterKind::Pool
                && desired
                    .bootstrap
                    .as_ref()
                    .is_some_and(|bootstrap| bootstrap.fresh_estate)
                && configured.principal.is_none();
            if configured.kind == DesiredCanisterKind::Pool && !fresh_pool {
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
            observation,
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
    let temporary_pool_observation_controller = configured.kind == DesiredCanisterKind::Pool
        && desired
            .bootstrap
            .as_ref()
            .is_some_and(|bootstrap| bootstrap.fresh_estate)
        && configured.principal.is_none()
        && configured.controllers.is_empty()
        && configured.parent.as_ref().is_some_and(|parent| {
            configured.controller_canisters.as_slice() == std::slice::from_ref(parent)
        });
    if temporary_pool_observation_controller {
        let admissible_burn_cycles = checked_add(
            bounds.observation_burn,
            bounds.update_burn,
            "fresh pool pre-import burn",
        )?;
        let required_creation_funding_cycles = checked_add(
            cycle_policy.minimum_cycles,
            admissible_burn_cycles,
            "fresh pool creation funding",
        )?;
        if cycle_policy.initial_cycles < required_creation_funding_cycles {
            return Err(EnsurePolicyError::FreshPoolCreationFundingInsufficient {
                admissible_burn_cycles,
                creation_funding_cycles: cycle_policy.initial_cycles,
                name: configured.name.clone(),
                readiness_floor_cycles: cycle_policy.minimum_cycles,
                required_creation_funding_cycles,
                shortfall_cycles: required_creation_funding_cycles - cycle_policy.initial_cycles,
            });
        }
    }
    accumulator.add_funding(cycle_policy.initial_cycles)?;
    accumulator.add_fee(bounds.ledger_fee)?;
    accumulator.add_fee(bounds.management_creation_fee)?;
    let symbolic = format!("created:{}", configured.name);
    let mut creation_controllers = configured.controllers.clone();
    if temporary_pool_observation_controller {
        creation_controllers.push(desired.operator.clone());
        creation_controllers.sort();
        creation_controllers.dedup();
    }
    let mut actions = vec![EnsureAction::Create {
        controller_canisters: configured.controller_canisters.clone(),
        controllers: creation_controllers,
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
            principal: symbolic.clone(),
            wasm: wasm.clone(),
            wasm_sha256,
        });
        accumulator.add_burn(bounds.update_burn)?;
    }
    if temporary_pool_observation_controller {
        actions.push(EnsureAction::SetControllers {
            controller_canisters: configured.controller_canisters.clone(),
            controllers: configured.controllers.clone(),
            name: configured.name.clone(),
            principal: symbolic,
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
    observation: &FleetObservation,
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
    let mut desired_controllers = resolved_controllers(configured, observation)?;
    desired_controllers.sort();
    if configured.kind != DesiredCanisterKind::Pool && actual_controllers != desired_controllers {
        actions.push(EnsureAction::SetControllers {
            controller_canisters: configured.controller_canisters.clone(),
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

fn resolved_controllers(
    configured: &crate::fleet_ensure::model::DesiredCanister,
    observation: &FleetObservation,
) -> Result<Vec<String>, EnsurePolicyError> {
    let mut controllers = configured.controllers.clone();
    for controller in &configured.controller_canisters {
        let principal = observation
            .canisters
            .get(controller)
            .and_then(Option::as_ref)
            .map(|live| live.principal.clone())
            .ok_or_else(|| EnsurePolicyError::ControllerCanisterUnavailable {
                controller: controller.clone(),
                name: configured.name.clone(),
            })?;
        controllers.push(principal);
    }
    controllers.sort();
    controllers.dedup();
    Ok(controllers)
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
    if let Some(bootstrap) = &desired.bootstrap {
        for root in &bootstrap.roots {
            validate_bootstrap_root_pool_import_capacity(root)?;
        }
    }
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
        for controller in &configured.controller_canisters {
            if controller.is_empty() {
                return Err(EnsurePolicyError::InvalidTopology {
                    name: configured.name.clone(),
                    reason: "controller canister name must not be empty",
                });
            }
        }
        if let Some(principal) = &configured.principal {
            validate_principal(
                &format!("canisters.{}.principal", configured.name),
                principal,
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
        let fresh_pool = configured.kind == DesiredCanisterKind::Pool
            && desired
                .bootstrap
                .as_ref()
                .is_some_and(|bootstrap| bootstrap.fresh_estate)
            && configured.principal.is_none();
        if configured.kind == DesiredCanisterKind::Pool
            && (configured.presence != DesiredPresence::Present
                || (configured.principal.is_none() && !fresh_pool)
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
    for (index, configured) in desired.canisters.iter().enumerate() {
        for controller in &configured.controller_canisters {
            let Some(controller_index) = desired
                .canisters
                .iter()
                .position(|candidate| candidate.name == *controller)
            else {
                return Err(EnsurePolicyError::ControllerCanisterUnavailable {
                    controller: controller.clone(),
                    name: configured.name.clone(),
                });
            };
            if controller_index >= index || controller == &configured.name {
                return Err(EnsurePolicyError::InvalidTopology {
                    name: configured.name.clone(),
                    reason: "controller canister must be a distinct earlier desired role",
                });
            }
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
        configured.presence == DesiredPresence::Present && configured.name == desired.treasury
    }) {
        return Err(EnsurePolicyError::MissingTreasury {
            treasury: desired.treasury.clone(),
        });
    }
    if desired
        .canisters
        .iter()
        .any(|configured| configured.name == desired.treasury && configured.replace)
    {
        return Err(EnsurePolicyError::TreasuryReplacement {
            treasury: desired.treasury.clone(),
        });
    }
    Ok(())
}

fn validate_bootstrap_root_pool_import_capacity(
    root: &crate::fleet_ensure::model::DesiredFleetBootstrapRoot,
) -> Result<(), EnsurePolicyError> {
    validate_root_pool_import_capacity(&RootPoolImportCapacityInput {
        import_count: root.canister_pool_imports.len(),
        maximum_size: root.limits.canister_pool.maximum_size,
        root: root.root.clone(),
    })?;
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

fn compile_estate_funding_domains(
    desired: &DesiredFleet,
    observation: &FleetObservation,
    bounds: CycleBounds,
) -> Result<Vec<EstateFundingDomainPlan>, EnsurePolicyError> {
    let Some(bootstrap) = desired.bootstrap.as_ref() else {
        if observation.estate_funding_domains.is_empty() {
            return Ok(Vec::new());
        }
        return Err(EnsurePolicyError::EstateFundingTopology {
            reason: "observed Root funding accounts without bootstrap authority".to_string(),
        });
    };
    if observation.estate_funding_domains.len() != bootstrap.roots.len() {
        return Err(EnsurePolicyError::EstateFundingTopology {
            reason: "observed Root funding-account set differs from bootstrap authority"
                .to_string(),
        });
    }

    let mut domains = Vec::with_capacity(bootstrap.roots.len());
    for root in &bootstrap.roots {
        domains.push(compile_estate_funding_domain(
            desired,
            root,
            observation,
            bounds,
        )?);
    }
    Ok(domains)
}

fn compile_estate_funding_domain(
    desired: &DesiredFleet,
    root: &DesiredFleetBootstrapRoot,
    observation: &FleetObservation,
    bounds: CycleBounds,
) -> Result<EstateFundingDomainPlan, EnsurePolicyError> {
    let observed = observation
        .estate_funding_domains
        .get(&root.root)
        .ok_or_else(|| EnsurePolicyError::EstateFundingTopology {
            reason: format!("Root {} has no funding-account observation", root.root),
        })?;
    validate_estate_funding_owner(desired, observation, root, observed)?;
    let asset_cycles = root.limits.canister_pool.canister_cycles.to_u128();
    let pool_forecast = required_root_pool_creations(
        desired,
        &root.root,
        root.limits.canister_pool.minimum_size,
        asset_cycles,
        bounds.management_creation_fee,
        observation,
    )?;
    let creation_execution_margin_cycles = root
        .limits
        .canister_pool
        .creation_execution_margin
        .to_u128();
    if creation_execution_margin_cycles == 0 {
        return Err(EnsurePolicyError::EstateFundingTopology {
            reason: format!(
                "Root {} has no autonomous-creation execution margin",
                root.root
            ),
        });
    }
    let creation_amount_cycles = asset_cycles
        .checked_add(creation_execution_margin_cycles)
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "estate creation margin",
        })?
        .checked_add(bounds.management_creation_fee)
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "estate creation amount",
        })?;
    let debit_per_creation = creation_amount_cycles
        .checked_add(bounds.ledger_fee)
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "estate creation debit",
        })?;
    let fee_per_creation = bounds
        .management_creation_fee
        .checked_add(bounds.ledger_fee)
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "estate creation fees",
        })?;
    let count = u128::from(pool_forecast.required_creation_count);
    let maximum_creation_debit_cycles =
        debit_per_creation
            .checked_mul(count)
            .ok_or(EnsurePolicyError::ArithmeticOverflow {
                field: "estate creation debit",
            })?;
    let maximum_creation_fee_cycles =
        fee_per_creation
            .checked_mul(count)
            .ok_or(EnsurePolicyError::ArithmeticOverflow {
                field: "estate creation fees",
            })?;
    let available_cycles = observed.balance_cycles;
    let shortfall_cycles =
        maximum_creation_debit_cycles.saturating_sub(available_cycles.unwrap_or_default());
    let initial_pool_assets = initial_pool_asset_authority(desired, root, observed);
    Ok(EstateFundingDomainPlan {
        allocated_workloads: pool_forecast.allocated_workloads,
        available_cycles,
        available_pool_slots: pool_forecast.available_slots,
        creation_amount_cycles,
        creation_execution_margin_cycles,
        readiness_floor_cycles: asset_cycles,
        cycles_ledger: observed.cycles_ledger.clone(),
        eligible_ready_pool_assets: pool_forecast.eligible_ready_assets,
        initial_pool_assets,
        ledger_fee_cycles: bounds.ledger_fee,
        management_creation_fee_cycles: bounds.management_creation_fee,
        maximum_creation_debit_cycles,
        maximum_creation_fee_cycles,
        maximum_funding_cycles: shortfall_cycles,
        occupied_pool_assets: pool_forecast.occupied_assets,
        pending_creation_count: pool_forecast.pending_creations,
        pending_creation: pool_forecast.pending_creation,
        planned_initial_workloads: pool_forecast.planned_workloads,
        pool_maximum_size: pool_forecast.maximum_size,
        required_creation_count: pool_forecast.required_creation_count,
        root: root.root.clone(),
        root_principal: observed.root_principal.clone(),
        shortfall_cycles,
    })
}

fn initial_pool_asset_authority(
    desired: &DesiredFleet,
    root: &DesiredFleetBootstrapRoot,
    observed: &crate::fleet_ensure::model::EstateFundingDomainObservation,
) -> Vec<String> {
    let mut assets = desired
        .canisters
        .iter()
        .filter(|canister| {
            canister.kind == DesiredCanisterKind::Pool
                && canister.presence == DesiredPresence::Present
                && canister.parent.as_deref() == Some(&root.root)
        })
        .map(|canister| {
            canister
                .principal
                .clone()
                .unwrap_or_else(|| format!("created:{}", canister.name))
        })
        .collect::<BTreeSet<_>>();
    if let Some(pool) = observed.pool.as_ref() {
        assets.extend(pool.assets.iter().map(|asset| asset.principal.clone()));
        if let Some(created) = pool
            .pending_creation
            .as_ref()
            .and_then(|pending| pending.created_principal.clone())
        {
            assets.insert(created);
        }
    }
    assets.into_iter().collect()
}

fn validate_estate_funding_owner(
    desired: &DesiredFleet,
    observation: &FleetObservation,
    root: &DesiredFleetBootstrapRoot,
    observed: &crate::fleet_ensure::model::EstateFundingDomainObservation,
) -> Result<(), EnsurePolicyError> {
    if observed.cycles_ledger != desired.cycles_ledger {
        return Err(EnsurePolicyError::EstateFundingTopology {
            reason: format!("Root {} funding-account Ledger differs", root.root),
        });
    }
    let Some(principal) = observed.root_principal.as_deref() else {
        return Ok(());
    };
    validate_principal("estate_funding_domains.root_principal", principal)?;
    let configured = desired
        .canisters
        .iter()
        .find(|canister| canister.name == root.root)
        .ok_or_else(|| EnsurePolicyError::EstateFundingTopology {
            reason: format!("Root {} is absent from desired canisters", root.root),
        })?;
    let live = observation
        .canisters
        .get(&root.root)
        .and_then(Option::as_ref);
    if live.is_some_and(|live| live.principal != principal)
        || configured
            .principal
            .as_deref()
            .is_some_and(|expected| expected != principal)
    {
        return Err(EnsurePolicyError::EstateFundingTopology {
            reason: format!("Root {} funding-account owner differs", root.root),
        });
    }
    Ok(())
}

fn required_root_pool_creations(
    desired: &DesiredFleet,
    root: &str,
    ready_floor: u32,
    required_asset_cycles: u128,
    management_creation_fee: u128,
    observation: &FleetObservation,
) -> Result<RootPoolCreationForecast, EnsurePolicyError> {
    let workload_count = desired_initial_root_workloads(desired, root)?;
    let desired_pool = desired_root_pool_policy(desired, root)?;
    let domain = observation
        .estate_funding_domains
        .get(root)
        .ok_or_else(|| EnsurePolicyError::EstateFundingTopology {
            reason: format!("Root {root} has no funding-account observation"),
        })?;
    if let Some(pool) = domain.pool.as_ref() {
        return forecast_observed_root_pool(
            root,
            pool,
            desired_pool,
            workload_count,
            ready_floor,
            required_asset_cycles,
            management_creation_fee,
        );
    }
    forecast_configured_root_pool(
        desired,
        observation,
        root,
        workload_count,
        ready_floor,
        required_asset_cycles,
        desired_pool.maximum_size,
    )
}

fn desired_root_pool_policy<'a>(
    desired: &'a DesiredFleet,
    root: &str,
) -> Result<&'a FleetSubnetCanisterPoolConfig, EnsurePolicyError> {
    desired
        .bootstrap
        .as_ref()
        .and_then(|bootstrap| {
            bootstrap
                .roots
                .iter()
                .find(|candidate| candidate.root == root)
        })
        .map(|root| &root.limits.canister_pool)
        .ok_or_else(|| EnsurePolicyError::EstateFundingTopology {
            reason: format!("Root {root} has no desired pool authority"),
        })
}

fn forecast_configured_root_pool(
    desired: &DesiredFleet,
    observation: &FleetObservation,
    root: &str,
    workload_count: u32,
    ready_floor: u32,
    required_asset_cycles: u128,
    maximum_size: u32,
) -> Result<RootPoolCreationForecast, EnsurePolicyError> {
    let (reusable_ready_assets, completed_workloads) = desired
        .canisters
        .iter()
        .filter(|canister| {
            canister.kind == DesiredCanisterKind::Pool
                && canister.parent.as_deref() == Some(root)
                && canister.presence == DesiredPresence::Present
        })
        .try_fold(
            (0_u32, 0_u32),
            |(ready, workload), canister| -> Result<(u32, u32), EnsurePolicyError> {
                let live = observation
                    .canisters
                    .get(&canister.name)
                    .and_then(Option::as_ref);
                match live.map(|live| (live.root_owned_lifecycle, live.cycles)) {
                    None => Ok((
                        ready
                            .checked_add(1)
                            .ok_or(EnsurePolicyError::ArithmeticOverflow {
                                field: "planned Ready-pool asset count",
                            })?,
                        workload,
                    )),
                    Some((Some(RootOwnedCanisterLifecycle::Idle), cycles))
                        if cycles >= required_asset_cycles =>
                    {
                        Ok((
                            ready
                                .checked_add(1)
                                .ok_or(EnsurePolicyError::ArithmeticOverflow {
                                    field: "reusable Ready-pool asset count",
                                })?,
                            workload,
                        ))
                    }
                    Some((
                        Some(
                            RootOwnedCanisterLifecycle::Claimed
                            | RootOwnedCanisterLifecycle::Workload,
                        ),
                        _,
                    )) => Ok((
                        ready,
                        workload
                            .checked_add(1)
                            .ok_or(EnsurePolicyError::ArithmeticOverflow {
                                field: "completed Root workload count",
                            })?,
                    )),
                    _ => Ok((ready, workload)),
                }
            },
        )?;
    let required_creation_count = required_estate_creation_count(
        workload_count,
        ready_floor,
        reusable_ready_assets,
        completed_workloads,
    )?;
    let occupied_assets = reusable_ready_assets
        .checked_add(completed_workloads)
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "configured Root pool asset count",
        })?;
    Ok(RootPoolCreationForecast {
        allocated_workloads: completed_workloads,
        available_slots: maximum_size.saturating_sub(occupied_assets),
        eligible_ready_assets: reusable_ready_assets,
        maximum_size,
        occupied_assets,
        pending_creations: 0,
        pending_creation: None,
        planned_workloads: workload_count,
        required_creation_count,
    })
}

fn forecast_observed_root_pool(
    root: &str,
    pool: &crate::fleet_ensure::model::EstatePoolInventoryObservation,
    desired_pool: &FleetSubnetCanisterPoolConfig,
    workload_count: u32,
    ready_floor: u32,
    required_asset_cycles: u128,
    management_creation_fee: u128,
) -> Result<RootPoolCreationForecast, EnsurePolicyError> {
    let pending = pending_pool_creation_forecast(
        root,
        pool,
        desired_pool,
        required_asset_cycles,
        management_creation_fee,
    )?;
    let counts = observed_pool_counts(pool, pending.unmaterialized, required_asset_cycles)?;
    let available_supply = counts
        .eligible_ready_assets
        .checked_add(pending.unmaterialized)
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "available Root pool supply",
        })?;
    let required_additional_creations = required_estate_creation_count(
        workload_count,
        ready_floor,
        available_supply,
        counts.allocated_workloads,
    )?;
    let required_creation_count = required_additional_creations
        .checked_add(pending.debit)
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "required Root pool creation debit count",
        })?;
    let available_slots = desired_pool
        .maximum_size
        .saturating_sub(counts.occupied_assets);
    let inventory_exceeds_capacity = counts.occupied_assets > desired_pool.maximum_size;
    let demand_exceeds_capacity = required_additional_creations > available_slots;
    if inventory_exceeds_capacity || demand_exceeds_capacity {
        return Err(EnsurePolicyError::EstatePoolCapacity {
            allocated_workloads: counts.allocated_workloads,
            available_slots,
            capacity_shortfall: required_additional_creations.saturating_sub(available_slots),
            eligible_ready_assets: counts.eligible_ready_assets,
            maximum_size: desired_pool.maximum_size,
            occupied_assets: counts.occupied_assets,
            pending_creations: pending.unmaterialized,
            required_creation_count: required_additional_creations,
            root: root.to_string(),
        });
    }
    Ok(RootPoolCreationForecast {
        allocated_workloads: counts.allocated_workloads,
        available_slots,
        eligible_ready_assets: counts.eligible_ready_assets,
        maximum_size: desired_pool.maximum_size,
        occupied_assets: counts.occupied_assets,
        pending_creations: pending.unmaterialized,
        pending_creation: pool.pending_creation.clone(),
        planned_workloads: workload_count,
        required_creation_count,
    })
}

#[derive(Clone, Copy)]
struct PendingPoolCreationForecast {
    debit: u32,
    unmaterialized: u32,
}

fn pending_pool_creation_forecast(
    root: &str,
    pool: &crate::fleet_ensure::model::EstatePoolInventoryObservation,
    desired: &FleetSubnetCanisterPoolConfig,
    required_asset_cycles: u128,
    management_creation_fee: u128,
) -> Result<PendingPoolCreationForecast, EnsurePolicyError> {
    let Some(pending) = pool.pending_creation.as_ref() else {
        return Ok(PendingPoolCreationForecast {
            debit: 0,
            unmaterialized: 0,
        });
    };
    if pending.uncertain_result {
        return Err(EnsurePolicyError::EstateFundingTopology {
            reason: format!(
                "Root {root} has an unresolved creation response; reconcile that exact operation before funding"
            ),
        });
    }
    let expected_amount = required_asset_cycles
        .checked_add(desired.creation_execution_margin.to_u128())
        .and_then(|amount| amount.checked_add(management_creation_fee))
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "pending estate creation amount",
        })?;
    if pending.creation_amount_cycles != expected_amount {
        return Err(EnsurePolicyError::EstateFundingTopology {
            reason: format!(
                "Root {root} pending creation amount {} differs from current authority {expected_amount}",
                pending.creation_amount_cycles
            ),
        });
    }
    let materialized = pending.created_principal.as_ref().is_some_and(|principal| {
        pool.assets
            .iter()
            .any(|asset| &asset.principal == principal)
    });
    Ok(PendingPoolCreationForecast {
        debit: u32::from(pending.created_principal.is_none()),
        unmaterialized: u32::from(!materialized),
    })
}

#[derive(Clone, Copy)]
struct ObservedPoolCounts {
    allocated_workloads: u32,
    eligible_ready_assets: u32,
    occupied_assets: u32,
}

fn observed_pool_counts(
    pool: &crate::fleet_ensure::model::EstatePoolInventoryObservation,
    pending_unmaterialized: u32,
    required_asset_cycles: u128,
) -> Result<ObservedPoolCounts, EnsurePolicyError> {
    let bounded_count = |count, field| {
        u32::try_from(count).map_err(|_| EnsurePolicyError::ArithmeticOverflow { field })
    };
    let asset_count = bounded_count(pool.assets.len(), "observed Root pool asset count")?;
    let occupied_assets = asset_count.checked_add(pending_unmaterialized).ok_or(
        EnsurePolicyError::ArithmeticOverflow {
            field: "observed Root pool capacity",
        },
    )?;
    let eligible_ready_assets = bounded_count(
        pool.assets
            .iter()
            .filter(|asset| {
                asset.lifecycle == EstatePoolAssetLifecycle::Ready
                    && asset.cycles >= required_asset_cycles
            })
            .count(),
        "eligible Ready-pool asset count",
    )?;
    let allocated_workloads = bounded_count(
        pool.assets
            .iter()
            .filter(|asset| {
                matches!(
                    asset.lifecycle,
                    EstatePoolAssetLifecycle::Claimed | EstatePoolAssetLifecycle::Workload
                )
            })
            .count(),
        "allocated Root workload count",
    )?;
    Ok(ObservedPoolCounts {
        allocated_workloads,
        eligible_ready_assets,
        occupied_assets,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RootPoolCreationForecast {
    allocated_workloads: u32,
    available_slots: u32,
    eligible_ready_assets: u32,
    maximum_size: u32,
    occupied_assets: u32,
    pending_creations: u32,
    pending_creation: Option<crate::fleet_ensure::model::EstatePoolPendingCreationObservation>,
    planned_workloads: u32,
    required_creation_count: u32,
}

fn desired_initial_root_workloads(
    desired: &DesiredFleet,
    root: &str,
) -> Result<u32, EnsurePolicyError> {
    let bootstrap = desired
        .bootstrap
        .as_ref()
        .expect("estate funding is compiled only with bootstrap authority");
    let protocol =
        desired
            .protocol
            .as_ref()
            .ok_or_else(|| EnsurePolicyError::EstateFundingTopology {
                reason: "estate workload forecast requires typed Fleet protocol".to_string(),
            })?;
    let configuration = &bootstrap.component_deployment_configuration;
    protocol
        .component_group_placements
        .iter()
        .filter(|placement| placement.root == root)
        .try_fold(0_u32, |total, placement| {
            let deployment = configuration
                .deployment_topology
                .component_group_deployments
                .iter()
                .find(|deployment| deployment.deployment.as_str() == placement.deployment)
                .ok_or_else(|| EnsurePolicyError::EstateFundingTopology {
                    reason: format!(
                        "Root {root} placement references unknown deployment {}",
                        placement.deployment
                    ),
                })?;
            deployment.members.iter().try_fold(total, |total, member| {
                let spec = configuration
                    .component_topology
                    .component_specs
                    .iter()
                    .find(|spec| spec.component_spec == member.component_spec)
                    .ok_or_else(|| EnsurePolicyError::EstateFundingTopology {
                        reason: format!(
                            "Root {root} deployment references unknown Component Spec {}",
                            member.component_spec
                        ),
                    })?;
                total.checked_add(initial_role_tree_size(spec)?).ok_or(
                    EnsurePolicyError::ArithmeticOverflow {
                        field: "estate initial workload count",
                    },
                )
            })
        })
}

fn required_estate_creation_count(
    workload_count: u32,
    ready_floor: u32,
    reusable_ready_assets: u32,
    completed_workloads: u32,
) -> Result<u32, EnsurePolicyError> {
    workload_count
        .saturating_sub(completed_workloads)
        .checked_add(ready_floor)
        .map(|required_assets| required_assets.saturating_sub(reusable_ready_assets))
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "estate creation count",
        })
}

fn initial_role_tree_size(
    spec: &canic_core::control_plane_support::config::ComponentSpec,
) -> Result<u32, EnsurePolicyError> {
    let mut instances = BTreeMap::from([(spec.component_role.clone(), 1_u32)]);
    let mut total = 0_u32;
    for _ in 0..=spec.spawn_grants.len() {
        let mut advanced = false;
        for grant in &spec.spawn_grants {
            if instances.contains_key(&grant.child_role) {
                continue;
            }
            let Some(parent_instances) = instances.get(&grant.parent_role).copied() else {
                continue;
            };
            let child_instances = parent_instances
                .checked_mul(grant.initial_instances_per_parent)
                .ok_or(EnsurePolicyError::ArithmeticOverflow {
                    field: "estate initial child count",
                })?;
            instances.insert(grant.child_role.clone(), child_instances);
            advanced = true;
        }
        if !advanced {
            total = instances.values().try_fold(0_u32, |sum, count| {
                sum.checked_add(*count)
                    .ok_or(EnsurePolicyError::ArithmeticOverflow {
                        field: "estate initial workload count",
                    })
            })?;
            break;
        }
    }
    if total == 0 {
        return Err(EnsurePolicyError::EstateFundingTopology {
            reason: format!(
                "Component Spec {} initial child graph did not converge",
                spec.component_spec
            ),
        });
    }
    Ok(total)
}

fn tranche_protocol_actions(
    desired: &DesiredFleet,
    observation: &FleetObservation,
    bounds: CycleBounds,
    observed_estate_cycles: u128,
    accumulator: &mut PlanAccumulator,
    protocol_actions: &mut Vec<EnsureAction>,
) -> Result<(), EnsurePolicyError> {
    let candidates = std::mem::take(protocol_actions);
    let mut selected = Vec::with_capacity(candidates.len());
    for action in candidates {
        let execution_burn = accumulator.execution_burn;
        let transfers = accumulator.transfers;

        account_protocol_action(accumulator, &action)?;
        selected.push(action);

        let observation_count = maximum_observation_count(
            desired,
            &accumulator.canisters,
            &selected,
            observation.additional_controlled_cycles.len(),
        )?;
        let observation_burn = bounds
            .observation_burn
            .checked_mul(observation_count)
            .ok_or(EnsurePolicyError::ArithmeticOverflow {
                field: "observation burn",
            })?;
        let required = checked_add(
            accumulator.execution_burn,
            observation_burn,
            "cycle-conservation required burn",
        )?;
        let available = checked_add(
            observed_estate_cycles,
            accumulator.new_funding,
            "cycle-conservation available balance",
        )?;
        if required > available {
            let error =
                insufficient_cycle_conservation(accumulator, selected.len(), available, required);
            accumulator.execution_burn = execution_burn;
            accumulator.transfers = transfers;
            selected.pop();
            if selected.is_empty() {
                return Err(error);
            }
            break;
        }
    }
    *protocol_actions = selected;
    Ok(())
}

fn account_protocol_action(
    accumulator: &mut PlanAccumulator,
    action: &EnsureAction,
) -> Result<(), EnsurePolicyError> {
    match action {
        EnsureAction::FleetProtocol {
            maximum_execution_burn_cycles,
            ..
        }
        | EnsureAction::Protocol {
            maximum_execution_burn_cycles,
            ..
        } => accumulator.add_burn(*maximum_execution_burn_cycles),
        action => Err(EnsurePolicyError::InvalidProtocolStep(
            action.name().to_string(),
        )),
    }
}

fn insufficient_cycle_conservation(
    accumulator: &PlanAccumulator,
    protocol_action_count: usize,
    available: u128,
    required: u128,
) -> EnsurePolicyError {
    let canister_action_count = accumulator
        .canisters
        .iter()
        .map(|canister| canister.actions.len())
        .sum::<usize>();
    EnsurePolicyError::InsufficientCycleConservation {
        action_count: canister_action_count.saturating_add(protocol_action_count),
        available,
        required,
        shortfall: required.saturating_sub(available),
    }
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
                EnsureAction::Fund { .. }
                | EnsureAction::FundEstate { .. }
                | EnsureAction::Transfer { .. } => 2,
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
        component_provisioning_observation_bound(&request.plan.batches)
            .map(|current_bound| bound.max(current_bound))
    })?;
    Ok(observed_bound.max(planned_bound))
}

/// Pure pending-observation policy for one exact reviewed effect.
pub(super) struct EffectObservationPolicy {
    pub maximum_stalled_observations: u32,
    pub paced: bool,
}

/// Keep ordinary effects on the configured bound while giving the one
/// long-running protocol operation a topology-derived, globally bounded lane.
pub(super) fn effect_observation_policy(
    desired: &DesiredFleet,
    action: &EnsureAction,
) -> Result<EffectObservationPolicy, EnsurePolicyError> {
    let EnsureAction::FleetProtocol {
        action: current, ..
    } = action
    else {
        return Ok(EffectObservationPolicy {
            maximum_stalled_observations: desired.maximum_stalled_observations,
            paced: false,
        });
    };
    let crate::fleet_ensure::model::CurrentFleetProtocolAction::ProvisionComponents {
        request, ..
    } = current.as_ref()
    else {
        return Ok(EffectObservationPolicy {
            maximum_stalled_observations: desired.maximum_stalled_observations,
            paced: false,
        });
    };
    let topology_bound = component_provisioning_observation_bound(&request.plan.batches)?;
    Ok(EffectObservationPolicy {
        maximum_stalled_observations: paced_protocol_stall_limit(
            desired.maximum_stalled_observations,
            topology_bound,
        ),
        paced: true,
    })
}

const MAXIMUM_PACED_PROTOCOL_STALL_OBSERVATIONS: u32 = 64;

fn paced_protocol_stall_limit(configured: u32, topology_bound: u128) -> u32 {
    let topology_bound = u32::try_from(topology_bound)
        .unwrap_or(MAXIMUM_PACED_PROTOCOL_STALL_OBSERVATIONS)
        .min(MAXIMUM_PACED_PROTOCOL_STALL_OBSERVATIONS);
    configured.max(topology_bound)
}

fn component_provisioning_observation_bound(
    batches: &[canic_core::dto::component_provisioning::FleetSubnetRootProvisioningBatch],
) -> Result<u128, EnsurePolicyError> {
    let root_count =
        u128::try_from(batches.len()).map_err(|_| EnsurePolicyError::ArithmeticOverflow {
            field: "terminal protocol observation count",
        })?;
    let component_count = batches.iter().try_fold(0_u128, |total, batch| {
        batch
            .placements
            .iter()
            .try_fold(total, |subtotal, placement| {
                u128::try_from(placement.entries.len())
                    .ok()
                    .and_then(|count| subtotal.checked_add(count))
                    .ok_or(EnsurePolicyError::ArithmeticOverflow {
                        field: "terminal protocol observation count",
                    })
            })
    })?;
    component_provisioning_observation_bound_from_counts(root_count, component_count)
}

fn component_provisioning_observation_bound_from_counts(
    root_count: u128,
    component_count: u128,
) -> Result<u128, EnsurePolicyError> {
    let per_component = terminal_initial_component_observation_count(0);
    root_count
        .checked_mul(5)
        .and_then(|root_observations| {
            component_count
                .checked_mul(per_component)
                .and_then(|component_observations| {
                    root_observations.checked_add(component_observations)
                })
        })
        .and_then(|total| total.checked_add(1))
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "terminal protocol observation count",
        })
}

const fn terminal_initial_component_observation_count(_future_descendant_capacity: u32) -> u128 {
    // The fresh-estate terminal walk observes only the provisioned top-level
    // Component here. Future descendant capacity bounds later pagination; it
    // does not multiply the initial Component proof.
    3
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
        .parse::<Cycles>()
        .map(|cycles| cycles.to_u128())
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
        .parse::<Cycles>()
        .map(|cycles| cycles.to_u128())
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

#[cfg(test)]
mod tests {
    use super::{
        EnsurePolicyError, PlanAccumulator, append_estate_funding_actions,
        component_provisioning_observation_bound_from_counts, forecast_observed_root_pool,
        initial_role_tree_size, paced_protocol_stall_limit, required_estate_creation_count,
        terminal_initial_component_observation_count, validate_bootstrap_root_pool_import_capacity,
    };
    use crate::fleet_ensure::model::{
        CanisterDisposition, CanisterPlan, DesiredFleetBootstrapRoot, EnsureAction,
        EstateFundingDomainPlan, EstatePoolAssetLifecycle, EstatePoolAssetObservation,
        EstatePoolAssetOrigin, EstatePoolInventoryObservation,
        EstatePoolPendingCreationObservation,
    };
    use candid::Principal;
    use canic_core::{
        cdk::types::Cycles,
        control_plane_support::config::{ComponentLimits, ComponentSpawnGrant, ComponentSpec},
        ids::{
            CanisterRole, ComponentSpecId, ComponentTopologyDigest, CyclesFundingBudget,
            FleetSubnetCanisterPoolConfig, FleetSubnetRootLimits, SubnetId,
        },
    };

    #[test]
    fn estate_workload_forecast_includes_recursive_initial_children() {
        let spec = ComponentSpec {
            component_spec: ComponentSpecId::try_from(String::from("hub"))
                .expect("hub Component Spec ID"),
            spec_hash: [1; 32],
            component_role: CanisterRole::from("hub"),
            maximum_fleet_instances: 1,
            limits: ComponentLimits {
                maximum_descendants: 8,
                maximum_registry_bytes: 1,
                cycles_funding: CyclesFundingBudget {
                    window_secs: 1,
                    maximum_cycles: Cycles::new(1),
                },
            },
            children: Vec::new(),
            spawn_grants: vec![
                ComponentSpawnGrant {
                    parent_role: CanisterRole::from("hub"),
                    child_role: CanisterRole::from("shard"),
                    initial_instances_per_parent: 2,
                    maximum_instances_per_parent: 2,
                },
                ComponentSpawnGrant {
                    parent_role: CanisterRole::from("shard"),
                    child_role: CanisterRole::from("leaf"),
                    initial_instances_per_parent: 3,
                    maximum_instances_per_parent: 3,
                },
            ],
        };

        assert_eq!(initial_role_tree_size(&spec), Ok(9));
    }

    #[test]
    fn estate_creation_forecast_retains_ready_floor_without_repeating_workloads() {
        assert_eq!(required_estate_creation_count(8, 10, 10, 0), Ok(8));
        assert_eq!(required_estate_creation_count(8, 10, 10, 8), Ok(0));
        assert_eq!(required_estate_creation_count(8, 2, 2, 0), Ok(8));
        assert_eq!(required_estate_creation_count(8, 2, 10, 0), Ok(0));
    }

    #[test]
    fn estate_shortfall_compiles_one_exact_plan_owned_ledger_transfer() {
        let mut accumulator = PlanAccumulator::new();
        accumulator.canisters.push(CanisterPlan {
            actions: Vec::new(),
            disposition: CanisterDisposition::Reuse,
            name: "root-0".to_string(),
            observed_cycles: 30,
            principal: Some("rrkah-fqaaa-aaaaa-aaaaq-cai".to_string()),
        });
        let domain = EstateFundingDomainPlan {
            allocated_workloads: 1,
            available_cycles: Some(40),
            available_pool_slots: 1,
            creation_amount_cycles: 45,
            creation_execution_margin_cycles: 5,
            readiness_floor_cycles: 35,
            cycles_ledger: "aaaaa-aa".to_string(),
            eligible_ready_pool_assets: 1,
            initial_pool_assets: vec!["existing-pool".to_string()],
            ledger_fee_cycles: 5,
            management_creation_fee_cycles: 5,
            maximum_creation_debit_cycles: 100,
            maximum_creation_fee_cycles: 20,
            maximum_funding_cycles: 60,
            occupied_pool_assets: 1,
            pending_creation_count: 0,
            pending_creation: None,
            planned_initial_workloads: 2,
            pool_maximum_size: 2,
            required_creation_count: 2,
            root: "root-0".to_string(),
            root_principal: Some("rrkah-fqaaa-aaaaa-aaaaq-cai".to_string()),
            shortfall_cycles: 60,
        };

        append_estate_funding_actions(&[domain], 100, 4, &mut accumulator)
            .expect("compile exact estate funding action");

        assert_eq!(accumulator.new_funding, 60);
        assert_eq!(accumulator.fees, 5);
        assert!(matches!(
            accumulator.canisters[0].actions.as_slice(),
            [EnsureAction::FundEstate {
                amount: 60,
                created_at_time: 104,
                expected_post_cycles: 100,
                ledger,
                ledger_fee_cycles: 5,
                name,
                principal,
            }] if ledger == "aaaaa-aa"
                && name == "root-0"
                && principal == "rrkah-fqaaa-aaaaa-aaaaq-cai"
        ));
    }

    #[test]
    fn complete_pool_inventory_owns_creation_capacity_forecast() {
        let policy = pool_policy(4, 8);
        let exhausted = observed_pool([
            EstatePoolAssetLifecycle::Workload,
            EstatePoolAssetLifecycle::Workload,
            EstatePoolAssetLifecycle::Workload,
            EstatePoolAssetLifecycle::Workload,
            EstatePoolAssetLifecycle::Failed,
            EstatePoolAssetLifecycle::Failed,
            EstatePoolAssetLifecycle::Failed,
            EstatePoolAssetLifecycle::Failed,
        ]);
        assert!(matches!(
            forecast_observed_root_pool("root", &exhausted, &policy, 4, 4, 1_900, 500),
            Err(EnsurePolicyError::EstatePoolCapacity {
                allocated_workloads: 4,
                available_slots: 0,
                capacity_shortfall: 4,
                eligible_ready_assets: 0,
                maximum_size: 8,
                occupied_assets: 8,
                pending_creations: 0,
                required_creation_count: 4,
                ..
            })
        ));

        let recoverable = observed_pool([
            EstatePoolAssetLifecycle::Workload,
            EstatePoolAssetLifecycle::Workload,
            EstatePoolAssetLifecycle::Workload,
            EstatePoolAssetLifecycle::Workload,
            EstatePoolAssetLifecycle::Ready,
            EstatePoolAssetLifecycle::Ready,
        ]);
        let forecast = forecast_observed_root_pool("root", &recoverable, &policy, 4, 4, 1_900, 500)
            .expect("two free slots can restore the four-Ready floor");
        assert_eq!(forecast.occupied_assets, 6);
        assert_eq!(forecast.eligible_ready_assets, 2);
        assert_eq!(forecast.required_creation_count, 2);
        assert_eq!(forecast.available_slots, 2);
    }

    #[test]
    fn every_retained_pool_lifecycle_and_pending_creation_consumes_capacity_once() {
        let policy = pool_policy(2, 10);
        let mut observed = observed_pool([
            EstatePoolAssetLifecycle::Claimed,
            EstatePoolAssetLifecycle::Failed,
            EstatePoolAssetLifecycle::HandingOff,
            EstatePoolAssetLifecycle::PendingReset,
            EstatePoolAssetLifecycle::Ready,
            EstatePoolAssetLifecycle::Recycling,
            EstatePoolAssetLifecycle::Workload,
        ]);
        observed.minimum_size = 2;
        observed.maximum_size = 10;
        observed.pending_creation = Some(EstatePoolPendingCreationObservation {
            attempt_count: 1,
            available_cycles: None,
            creation_amount_cycles: 2_500,
            created_principal: None,
            diagnostic: None,
            last_attempt_at_ns: Some(1),
            operation_id: "11".repeat(32),
            required_cycles: None,
            retry_at_ns: Some(2),
            shortfall_cycles: Some(2_500),
            uncertain_result: false,
        });
        let forecast = forecast_observed_root_pool("root", &observed, &policy, 2, 2, 1_900, 500)
            .expect("all bounded lifecycles fit with one free slot");
        assert_eq!(forecast.occupied_assets, 8);
        assert_eq!(forecast.allocated_workloads, 2);
        assert_eq!(forecast.eligible_ready_assets, 1);
        assert_eq!(forecast.pending_creations, 1);
        assert_eq!(forecast.required_creation_count, 1);

        observed.maximum_size = 9;
        let drifted = forecast_observed_root_pool("root", &observed, &policy, 2, 2, 1_900, 500)
            .expect("observed policy drift retains exact assets under desired limits");
        assert_eq!(drifted, forecast);
    }

    #[test]
    fn unresolved_pool_creation_rejects_before_funding_authority() {
        let policy = pool_policy(1, 2);
        let mut observed = observed_pool([]);
        observed.pending_creation = Some(EstatePoolPendingCreationObservation {
            attempt_count: 1,
            available_cycles: None,
            creation_amount_cycles: 2_500,
            created_principal: None,
            diagnostic: None,
            last_attempt_at_ns: Some(1),
            operation_id: "11".repeat(32),
            required_cycles: None,
            retry_at_ns: None,
            shortfall_cycles: None,
            uncertain_result: true,
        });
        assert!(matches!(
            forecast_observed_root_pool("root", &observed, &policy, 1, 1, 1_900, 500),
            Err(EnsurePolicyError::EstateFundingTopology { reason })
                if reason.contains("unresolved creation response")
        ));
    }

    fn pool_policy(minimum_size: u32, maximum_size: u32) -> FleetSubnetCanisterPoolConfig {
        FleetSubnetCanisterPoolConfig {
            minimum_size,
            maximum_size,
            canister_cycles: Cycles::new(1_900),
            creation_execution_margin: Cycles::new(100),
        }
    }

    fn observed_pool(
        lifecycles: impl IntoIterator<Item = EstatePoolAssetLifecycle>,
    ) -> EstatePoolInventoryObservation {
        EstatePoolInventoryObservation {
            assets: lifecycles
                .into_iter()
                .enumerate()
                .map(|(index, lifecycle)| EstatePoolAssetObservation {
                    creation_receipt: None,
                    cycles: 1_900,
                    lifecycle,
                    origin: EstatePoolAssetOrigin::Imported,
                    principal: format!("asset-{index}"),
                })
                .collect(),
            maximum_size: 8,
            minimum_size: 4,
            pending_creation: None,
            readiness_floor_cycles: 1_900,
            creation_execution_margin_cycles: 100,
        }
    }

    #[test]
    fn toko_fresh_fleet_descendant_capacity_does_not_multiply_terminal_proof() {
        for maximum_descendants in [0, 1, 10_000, u32::MAX] {
            assert_eq!(
                terminal_initial_component_observation_count(maximum_descendants),
                3,
            );
        }
    }

    #[test]
    fn component_provisioning_stall_floor_scales_and_caps() {
        let topology_bound = component_provisioning_observation_bound_from_counts(1, 11)
            .expect("bounded one-Root eleven-Component topology");
        assert_eq!(paced_protocol_stall_limit(8, topology_bound), 39);
        assert_eq!(paced_protocol_stall_limit(8, 10_000), 64);
        assert_eq!(paced_protocol_stall_limit(80, topology_bound), 80);
    }

    #[test]
    fn apply_policy_rejects_bootstrap_imports_above_the_root_maximum() {
        let root = DesiredFleetBootstrapRoot {
            canister_pool_imports: vec![
                "pool-0".to_string(),
                "pool-1".to_string(),
                "pool-2".to_string(),
            ],
            component_admissions: Vec::new(),
            component_topology_digest: ComponentTopologyDigest::from_bytes([1; 32]),
            funding: crate::test_support::fleet_subnet_root_funding_authority(),
            limits: FleetSubnetRootLimits {
                maximum_component_instances: 1,
                maximum_registry_bytes: 1,
                maximum_wasm_store_bytes: 1,
                canister_pool: FleetSubnetCanisterPoolConfig {
                    minimum_size: 2,
                    maximum_size: 2,
                    canister_cycles: Cycles::new(1),
                    creation_execution_margin: Cycles::new(1),
                },
                cycles_funding: CyclesFundingBudget {
                    window_secs: 1,
                    maximum_cycles: Cycles::new(1),
                },
                maximum_group_placements: 1,
            },
            placement_subnet: SubnetId::from_principal(Principal::from_slice(&[2])),
            root: "root-0".to_string(),
            store: "store-0".to_string(),
        };

        assert!(matches!(
            validate_bootstrap_root_pool_import_capacity(&root),
            Err(EnsurePolicyError::PoolImportCapacity(error))
                if error.import_count == 3
                    && error.maximum_size == 2
                    && error.root == "root-0"
        ));
    }
}
