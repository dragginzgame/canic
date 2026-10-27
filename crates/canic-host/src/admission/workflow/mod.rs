//! Fleet admission workflow.
//!
//! Host owns authority and operations; command parsing and rendering belong to CLI.

use crate::{
    admission::{
        AdmissionError, AdmissionTarget, ROOT_STATUS_PAGE_SIZE, authority_error,
        model::{
            AdmissionConnection, AdmissionMutation, AdmissionParticipantCatalogRecord,
            AdmissionPlanRecord,
        },
        ops::{
            assemble_plan, exact_retained_operation, operation_identity, operation_successor,
            project_participant_catalog, query_coordinator_admission, query_coordinator_registry,
            query_coordinator_registry_version, require_idle_admission, root_report,
            same_root_status_head, submit, successor_policy, validate_live_authority,
            validate_live_plan_state, validate_plan_file,
        },
        view::{AdmissionRootReport, AdmissionStatusReport},
    },
    fleet_ensure::resolve_current_fleet,
    icp::IcpCli,
    icp_config::resolve_current_canic_icp_root,
};
use canic_contracts::{
    dto::{
        fleet_admission::{
            FleetAdmissionMutationAction, FleetAdmissionRootParticipantStatus,
            FleetAdmissionRootStatusResponse,
        },
        fleet_registry::FleetSubnetRootStatus,
        page::PageRequest,
    },
    ids::{FleetAdmissionPolicy, FleetAdmissionSelector},
};
use canic_core::cdk::utils::hash::hex_bytes;
use ic_platform::principal::ExternalPrincipal as Principal;
use std::collections::BTreeMap;

/// Observe Coordinator and Root convergence against the same current Fleet authority.
pub fn collect_status(
    fleet: &str,
    target: &AdmissionTarget,
) -> Result<AdmissionStatusReport, AdmissionError> {
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

fn connect(fleet: &str, target: &AdmissionTarget) -> Result<AdmissionConnection, AdmissionError> {
    let root = resolve_current_canic_icp_root().map_err(AdmissionError::IcpRoot)?;
    let current = resolve_current_fleet(&root, &target.environment, fleet)?;
    let initial_registry = current.initial_active_registry(fleet)?.clone();
    let coordinator = initial_registry.authority.binding.coordinator;
    let coordinator_schema = super::ops::current_admission_schema(&current, &root, coordinator)?;
    let mut root_schemas = BTreeMap::new();
    for fleet_subnet_root in &current.topology.fleet_subnet_root_canister_ids {
        let fleet_subnet_root = fleet_subnet_root
            .parse::<Principal>()
            .map_err(|error| authority_error(error.to_string()))?;
        root_schemas.insert(
            fleet_subnet_root,
            super::ops::current_admission_schema(&current, &root, fleet_subnet_root)?,
        );
    }
    let icp = IcpCli::new(&target.icp, Some(target.environment.clone())).with_cwd(&root);
    let registry = query_coordinator_registry(&icp, coordinator)?;
    let registry_version = query_coordinator_registry_version(&icp, &current, &root, coordinator)?;
    let admission = query_coordinator_admission(&icp, &coordinator_schema, coordinator)?;
    validate_live_authority(&initial_registry, &registry, &registry_version, &admission)?;
    Ok(AdmissionConnection {
        coordinator,
        icp,
        root_schemas,
        registry,
        registry_version,
        admission,
    })
}

fn build_plan(
    fleet: &str,
    environment: &str,
    connection: &AdmissionConnection,
    action: FleetAdmissionMutationAction,
    selector: FleetAdmissionSelector,
    principal: Principal,
) -> Result<AdmissionPlanRecord, AdmissionError> {
    let successor = successor_policy(connection, action, &selector, principal)?;
    let catalogs = participant_catalogs(connection, &successor)?;
    assemble_plan(
        fleet,
        environment,
        connection,
        AdmissionMutation {
            action,
            selector,
            principal,
        },
        successor,
        catalogs,
    )
}

fn validate_live_plan(
    connection: &AdmissionConnection,
    plan: &AdmissionPlanRecord,
) -> Result<(), AdmissionError> {
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

fn query_root_report(
    connection: &AdmissionConnection,
    root: Principal,
) -> Result<AdmissionRootReport, AdmissionError> {
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
    AdmissionError,
> {
    let mut offset = 0_u64;
    let mut retained: Option<FleetAdmissionRootStatusResponse> = None;
    let mut participants = Vec::new();
    loop {
        let schema = connection
            .root_schemas
            .get(&root)
            .ok_or_else(|| authority_error("Root is absent from current ensure authority"))?;
        let page = super::ops::query_root_admission(
            &connection.icp,
            schema,
            root,
            PageRequest {
                limit: ROOT_STATUS_PAGE_SIZE,
                offset,
            },
        )?;
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
) -> Result<Vec<AdmissionParticipantCatalogRecord>, AdmissionError> {
    let mut catalogs = Vec::new();
    for root in connection
        .registry
        .fleet_subnet_roots
        .iter()
        .filter(|root| root.status == FleetSubnetRootStatus::Active)
    {
        let fleet_subnet_root = root.fleet_subnet_root;
        let (status, participants) = query_root_status(connection, fleet_subnet_root)?;
        catalogs.push(project_participant_catalog(
            fleet_subnet_root,
            status,
            participants,
            successor,
        )?);
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

/// Build an effect-free plan bound to current Fleet authority and participant catalogs.
pub fn plan(
    fleet: &str,
    target: &AdmissionTarget,
    action: FleetAdmissionMutationAction,
    selector: FleetAdmissionSelector,
    principal: Principal,
) -> Result<AdmissionPlanRecord, AdmissionError> {
    let connection = connect(fleet, target)?;
    require_idle_admission(&connection.admission)?;
    build_plan(
        fleet,
        &target.environment,
        &connection,
        action,
        selector,
        principal,
    )
}

/// Revalidate an accepted plan against live authority before sending its exact mutation.
pub fn apply(
    fleet: &str,
    target: &AdmissionTarget,
    plan: &AdmissionPlanRecord,
) -> Result<canic_contracts::dto::fleet_admission::FleetAdmissionMutationResponse, AdmissionError> {
    validate_plan_file(fleet, &target.environment, plan)?;
    let connection = connect(fleet, target)?;
    validate_live_plan(&connection, plan)?;
    submit(&connection, plan)
}
