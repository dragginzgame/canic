//! Module: canic_cli::admission
//!
//! Responsibility: plan, apply, and inspect one protected Fleet-admission mutation.
//! Does not own: admission policy decisions, canister journals, or participant convergence.
//! Boundary: terminal current ensure authority and live protected status bind one no-effect plan.

#[cfg(test)]
mod tests;

use crate::{
    cli::{
        clap::{
            flag_arg, parse_matches, parse_required_subcommand, passthrough_subcommand,
            render_usage, required_path, required_string, string_option, value_arg,
        },
        globals::{internal_environment_arg, internal_icp_arg},
        help::print_help_or_version,
    },
    support::icp_target::IcpTargetOptions,
    version_text,
};
use candid::{CandidType, Principal};
use canic_core::{
    dto::{
        error::Error,
        fleet_admission::{
            FleetAdmissionMutationAction, FleetAdmissionMutationOutcome,
            FleetAdmissionMutationRequest, FleetAdmissionMutationResponse,
            FleetAdmissionOperationPhase, FleetAdmissionRootParticipantPhase,
            FleetAdmissionRootParticipantStatus, FleetAdmissionRootStatusResponse,
            FleetAdmissionRootTransitionPhase, FleetAdmissionStatusRequest,
            FleetAdmissionStatusResponse,
        },
        fleet_registry::{FleetRegistry, FleetRegistryVersion, FleetSubnetRootStatus},
        page::PageRequest,
    },
    ids::{
        ComponentInstanceId, FleetAdmissionPolicy, FleetAdmissionSelector, ManagedCanisterBinding,
        SubnetId,
    },
    shared_support::{
        fleet_admission_authority::{
            FleetAdmissionMutationActionModel, FleetAdmissionMutationOperationInput,
            FleetAdmissionRootCatalogAuthorityModel, fleet_admission_mutation_operation_id,
            mutate_fleet_admission_membership,
        },
        fleet_admission_policy::compile_installed_fleet_admission_policy,
        fleet_admission_policy::{
            effective_fleet_admission_principals, fleet_admission_participant_catalog_digest,
            fleet_admission_root_participant_catalog_digest, fleet_admission_target_for_binding,
            materialize_fleet_admission_projection,
        },
        fleet_admission_root::MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS,
    },
};
use canic_host::{
    CanisterProtocolError, call_canister_with_arg,
    durable_io::write_bytes,
    fleet_ensure::{CurrentFleetInventoryError, CurrentFleetResolution, resolve_current_fleet},
    icp::IcpCli,
    icp_config::{IcpConfigError, resolve_current_canic_icp_root},
    protocol_binding::{ResolvedProtocolBinding, resolve_registry_protocol_binding},
    query_canister_with_arg,
};
use clap::{ArgGroup, Command as ClapCommand};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, ffi::OsString, fs, path::PathBuf};
use thiserror::Error as ThisError;

const FLEET_ARG: &str = "fleet";
const ADD_ARG: &str = "add";
const APPLY_COMMAND: &str = "apply";
const COMPONENT_INSTANCE_ARG: &str = "component-instance";
const COMPONENT_SPEC_ARG: &str = "component-spec";
const FLEET_SELECTOR_ARG: &str = "fleet-selector";
const FLEET_SUBNET_ROOT_ARG: &str = "fleet-subnet-root";
const JSON_ARG: &str = "json";
const OUT_ARG: &str = "out";
const PLAN_COMMAND: &str = "plan";
const PLAN_FILE_ARG: &str = "plan-file";
const REMOVE_ARG: &str = "remove";
const STATUS_COMMAND: &str = "status";
const ADMISSION_PLAN_SCHEMA_VERSION: u16 = 1;
const ROOT_STATUS_PAGE_SIZE: u64 = 32;

