//! Module: cycles::funding
//!
//! Responsibility: render protected Coordinator/Root funding diagnostics for one installed Fleet.
//! Does not own: funding decisions, installed authority, or value-transfer mutation.
//! Boundary: verified host authority selects exact infrastructure targets and protocol bindings.

use crate::{
    cli::{
        clap::{flag_arg, parse_matches, render_usage, required_string, string_option, value_arg},
        globals::{internal_environment_arg, internal_icp_arg},
    },
    cycles::CyclesCommandError,
    support::icp_target::IcpTargetOptions,
};
use candid::{CandidType, Reserved, decode_one};
use canic_core::{
    cdk::types::{Cycles, Principal},
    cdk::utils::hash::{decode_hex, hex_bytes},
    dto::{
        error::Error,
        fleet_funding::{
            FleetFundingPolicyRotationApplyRequest, FleetFundingPolicyRotationBeginRequest,
            FleetFundingPolicyRotationFundingSource, FleetFundingPolicyRotationPlacementEvidence,
            FleetFundingPolicyRotationPlan, FleetFundingPolicyRotationPlanHeader,
            FleetFundingPolicyRotationRootPlan, FleetFundingPolicyRotationStageRootRequest,
            FleetFundingPolicyUsage, FleetRootFundingRequest, FleetRootFundingResponse,
        },
        fleet_registry::{FleetRegistryVersion, FleetSubnetRootStatus},
        icp_refill::{IcpRefillResponse, IcpRefillTrigger},
        role::OperationReceipt,
    },
    ids::{
        FleetCoordinatorRootFundingPolicy, FleetFundingProfile, FleetSubnetRootFundingPolicy,
        FleetSubnetRootIcpRefillPolicy,
    },
    shared_support::fleet_funding_policy::{
        coordinator_root_funding_policy_hash, fleet_funding_policy_rotation_operation_id,
        fleet_funding_policy_rotation_plan_digest, fleet_funding_policy_rotation_roots_digest,
        fleet_subnet_root_funding_policy_hash, validate_fleet_funding_policy_rotation_plan,
    },
};
use canic_host::{
    call_canister_with_arg,
    durable_io::write_bytes,
    format::cycles_tc,
    icp_config::resolve_current_canic_icp_root,
    installed_fleet::{
        InstalledFleetFundingResolution, InstalledFleetRequest,
        InstalledFleetRootFundingResolution, resolve_installed_fleet_funding_from_root,
    },
    protocol_binding::{ResolvedProtocolBinding, resolve_infrastructure_protocol_binding},
    query_canister_with_arg,
    release_set::{CanicInfrastructureRole, load_persisted_canic_infrastructure_artifact_manifest},
};
use clap::Command as ClapCommand;
use serde::{Deserialize, Serialize};
use std::{ffi::OsString, fs, path::PathBuf};

const FLEET_ARG: &str = "fleet";
const JSON_ARG: &str = "json";
const APPLY_ROTATION_ARG: &str = "apply-rotation";
const PLAN_ROTATION_ARG: &str = "plan-rotation";

#[derive(Clone, Debug, Eq, PartialEq)]
struct FundingOptions {
    target: IcpTargetOptions,
    fleet: String,
    json: bool,
    apply_rotation: Option<PathBuf>,
    plan_rotation: Option<PathBuf>,
}

#[derive(CandidType, Deserialize)]
enum RemoteCoordinatorStatusResponse {
    Funding(Box<RemoteCoordinatorFundingStatus>),
    RegistryVersion(FleetRegistryVersion),
}

#[derive(CandidType)]
enum RemoteCoordinatorStatusRequest {
    RegistryVersion,
}

#[derive(CandidType)]
enum RemoteCoordinatorCommand {
    ApplyFundingPolicyRotation(FleetFundingPolicyRotationApplyRequest),
    BeginFundingPolicyRotation(Box<FleetFundingPolicyRotationBeginRequest>),
    StageFundingPolicyRotationRoot(Box<FleetFundingPolicyRotationStageRootRequest>),
}

#[derive(CandidType, Deserialize)]
enum RemoteCoordinatorCommandResponse {
    OperationAccepted(OperationReceipt),
}

#[derive(CandidType, Deserialize)]
enum RemoteRootStatusResponse {
    Funding(RemoteRootFundingStatus),
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct RemoteFundingWindowStatus {
    window_start_secs: u64,
    spent_cycles: Cycles,
    reserved_cycles: Cycles,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct RemoteCoordinatorRootFundingStatus {
    fleet_subnet_root: Principal,
    lifecycle_status: FleetSubnetRootStatus,
    policy_hash: [u8; 32],
    policy: FleetSubnetRootFundingPolicy,
    window: RemoteFundingWindowStatus,
    historical_automatic_grants: u64,
    historical_automatic_cycles: Cycles,
    automatic_grants: u32,
    automatic_cycles: Cycles,
    last_successful_grant_at_ns: Option<u64>,
    current_operation: Option<FleetRootFundingRequest>,
    last_result: Option<FleetRootFundingResponse>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
enum RemoteFundingPolicyRotationPhase {
    ActivatingRoots {
        activated_root_count: u32,
        expected_root_count: u32,
        successor_registry: FleetRegistryVersion,
    },
    Completed(Reserved),
    PreparingRoots {
        prepared_root_count: u32,
        expected_root_count: u32,
    },
    Staging {
        staged_root_count: u32,
        expected_root_count: u32,
    },
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct RemoteFundingPolicyRotationStatus {
    operation_id: [u8; 32],
    plan_digest: [u8; 32],
    predecessor_generation: u64,
    successor_generation: u64,
    phase: RemoteFundingPolicyRotationPhase,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct RemoteCoordinatorFundingStatus {
    coordinator: Principal,
    current_cycles: Cycles,
    policy_generation: u64,
    funding_enabled: bool,
    funding_profile: Option<FleetFundingProfile>,
    policy: Option<FleetCoordinatorRootFundingPolicy>,
    fleet_window: Option<RemoteFundingWindowStatus>,
    historical_automatic_grants: u64,
    historical_automatic_cycles: Cycles,
    automatic_grants: u32,
    automatic_cycles: Cycles,
    rotation_checkpoint_count: u32,
    rotation_checkpoint_root_count: u32,
    rotation_checkpoint_root_capacity_remaining: u32,
    rotation: Option<RemoteFundingPolicyRotationStatus>,
    roots: Vec<RemoteCoordinatorRootFundingStatus>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct RemoteRootIcpRefillStatus {
    trigger: IcpRefillTrigger,
    amount_e8s: u64,
    fee_e8s: u64,
    budget_window_start_secs: u64,
    resumable: bool,
    response: IcpRefillResponse,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
struct RemoteRootFundingStatus {
    fleet_subnet_root: Principal,
    lifecycle_status: FleetSubnetRootStatus,
    funding_eligible: bool,
    cycles_funding_enabled: bool,
    current_cycles: Cycles,
    policy_generation: u64,
    funding_profile: FleetFundingProfile,
    policy_hash: [u8; 32],
    root_policy: FleetSubnetRootFundingPolicy,
    current_operation: Option<FleetRootFundingRequest>,
    last_result: Option<FleetRootFundingResponse>,
    historical_automatic_grants: u64,
    historical_automatic_cycles: Cycles,
    automatic_grants: u32,
    automatic_cycles: Cycles,
    rotation_current: Option<Reserved>,
    rotation_last: Option<Reserved>,
    icp_refill_policy: Option<FleetSubnetRootIcpRefillPolicy>,
    icp_window_start_secs: Option<u64>,
    icp_window_reserved_e8s: u64,
    automatic_icp_refills: u32,
    automatic_icp_refill_e8s: u64,
    latest_icp_refill: Option<RemoteRootIcpRefillStatus>,
}

#[derive(Clone, Debug, Serialize)]
struct PlacementReport {
    subnet: String,
    specialization: String,
    node_count: u64,
    cost_multiplier_numerator: u64,
    cost_multiplier_denominator: u64,
    acknowledge_fiduciary_cost: bool,
    warning: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct RootFundingReport {
    canister_id: Principal,
    placement: PlacementReport,
    status: RemoteRootFundingStatus,
    direct_topup_command: String,
}

#[derive(Clone, Debug, Serialize)]
struct FundingReport {
    fleet: String,
    environment: String,
    coordinator_canister_id: Principal,
    coordinator_placement: PlacementReport,
    coordinator: RemoteCoordinatorFundingStatus,
    coordinator_direct_topup_command: String,
    roots: Vec<RootFundingReport>,
}

struct CollectedFunding {
    report: FundingReport,
    installed: InstalledFleetFundingResolution,
    coordinator_binding: ResolvedProtocolBinding,
    icp: canic_host::icp::IcpCli,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FundingPolicyRotationPlanFile {
    schema_version: u16,
    fleet: String,
    environment: String,
    coordinator: Principal,
    operation_id: [u8; 32],
    plan_digest: [u8; 32],
    plan: FleetFundingPolicyRotationPlan,
}

pub(super) fn run(args: Vec<OsString>) -> Result<(), CyclesCommandError> {
    let options = FundingOptions::parse(args)?;
    let collected = collect_funding(&options)?;
    if let Some(path) = options.plan_rotation.as_ref() {
        return write_rotation_plan(&collected, path);
    }
    if let Some(path) = options.apply_rotation.as_ref() {
        return apply_rotation_plan(&options, &collected, path);
    }
    let report = &collected.report;
    if options.json {
        println!("{}", serde_json::to_string_pretty(report)?);
    } else {
        println!("{}", render_report(report));
    }
    Ok(())
}

pub(super) fn usage() -> String {
    render_usage(command)
}

impl FundingOptions {
    fn parse<I>(args: I) -> Result<Self, CyclesCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| CyclesCommandError::Usage(usage()))?;
        Ok(Self {
            target: IcpTargetOptions::parse(&matches),
            fleet: required_string(&matches, FLEET_ARG),
            json: matches.get_flag(JSON_ARG),
            apply_rotation: string_option(&matches, APPLY_ROTATION_ARG).map(PathBuf::from),
            plan_rotation: string_option(&matches, PLAN_ROTATION_ARG).map(PathBuf::from),
        })
    }
}

fn command() -> ClapCommand {
    ClapCommand::new("funding")
        .bin_name("canic cycles funding")
        .about("Inspect protected Coordinator and Root funding headroom")
        .disable_help_flag(true)
        .arg(value_arg(FLEET_ARG).value_name(FLEET_ARG).required(true))
        .arg(
            value_arg(APPLY_ROTATION_ARG)
                .long(APPLY_ROTATION_ARG)
                .value_name("FILE")
                .conflicts_with_all([JSON_ARG, PLAN_ROTATION_ARG]),
        )
        .arg(
            flag_arg(JSON_ARG)
                .long(JSON_ARG)
                .conflicts_with_all([APPLY_ROTATION_ARG, PLAN_ROTATION_ARG]),
        )
        .arg(
            value_arg(PLAN_ROTATION_ARG)
                .long(PLAN_ROTATION_ARG)
                .value_name("FILE")
                .conflicts_with_all([APPLY_ROTATION_ARG, JSON_ARG]),
        )
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
        .after_help(
            "Examples:\n  canic cycles funding demo\n  canic cycles funding demo --plan-rotation funding-plan.json\n  canic cycles funding demo --apply-rotation funding-plan.json",
        )
}

fn collect_funding(options: &FundingOptions) -> Result<CollectedFunding, CyclesCommandError> {
    let root = resolve_current_canic_icp_root().map_err(CyclesCommandError::IcpRoot)?;
    let installed = resolve_installed_fleet_funding_from_root(
        &InstalledFleetRequest {
            fleet: options.fleet.clone(),
            environment: options.target.environment.clone(),
        },
        &root,
    )?;
    let manifest = load_persisted_canic_infrastructure_artifact_manifest(
        &root,
        installed.fleet.release_build_id,
    )
    .map_err(|error| CyclesCommandError::Usage(error.to_string()))?;
    let coordinator_binding = infrastructure_binding(
        &root,
        &options.target.environment,
        &manifest.manifest.entries,
        CanicInfrastructureRole::FleetCoordinator,
    )?;
    let root_binding = infrastructure_binding(
        &root,
        &options.target.environment,
        &manifest.manifest.entries,
        CanicInfrastructureRole::FleetSubnetRoot,
    )?;
    let icp = options.target.icp_cli(&root);
    let coordinator = query_coordinator(
        &icp,
        installed.coordinator_canister_id,
        &coordinator_binding,
    )?;
    validate_coordinator_status(&installed, &coordinator)?;
    let mut roots = Vec::new();
    for installed_root in installed
        .roots
        .iter()
        .filter(|root| root.status != FleetSubnetRootStatus::Removed)
    {
        let status = query_root(&icp, installed_root.fleet_subnet_root, &root_binding)?;
        if !root_status_matches_authority(&status, installed_root, &coordinator) {
            return Err(CyclesCommandError::FundingAuthority(
                "Root funding status conflicts with verified installed authority".to_string(),
            ));
        }
        roots.push(RootFundingReport {
            canister_id: installed_root.fleet_subnet_root,
            placement: placement_report(&installed_root.placement_cost),
            status,
            direct_topup_command: format!(
                "canic cycles topup {} {} <amount>",
                options.fleet, installed_root.fleet_subnet_root
            ),
        });
    }
    let report = FundingReport {
        fleet: options.fleet.clone(),
        environment: options.target.environment.clone(),
        coordinator_canister_id: installed.coordinator_canister_id,
        coordinator_placement: placement_report(&installed.coordinator_placement_cost),
        coordinator,
        coordinator_direct_topup_command: format!(
            "canic cycles topup {} coordinator <amount>",
            options.fleet
        ),
        roots,
    };
    Ok(CollectedFunding {
        report,
        installed,
        coordinator_binding,
        icp,
    })
}

fn write_rotation_plan(
    collected: &CollectedFunding,
    path: &std::path::Path,
) -> Result<(), CyclesCommandError> {
    let (plan, rebound) = if path.try_exists()? {
        let proposal: FundingPolicyRotationPlanFile = serde_json::from_slice(&fs::read(path)?)?;
        (
            build_rotation_plan_from_live(collected, &proposal.plan)?,
            true,
        )
    } else {
        (build_rotation_plan(collected)?, false)
    };
    let mut bytes = serde_json::to_vec_pretty(&plan)?;
    bytes.push(b'\n');
    write_bytes(path, &bytes)?;
    println!(
        "Funding-policy rotation plan {} {}\nOperation: {}\nPlan digest: {}\nApply debit: 0 cycles",
        if rebound { "rebound at" } else { "written to" },
        path.display(),
        hex_bytes(plan.operation_id),
        hex_bytes(plan.plan_digest),
    );
    Ok(())
}

fn apply_rotation_plan(
    options: &FundingOptions,
    collected: &CollectedFunding,
    path: &std::path::Path,
) -> Result<(), CyclesCommandError> {
    let plan_file: FundingPolicyRotationPlanFile = serde_json::from_slice(&fs::read(path)?)?;
    validate_rotation_plan_file(options, collected, &plan_file)?;
    let coordinator = &collected.report.coordinator;
    let active = coordinator.rotation.as_ref();
    if let Some(active) = active
        && (active.operation_id != plan_file.operation_id
            || active.plan_digest != plan_file.plan_digest
            || active.predecessor_generation != plan_file.plan.header.predecessor_generation
            || active.successor_generation != plan_file.plan.header.successor_generation)
    {
        return Err(funding_authority_error(
            "another funding-policy rotation is already active",
        ));
    }

    match coordinator.policy_generation {
        generation if generation == plan_file.plan.header.predecessor_generation => {
            let staging = active.is_none_or(|active| {
                matches!(
                    active.phase,
                    RemoteFundingPolicyRotationPhase::Staging { .. }
                )
            });
            if staging {
                validate_live_predecessor(&plan_file, collected)?;
                call_rotation_command(
                    collected,
                    RemoteCoordinatorCommand::BeginFundingPolicyRotation(Box::new(
                        FleetFundingPolicyRotationBeginRequest {
                            operation_id: plan_file.operation_id,
                            plan_digest: plan_file.plan_digest,
                            header: plan_file.plan.header.clone(),
                        },
                    )),
                )?;
                for root in &plan_file.plan.roots {
                    call_rotation_command(
                        collected,
                        RemoteCoordinatorCommand::StageFundingPolicyRotationRoot(Box::new(
                            FleetFundingPolicyRotationStageRootRequest {
                                operation_id: plan_file.operation_id,
                                plan_digest: plan_file.plan_digest,
                                root: root.clone(),
                            },
                        )),
                    )?;
                }
            }
            call_rotation_apply(collected, &plan_file)?;
        }
        generation if generation == plan_file.plan.header.successor_generation => {
            call_rotation_apply(collected, &plan_file)?;
        }
        _ => {
            return Err(funding_authority_error(
                "rotation plan generation is stale or skips the live generation",
            ));
        }
    }
    println!(
        "Funding-policy rotation accepted\nOperation: {}\nPlan digest: {}\nThe Coordinator will resume the durable Root convergence workflow until terminal.",
        hex_bytes(plan_file.operation_id),
        hex_bytes(plan_file.plan_digest),
    );
    Ok(())
}

fn call_rotation_apply(
    collected: &CollectedFunding,
    plan_file: &FundingPolicyRotationPlanFile,
) -> Result<OperationReceipt, CyclesCommandError> {
    call_rotation_command(
        collected,
        RemoteCoordinatorCommand::ApplyFundingPolicyRotation(
            FleetFundingPolicyRotationApplyRequest {
                operation_id: plan_file.operation_id,
                plan_digest: plan_file.plan_digest,
                expected_predecessor_generation: plan_file.plan.header.predecessor_generation,
            },
        ),
    )
}

fn call_rotation_command(
    collected: &CollectedFunding,
    command: RemoteCoordinatorCommand,
) -> Result<OperationReceipt, CyclesCommandError> {
    let response: Result<RemoteCoordinatorCommandResponse, Error> = call_canister_with_arg(
        &collected.icp,
        &collected.coordinator_binding,
        collected.report.coordinator_canister_id,
        canic_core::protocol::CANIC_COMMAND,
        &command,
    )?;
    match response {
        Ok(RemoteCoordinatorCommandResponse::OperationAccepted(receipt)) => Ok(receipt),
        Err(error) => Err(funding_rejected(error)),
    }
}

fn build_rotation_plan(
    collected: &CollectedFunding,
) -> Result<FundingPolicyRotationPlanFile, CyclesCommandError> {
    build_rotation_plan_inner(collected, true)
}

fn build_rotation_plan_inner(
    collected: &CollectedFunding,
    require_idle: bool,
) -> Result<FundingPolicyRotationPlanFile, CyclesCommandError> {
    let report = &collected.report;
    let coordinator = &report.coordinator;
    if require_idle && coordinator.rotation.is_some() {
        return Err(funding_authority_error(
            "a funding-policy rotation is already active",
        ));
    }
    let predecessor_registry = query_registry_version(collected)?;
    if predecessor_registry.authority.binding.coordinator != report.coordinator_canister_id {
        return Err(funding_authority_error(
            "live Registry authority does not name the installed Coordinator",
        ));
    }
    let proposed_coordinator_policy = coordinator
        .policy
        .clone()
        .ok_or_else(|| funding_authority_error("Coordinator funding policy is unavailable"))?;
    let successor_generation = coordinator
        .policy_generation
        .checked_add(1)
        .ok_or_else(|| funding_authority_error("funding-policy generation is exhausted"))?;
    let roots = rotation_root_plans(collected)?;
    if roots.len()
        > usize::try_from(coordinator.rotation_checkpoint_root_capacity_remaining)
            .map_err(|_| funding_authority_error("rotation checkpoint capacity is invalid"))?
    {
        return Err(funding_authority_error(
            "the bounded funding-policy rotation checkpoint history is exhausted",
        ));
    }
    let mut plan = FleetFundingPolicyRotationPlan {
        header: FleetFundingPolicyRotationPlanHeader {
            predecessor_registry,
            predecessor_generation: coordinator.policy_generation,
            successor_generation,
            predecessor_coordinator_policy_hash: coordinator_root_funding_policy_hash(
                &proposed_coordinator_policy,
            ),
            predecessor_usage: coordinator_usage(coordinator),
            proposed_coordinator_policy: proposed_coordinator_policy.clone(),
            topology_catalog_digest: topology_catalog_digest(&collected.installed)?,
            coordinator_placement: rotation_placement(
                &collected.installed.coordinator_placement_cost,
            ),
            affected_root_count: u32::try_from(roots.len())
                .map_err(|_| funding_authority_error("too many affected Roots"))?,
            roots_digest: [0; 32],
            maximum_new_automatic_cycles: proposed_coordinator_policy.maximum_automatic_cycles,
            apply_operator_debit: Cycles::new(0),
            funding_source: FleetFundingPolicyRotationFundingSource::CoordinatorTreasury,
        },
        roots,
    };
    plan.header.roots_digest = fleet_funding_policy_rotation_roots_digest(&plan.roots);
    let plan_digest = fleet_funding_policy_rotation_plan_digest(&plan);
    let operation_id =
        fleet_funding_policy_rotation_operation_id(report.coordinator_canister_id, plan_digest);
    validate_fleet_funding_policy_rotation_plan(&plan)
        .map_err(|error| funding_authority_error(error.to_string()))?;
    Ok(FundingPolicyRotationPlanFile {
        schema_version: 1,
        fleet: report.fleet.clone(),
        environment: report.environment.clone(),
        coordinator: report.coordinator_canister_id,
        operation_id,
        plan_digest,
        plan,
    })
}

fn validate_rotation_plan_file(
    options: &FundingOptions,
    collected: &CollectedFunding,
    file: &FundingPolicyRotationPlanFile,
) -> Result<(), CyclesCommandError> {
    let computed_digest = fleet_funding_policy_rotation_plan_digest(&file.plan);
    let computed_operation = fleet_funding_policy_rotation_operation_id(
        collected.report.coordinator_canister_id,
        computed_digest,
    );
    if file.schema_version != 1
        || file.fleet != options.fleet
        || file.environment != options.target.environment
        || file.coordinator != collected.report.coordinator_canister_id
        || file.plan.header.roots_digest
            != fleet_funding_policy_rotation_roots_digest(&file.plan.roots)
        || file.plan_digest != computed_digest
        || file.operation_id != computed_operation
    {
        return Err(funding_authority_error(
            "rotation plan identity, digest or installed Fleet binding is invalid",
        ));
    }
    validate_fleet_funding_policy_rotation_plan(&file.plan)
        .map_err(|error| funding_authority_error(error.to_string()))
}

fn validate_live_predecessor(
    file: &FundingPolicyRotationPlanFile,
    collected: &CollectedFunding,
) -> Result<(), CyclesCommandError> {
    let expected = build_rotation_plan_from_live(collected, &file.plan)?;
    if expected.plan != file.plan
        || expected.operation_id != file.operation_id
        || expected.plan_digest != file.plan_digest
    {
        return Err(funding_authority_error(
            "rotation plan no longer matches live policy, usage, topology or Registry evidence",
        ));
    }
    Ok(())
}

fn build_rotation_plan_from_live(
    collected: &CollectedFunding,
    accepted: &FleetFundingPolicyRotationPlan,
) -> Result<FundingPolicyRotationPlanFile, CyclesCommandError> {
    let mut current = build_rotation_plan_inner(collected, false)?;
    if accepted.roots.len() != current.plan.roots.len()
        || accepted
            .header
            .proposed_coordinator_policy
            .budget
            .window_secs
            != current
                .plan
                .header
                .proposed_coordinator_policy
                .budget
                .window_secs
    {
        return Err(funding_authority_error(
            "proposal changes the affected-Root set or retained Coordinator window",
        ));
    }
    current.plan.header.proposed_coordinator_policy =
        accepted.header.proposed_coordinator_policy.clone();
    current.plan.header.maximum_new_automatic_cycles = current
        .plan
        .header
        .proposed_coordinator_policy
        .maximum_automatic_cycles
        .clone();
    for (current_root, accepted_root) in current.plan.roots.iter_mut().zip(&accepted.roots) {
        if current_root.fleet_subnet_root != accepted_root.fleet_subnet_root
            || current_root.proposed_policy.cooldown_secs
                != accepted_root.proposed_policy.cooldown_secs
            || current_root.proposed_policy.budget.window_secs
                != accepted_root.proposed_policy.budget.window_secs
        {
            return Err(funding_authority_error(
                "proposal changes a Root identity, cooldown or retained accounting window",
            ));
        }
        current_root.proposed_policy = accepted_root.proposed_policy.clone();
    }
    current.plan.header.roots_digest =
        fleet_funding_policy_rotation_roots_digest(&current.plan.roots);
    current.plan_digest = fleet_funding_policy_rotation_plan_digest(&current.plan);
    current.operation_id =
        fleet_funding_policy_rotation_operation_id(current.coordinator, current.plan_digest);
    validate_fleet_funding_policy_rotation_plan(&current.plan)
        .map_err(|error| funding_authority_error(error.to_string()))?;
    Ok(current)
}

fn rotation_root_plans(
    collected: &CollectedFunding,
) -> Result<Vec<FleetFundingPolicyRotationRootPlan>, CyclesCommandError> {
    let mut roots = Vec::with_capacity(collected.installed.roots.len());
    for installed in &collected.installed.roots {
        if installed.status != FleetSubnetRootStatus::Active {
            return Err(funding_authority_error(
                "every affected Root must be Active before rotation",
            ));
        }
        let report = collected
            .report
            .roots
            .iter()
            .find(|root| root.canister_id == installed.fleet_subnet_root)
            .ok_or_else(|| funding_authority_error("an installed Root status is unavailable"))?;
        let coordinator = collected
            .report
            .coordinator
            .roots
            .iter()
            .find(|root| root.fleet_subnet_root == installed.fleet_subnet_root)
            .ok_or_else(|| funding_authority_error("Coordinator Root ledger is unavailable"))?;
        let local_usage = root_usage(&report.status);
        let coordinator_usage = coordinator_root_usage(coordinator);
        if report.status.policy_generation != collected.report.coordinator.policy_generation
            || report.status.current_operation.is_some()
            || report
                .status
                .latest_icp_refill
                .as_ref()
                .is_some_and(|refill| refill.resumable)
            || report.status.rotation_current.is_some()
            || coordinator.current_operation.is_some()
            || local_usage != coordinator_usage
            || report.status.policy_hash != coordinator.policy_hash
            || report.status.root_policy != coordinator.policy
        {
            return Err(funding_authority_error(
                "Root funding state is not quiescent and consistent with the Coordinator",
            ));
        }
        roots.push(FleetFundingPolicyRotationRootPlan {
            fleet_subnet_root: installed.fleet_subnet_root,
            predecessor_policy_hash: coordinator.policy_hash,
            predecessor_usage: coordinator_usage,
            proposed_policy: coordinator.policy.clone(),
            placement: rotation_placement(&installed.placement_cost),
        });
    }
    roots.sort_by_key(|root| root.fleet_subnet_root);
    Ok(roots)
}

fn query_registry_version(
    collected: &CollectedFunding,
) -> Result<FleetRegistryVersion, CyclesCommandError> {
    let response: Result<RemoteCoordinatorStatusResponse, Error> = query_canister_with_arg(
        &collected.icp,
        &collected.coordinator_binding,
        collected.report.coordinator_canister_id,
        canic_core::protocol::CANIC_STATUS,
        &RemoteCoordinatorStatusRequest::RegistryVersion,
    )?;
    match response {
        Ok(RemoteCoordinatorStatusResponse::RegistryVersion(version)) => Ok(version),
        Ok(RemoteCoordinatorStatusResponse::Funding(_)) => Err(funding_authority_error(
            "Coordinator returned the wrong protected status variant",
        )),
        Err(error) => Err(funding_rejected(error)),
    }
}

fn topology_catalog_digest(
    installed: &InstalledFleetFundingResolution,
) -> Result<[u8; 32], CyclesCommandError> {
    let mut digest = None;
    for placement in std::iter::once(&installed.coordinator_placement_cost)
        .chain(installed.roots.iter().map(|root| &root.placement_cost))
    {
        let Some(encoded) = placement.catalog_sha256.as_deref() else {
            if digest.is_some() {
                return Err(funding_authority_error(
                    "placement evidence mixes catalog-bound and local entries",
                ));
            }
            continue;
        };
        let bytes: [u8; 32] = decode_hex(encoded)
            .map_err(CyclesCommandError::FundingResponseHex)?
            .try_into()
            .map_err(|_| funding_authority_error("placement catalog digest is not SHA-256"))?;
        if digest.is_some_and(|current| current != bytes) {
            return Err(funding_authority_error(
                "placement evidence names more than one catalog digest",
            ));
        }
        digest = Some(bytes);
    }
    Ok(digest.unwrap_or([0; 32]))
}

fn rotation_placement(
    placement: &canic_host::fleet_install_plan::PlannedSubnetPlacementCostEvidence,
) -> FleetFundingPolicyRotationPlacementEvidence {
    FleetFundingPolicyRotationPlacementEvidence {
        subnet: placement.subnet,
        node_count: placement.node_count,
        cost_multiplier_numerator: placement.cost_multiplier_numerator,
        cost_multiplier_denominator: placement.cost_multiplier_denominator,
        fiduciary: placement.subnet_specialization == "fiduciary",
        acknowledge_fiduciary_cost: placement.acknowledge_fiduciary_cost,
    }
}

fn coordinator_usage(status: &RemoteCoordinatorFundingStatus) -> FleetFundingPolicyUsage {
    FleetFundingPolicyUsage {
        historical_automatic_grants: status.historical_automatic_grants,
        historical_automatic_cycles: status.historical_automatic_cycles.clone(),
        generation_automatic_grants: status.automatic_grants,
        generation_automatic_cycles: status.automatic_cycles.clone(),
    }
}

fn coordinator_root_usage(status: &RemoteCoordinatorRootFundingStatus) -> FleetFundingPolicyUsage {
    FleetFundingPolicyUsage {
        historical_automatic_grants: status.historical_automatic_grants,
        historical_automatic_cycles: status.historical_automatic_cycles.clone(),
        generation_automatic_grants: status.automatic_grants,
        generation_automatic_cycles: status.automatic_cycles.clone(),
    }
}

fn root_usage(status: &RemoteRootFundingStatus) -> FleetFundingPolicyUsage {
    FleetFundingPolicyUsage {
        historical_automatic_grants: status.historical_automatic_grants,
        historical_automatic_cycles: status.historical_automatic_cycles.clone(),
        generation_automatic_grants: status.automatic_grants,
        generation_automatic_cycles: status.automatic_cycles.clone(),
    }
}

fn funding_authority_error(message: impl Into<String>) -> CyclesCommandError {
    CyclesCommandError::FundingAuthority(message.into())
}

fn infrastructure_binding(
    root: &std::path::Path,
    environment: &str,
    entries: &[canic_host::release_set::CanicInfrastructureArtifactEntry],
    role: CanicInfrastructureRole,
) -> Result<ResolvedProtocolBinding, CyclesCommandError> {
    let artifact = entries
        .iter()
        .find(|entry| entry.role == role)
        .ok_or_else(|| {
            CyclesCommandError::FundingAuthority(format!(
                "installed release is missing {} protocol metadata",
                role.as_str()
            ))
        })?;
    resolve_infrastructure_protocol_binding(root, environment, artifact)
        .map_err(|error| CyclesCommandError::FundingAuthority(error.to_string()))
}

fn query_coordinator(
    icp: &canic_host::icp::IcpCli,
    coordinator: Principal,
    binding: &ResolvedProtocolBinding,
) -> Result<RemoteCoordinatorFundingStatus, CyclesCommandError> {
    let output = icp.canister_query_arg_output_with_candid(
        &coordinator.to_text(),
        canic_core::protocol::CANIC_STATUS,
        "(variant { Funding })",
        Some("hex"),
        Some(binding.candid_path()),
    )?;
    let bytes = decode_hex(output.trim()).map_err(CyclesCommandError::FundingResponseHex)?;
    let response = decode_one::<Result<RemoteCoordinatorStatusResponse, Error>>(&bytes)
        .map_err(CyclesCommandError::FundingResponseCandid)?;
    match response {
        Ok(RemoteCoordinatorStatusResponse::Funding(status)) => Ok(*status),
        Ok(RemoteCoordinatorStatusResponse::RegistryVersion(_)) => Err(funding_authority_error(
            "Coordinator returned the wrong protected status variant",
        )),
        Err(error) => Err(funding_rejected(error)),
    }
}

fn root_status_matches_authority(
    status: &RemoteRootFundingStatus,
    installed: &InstalledFleetRootFundingResolution,
    coordinator: &RemoteCoordinatorFundingStatus,
) -> bool {
    let identity_is_exact = status.fleet_subnet_root == installed.fleet_subnet_root;
    let lifecycle_is_exact = status.lifecycle_status == installed.status;
    let icp_refill_is_exact = status.icp_refill_policy == installed.funding.icp_refill;
    let mut authority = installed.funding.clone();
    authority.root_funding = status.root_policy.clone();
    let policy_hash_is_exact =
        status.policy_hash == fleet_subnet_root_funding_policy_hash(&authority);
    let coordinator_root = coordinator
        .roots
        .iter()
        .find(|root| root.fleet_subnet_root == status.fleet_subnet_root);
    let generation_is_admitted = coordinator.rotation.as_ref().map_or_else(
        || status.policy_generation == coordinator.policy_generation,
        |rotation| {
            status.policy_generation == rotation.predecessor_generation
                || status.policy_generation == rotation.successor_generation
        },
    );
    let converged_policy_is_exact = coordinator.rotation.is_some()
        || coordinator_root.is_some_and(|root| {
            (&root.policy, root.policy_hash) == (&status.root_policy, status.policy_hash)
        });
    [
        identity_is_exact,
        lifecycle_is_exact,
        icp_refill_is_exact,
        policy_hash_is_exact,
        generation_is_admitted,
        converged_policy_is_exact,
    ]
    .into_iter()
    .all(|is_exact| is_exact)
}

fn coordinator_root_status_matches_installed(
    status: &RemoteCoordinatorRootFundingStatus,
    installed: &InstalledFleetRootFundingResolution,
) -> bool {
    let identity_is_exact = status.fleet_subnet_root == installed.fleet_subnet_root;
    let lifecycle_is_exact = status.lifecycle_status == installed.status;
    let mut authority = installed.funding.clone();
    authority.root_funding = status.policy.clone();
    let policy_hash_is_exact =
        status.policy_hash == fleet_subnet_root_funding_policy_hash(&authority);
    [identity_is_exact, lifecycle_is_exact, policy_hash_is_exact]
        .into_iter()
        .all(|is_exact| is_exact)
}

fn query_root(
    icp: &canic_host::icp::IcpCli,
    root: Principal,
    binding: &ResolvedProtocolBinding,
) -> Result<RemoteRootFundingStatus, CyclesCommandError> {
    let output = icp.canister_query_arg_output_with_candid(
        &root.to_text(),
        canic_core::protocol::CANIC_STATUS,
        "(variant { Funding })",
        Some("hex"),
        Some(binding.candid_path()),
    )?;
    let bytes = decode_hex(output.trim()).map_err(CyclesCommandError::FundingResponseHex)?;
    let response = decode_one::<Result<RemoteRootStatusResponse, Error>>(&bytes)
        .map_err(CyclesCommandError::FundingResponseCandid)?;
    match response {
        Ok(RemoteRootStatusResponse::Funding(status)) => Ok(status),
        Err(error) => Err(funding_rejected(error)),
    }
}

fn funding_rejected(error: Error) -> CyclesCommandError {
    let code = error.code();
    CyclesCommandError::FundingRejected {
        code,
        diagnostic: canic_host::diagnostics::render_diagnostic(code),
    }
}

fn validate_coordinator_status(
    installed: &InstalledFleetFundingResolution,
    status: &RemoteCoordinatorFundingStatus,
) -> Result<(), CyclesCommandError> {
    let coordinator_identity_is_exact = status.coordinator == installed.coordinator_canister_id;
    let generation_is_valid = status.policy_generation != 0;
    let root_count_is_exact = status.roots.len() == installed.roots.len();
    let roots_are_exact = installed.roots.iter().all(|root| {
        status
            .roots
            .iter()
            .any(|entry| coordinator_root_status_matches_installed(entry, root))
    });
    if ![
        coordinator_identity_is_exact,
        generation_is_valid,
        root_count_is_exact,
        roots_are_exact,
    ]
    .into_iter()
    .all(|is_exact| is_exact)
    {
        return Err(CyclesCommandError::FundingAuthority(
            "Coordinator funding status conflicts with verified installed authority".to_string(),
        ));
    }
    if status.policy_generation == 1 && status.policy != installed.coordinator_root_funding {
        return Err(CyclesCommandError::FundingAuthority(
            "Coordinator genesis funding policy conflicts with verified installed authority"
                .to_string(),
        ));
    }
    Ok(())
}

fn placement_report(
    placement: &canic_host::fleet_install_plan::PlannedSubnetPlacementCostEvidence,
) -> PlacementReport {
    PlacementReport {
        subnet: placement.subnet.to_string(),
        specialization: placement.subnet_specialization.clone(),
        node_count: placement.node_count,
        cost_multiplier_numerator: placement.cost_multiplier_numerator,
        cost_multiplier_denominator: placement.cost_multiplier_denominator,
        acknowledge_fiduciary_cost: placement.acknowledge_fiduciary_cost,
        warning: placement.warning.clone(),
    }
}

fn render_report(report: &FundingReport) -> String {
    let mut lines = vec![
        format!(
            "Fleet: {} (environment {})",
            report.fleet, report.environment
        ),
        format!(
            "Coordinator: {} | balance={} | funding={} | profile={}",
            report.coordinator_canister_id,
            cycles_tc(report.coordinator.current_cycles.to_u128()),
            if report.coordinator.funding_enabled {
                "enabled"
            } else {
                "disabled"
            },
            report
                .coordinator
                .funding_profile
                .map_or("none", funding_profile_label),
        ),
        format!(
            "  placement: subnet={} nodes={} multiplier={}/{} acknowledgement={}{}",
            report.coordinator_placement.subnet,
            report.coordinator_placement.node_count,
            report.coordinator_placement.cost_multiplier_numerator,
            report.coordinator_placement.cost_multiplier_denominator,
            report.coordinator_placement.acknowledge_fiduciary_cost,
            warning_suffix(report.coordinator_placement.warning.as_deref()),
        ),
    ];
    if let (Some(policy), Some(window)) = (
        report.coordinator.policy.as_ref(),
        report.coordinator.fleet_window.as_ref(),
    ) {
        lines.push(format!(
            "  treasury: reserve={} window={}+{}/{} auto={}/{} grants, {}/{} cycles",
            cycles_tc(policy.minimum_reserve_cycles.to_u128()),
            cycles_tc(window.spent_cycles.to_u128()),
            cycles_tc(window.reserved_cycles.to_u128()),
            cycles_tc(policy.budget.maximum_cycles.to_u128()),
            report.coordinator.automatic_grants,
            policy.maximum_automatic_grants,
            cycles_tc(report.coordinator.automatic_cycles.to_u128()),
            cycles_tc(policy.maximum_automatic_cycles.to_u128()),
        ));
    }
    lines.push(format!(
        "  recovery: {}",
        report.coordinator_direct_topup_command
    ));
    for root in &report.roots {
        append_root_report(&mut lines, report, root);
    }
    lines.join("\n")
}

fn append_root_report(lines: &mut Vec<String>, report: &FundingReport, root: &RootFundingReport) {
    let status = &root.status;
    lines.push(String::new());
    lines.push(format!(
        "Root: {} | lifecycle={:?} | balance={} | funding={} | eligible={}",
        root.canister_id,
        status.lifecycle_status,
        cycles_tc(status.current_cycles.to_u128()),
        if status.cycles_funding_enabled {
            "enabled"
        } else {
            "disabled"
        },
        status.funding_eligible,
    ));
    lines.push(format!(
        "  policy: request<={} target={} window={} auto={}/{} grants, {}/{} cycles",
        cycles_tc(status.root_policy.request_threshold.to_u128()),
        cycles_tc(status.root_policy.target_balance.to_u128()),
        coordinator_root_window(report, root).map_or_else(
            || "unavailable".to_string(),
            |window| {
                format!(
                    "{}+{}/{}",
                    cycles_tc(window.spent_cycles.to_u128()),
                    cycles_tc(window.reserved_cycles.to_u128()),
                    cycles_tc(status.root_policy.budget.maximum_cycles.to_u128())
                )
            }
        ),
        status.automatic_grants,
        status.root_policy.maximum_automatic_grants,
        cycles_tc(status.automatic_cycles.to_u128()),
        cycles_tc(status.root_policy.maximum_automatic_cycles.to_u128()),
    ));
    lines.push(format!(
        "  last Coordinator result: {} | pending={}",
        funding_result_label(status.last_result.as_ref()),
        status.current_operation.is_some(),
    ));
    lines.push(format!(
        "  placement: subnet={} nodes={} multiplier={}/{} acknowledgement={}{}",
        root.placement.subnet,
        root.placement.node_count,
        root.placement.cost_multiplier_numerator,
        root.placement.cost_multiplier_denominator,
        root.placement.acknowledge_fiduciary_cost,
        warning_suffix(root.placement.warning.as_deref()),
    ));
    lines.push(format!("  ICP fallback: {}", icp_refill_label(status)));
    lines.push(format!("  recovery: {}", root.direct_topup_command));
}

fn coordinator_root_window<'a>(
    report: &'a FundingReport,
    root: &RootFundingReport,
) -> Option<&'a RemoteFundingWindowStatus> {
    report
        .coordinator
        .roots
        .iter()
        .find(|status| status.fleet_subnet_root == root.canister_id)
        .map(|status| &status.window)
}

fn funding_result_label(result: Option<&FleetRootFundingResponse>) -> String {
    match result {
        Some(FleetRootFundingResponse::Granted(receipt)) => format!(
            "granted {} at {}",
            cycles_tc(receipt.request.granted_cycles.to_u128()),
            receipt.accepted_at_ns
        ),
        Some(FleetRootFundingResponse::NoGrant(receipt)) => {
            format!("no_grant {:?} at {}", receipt.reason, receipt.decided_at_ns)
        }
        None => "none".to_string(),
    }
}

fn icp_refill_label(status: &RemoteRootFundingStatus) -> String {
    let Some(policy) = status.icp_refill_policy.as_ref() else {
        return "not configured".to_string();
    };
    let automatic = policy.automatic.as_ref().map_or_else(
        || "manual only".to_string(),
        |automatic| {
            format!(
                "automatic<={} target={} used={}/{} refills, {}/{} e8s",
                cycles_tc(automatic.emergency_threshold.to_u128()),
                cycles_tc(automatic.target_balance.to_u128()),
                status.automatic_icp_refills,
                automatic.maximum_automatic_refills,
                status.automatic_icp_refill_e8s,
                automatic.maximum_automatic_refill_e8s,
            )
        },
    );
    let latest = status.latest_icp_refill.as_ref().map_or_else(
        || "none".to_string(),
        |latest| {
            format!(
                "{:?}/{}{}",
                latest.trigger,
                hex_bytes(latest.response.operation_id),
                if latest.resumable {
                    "/recovery_required"
                } else {
                    ""
                }
            )
        },
    );
    format!(
        "{automatic}; window={}/{} e8s; last={latest}",
        status.icp_window_reserved_e8s, policy.maximum_refill_e8s
    )
}

const fn funding_profile_label(profile: FleetFundingProfile) -> &'static str {
    match profile {
        FleetFundingProfile::SingleSubnet => "single_subnet",
        FleetFundingProfile::PreviewMultiSubnet => "preview_multi_subnet",
        FleetFundingProfile::MultiSubnet => "multi_subnet",
    }
}

fn warning_suffix(warning: Option<&str>) -> String {
    warning.map_or_else(String::new, |warning| format!(" | {warning}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn funding_options_require_one_fleet_and_preserve_target_context() {
        let options = FundingOptions::parse([
            OsString::from("demo"),
            OsString::from("--json"),
            OsString::from(crate::cli::globals::INTERNAL_ENVIRONMENT_OPTION),
            OsString::from("staging"),
        ])
        .expect("parse funding options");

        assert_eq!(options.fleet, "demo");
        assert_eq!(options.target.environment, "staging");
        assert!(options.json);
        assert!(options.apply_rotation.is_none());
        assert!(options.plan_rotation.is_none());
        assert!(FundingOptions::parse(std::iter::empty::<OsString>()).is_err());
    }

    #[test]
    fn funding_rotation_modes_are_explicit_and_mutually_exclusive() {
        let plan = FundingOptions::parse([
            OsString::from("demo"),
            OsString::from("--plan-rotation"),
            OsString::from("funding.json"),
        ])
        .expect("parse planning mode");
        assert_eq!(plan.plan_rotation, Some(PathBuf::from("funding.json")));
        assert!(plan.apply_rotation.is_none());

        let apply = FundingOptions::parse([
            OsString::from("demo"),
            OsString::from("--apply-rotation"),
            OsString::from("funding.json"),
        ])
        .expect("parse apply mode");
        assert_eq!(apply.apply_rotation, Some(PathBuf::from("funding.json")));
        assert!(apply.plan_rotation.is_none());

        assert!(
            FundingOptions::parse([
                OsString::from("demo"),
                OsString::from("--json"),
                OsString::from("--plan-rotation"),
                OsString::from("funding.json"),
            ])
            .is_err()
        );
    }

    #[test]
    fn funding_result_labels_are_explicit() {
        assert_eq!(funding_result_label(None), "none");
    }
}