/// Failures from the protected Fleet-admission operator surface.
#[derive(Debug, ThisError)]
pub enum AdmissionCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("failed to read Canic Fleet state: {0}")]
    IcpRoot(#[source] IcpConfigError),

    #[error(transparent)]
    CurrentFleet(#[from] CurrentFleetInventoryError),

    #[error(transparent)]
    Protocol(#[from] CanisterProtocolError),

    #[error("invalid admission Principal {value:?}: {reason}")]
    InvalidPrincipal { value: String, reason: String },

    #[error("invalid admission selector: {0}")]
    InvalidSelector(String),

    #[error("invalid Fleet-admission mutation: {0}")]
    InvalidMutation(String),

    #[error("current Fleet-admission authority is invalid: {0}")]
    Authority(String),

    #[error("Fleet-admission request rejected: {0}")]
    Rejected(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AdmissionPlanOptions {
    target: IcpTargetOptions,
    fleet: String,
    action: FleetAdmissionMutationAction,
    principal: Principal,
    selector: FleetAdmissionSelector,
    out: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AdmissionApplyOptions {
    target: IcpTargetOptions,
    fleet: String,
    plan_file: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AdmissionStatusOptions {
    target: IcpTargetOptions,
    fleet: String,
    json: bool,
}

#[derive(CandidType)]
enum RemoteCoordinatorStatusRequest {
    Admission(FleetAdmissionStatusRequest),
    Registry,
    RegistryVersion,
}

#[derive(CandidType, Deserialize)]
enum RemoteCoordinatorStatusResponse {
    Admission(FleetAdmissionStatusResponse),
    Registry(FleetRegistry),
    RegistryVersion(FleetRegistryVersion),
}

#[derive(CandidType)]
enum RemoteCoordinatorCommand {
    MutateAdmission(FleetAdmissionMutationRequest),
}

#[derive(CandidType, Deserialize)]
enum RemoteCoordinatorCommandResponse {
    MutateAdmission(FleetAdmissionMutationResponse),
}

#[derive(CandidType)]
enum RemoteRootStatusRequest {
    Admission(PageRequest),
}

#[derive(CandidType, Deserialize)]
enum RemoteRootStatusResponse {
    Admission(FleetAdmissionRootStatusResponse),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct AdmissionPlanFile {
    schema_version: u16,
    fleet: String,
    environment: String,
    coordinator: Principal,
    predecessor_registry: FleetRegistryVersion,
    predecessor_policy: FleetAdmissionPolicy,
    successor_policy: FleetAdmissionPolicy,
    participant_catalogs: Vec<AdmissionParticipantCatalog>,
    request: FleetAdmissionMutationRequest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct AdmissionParticipantCatalog {
    fleet_subnet_root: Principal,
    participant_catalog_digest: [u8; 32],
    participants: Vec<ManagedCanisterBinding>,
}

struct AdmissionConnection {
    coordinator: Principal,
    icp: IcpCli,
    coordinator_binding: ResolvedProtocolBinding,
    root_bindings: BTreeMap<Principal, ResolvedProtocolBinding>,
    registry: FleetRegistry,
    registry_version: FleetRegistryVersion,
    admission: FleetAdmissionStatusResponse,
}

/// One Root's compact convergence report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AdmissionRootReport {
    pub root: Principal,
    pub active_generation: u64,
    pub active_policy_digest: String,
    pub operation_id: Option<String>,
    pub phase: Option<String>,
    pub participant_count: u64,
    pub pending_count: u64,
    pub prepared_count: u64,
    pub activated_count: u64,
    pub open_count: u64,
    pub first_unresolved: Option<Principal>,
}

/// Protected admission health consumed by both status and Medic.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AdmissionStatusReport {
    pub fleet: String,
    pub environment: String,
    pub coordinator: Principal,
    pub registry_revision: u64,
    pub generation: u64,
    pub policy_digest: String,
    pub fleet_principal_count: u64,
    pub narrower_rule_count: u16,
    pub narrower_principal_reference_count: u16,
    pub current_operation: Option<String>,
    pub current_phase: Option<String>,
    pub last_operation: Option<String>,
    pub last_phase: Option<String>,
    pub roots: Vec<AdmissionRootReport>,
}

pub fn run<I>(args: I) -> Result<(), AdmissionCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, admission_usage, version_text()) {
        return Ok(());
    }
    let (command, args) = parse_required_subcommand(admission_command(), args)
        .map_err(|_| AdmissionCommandError::Usage(admission_usage()))?;
    match command.as_str() {
        APPLY_COMMAND => {
            if print_help_or_version(&args, apply_usage, version_text()) {
                Ok(())
            } else {
                run_apply(AdmissionApplyOptions::parse(args)?)
            }
        }
        PLAN_COMMAND => {
            if print_help_or_version(&args, plan_usage, version_text()) {
                Ok(())
            } else {
                run_plan(AdmissionPlanOptions::parse(args)?)
            }
        }
        STATUS_COMMAND => {
            if print_help_or_version(&args, status_usage, version_text()) {
                Ok(())
            } else {
                run_status(AdmissionStatusOptions::parse(args)?)
            }
        }
        _ => unreachable!("admission command declares only known subcommands"),
    }
}

fn run_plan(options: AdmissionPlanOptions) -> Result<(), AdmissionCommandError> {
    let connection = connect(&options.fleet, &options.target)?;
    require_idle_admission(&connection.admission)?;
    let plan = build_plan(
        &options.fleet,
        &options.target.environment,
        &connection,
        options.action,
        options.selector,
        options.principal,
    )?;
    let mut bytes = serde_json::to_vec_pretty(&plan)?;
    bytes.push(b'\n');
    write_bytes(&options.out, &bytes)?;
    println!(
        "Fleet-admission plan written to {}\nOperation: {}\nGeneration: {} -> {}\nRegistry revision: {}\nParticipant Roots: {}\nManaged participants: {}",
        options.out.display(),
        hex_bytes(plan.request.operation_id),
        plan.predecessor_policy.generation,
        plan.successor_policy.generation,
        plan.predecessor_registry.revision,
        plan.participant_catalogs.len(),
        plan.participant_catalogs
            .iter()
            .map(|catalog| catalog.participants.len())
            .sum::<usize>(),
    );
    Ok(())
}

fn run_apply(options: AdmissionApplyOptions) -> Result<(), AdmissionCommandError> {
    let plan: AdmissionPlanFile = serde_json::from_slice(&fs::read(&options.plan_file)?)?;
    validate_plan_file(&options, &plan)?;
    let connection = connect(&options.fleet, &options.target)?;
    validate_live_plan(&connection, &plan)?;
    let response: Result<RemoteCoordinatorCommandResponse, Error> = call_canister_with_arg(
        &connection.icp,
        &connection.coordinator_binding,
        connection.coordinator,
        canic_core::protocol::CANIC_COMMAND,
        &RemoteCoordinatorCommand::MutateAdmission(plan.request.clone()),
    )?;
    let response = match response {
        Ok(RemoteCoordinatorCommandResponse::MutateAdmission(response)) => response,
        Err(error) => return Err(rejected(error)),
    };
    if response.operation_id != plan.request.operation_id
        || response.generation != plan.successor_policy.generation
        || response.policy_digest != plan.successor_policy.policy_digest
    {
        return Err(authority_error(
            "Coordinator mutation response differs from the accepted plan",
        ));
    }
    println!(
        "Fleet-admission mutation accepted\nOperation: {}\nOutcome: {}\nGeneration: {}\nPolicy digest: {}",
        hex_bytes(response.operation_id),
        mutation_outcome_label(response.outcome),
        response.generation,
        hex_bytes(response.policy_digest),
    );
    Ok(())
}

fn run_status(options: AdmissionStatusOptions) -> Result<(), AdmissionCommandError> {
    let report = collect_status(&options.fleet, &options.target)?;
    if options.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", render_status(&report));
    }
    Ok(())
}

pub fn collect_status(
    fleet: &str,
    target: &IcpTargetOptions,
) -> Result<AdmissionStatusReport, AdmissionCommandError> {
    let connection = connect(fleet, target)?;
    let mut roots = Vec::new();
    for root in connection
        .registry
        .fleet_subnet_roots
        .iter()
        .filter(|root| root.status != FleetSubnetRootStatus::Removed)
    {
        roots.push(query_root_report(&connection, root.fleet_subnet_root)?);
    }
    roots.sort_unstable_by(|left, right| left.root.as_slice().cmp(right.root.as_slice()));
    let (current_operation, current_phase) = connection
        .admission
        .current_operation
        .as_ref()
        .map(operation_identity)
        .unzip();
    let (last_operation, last_phase) = connection
        .admission
        .last_result
        .as_ref()
        .map(operation_identity)
        .unzip();
    Ok(AdmissionStatusReport {
        fleet: fleet.to_string(),
        environment: target.environment.clone(),
        coordinator: connection.coordinator,
        registry_revision: connection.registry_version.revision,
        generation: connection.admission.active.generation,
        policy_digest: hex_bytes(connection.admission.active.policy_digest),
        fleet_principal_count: connection.admission.principals.total,
        narrower_rule_count: connection.admission.active.narrower_rule_count,
        narrower_principal_reference_count: connection
            .admission
            .active
            .narrower_principal_reference_count,
        current_operation,
        current_phase,
        last_operation,
        last_phase,
        roots,
    })
}

impl AdmissionPlanOptions {
    fn parse(args: Vec<OsString>) -> Result<Self, AdmissionCommandError> {
        let matches = parse_matches(plan_command(), args)
            .map_err(|_| AdmissionCommandError::Usage(plan_usage()))?;
        let (action, principal_text) = match (
            string_option(&matches, ADD_ARG),
            string_option(&matches, REMOVE_ARG),
        ) {
            (Some(principal), None) => (FleetAdmissionMutationAction::Add, principal),
            (None, Some(principal)) => (FleetAdmissionMutationAction::Remove, principal),
            _ => unreachable!("Clap requires exactly one admission action"),
        };
        let principal = parse_principal(&principal_text)?;
        Ok(Self {
            target: IcpTargetOptions::parse(&matches),
            fleet: required_string(&matches, FLEET_ARG),
            action,
            principal,
            selector: parse_selector(&matches)?,
            out: required_path(&matches, OUT_ARG),
        })
    }
}

impl AdmissionApplyOptions {
    fn parse(args: Vec<OsString>) -> Result<Self, AdmissionCommandError> {
        let matches = parse_matches(apply_command(), args)
            .map_err(|_| AdmissionCommandError::Usage(apply_usage()))?;
        Ok(Self {
            target: IcpTargetOptions::parse(&matches),
            fleet: required_string(&matches, FLEET_ARG),
            plan_file: required_path(&matches, PLAN_FILE_ARG),
        })
    }
}

impl AdmissionStatusOptions {
    fn parse(args: Vec<OsString>) -> Result<Self, AdmissionCommandError> {
        let matches = parse_matches(status_command(), args)
            .map_err(|_| AdmissionCommandError::Usage(status_usage()))?;
        Ok(Self {
            target: IcpTargetOptions::parse(&matches),
            fleet: required_string(&matches, FLEET_ARG),
            json: matches.get_flag(JSON_ARG),
        })
    }
}

fn connect(
    fleet: &str,
    target: &IcpTargetOptions,
) -> Result<AdmissionConnection, AdmissionCommandError> {
    let root = resolve_current_canic_icp_root().map_err(AdmissionCommandError::IcpRoot)?;
    let current = resolve_current_fleet(&root, &target.environment, fleet)?;
    let initial_registry = current.initial_active_registry(fleet)?.clone();
    let coordinator = initial_registry.authority.binding.coordinator;
    let coordinator_binding =
        current_binding(&current, &root, &target.environment, &coordinator, fleet)?;
    let mut root_bindings = BTreeMap::new();
    for fleet_subnet_root in &current.topology.fleet_subnet_root_canister_ids {
        let fleet_subnet_root = fleet_subnet_root
            .parse::<Principal>()
            .map_err(|error| authority_error(error.to_string()))?;
        root_bindings.insert(
            fleet_subnet_root,
            current_binding(
                &current,
                &root,
                &target.environment,
                &fleet_subnet_root,
                fleet,
            )?,
        );
    }
    let icp = target.icp_cli(&root);
    let registry = query_coordinator_registry(&icp, &coordinator_binding, coordinator)?;
    let registry_version =
        query_coordinator_registry_version(&icp, &coordinator_binding, coordinator)?;
    let admission = query_coordinator_admission(&icp, &coordinator_binding, coordinator)?;
    validate_live_authority(&initial_registry, &registry, &registry_version, &admission)?;
    Ok(AdmissionConnection {
        coordinator,
        icp,
        coordinator_binding,
        root_bindings,
        registry,
        registry_version,
        admission,
    })
}

fn current_binding(
    current: &CurrentFleetResolution,
    root: &std::path::Path,
    environment: &str,
    principal: &Principal,
    fleet: &str,
) -> Result<ResolvedProtocolBinding, AdmissionCommandError> {
    let principal = principal.to_text();
    let entry = current
        .registry
        .entries
        .iter()
        .find(|entry| entry.pid == principal)
        .ok_or_else(|| {
            authority_error(format!(
                "Fleet {fleet} omits current participant {principal}"
            ))
        })?;
    resolve_registry_protocol_binding(root, environment, entry)
        .map_err(|error| authority_error(error.to_string()))
}

fn build_plan(
    fleet: &str,
    environment: &str,
    connection: &AdmissionConnection,
    action: FleetAdmissionMutationAction,
    selector: FleetAdmissionSelector,
    principal: Principal,
) -> Result<AdmissionPlanFile, AdmissionCommandError> {
    require_selector_exists(&connection.registry, &selector)?;
    let action_model = action_model(action);
    let predecessor = &connection.registry.admission;
    let membership =
        mutate_fleet_admission_membership(predecessor, action_model, &selector, principal)
            .map_err(|error| AdmissionCommandError::InvalidMutation(error.to_string()))?;
    let generation = if membership.changed {
        predecessor.generation.checked_add(1).ok_or_else(|| {
            AdmissionCommandError::InvalidMutation("generation is exhausted".to_string())
        })?
    } else {
        predecessor.generation
    };
    let successor = compile_installed_fleet_admission_policy(
        predecessor.fleet.clone(),
        generation,
        membership.fleet_principals,
        membership.rules,
    )
    .map_err(|error| AdmissionCommandError::InvalidMutation(error.to_string()))?;
    let participant_catalogs = participant_catalogs(connection, &successor)?;
    let catalog_authorities = participant_catalog_authorities(&participant_catalogs)?;
    let (participant_catalog_digest, participant_count) =
        aggregate_participant_catalog_authority(&catalog_authorities)?;
    let operation_id = fleet_admission_mutation_operation_id(
        &connection.registry_version,
        &FleetAdmissionMutationOperationInput {
            expected_generation: predecessor.generation,
            expected_policy_digest: predecessor.policy_digest,
            action: action_model,
            selector: selector.clone(),
            principal,
            successor_policy_digest: successor.policy_digest,
            participant_catalog_digest,
            participant_count,
        },
    );
    Ok(AdmissionPlanFile {
        schema_version: ADMISSION_PLAN_SCHEMA_VERSION,
        fleet: fleet.to_string(),
        environment: environment.to_string(),
        coordinator: connection.coordinator,
        predecessor_registry: connection.registry_version.clone(),
        predecessor_policy: predecessor.clone(),
        successor_policy: successor.clone(),
        participant_catalogs,
        request: FleetAdmissionMutationRequest {
            authority: connection.registry.authority.binding.clone(),
            expected_generation: predecessor.generation,
            expected_policy_digest: predecessor.policy_digest,
            action,
            selector,
            principal,
            operation_id,
            successor_policy_digest: successor.policy_digest,
            participant_catalog_digest,
            participant_count,
        },
    })
}

fn validate_plan_file(
    options: &AdmissionApplyOptions,
    plan: &AdmissionPlanFile,
) -> Result<(), AdmissionCommandError> {
    let expected = validate_plan_semantics(plan)?;
    if plan.schema_version != ADMISSION_PLAN_SCHEMA_VERSION
        || plan.fleet != options.fleet
        || plan.environment != options.target.environment
        || expected != plan.successor_policy
    {
        return Err(authority_error(
            "plan identity, environment, or successor policy is invalid",
        ));
    }
    let catalog_authorities = participant_catalog_authorities(&plan.participant_catalogs)?;
    let (participant_catalog_digest, participant_count) =
        aggregate_participant_catalog_authority(&catalog_authorities)?;
    let operation_id = fleet_admission_mutation_operation_id(
        &plan.predecessor_registry,
        &FleetAdmissionMutationOperationInput {
            expected_generation: plan.request.expected_generation,
            expected_policy_digest: plan.request.expected_policy_digest,
            action: action_model(plan.request.action),
            selector: plan.request.selector.clone(),
            principal: plan.request.principal,
            successor_policy_digest: plan.request.successor_policy_digest,
            participant_catalog_digest,
            participant_count,
        },
    );
    if plan.coordinator != plan.predecessor_registry.authority.binding.coordinator
        || plan.request.authority != plan.predecessor_registry.authority.binding
        || plan.request.expected_generation != plan.predecessor_policy.generation
        || plan.request.expected_policy_digest != plan.predecessor_policy.policy_digest
        || plan.request.successor_policy_digest != plan.successor_policy.policy_digest
        || plan.request.participant_catalog_digest != participant_catalog_digest
        || plan.request.participant_count != participant_count
        || plan.request.operation_id != operation_id
    {
        return Err(authority_error(
            "plan Registry, request, or operation identity is invalid",
        ));
    }
    Ok(())
}

fn validate_plan_semantics(
    plan: &AdmissionPlanFile,
) -> Result<FleetAdmissionPolicy, AdmissionCommandError> {
    let membership = mutate_fleet_admission_membership(
        &plan.predecessor_policy,
        action_model(plan.request.action),
        &plan.request.selector,
        plan.request.principal,
    )
    .map_err(|error| AdmissionCommandError::InvalidMutation(error.to_string()))?;
    let generation = if membership.changed {
        plan.predecessor_policy
            .generation
            .checked_add(1)
            .ok_or_else(|| {
                AdmissionCommandError::InvalidMutation("generation is exhausted".to_string())
            })?
    } else {
        plan.predecessor_policy.generation
    };
    compile_installed_fleet_admission_policy(
        plan.predecessor_policy.fleet.clone(),
        generation,
        membership.fleet_principals,
        membership.rules,
    )
    .map_err(|error| AdmissionCommandError::InvalidMutation(error.to_string()))
}

fn validate_live_plan(
    connection: &AdmissionConnection,
    plan: &AdmissionPlanFile,
) -> Result<(), AdmissionCommandError> {
    if connection.coordinator != plan.coordinator {
        return Err(authority_error("plan names another current Coordinator"));
    }
    if exact_retained_operation(&connection.admission, plan) {
        return Ok(());
    }
    let predecessor_matches = connection.registry_version == plan.predecessor_registry
        && connection.registry.admission == plan.predecessor_policy
        && participant_catalogs(connection, &plan.successor_policy)? == plan.participant_catalogs;
    validate_live_plan_state(&connection.admission, plan, predecessor_matches)
}

fn validate_live_plan_state(
    admission: &FleetAdmissionStatusResponse,
    plan: &AdmissionPlanFile,
    predecessor_matches: bool,
) -> Result<(), AdmissionCommandError> {
    if exact_retained_operation(admission, plan) {
        return Ok(());
    }
    if predecessor_matches {
        return require_idle_admission(admission);
    }
    Err(authority_error(
        "plan no longer matches the live Registry, participants, or retained operation",
    ))
}

fn exact_retained_operation(
    admission: &FleetAdmissionStatusResponse,
    plan: &AdmissionPlanFile,
) -> bool {
    admission
        .current_operation
        .as_ref()
        .or(admission.last_result.as_ref())
        .is_some_and(|operation| {
            operation.operation_id == plan.request.operation_id
                && operation.action == plan.request.action
                && operation.selector == plan.request.selector
                && operation.principal == plan.request.principal
                && operation_successor(operation)
                    == (
                        plan.successor_policy.generation,
                        plan.successor_policy.policy_digest,
                    )
        })
}

fn validate_live_authority(
    initial: &FleetRegistry,
    registry: &FleetRegistry,
    version: &FleetRegistryVersion,
    admission: &FleetAdmissionStatusResponse,
) -> Result<(), AdmissionCommandError> {
    let fleet = &registry.authority.binding.fleet;
    let narrower_references = registry
        .admission
        .rules
        .iter()
        .map(|rule| rule.principals.len())
        .sum::<usize>();
    let registry_policy_matches_status = if admission.active.generation
        == registry.admission.generation
        && admission.active.policy_digest == registry.admission.policy_digest
    {
        usize::from(admission.active.fleet_principal_count)
            == registry.admission.fleet_principals.len()
            && usize::from(admission.active.narrower_rule_count) == registry.admission.rules.len()
            && usize::from(admission.active.narrower_principal_reference_count)
                == narrower_references
    } else {
        admission
            .current_operation
            .as_ref()
            .is_some_and(|operation| {
                operation_successor_status(operation).is_some_and(|successor| {
                    successor.generation == registry.admission.generation
                        && successor.policy_digest == registry.admission.policy_digest
                        && usize::from(successor.fleet_principal_count)
                            == registry.admission.fleet_principals.len()
                        && usize::from(successor.narrower_rule_count)
                            == registry.admission.rules.len()
                        && usize::from(successor.narrower_principal_reference_count)
                            == narrower_references
                })
            })
    };
    let exact = registry.authority == version.authority
        && registry.revision == version.revision
        && registry.authority == initial.authority
        && registry.fleet_subnet_roots == initial.fleet_subnet_roots
        && admission.fleet == fleet.clone()
        && registry_policy_matches_status;
    if exact {
        Ok(())
    } else {
        Err(authority_error(
            "live Coordinator Registry and admission status disagree with current ensure authority",
        ))
    }
}

fn query_coordinator_registry(
    icp: &IcpCli,
    binding: &ResolvedProtocolBinding,
    coordinator: Principal,
) -> Result<FleetRegistry, AdmissionCommandError> {
    let response: Result<RemoteCoordinatorStatusResponse, Error> = query_canister_with_arg(
        icp,
        binding,
        coordinator,
        canic_core::protocol::CANIC_STATUS,
        &RemoteCoordinatorStatusRequest::Registry,
    )?;
    match response {
        Ok(RemoteCoordinatorStatusResponse::Registry(registry)) => Ok(registry),
        Ok(_) => Err(authority_error(
            "Coordinator returned the wrong Registry status variant",
        )),
        Err(error) => Err(rejected(error)),
    }
}

fn query_coordinator_registry_version(
    icp: &IcpCli,
    binding: &ResolvedProtocolBinding,
    coordinator: Principal,
) -> Result<FleetRegistryVersion, AdmissionCommandError> {
    let response: Result<RemoteCoordinatorStatusResponse, Error> = query_canister_with_arg(
        icp,
        binding,
        coordinator,
        canic_core::protocol::CANIC_STATUS,
        &RemoteCoordinatorStatusRequest::RegistryVersion,
    )?;
    match response {
        Ok(RemoteCoordinatorStatusResponse::RegistryVersion(version)) => Ok(version),
        Ok(_) => Err(authority_error(
            "Coordinator returned the wrong Registry-version variant",
        )),
        Err(error) => Err(rejected(error)),
    }
}

fn query_coordinator_admission(
    icp: &IcpCli,
    binding: &ResolvedProtocolBinding,
    coordinator: Principal,
) -> Result<FleetAdmissionStatusResponse, AdmissionCommandError> {
    let response: Result<RemoteCoordinatorStatusResponse, Error> = query_canister_with_arg(
        icp,
        binding,
        coordinator,
        canic_core::protocol::CANIC_STATUS,
        &RemoteCoordinatorStatusRequest::Admission(FleetAdmissionStatusRequest {
            selector: FleetAdmissionSelector::Fleet,
            page: PageRequest {
                limit: 1,
                offset: 0,
            },
        }),
    )?;
    match response {
        Ok(RemoteCoordinatorStatusResponse::Admission(status)) => Ok(status),
        Ok(_) => Err(authority_error(
            "Coordinator returned the wrong admission status variant",
        )),
        Err(error) => Err(rejected(error)),
    }
}

fn query_root_report(
    connection: &AdmissionConnection,
    root: Principal,
) -> Result<AdmissionRootReport, AdmissionCommandError> {
    let (status, participants) = query_root_status(connection, root)?;
    Ok(root_report(root, &status, &participants))
}

fn query_root_status(
    connection: &AdmissionConnection,
    root: Principal,
) -> Result<
    (
        FleetAdmissionRootStatusResponse,
        Vec<FleetAdmissionRootParticipantStatus>,
    ),
    AdmissionCommandError,
> {
    let mut offset = 0_u64;
    let mut retained: Option<FleetAdmissionRootStatusResponse> = None;
    let mut participants = Vec::new();
    loop {
        let binding = connection
            .root_bindings
            .get(&root)
            .ok_or_else(|| authority_error("Root is absent from current ensure authority"))?;
        let response: Result<RemoteRootStatusResponse, Error> = query_canister_with_arg(
            &connection.icp,
            binding,
            root,
            canic_core::protocol::CANIC_STATUS,
            &RemoteRootStatusRequest::Admission(PageRequest {
                limit: ROOT_STATUS_PAGE_SIZE,
                offset,
            }),
        )?;
        let page = match response {
            Ok(RemoteRootStatusResponse::Admission(status)) => status,
            Err(error) => return Err(rejected(error)),
        };
        if let Some(first) = retained.as_ref()
            && !same_root_status_head(first, &page)
        {
            return Err(authority_error(
                "Root admission status changed during pagination",
            ));
        }
        let total = page.participants.total;
        participants.extend(page.participants.entries.iter().cloned());
        retained.get_or_insert(page);
        if u64::try_from(participants.len()).unwrap_or(u64::MAX) >= total {
            break;
        }
        let next = u64::try_from(participants.len()).map_err(|_| {
            authority_error("Root admission participant count does not fit status offset")
        })?;
        if next <= offset {
            return Err(authority_error("Root admission pagination did not advance"));
        }
        offset = next;
    }
    let status = retained.ok_or_else(|| authority_error("Root returned no admission status"))?;
    let active_matches = status.active_generation == connection.admission.active.generation
        && status.active_policy_digest == connection.admission.active.policy_digest;
    let successor_matches = connection
        .admission
        .current_operation
        .as_ref()
        .map(operation_successor)
        .is_some_and(|(generation, digest)| {
            status.active_generation == generation && status.active_policy_digest == digest
        });
    if !active_matches && !successor_matches {
        return Err(authority_error(
            "Root admission policy differs from the Coordinator active policy",
        ));
    }
    Ok((status, participants))
}

fn participant_catalogs(
    connection: &AdmissionConnection,
    successor: &FleetAdmissionPolicy,
) -> Result<Vec<AdmissionParticipantCatalog>, AdmissionCommandError> {
    let mut catalogs = Vec::new();
    for root in connection
        .registry
        .fleet_subnet_roots
        .iter()
        .filter(|root| root.status == FleetSubnetRootStatus::Active)
    {
        let fleet_subnet_root = root.fleet_subnet_root;
        let (status, participants) = query_root_status(connection, fleet_subnet_root)?;
        status
            .participant_catalog_digest
            .filter(|digest| digest != &[0; 32])
            .ok_or_else(|| authority_error("Root returned no live participant catalog"))?;
        if status.operation_id.is_some()
            || status.phase.is_some()
            || participants
                .iter()
                .any(|participant| participant.phase != FleetAdmissionRootParticipantPhase::Open)
            || usize::try_from(status.participants.total).ok() != Some(participants.len())
        {
            return Err(authority_error(
                "Root participant catalog is incomplete or not converged",
            ));
        }
        let targets = participants
            .into_iter()
            .map(|participant| participant.target)
            .collect::<Vec<_>>();
        let projections = targets
            .iter()
            .map(|target| {
                let selector = fleet_admission_target_for_binding(target);
                let principals = effective_fleet_admission_principals(successor, &selector);
                materialize_fleet_admission_projection(successor, target.clone(), principals)
                    .map_err(|error| authority_error(error.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        catalogs.push(AdmissionParticipantCatalog {
            fleet_subnet_root,
            participant_catalog_digest: fleet_admission_root_participant_catalog_digest(
                &projections,
            ),
            participants: targets,
        });
    }
    catalogs.sort_unstable_by(|left, right| {
        left.fleet_subnet_root
            .as_slice()
            .cmp(right.fleet_subnet_root.as_slice())
    });
    if catalogs.is_empty()
        || catalogs
            .windows(2)
            .any(|pair| pair[0].fleet_subnet_root == pair[1].fleet_subnet_root)
    {
        return Err(authority_error(
            "Fleet has no canonical active Root participant catalogs",
        ));
    }
    Ok(catalogs)
}

fn participant_catalog_authorities(
    catalogs: &[AdmissionParticipantCatalog],
) -> Result<Vec<FleetAdmissionRootCatalogAuthorityModel>, AdmissionCommandError> {
    let catalogs_are_canonical = !catalogs.is_empty()
        && catalogs.windows(2).all(|pair| {
            pair[0].fleet_subnet_root.as_slice() < pair[1].fleet_subnet_root.as_slice()
        });
    let total = catalogs.iter().try_fold(0_usize, |total, catalog| {
        total.checked_add(catalog.participants.len())
    });
    let participants_are_canonical = catalogs.iter().all(|catalog| {
        catalog.participant_catalog_digest != [0; 32]
            && catalog.participants.len() <= MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS
            && catalog
                .participants
                .iter()
                .all(|target| target_root(target) == catalog.fleet_subnet_root)
            && catalog.participants.windows(2).all(|pair| {
                target_principal(&pair[0]).as_slice() < target_principal(&pair[1]).as_slice()
            })
    });
    if !catalogs_are_canonical
        || !participants_are_canonical
        || total.is_none_or(|total| total > MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS)
    {
        return Err(authority_error(
            "plan participant catalogs are noncanonical or exceed the protocol bound",
        ));
    }
    catalogs
        .iter()
        .map(|catalog| {
            let participant_count = u32::try_from(catalog.participants.len()).map_err(|_| {
                authority_error("Root participant count exceeds the protocol bound")
            })?;
            Ok(FleetAdmissionRootCatalogAuthorityModel {
                fleet_subnet_root: catalog.fleet_subnet_root,
                participant_catalog_digest: catalog.participant_catalog_digest,
                participant_count,
            })
        })
        .collect()
}

fn aggregate_participant_catalog_authority(
    catalogs: &[FleetAdmissionRootCatalogAuthorityModel],
) -> Result<([u8; 32], u32), AdmissionCommandError> {
    let participant_count = catalogs.iter().try_fold(0_u32, |total, catalog| {
        total.checked_add(catalog.participant_count)
    });
    let participant_count = participant_count
        .filter(|count| {
            usize::try_from(*count)
                .is_ok_and(|count| count <= MAX_FLEET_ADMISSION_ROOT_PARTICIPANTS)
        })
        .ok_or_else(|| authority_error("participant catalog count exceeds the protocol bound"))?;
    Ok((
        fleet_admission_participant_catalog_digest(catalogs),
        participant_count,
    ))
}

fn root_report(
    root: Principal,
    status: &FleetAdmissionRootStatusResponse,
    participants: &[FleetAdmissionRootParticipantStatus],
) -> AdmissionRootReport {
    let count = |phase| {
        u64::try_from(
            participants
                .iter()
                .filter(|participant| participant.phase == phase)
                .count(),
        )
        .unwrap_or(u64::MAX)
    };
    AdmissionRootReport {
        root,
        active_generation: status.active_generation,
        active_policy_digest: hex_bytes(status.active_policy_digest),
        operation_id: status.operation_id.map(hex_bytes),
        phase: status.phase.map(root_phase_label).map(str::to_string),
        participant_count: status.participants.total,
        pending_count: count(FleetAdmissionRootParticipantPhase::Pending),
        prepared_count: count(FleetAdmissionRootParticipantPhase::Prepared),
        activated_count: count(FleetAdmissionRootParticipantPhase::Activated),
        open_count: count(FleetAdmissionRootParticipantPhase::Open),
        first_unresolved: participants
            .iter()
            .find(|participant| participant.phase != FleetAdmissionRootParticipantPhase::Open)
            .map(|participant| target_principal(&participant.target)),
    }
}

fn same_root_status_head(
    left: &FleetAdmissionRootStatusResponse,
    right: &FleetAdmissionRootStatusResponse,
) -> bool {
    left.operation_id == right.operation_id
        && left.phase == right.phase
        && left.active_generation == right.active_generation
        && left.active_policy_digest == right.active_policy_digest
        && left.successor_generation == right.successor_generation
        && left.successor_policy_digest == right.successor_policy_digest
        && left.participant_catalog_digest == right.participant_catalog_digest
        && left.participants.total == right.participants.total
}

fn require_idle_admission(
    status: &FleetAdmissionStatusResponse,
) -> Result<(), AdmissionCommandError> {
    if status.current_operation.is_some() {
        Err(authority_error(
            "another Fleet-admission mutation is active",
        ))
    } else {
        Ok(())
    }
}

fn require_selector_exists(
    registry: &FleetRegistry,
    selector: &FleetAdmissionSelector,
) -> Result<(), AdmissionCommandError> {
    let exists = match selector {
        FleetAdmissionSelector::Fleet => true,
        FleetAdmissionSelector::ComponentSpec(spec) => registry
            .component_specs
            .iter()
            .any(|entry| &entry.component_spec == spec),
        FleetAdmissionSelector::ComponentInstance(component) => {
            registry.services.iter().any(|service| {
                service
                    .members
                    .iter()
                    .any(|member| &member.component == component)
            })
        }
        FleetAdmissionSelector::FleetSubnetRoot(subnet) => {
            registry.fleet_subnet_roots.iter().any(|root| {
                &root.placement_subnet == subnet && root.status == FleetSubnetRootStatus::Active
            })
        }
    };
    if exists {
        Ok(())
    } else {
        Err(AdmissionCommandError::InvalidSelector(
            "selector is absent from the active Fleet Registry".to_string(),
        ))
    }
}

fn parse_principal(value: &str) -> Result<Principal, AdmissionCommandError> {
    Principal::from_text(value).map_err(|error| AdmissionCommandError::InvalidPrincipal {
        value: value.to_string(),
        reason: error.to_string(),
    })
}

fn parse_selector(
    matches: &clap::ArgMatches,
) -> Result<FleetAdmissionSelector, AdmissionCommandError> {
    if matches.get_flag(FLEET_SELECTOR_ARG) {
        return Ok(FleetAdmissionSelector::Fleet);
    }
    if let Some(value) = string_option(matches, COMPONENT_SPEC_ARG) {
        return value
            .parse()
            .map(FleetAdmissionSelector::ComponentSpec)
            .map_err(|error: canic_core::ids::ComponentSpecIdParseError| {
                AdmissionCommandError::InvalidSelector(error.to_string())
            });
    }
    if let Some(value) = string_option(matches, COMPONENT_INSTANCE_ARG) {
        return value
            .parse::<ComponentInstanceId>()
            .map(FleetAdmissionSelector::ComponentInstance)
            .map_err(|error| AdmissionCommandError::InvalidSelector(error.to_string()));
    }
    if let Some(value) = string_option(matches, FLEET_SUBNET_ROOT_ARG) {
        return Principal::from_text(&value)
            .map(SubnetId::from_principal)
            .map(FleetAdmissionSelector::FleetSubnetRoot)
            .map_err(|error| AdmissionCommandError::InvalidSelector(error.to_string()));
    }
    unreachable!("Clap requires one admission selector")
}

const fn action_model(action: FleetAdmissionMutationAction) -> FleetAdmissionMutationActionModel {
    match action {
        FleetAdmissionMutationAction::Add => FleetAdmissionMutationActionModel::Add,
        FleetAdmissionMutationAction::Remove => FleetAdmissionMutationActionModel::Remove,
    }
}

fn operation_identity(
    operation: &canic_core::dto::fleet_admission::FleetAdmissionOperationStatusResponse,
) -> (String, String) {
    (
        hex_bytes(operation.operation_id),
        operation_phase_label(&operation.phase).to_string(),
    )
}

const fn operation_successor(
    operation: &canic_core::dto::fleet_admission::FleetAdmissionOperationStatusResponse,
) -> (u64, [u8; 32]) {
    match &operation.phase {
        FleetAdmissionOperationPhase::Planned { successor }
        | FleetAdmissionOperationPhase::Preparing { successor }
        | FleetAdmissionOperationPhase::Releasing { successor }
        | FleetAdmissionOperationPhase::PerimeterFenced { successor }
        | FleetAdmissionOperationPhase::Activating { successor }
        | FleetAdmissionOperationPhase::Opening { successor } => {
            (successor.generation, successor.policy_digest)
        }
        FleetAdmissionOperationPhase::Completed(response) => {
            (response.generation, response.policy_digest)
        }
    }
}

const fn operation_successor_status(
    operation: &canic_core::dto::fleet_admission::FleetAdmissionOperationStatusResponse,
) -> Option<&canic_core::dto::fleet_admission::FleetAdmissionPolicyStatus> {
    match &operation.phase {
        FleetAdmissionOperationPhase::Planned { successor }
        | FleetAdmissionOperationPhase::Preparing { successor }
        | FleetAdmissionOperationPhase::Releasing { successor }
        | FleetAdmissionOperationPhase::PerimeterFenced { successor }
        | FleetAdmissionOperationPhase::Activating { successor }
        | FleetAdmissionOperationPhase::Opening { successor } => Some(successor),
        FleetAdmissionOperationPhase::Completed(_) => None,
    }
}

const fn operation_phase_label(phase: &FleetAdmissionOperationPhase) -> &'static str {
    match phase {
        FleetAdmissionOperationPhase::Planned { .. } => "planned",
        FleetAdmissionOperationPhase::Preparing { .. } => "preparing",
        FleetAdmissionOperationPhase::Releasing { .. } => "releasing",
        FleetAdmissionOperationPhase::PerimeterFenced { .. } => "perimeter_fenced",
        FleetAdmissionOperationPhase::Activating { .. } => "activating",
        FleetAdmissionOperationPhase::Opening { .. } => "opening",
        FleetAdmissionOperationPhase::Completed(_) => "completed",
    }
}

const fn root_phase_label(phase: FleetAdmissionRootTransitionPhase) -> &'static str {
    match phase {
        FleetAdmissionRootTransitionPhase::Preparing => "preparing",
        FleetAdmissionRootTransitionPhase::PerimeterFenced => "perimeter_fenced",
        FleetAdmissionRootTransitionPhase::Activating => "activating",
        FleetAdmissionRootTransitionPhase::Opening => "opening",
        FleetAdmissionRootTransitionPhase::Converged => "converged",
        FleetAdmissionRootTransitionPhase::Released => "released",
    }
}

const fn mutation_outcome_label(outcome: FleetAdmissionMutationOutcome) -> &'static str {
    match outcome {
        FleetAdmissionMutationOutcome::Planned => "planned",
        FleetAdmissionMutationOutcome::Converged => "converged",
        FleetAdmissionMutationOutcome::CatalogChanged => "catalog_changed",
        FleetAdmissionMutationOutcome::AlreadyPresent => "already_present",
        FleetAdmissionMutationOutcome::AlreadyAbsent => "already_absent",
    }
}

const fn target_principal(target: &canic_core::ids::ManagedCanisterBinding) -> Principal {
    match target {
        canic_core::ids::ManagedCanisterBinding::Component(binding) => binding.canister_id,
        canic_core::ids::ManagedCanisterBinding::ComponentChild(binding) => binding.canister_id,
    }
}

const fn target_root(target: &ManagedCanisterBinding) -> Principal {
    match target {
        ManagedCanisterBinding::Component(component) => component.fleet_subnet_root,
        ManagedCanisterBinding::ComponentChild(child) => child.component.fleet_subnet_root,
    }
}

fn render_status(report: &AdmissionStatusReport) -> String {
    let mut lines = vec![
        format!("Fleet admission: {}/{}", report.environment, report.fleet),
        format!("  coordinator: {}", report.coordinator),
        format!("  Registry revision: {}", report.registry_revision),
        format!("  generation: {}", report.generation),
        format!("  policy digest: {}", report.policy_digest),
        format!("  Fleet Principals: {}", report.fleet_principal_count),
        format!(
            "  narrower rules: {} ({} Principal references)",
            report.narrower_rule_count, report.narrower_principal_reference_count
        ),
        format!(
            "  current: {}",
            report.current_phase.as_deref().unwrap_or("none")
        ),
    ];
    for root in &report.roots {
        lines.push(format!(
            "  root {}: generation={} phase={} participants={} pending={} prepared={} activated={} open={}{}",
            root.root,
            root.active_generation,
            root.phase.as_deref().unwrap_or("idle"),
            root.participant_count,
            root.pending_count,
            root.prepared_count,
            root.activated_count,
            root.open_count,
            root.first_unresolved
                .map(|target| format!(" first_unresolved={target}"))
                .unwrap_or_default(),
        ));
    }
    lines.join("\n")
}

fn rejected(error: Error) -> AdmissionCommandError {
    AdmissionCommandError::Rejected(canic_host::diagnostics::render_diagnostic(error.code()))
}

fn authority_error(message: impl Into<String>) -> AdmissionCommandError {
    AdmissionCommandError::Authority(message.into())
}

fn hex_bytes(bytes: [u8; 32]) -> String {
    canic_core::cdk::utils::hash::hex_bytes(bytes)
}

fn admission_command() -> ClapCommand {
    [APPLY_COMMAND, PLAN_COMMAND, STATUS_COMMAND]
        .into_iter()
        .fold(
            ClapCommand::new("admission").bin_name("canic admission"),
            |command, name| {
                command.subcommand(passthrough_subcommand(
                    ClapCommand::new(name).disable_help_flag(true),
                ))
            },
        )
}

fn admission_usage() -> String {
    "Manage protected Fleet-wide ingress admission\n\nUsage: canic admission <command> [OPTIONS]\n\nCommands:\n  apply   Apply one exact accepted admission plan\n  plan    Build a read-only exact admission plan\n  status  Inspect protected Fleet admission convergence\n  help    Print this message or the help of the given subcommand(s)\n\nExamples:\n  canic admission plan demo --add <principal> --fleet --out admission.json\n  canic admission apply demo admission.json\n  canic admission status demo".to_string()
}

fn plan_command() -> ClapCommand {
    ClapCommand::new(PLAN_COMMAND)
        .bin_name("canic admission plan")
        .about("Build a read-only exact Fleet-admission mutation plan")
        .disable_help_flag(true)
        .arg(value_arg(FLEET_ARG).value_name(FLEET_ARG).required(true))
        .arg(value_arg(ADD_ARG).long(ADD_ARG).value_name("PRINCIPAL"))
        .arg(value_arg(REMOVE_ARG).long(REMOVE_ARG).value_name("PRINCIPAL"))
        .group(ArgGroup::new("action").args([ADD_ARG, REMOVE_ARG]).required(true).multiple(false))
        .arg(flag_arg(FLEET_SELECTOR_ARG).long("fleet"))
        .arg(value_arg(COMPONENT_INSTANCE_ARG).long(COMPONENT_INSTANCE_ARG).value_name("ID"))
        .arg(value_arg(COMPONENT_SPEC_ARG).long(COMPONENT_SPEC_ARG).value_name("ID"))
        .arg(value_arg(FLEET_SUBNET_ROOT_ARG).long(FLEET_SUBNET_ROOT_ARG).value_name("SUBNET_ID"))
        .group(
            ArgGroup::new("selector")
                .args([
                    FLEET_SELECTOR_ARG,
                    COMPONENT_INSTANCE_ARG,
                    COMPONENT_SPEC_ARG,
                    FLEET_SUBNET_ROOT_ARG,
                ])
                .required(true)
                .multiple(false),
        )
        .arg(value_arg(OUT_ARG).long(OUT_ARG).value_name("FILE").required(true))
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
        .after_help("Examples:\n  canic admission plan demo --add <principal> --fleet --out admission.json\n  canic admission plan demo --remove <principal> --component-spec core --out admission.json")
}

fn apply_command() -> ClapCommand {
    ClapCommand::new(APPLY_COMMAND)
        .bin_name("canic admission apply")
        .about("Apply one exact accepted Fleet-admission plan")
        .disable_help_flag(true)
        .arg(value_arg(FLEET_ARG).value_name(FLEET_ARG).required(true))
        .arg(
            value_arg(PLAN_FILE_ARG)
                .value_name("PLAN_FILE")
                .required(true),
        )
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
        .after_help("Example:\n  canic admission apply demo admission.json")
}

fn status_command() -> ClapCommand {
    ClapCommand::new(STATUS_COMMAND)
        .bin_name("canic admission status")
        .about("Inspect protected Fleet-admission convergence")
        .disable_help_flag(true)
        .arg(value_arg(FLEET_ARG).value_name(FLEET_ARG).required(true))
        .arg(flag_arg(JSON_ARG).long(JSON_ARG))
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
        .after_help(
            "Examples:\n  canic admission status demo\n  canic admission status demo --json",
        )
}

fn plan_usage() -> String {
    render_usage(plan_command)
}

fn apply_usage() -> String {
    render_usage(apply_command)
}

fn status_usage() -> String {
    render_usage(status_command)
}
