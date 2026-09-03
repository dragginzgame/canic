//! Module: fleet_ensure::ops::current_inventory
//!
//! Responsibility: derive one complete terminal live inventory from current protocol authority.
//! Does not own: topology decisions, convergence sequencing, or historical installation state.
//! Boundary: exact terminal Registry, Root children, and current release artifacts are required.

use super::TerminalFleetInventory;
use super::current_protocol::{
    CurrentProtocolError, operation_bytes, query_current_root_authorities, query_operation,
    query_registry,
};
use crate::{
    canister_protocol::{call_with_candid, query_with_candid},
    durable_io::{RegularFileReadError, read_optional_regular_bytes},
    fleet_ensure::model::{
        DesiredCanisterKind, DesiredFleet, DesiredPresence, FleetEnsureStateRecord,
    },
    icp::IcpCli,
    protocol_binding::RegistryProtocolBinding,
    registry::RegistryEntry,
    release_build::load_finalized_release_build,
    release_set::{
        AppConfigSnapshot, CanicInfrastructureArtifactEntry, CanicInfrastructureRole,
        load_persisted_application_artifact_union,
        load_persisted_canic_infrastructure_artifact_manifest,
        validate_release_artifact_relative_path,
    },
    role_contract::{PackageValidationMode, resolve_declared_role_contract},
};
use candid::{CandidType, Principal};
use canic_control_plane::dto::root::RootOperationStatusResponse;
use canic_core::{
    cdk::utils::hash::hex_bytes,
    control_plane_support::{config::ComponentTopology, ops::fleet_registry::FleetRegistryOps},
    dto::{
        canister::{CanisterInfo, CanisterInspectionRequest, CanisterStatusResponse},
        component_provisioning::{
            FleetComponentProvisioningPhase, FleetComponentProvisioningStatusResponse,
            RootComponentProvisioningPhase, RootComponentProvisioningResult,
            RootComponentProvisioningStatusResponse,
        },
        component_registry::{
            ComponentLifecycleStatus, ComponentProvisioningOrigin,
            ComponentRegistryActivePartitionRequest, ComponentRegistryActivePartitionResponse,
            ComponentRegistryPartitionResponse, RootComponentAllocationPhase,
            RootComponentChildAllocationResponse,
        },
        fleet_registry::{
            FleetRegistry, FleetRegistryVersion, FleetSubnetRootEntry, FleetSubnetRootStatus,
        },
        page::{Page, PageRequest},
        pool::{
            CanisterPoolAssetStatus, CanisterPoolClaim, CanisterPoolResponse,
            CanisterPoolStatusRequest,
        },
        role::OperationStatusRequest,
    },
    ids::{
        CanisterRole, ComponentBinding, ComponentDeploymentConfigurationDigest,
        ComponentInstanceId, FleetSubnetRootReleaseSet,
    },
    protocol,
    role_contract::{RoleCapabilityKey, RoleContractResolution, derive_protocol_profile_hashes},
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::{Path, PathBuf},
};

const CHILD_PAGE_LIMIT: u64 = 1_000;

#[derive(CandidType)]
enum ChildrenStatusRequest {
    Children(PageRequest),
}

#[derive(CandidType, Deserialize)]
enum ChildrenStatusResponse {
    Children(Page<CanisterInfo>),
}

#[derive(CandidType)]
enum RootInventoryCommand {
    InspectCanister(CanisterInspectionRequest),
}

#[derive(CandidType, Deserialize)]
enum RootInventoryCommandResponse {
    InspectCanister(CanisterStatusResponse),
}

#[derive(CandidType)]
enum RootInventoryStatusRequest {
    ComponentRegistryActivePartition(ComponentRegistryActivePartitionRequest),
    Operation(OperationStatusRequest),
    Pool(CanisterPoolStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum RootInventoryStatusResponse {
    ComponentRegistryActivePartition(Box<ComponentRegistryActivePartitionResponse>),
    Operation(Box<RootOperationStatusResponse>),
    Pool(Box<CanisterPoolResponse>),
}

struct ProtocolCatalog {
    by_role: BTreeMap<CanisterRole, ProtocolEntry>,
    coordinator: ProtocolEntry,
    root: ProtocolEntry,
}

struct ProtocolEntry {
    binding: RegistryProtocolBinding,
    candid_path: PathBuf,
    module_hash: String,
}

struct ComponentPartitionAuthority<'a> {
    active_release_set: &'a FleetSubnetRootReleaseSet,
    group_placement: &'a canic_core::ids::ComponentGroupPlacementId,
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
}

struct TerminalRootComponentAuthority<'a> {
    active_release_set: &'a FleetSubnetRootReleaseSet,
    active_fleet_registry: &'a FleetRegistryVersion,
    configuration_digest: ComponentDeploymentConfigurationDigest,
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    registry: &'a FleetRegistry,
    root: &'a FleetSubnetRootEntry,
    source_fleet_registry: &'a FleetRegistryVersion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TerminalWorkloadAuthority {
    component: ComponentInstanceId,
    operation_id: [u8; 32],
    root: Principal,
}

struct RootComponentSummary {
    component_bindings: BTreeMap<ComponentInstanceId, ComponentBinding>,
    component_ids: BTreeSet<Principal>,
    maximum_workloads: u64,
    workloads: BTreeMap<Principal, TerminalWorkloadAuthority>,
}

struct DescendantParent {
    canister_id: Principal,
    candid_path: PathBuf,
    maximum_children: u64,
    release_set: FleetSubnetRootReleaseSet,
    role: CanisterRole,
    root: Principal,
}

struct TerminalDescendantAuthority<'a> {
    child: &'a CanisterInfo,
    component: &'a ComponentBinding,
    parent_role: &'a CanisterRole,
    protocol: &'a ProtocolEntry,
    release_set: &'a FleetSubnetRootReleaseSet,
    root: Principal,
    workload: &'a TerminalWorkloadAuthority,
}

/// Query one complete bounded current Fleet tree after protocol convergence.
pub(super) fn terminal_inventory(
    icp: &IcpCli,
    root: &Path,
    desired: &DesiredFleet,
    operation_id: &str,
    state: &FleetEnsureStateRecord,
) -> Result<TerminalFleetInventory, CurrentProtocolError> {
    let protocol_intent = desired.protocol.as_ref().ok_or_else(|| {
        inventory_error("managed terminal inventory has no typed protocol intent")
    })?;
    let coordinator = desired
        .canisters
        .iter()
        .find(|canister| {
            canister.presence == DesiredPresence::Present
                && canister.kind == DesiredCanisterKind::Coordinator
        })
        .ok_or_else(|| inventory_error("terminal inventory has no exact Coordinator"))?;
    let coordinator_principal = retained_principal(desired, state, &coordinator.name)?;
    let coordinator_candid = resolve_path(root, &protocol_intent.coordinator_candid);
    let root_candid = resolve_path(root, &protocol_intent.root_candid);
    let store_candid = resolve_path(root, &protocol_intent.store_candid);
    let registry = query_registry(icp, &coordinator_candid, coordinator_principal)?;
    let config_path = resolve_path(root, &protocol_intent.app_config);
    let config = AppConfigSnapshot::load(&config_path)
        .map_err(|error| inventory_error(error.to_string()))?;
    FleetRegistryOps::validate(&registry.authority, config.component_topology(), &registry)
        .map_err(|error| inventory_error(error.to_string()))?;
    let registry_version =
        FleetRegistryOps::version(&registry.authority, config.component_topology(), &registry)
            .map_err(|error| inventory_error(error.to_string()))?;
    let operation_id = operation_bytes(operation_id)?;
    let component_operation = query_operation(
        icp,
        &coordinator_candid,
        coordinator_principal,
        operation_id,
    )?
    .ok_or_else(|| terminal_field_missing("coordinator.operation"))?;
    validate_terminal_coordinator_component_status(
        operation_id,
        &registry_version,
        &component_operation,
    )?;
    let authorities =
        query_current_root_authorities(icp, desired, state, &root_candid, &store_candid)?;
    validate_root_authority(&registry, &authorities)?;
    let release_set = common_release_set(&authorities)?;
    let protocols = ProtocolCatalog::load(
        root,
        &config_path,
        &config,
        release_set,
        &coordinator_candid,
        &root_candid,
        &store_candid,
    )?;
    let (entries, controlled_cycles_by_principal) = query_entries(
        icp,
        &registry,
        &registry_version,
        &component_operation,
        &config,
        &authorities,
        &protocols,
    )?;
    Ok(TerminalFleetInventory {
        active_registry: Some(registry),
        controlled_cycles_by_principal,
        entries,
    })
}

impl ProtocolCatalog {
    fn load(
        root: &Path,
        config_path: &Path,
        config: &AppConfigSnapshot,
        release_set: FleetSubnetRootReleaseSet,
        coordinator_candid: &Path,
        root_candid: &Path,
        store_candid: &Path,
    ) -> Result<Self, CurrentProtocolError> {
        let release_build_id = release_set.release_build_id;
        let infrastructure =
            load_persisted_canic_infrastructure_artifact_manifest(root, release_build_id)
                .map_err(|error| inventory_error(error.to_string()))?;
        let finalized = load_finalized_release_build(root, release_build_id)
            .map_err(|error| inventory_error(error.to_string()))?;
        let application = load_persisted_application_artifact_union(
            root,
            config.component_topology(),
            release_build_id,
        )
        .map_err(|error| inventory_error(error.to_string()))?;
        let infrastructure_entry = |role| {
            infrastructure
                .manifest
                .entries
                .iter()
                .find(|entry| entry.role == role)
                .ok_or_else(|| inventory_error(format!("current release is missing {role:?}")))
        };
        let coordinator = infrastructure_protocol(
            infrastructure_entry(CanicInfrastructureRole::FleetCoordinator)?,
            coordinator_candid,
        )?;
        let root_protocol = infrastructure_protocol(
            infrastructure_entry(CanicInfrastructureRole::FleetSubnetRoot)?,
            root_candid,
        )?;
        let store = infrastructure_protocol(
            infrastructure_entry(CanicInfrastructureRole::WasmStore)?,
            store_candid,
        )?;
        let mut by_role = BTreeMap::from([(store.binding.role.clone(), store)]);
        for artifact in application.union.entries {
            let contract = match resolve_declared_role_contract(
                config_path,
                config.model(),
                &artifact.role,
                PackageValidationMode::Passive,
            ) {
                RoleContractResolution::Resolved { contract } => contract,
                RoleContractResolution::Rejected { errors } => {
                    return Err(inventory_error(format!(
                        "role {} protocol authority is unavailable: {errors:?}",
                        artifact.role
                    )));
                }
            };
            let candid_path = application_candid_sidecar(root, &artifact.wasm_relative_path)?;
            let binding = RegistryProtocolBinding {
                release_identity: finalized.record.builder_version.clone(),
                role: artifact.role.clone(),
                capabilities: contract.capabilities,
                candid_sha256: artifact.candid_sha256,
                protocol_profile_digest: artifact.protocol_profile_digest,
            };
            verify_protocol(&candid_path, &binding)?;
            if by_role
                .insert(
                    artifact.role.clone(),
                    ProtocolEntry {
                        binding,
                        candid_path,
                        module_hash: artifact.wasm_sha256_hex.clone(),
                    },
                )
                .is_some()
            {
                return Err(inventory_error(format!(
                    "role {} has more than one current protocol authority",
                    artifact.role
                )));
            }
        }
        Ok(Self {
            by_role,
            coordinator,
            root: root_protocol,
        })
    }

    fn child(&self, role: &CanisterRole) -> Option<&ProtocolEntry> {
        self.by_role.get(role)
    }
}

fn infrastructure_protocol(
    artifact: &CanicInfrastructureArtifactEntry,
    candid_path: &Path,
) -> Result<ProtocolEntry, CurrentProtocolError> {
    let binding = RegistryProtocolBinding {
        release_identity: artifact.protocol_release_identity.clone(),
        role: artifact.protocol_role.clone(),
        capabilities: artifact.protocol_capabilities.clone(),
        candid_sha256: artifact.candid_sha256,
        protocol_profile_digest: artifact.protocol_profile_digest,
    };
    verify_protocol(candid_path, &binding)?;
    Ok(ProtocolEntry {
        binding,
        candid_path: candid_path.to_path_buf(),
        module_hash: artifact.wasm_sha256_hex.clone(),
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "one terminal walk keeps the exact Registry, release, and bounded output owners explicit"
)]
fn query_entries(
    icp: &IcpCli,
    registry: &FleetRegistry,
    registry_version: &FleetRegistryVersion,
    component_operation: &FleetComponentProvisioningStatusResponse,
    config: &AppConfigSnapshot,
    authorities: &[canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority],
    protocols: &ProtocolCatalog,
) -> Result<(Vec<RegistryEntry>, BTreeMap<String, u128>), CurrentProtocolError> {
    let operation_id = component_operation.operation_id;
    let coordinator = registry.authority.binding.coordinator;
    let coordinator_text = coordinator.to_text();
    let mut entries = vec![RegistryEntry {
        pid: coordinator_text.clone(),
        role: Some(CanisterRole::FLEET_COORDINATOR.to_string()),
        parent_pid: None,
        module_hash: Some(protocols.coordinator.module_hash.clone()),
        protocol_binding: Some(protocols.coordinator.binding.clone()),
    }];
    let mut seen = BTreeSet::from([coordinator]);
    let mut controlled_cycles_by_principal = BTreeMap::new();
    let mut parents = VecDeque::new();
    let mut component_bindings = BTreeMap::new();
    let mut component_workloads = BTreeMap::new();
    let mut pool_workloads = BTreeMap::new();
    for root in registry
        .fleet_subnet_roots
        .iter()
        .filter(|root| root.status != FleetSubnetRootStatus::Removed)
    {
        if !seen.insert(root.fleet_subnet_root) {
            return Err(inventory_error(format!(
                "Root {} is duplicated",
                root.fleet_subnet_root
            )));
        }
        entries.push(RegistryEntry {
            pid: root.fleet_subnet_root.to_text(),
            role: Some(CanisterRole::ROOT.to_string()),
            parent_pid: Some(coordinator_text.clone()),
            module_hash: Some(protocols.root.module_hash.clone()),
            protocol_binding: Some(protocols.root.binding.clone()),
        });
        let authority = authorities
            .iter()
            .find(|authority| authority.binding.fleet_subnet_root == root.fleet_subnet_root)
            .ok_or_else(|| inventory_error("terminal Root has no exact authority"))?;
        let store = authority.wasm_store_authority.wasm_store;
        if !seen.insert(store) {
            return Err(inventory_error(format!("Store {store} is duplicated")));
        }
        let store_protocol = protocols
            .child(&CanisterRole::WASM_STORE)
            .ok_or_else(|| inventory_error("current release has no Store protocol"))?;
        entries.push(RegistryEntry {
            pid: store.to_text(),
            role: Some(CanisterRole::WASM_STORE.to_string()),
            parent_pid: Some(root.fleet_subnet_root.to_text()),
            module_hash: Some(store_protocol.module_hash.clone()),
            protocol_binding: Some(store_protocol.binding.clone()),
        });
        let status = query_root_component_status(
            icp,
            &protocols.root.candid_path,
            root.fleet_subnet_root,
            operation_id,
        )?;
        let component_summary = append_root_components(
            icp,
            &TerminalRootComponentAuthority {
                active_release_set: &root.active_release_set,
                active_fleet_registry: registry_version,
                configuration_digest: component_operation.configuration_digest,
                operation_id,
                plan_hash: component_operation.plan_hash,
                registry,
                root,
                source_fleet_registry: &component_operation.fleet_registry,
            },
            &status,
            protocols,
            &mut seen,
            &mut entries,
            &mut parents,
            &mut controlled_cycles_by_principal,
        )?;
        for (component, binding) in component_summary.component_bindings {
            if component_bindings.insert(component, binding).is_some() {
                return Err(inventory_error(
                    "terminal Component identity has more than one top-level binding",
                ));
            }
        }
        for (canister_id, workload) in component_summary.workloads {
            insert_terminal_workload(
                &mut component_workloads,
                canister_id,
                workload,
                "terminal Component has more than one Root allocation authority",
            )?;
        }
        append_pool_assets(
            icp,
            &protocols.root.candid_path,
            root,
            store,
            &component_summary.component_ids,
            component_summary.maximum_workloads,
            &mut seen,
            &mut entries,
            &mut controlled_cycles_by_principal,
            &mut pool_workloads,
        )?;
    }
    let maximum_entries = maximum_inventory_entries(registry, config.component_topology())?;
    let descendant_bound = maximum_descendant_page(config.component_topology());
    while let Some(parent) = parents.pop_front() {
        for child in query_all_children(
            icp,
            &parent.candid_path,
            parent.canister_id,
            parent.maximum_children,
        )? {
            if child.parent_pid != Some(parent.canister_id) || !seen.insert(child.pid) {
                return Err(inventory_error(format!(
                    "Canister {} has conflicting current parent authority",
                    child.pid
                )));
            }
            let protocol = protocols.child(&child.role);
            if protocol.is_none() {
                return Err(inventory_error(format!(
                    "Canister {} role {} has no current protocol authority",
                    child.pid, child.role
                )));
            }
            let protocol = protocol.expect("checked current protocol");
            let workload = pool_workloads.get(&child.pid).ok_or_else(|| {
                inventory_error(format!(
                    "terminal descendant {} has no exact Root pool Workload claim",
                    child.pid
                ))
            })?;
            terminal_field_exact("descendant.root", &parent.root, &workload.root)?;
            let component = component_bindings.get(&workload.component).ok_or_else(|| {
                inventory_error(format!(
                    "terminal descendant {} names an unknown top-level Component",
                    child.pid
                ))
            })?;
            let descendant = TerminalDescendantAuthority {
                child: &child,
                component,
                parent_role: &parent.role,
                protocol,
                release_set: &parent.release_set,
                root: parent.root,
                workload,
            };
            validate_terminal_descendant_allocation(icp, &protocols.root.candid_path, &descendant)?;
            insert_terminal_workload(
                &mut component_workloads,
                child.pid,
                workload.clone(),
                "terminal Component has more than one Root allocation authority",
            )?;
            let entry = registry_entry(&child, protocol)?;
            insert_controlled_cycles(
                &mut controlled_cycles_by_principal,
                child.pid,
                inspect_root_controlled_cycle_balance(
                    icp,
                    &protocols.root.candid_path,
                    parent.root,
                    child.pid,
                )?,
            )?;
            if protocol
                .binding
                .capabilities
                .contains(&RoleCapabilityKey::ChildProvisioning)
            {
                parents.push_back(DescendantParent {
                    canister_id: child.pid,
                    candid_path: protocol.candid_path.clone(),
                    maximum_children: descendant_bound,
                    release_set: parent.release_set,
                    role: child.role.clone(),
                    root: parent.root,
                });
            }
            entries.push(entry);
            if entries.len() > maximum_entries {
                return Err(inventory_error(format!(
                    "current Fleet exceeds authority-derived inventory bound {maximum_entries}"
                )));
            }
        }
    }
    validate_complete_workload_coverage(&component_workloads, &pool_workloads)?;
    entries.sort_by(|left, right| {
        left.parent_pid
            .cmp(&right.parent_pid)
            .then(left.role.cmp(&right.role))
            .then(left.pid.cmp(&right.pid))
    });
    Ok((entries, controlled_cycles_by_principal))
}

fn query_root_component_status(
    icp: &IcpCli,
    candid_path: &Path,
    root: Principal,
    operation_id: [u8; 32],
) -> Result<RootComponentProvisioningStatusResponse, CurrentProtocolError> {
    let response: RootInventoryStatusResponse = query_with_candid(
        icp,
        candid_path,
        root,
        protocol::CANIC_STATUS,
        &RootInventoryStatusRequest::Operation(OperationStatusRequest { operation_id }),
    )?;
    let RootInventoryStatusResponse::Operation(operation) = response else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
    let RootOperationStatusResponse::ProvisionComponents(status) = *operation else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
    Ok(status)
}

#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "terminal projection keeps exact Registry, batch, result, and output owners explicit"
)]
fn append_root_components(
    icp: &IcpCli,
    authority: &TerminalRootComponentAuthority<'_>,
    status: &RootComponentProvisioningStatusResponse,
    protocols: &ProtocolCatalog,
    seen: &mut BTreeSet<Principal>,
    entries: &mut Vec<RegistryEntry>,
    parents: &mut VecDeque<DescendantParent>,
    controlled_cycles_by_principal: &mut BTreeMap<String, u128>,
) -> Result<RootComponentSummary, CurrentProtocolError> {
    let result = status
        .result
        .as_ref()
        .ok_or_else(|| terminal_field_missing("result"))?;
    let expected_count = validate_terminal_root_component_status(authority, status, result)?;
    let mut component_ids = BTreeSet::new();
    let mut component_bindings = BTreeMap::new();
    let mut workloads = BTreeMap::new();
    let descendant_bound = maximum_descendant_page_from_result(result);
    let maximum_workloads = maximum_workloads_from_result(result)?;
    let mut placements = BTreeSet::new();
    let mut component_counts_by_spec = BTreeMap::new();
    for placement in &result.placements {
        if !placements.insert(placement.group_placement.clone()) {
            return Err(inventory_error(
                "Root returned a duplicate Component placement",
            ));
        }
        let mut member_paths = BTreeSet::new();
        for member in &placement.members {
            if !member_paths.insert(member.member_path.clone()) {
                return Err(inventory_error(
                    "Root returned a duplicate Component member path",
                ));
            }
            let spec = authority
                .registry
                .component_specs
                .iter()
                .find(|spec| spec.component_spec == member.component_spec)
                .ok_or_else(|| inventory_error("Component member names an unknown Spec"))?;
            let binding = &member.binding;
            let admission = authority
                .root
                .component_admissions
                .iter()
                .find(|admission| admission.component_spec == member.component_spec)
                .ok_or_else(|| inventory_error("Component member names an unadmitted Spec"))?;
            let admitted = admission.spec_hash == spec.spec_hash;
            let spec_count = component_counts_by_spec
                .entry(member.component_spec.clone())
                .or_insert(0_u32);
            *spec_count = spec_count
                .checked_add(1)
                .ok_or_else(|| inventory_error("Component Spec inventory count overflowed"))?;
            if *spec_count > admission.maximum_root_instances {
                return Err(terminal_field_error(
                    "component_spec_instance_bound",
                    format!("at most {}", admission.maximum_root_instances),
                    spec_count.to_string(),
                ));
            }
            let member_matches_binding = binding.component_spec == member.component_spec;
            let authority_matches = binding.authority == authority.registry.authority;
            let spec_hash_matches = binding.spec_hash == spec.spec_hash;
            let role_matches = binding.role == spec.component_role;
            let binding_matches_registry =
                admitted && authority_matches && spec_hash_matches && role_matches;
            let binding_matches_root = binding.placement_subnet == authority.root.placement_subnet
                && binding.fleet_subnet_root == authority.root.fleet_subnet_root;
            let identity_is_new =
                component_ids.insert(binding.canister_id) && seen.insert(binding.canister_id);
            let component_identity_is_new = component_bindings
                .insert(binding.component, binding.clone())
                .is_none();
            if !member_matches_binding
                || !binding_matches_registry
                || !binding_matches_root
                || !identity_is_new
                || !component_identity_is_new
            {
                return Err(inventory_error(
                    "Root Component result conflicts with current Registry authority",
                ));
            }
            insert_terminal_workload(
                &mut workloads,
                binding.canister_id,
                TerminalWorkloadAuthority {
                    component: binding.component,
                    operation_id: member.member_operation_id,
                    root: authority.root.fleet_subnet_root,
                },
                "Root Component result duplicates one allocation authority",
            )?;
            let protocol = protocols.child(&binding.role).ok_or_else(|| {
                inventory_error(format!(
                    "Component role {} has no current protocol authority",
                    binding.role
                ))
            })?;
            let partition_authority = ComponentPartitionAuthority {
                active_release_set: authority.active_release_set,
                group_placement: &placement.group_placement,
                operation_id: status.operation_id,
                plan_hash: status.plan_hash,
            };
            validate_component_partition(
                icp,
                &protocols.root.candid_path,
                authority.root.fleet_subnet_root,
                &partition_authority,
                member,
                protocol,
            )?;
            entries.push(RegistryEntry {
                pid: binding.canister_id.to_text(),
                role: Some(binding.role.to_string()),
                parent_pid: Some(authority.root.fleet_subnet_root.to_text()),
                module_hash: Some(protocol.module_hash.clone()),
                protocol_binding: Some(protocol.binding.clone()),
            });
            insert_controlled_cycles(
                controlled_cycles_by_principal,
                binding.canister_id,
                inspect_root_controlled_cycle_balance(
                    icp,
                    &protocols.root.candid_path,
                    authority.root.fleet_subnet_root,
                    binding.canister_id,
                )?,
            )?;
            if protocol
                .binding
                .capabilities
                .contains(&RoleCapabilityKey::ChildProvisioning)
            {
                parents.push_back(DescendantParent {
                    canister_id: binding.canister_id,
                    candid_path: protocol.candid_path.clone(),
                    maximum_children: descendant_bound,
                    release_set: *authority.active_release_set,
                    role: binding.role.clone(),
                    root: authority.root.fleet_subnet_root,
                });
            }
        }
    }
    if component_ids.len() != usize::try_from(expected_count).unwrap_or(usize::MAX) {
        return Err(inventory_error(
            "Root Component result contains duplicate member identities",
        ));
    }
    Ok(RootComponentSummary {
        component_bindings,
        component_ids,
        maximum_workloads,
        workloads,
    })
}

fn validate_terminal_root_component_status(
    authority: &TerminalRootComponentAuthority<'_>,
    status: &RootComponentProvisioningStatusResponse,
    result: &RootComponentProvisioningResult,
) -> Result<u32, CurrentProtocolError> {
    let (placement_count, component_count) = terminal_component_counts(result)?;
    terminal_field_exact(
        "operation_id",
        &authority.operation_id,
        &status.operation_id,
    )?;
    terminal_field_exact("plan_hash", &authority.plan_hash, &status.plan_hash)?;
    terminal_field_exact(
        "configuration_digest",
        &authority.configuration_digest,
        &status.configuration_digest,
    )?;
    terminal_nonzero_hash("receipt_content_hash", status.receipt_content_hash)?;
    terminal_field_exact(
        "fleet_registry",
        authority.source_fleet_registry,
        &status.fleet_registry,
    )?;
    terminal_field_exact(
        "fleet_subnet_root",
        &authority.root.fleet_subnet_root,
        &status.fleet_subnet_root,
    )?;
    terminal_field_exact(
        "phase",
        &RootComponentProvisioningPhase::RuntimesActive,
        &status.phase,
    )?;
    terminal_field_exact("placement_count", &placement_count, &status.placement_count)?;
    terminal_field_exact("component_count", &component_count, &status.component_count)?;
    for (field, observed) in [
        ("reserved_component_count", status.reserved_component_count),
        ("claimed_component_count", status.claimed_component_count),
        (
            "installed_component_count",
            status.installed_component_count,
        ),
        (
            "registry_committed_component_count",
            status.registry_committed_component_count,
        ),
        (
            "published_component_count",
            status.published_component_count,
        ),
        (
            "activated_component_count",
            status.activated_component_count,
        ),
    ] {
        terminal_field_exact(field, &component_count, &observed)?;
    }
    terminal_field_exact("root_runtime_active", &true, &status.root_runtime_active)?;
    if placement_count > authority.root.limits.maximum_group_placements {
        return Err(terminal_field_error(
            "placement_count_bound",
            format!("at most {}", authority.root.limits.maximum_group_placements),
            placement_count.to_string(),
        ));
    }
    if component_count > authority.root.limits.maximum_component_instances {
        return Err(terminal_field_error(
            "component_count_bound",
            format!(
                "at most {}",
                authority.root.limits.maximum_component_instances
            ),
            component_count.to_string(),
        ));
    }
    let publication = status
        .publication
        .as_ref()
        .ok_or_else(|| terminal_field_missing("publication"))?;
    terminal_field_exact(
        "publication.fleet_registry",
        authority.active_fleet_registry,
        &publication.fleet_registry,
    )?;
    let activation = status
        .activation
        .as_ref()
        .ok_or_else(|| terminal_field_missing("activation"))?;
    terminal_field_exact(
        "activation.component_count",
        &component_count,
        &activation.component_count,
    )?;
    terminal_nonzero_hash(
        "activation.initial_inventory_hash",
        activation.initial_inventory_hash,
    )?;
    terminal_nonzero_hash(
        "activation.fleet_activation_operation_id",
        activation.fleet_activation_operation_id,
    )?;
    Ok(component_count)
}

fn validate_terminal_coordinator_component_status(
    operation_id: [u8; 32],
    active_fleet_registry: &FleetRegistryVersion,
    status: &FleetComponentProvisioningStatusResponse,
) -> Result<(), CurrentProtocolError> {
    terminal_field_exact(
        "coordinator.operation_id",
        &operation_id,
        &status.operation_id,
    )?;
    terminal_nonzero_hash("coordinator.plan_hash", status.plan_hash)?;
    terminal_nonzero_hash(
        "coordinator.configuration_digest",
        *status.configuration_digest.as_bytes(),
    )?;
    terminal_field_exact(
        "coordinator.fleet_registry.authority",
        &active_fleet_registry.authority,
        &status.fleet_registry.authority,
    )?;
    terminal_field_exact(
        "coordinator.phase",
        &FleetComponentProvisioningPhase::RuntimesActivated,
        &status.phase,
    )?;
    if let Some(failure) = status.pending_root_failure {
        return Err(terminal_field_error(
            "coordinator.pending_root_failure",
            "none".to_string(),
            format!("{failure:?}"),
        ));
    }
    let published = status
        .published_fleet_registry
        .as_ref()
        .ok_or_else(|| terminal_field_missing("coordinator.published_fleet_registry"))?;
    terminal_field_exact(
        "coordinator.published_fleet_registry",
        active_fleet_registry,
        published,
    )
}

fn terminal_component_counts(
    result: &RootComponentProvisioningResult,
) -> Result<(u32, u32), CurrentProtocolError> {
    let placement_count = u32::try_from(result.placements.len())
        .map_err(|_| inventory_error("terminal Root placement count does not fit u32"))?;
    let component_count = result
        .placements
        .iter()
        .try_fold(0_u32, |total, placement| {
            total
                .checked_add(u32::try_from(placement.members.len()).map_err(|_| {
                    inventory_error("terminal Root Component count does not fit u32")
                })?)
                .ok_or_else(|| inventory_error("terminal Root Component count overflowed"))
        })?;
    Ok((placement_count, component_count))
}

fn terminal_field_exact<T>(
    field: &'static str,
    expected: &T,
    observed: &T,
) -> Result<(), CurrentProtocolError>
where
    T: std::fmt::Debug + PartialEq,
{
    if expected == observed {
        return Ok(());
    }
    Err(terminal_field_error(
        field,
        format!("{expected:?}"),
        format!("{observed:?}"),
    ))
}

fn terminal_nonzero_hash(
    field: &'static str,
    observed: [u8; 32],
) -> Result<(), CurrentProtocolError> {
    if observed != [0; 32] {
        return Ok(());
    }
    Err(terminal_field_error(
        field,
        "nonzero SHA-256".to_string(),
        "all zeroes".to_string(),
    ))
}

fn terminal_field_missing(field: &'static str) -> CurrentProtocolError {
    terminal_field_error(field, "present".to_string(), "missing".to_string())
}

const fn terminal_field_error(
    field: &'static str,
    expected: String,
    observed: String,
) -> CurrentProtocolError {
    CurrentProtocolError::TerminalInventoryField {
        field,
        expected,
        observed,
    }
}

fn validate_component_partition(
    icp: &IcpCli,
    candid_path: &Path,
    root: Principal,
    authority: &ComponentPartitionAuthority<'_>,
    member: &canic_core::dto::component_provisioning::RootProvisionedGroupMember,
    protocol_entry: &ProtocolEntry,
) -> Result<(), CurrentProtocolError> {
    let response: RootInventoryStatusResponse = query_with_candid(
        icp,
        candid_path,
        root,
        protocol::CANIC_STATUS,
        &RootInventoryStatusRequest::ComponentRegistryActivePartition(
            ComponentRegistryActivePartitionRequest {
                component: member.binding.component,
                provisioning_operation_id: authority.operation_id,
                plan_hash: authority.plan_hash,
                group_placement: (*authority.group_placement).clone(),
                member_path: member.member_path.clone(),
            },
        ),
    )?;
    let RootInventoryStatusResponse::ComponentRegistryActivePartition(partition) = response else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
    validate_component_partition_response(authority, member, protocol_entry, &partition)
}

fn validate_component_partition_response(
    authority: &ComponentPartitionAuthority<'_>,
    member: &canic_core::dto::component_provisioning::RootProvisionedGroupMember,
    protocol_entry: &ProtocolEntry,
    partition: &ComponentRegistryActivePartitionResponse,
) -> Result<(), CurrentProtocolError> {
    let expected_origin = ComponentProvisioningOrigin::ComponentGroup {
        operation_id: authority.operation_id,
        plan_hash: authority.plan_hash,
        group_placement: (*authority.group_placement).clone(),
        member_path: member.member_path.clone(),
    };
    terminal_field_exact(
        "component.allocation_operation_id",
        &member.member_operation_id,
        &partition.allocation_operation_id,
    )?;
    validate_component_partition_authority(
        ComponentPartitionFieldNames::PREPARED,
        &partition.prepared,
        ComponentLifecycleStatus::Prepared,
        member,
        protocol_entry,
        &expected_origin,
        authority.active_release_set,
    )?;
    terminal_field_exact(
        "component.prepared.revision",
        &member.component_registry_revision,
        &partition.prepared.head.revision,
    )?;
    terminal_field_exact(
        "component.prepared.content_hash",
        &member.component_registry_content_hash,
        &partition.prepared.head.content_hash,
    )?;
    validate_component_partition_authority(
        ComponentPartitionFieldNames::ACTIVATION,
        &partition.activation,
        ComponentLifecycleStatus::Active,
        member,
        protocol_entry,
        &expected_origin,
        authority.active_release_set,
    )?;
    if partition.activation.head.revision <= partition.prepared.head.revision {
        return Err(terminal_field_error(
            "component.activation.revision",
            format!("greater than {}", partition.prepared.head.revision),
            partition.activation.head.revision.to_string(),
        ));
    }
    terminal_nonzero_hash(
        "component.activation.content_hash",
        partition.activation.head.content_hash,
    )?;
    validate_component_partition_authority(
        ComponentPartitionFieldNames::CURRENT,
        &partition.current,
        ComponentLifecycleStatus::Active,
        member,
        protocol_entry,
        &expected_origin,
        authority.active_release_set,
    )?;
    if partition.current.head.revision < partition.activation.head.revision {
        return Err(terminal_field_error(
            "component.current.revision",
            format!("at least {}", partition.activation.head.revision),
            partition.current.head.revision.to_string(),
        ));
    }
    if partition.current.head.revision == partition.activation.head.revision {
        terminal_field_exact(
            "component.current.content_hash",
            &partition.activation.head.content_hash,
            &partition.current.head.content_hash,
        )?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct ComponentPartitionFieldNames {
    component: &'static str,
    binding: &'static str,
    protocol_profile_digest: &'static str,
    provisioning_origin: &'static str,
    release_set: &'static str,
    status: &'static str,
}

impl ComponentPartitionFieldNames {
    const PREPARED: Self = Self {
        component: "component.prepared.component",
        binding: "component.prepared.binding",
        protocol_profile_digest: "component.prepared.protocol_profile_digest",
        provisioning_origin: "component.prepared.provisioning_origin",
        release_set: "component.prepared.release_set",
        status: "component.prepared.status",
    };
    const ACTIVATION: Self = Self {
        component: "component.activation.component",
        binding: "component.activation.binding",
        protocol_profile_digest: "component.activation.protocol_profile_digest",
        provisioning_origin: "component.activation.provisioning_origin",
        release_set: "component.activation.release_set",
        status: "component.activation.status",
    };
    const CURRENT: Self = Self {
        component: "component.current.component",
        binding: "component.current.binding",
        protocol_profile_digest: "component.current.protocol_profile_digest",
        provisioning_origin: "component.current.provisioning_origin",
        release_set: "component.current.release_set",
        status: "component.current.status",
    };
}

fn validate_component_partition_authority(
    fields: ComponentPartitionFieldNames,
    partition: &ComponentRegistryPartitionResponse,
    expected_status: ComponentLifecycleStatus,
    member: &canic_core::dto::component_provisioning::RootProvisionedGroupMember,
    protocol_entry: &ProtocolEntry,
    expected_origin: &ComponentProvisioningOrigin,
    active_release_set: &FleetSubnetRootReleaseSet,
) -> Result<(), CurrentProtocolError> {
    terminal_field_exact(
        fields.component,
        &member.binding.component,
        &partition.head.component,
    )?;
    terminal_field_exact(fields.binding, &member.binding, &partition.binding)?;
    terminal_field_exact(
        fields.protocol_profile_digest,
        &protocol_entry.binding.protocol_profile_digest,
        &partition.protocol_profile_digest,
    )?;
    terminal_field_exact(
        fields.provisioning_origin,
        expected_origin,
        &partition.provisioning_origin,
    )?;
    terminal_field_exact(
        fields.release_set,
        active_release_set,
        &partition.release_set,
    )?;
    terminal_field_exact(fields.status, &expected_status, &partition.status)
}

fn validate_terminal_descendant_allocation(
    icp: &IcpCli,
    root_candid_path: &Path,
    authority: &TerminalDescendantAuthority<'_>,
) -> Result<(), CurrentProtocolError> {
    let response: RootInventoryStatusResponse = query_with_candid(
        icp,
        root_candid_path,
        authority.root,
        protocol::CANIC_STATUS,
        &RootInventoryStatusRequest::Operation(OperationStatusRequest {
            operation_id: authority.workload.operation_id,
        }),
    )?;
    let RootInventoryStatusResponse::Operation(operation) = response else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
    let RootOperationStatusResponse::ProvisionChild(status) = *operation else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
    validate_terminal_descendant_allocation_response(authority, &status.allocation)
}

fn validate_terminal_descendant_allocation_response(
    authority: &TerminalDescendantAuthority<'_>,
    allocation: &RootComponentChildAllocationResponse,
) -> Result<(), CurrentProtocolError> {
    terminal_field_exact(
        "descendant.pool_root",
        &authority.root,
        &authority.workload.root,
    )?;
    terminal_field_exact(
        "descendant.operation_id",
        &authority.workload.operation_id,
        &allocation.operation_id,
    )?;
    terminal_field_exact(
        "descendant.component",
        &authority.workload.component,
        &allocation.component,
    )?;
    terminal_field_exact(
        "descendant.parent_canister_id",
        &authority.child.parent_pid,
        &Some(allocation.parent_canister_id),
    )?;
    terminal_field_exact(
        "descendant.parent_role",
        authority.parent_role,
        &allocation.parent_role,
    )?;
    terminal_field_exact(
        "descendant.child_role",
        &authority.child.role,
        &allocation.child_role,
    )?;
    terminal_field_exact(
        "descendant.release_set",
        authority.release_set,
        &allocation.release_set,
    )?;
    terminal_field_exact(
        "descendant.phase",
        &RootComponentAllocationPhase::Committed,
        &allocation.phase,
    )?;
    terminal_field_exact(
        "descendant.root",
        &authority.root,
        &authority.component.fleet_subnet_root,
    )?;
    let creation = allocation
        .creation
        .as_ref()
        .ok_or_else(|| terminal_field_missing("descendant.creation"))?;
    terminal_field_exact(
        "descendant.creation.canister",
        &Some(authority.child.pid),
        &creation.canister,
    )?;
    let installation = allocation
        .installation
        .as_ref()
        .ok_or_else(|| terminal_field_missing("descendant.installation"))?;
    terminal_field_exact(
        "descendant.binding.component",
        authority.component,
        &installation.binding.component,
    )?;
    terminal_field_exact(
        "descendant.binding.parent_canister_id",
        &authority.child.parent_pid,
        &Some(installation.binding.parent_canister_id),
    )?;
    terminal_field_exact(
        "descendant.binding.role",
        &authority.child.role,
        &installation.binding.role,
    )?;
    terminal_field_exact(
        "descendant.binding.canister_id",
        &authority.child.pid,
        &installation.binding.canister_id,
    )?;
    terminal_field_exact(
        "descendant.raw_module_hash",
        &authority.protocol.module_hash,
        &hex_bytes(installation.raw_module_hash),
    )
}

#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "Root pool pagination keeps its exact bounded validation and ownership projection visible"
)]
fn append_pool_assets(
    icp: &IcpCli,
    candid_path: &Path,
    root: &FleetSubnetRootEntry,
    store: Principal,
    component_ids: &BTreeSet<Principal>,
    maximum_workloads: u64,
    seen: &mut BTreeSet<Principal>,
    entries: &mut Vec<RegistryEntry>,
    controlled_cycles_by_principal: &mut BTreeMap<String, u128>,
    pool_workloads: &mut BTreeMap<Principal, TerminalWorkloadAuthority>,
) -> Result<(), CurrentProtocolError> {
    let maximum_assets = maximum_workloads
        .checked_add(u64::from(root.limits.canister_pool.maximum_size))
        .and_then(|count| count.checked_add(1))
        .ok_or_else(|| inventory_error("Root pool inventory bound overflowed"))?;
    let mut start_after = None;
    let mut asset_count = 0_u64;
    let mut store_seen = false;
    loop {
        let response: RootInventoryStatusResponse = query_with_candid(
            icp,
            candid_path,
            root.fleet_subnet_root,
            protocol::CANIC_STATUS,
            &RootInventoryStatusRequest::Pool(CanisterPoolStatusRequest {
                start_after,
                limit: 256,
            }),
        )?;
        let RootInventoryStatusResponse::Pool(page) = response else {
            return Err(CurrentProtocolError::ResponseMismatch);
        };
        if page.config != root.limits.canister_pool {
            return Err(inventory_error(
                "Root pool policy differs from the Registry",
            ));
        }
        asset_count = asset_count
            .checked_add(u64::try_from(page.entries.len()).map_err(|_| {
                inventory_error("Root pool page length does not fit the inventory bound")
            })?)
            .ok_or_else(|| inventory_error("Root pool inventory count overflowed"))?;
        if asset_count > maximum_assets {
            return Err(inventory_error(
                "Root pool exceeds its authority-derived bound",
            ));
        }
        for asset in page.entries {
            match asset.status {
                CanisterPoolAssetStatus::Store if asset.canister_id == store => {
                    if store_seen {
                        return Err(inventory_error("Root pool duplicates its Store"));
                    }
                    store_seen = true;
                }
                CanisterPoolAssetStatus::Workload { claim } => {
                    if seen.contains(&asset.canister_id)
                        && !component_ids.contains(&asset.canister_id)
                    {
                        return Err(inventory_error(
                            "Root pool Workload conflicts with infrastructure or another Root",
                        ));
                    }
                    insert_terminal_workload(
                        pool_workloads,
                        asset.canister_id,
                        workload_authority(root.fleet_subnet_root, &claim),
                        "Root pool duplicates a Component workload",
                    )?;
                }
                CanisterPoolAssetStatus::Claimed { .. }
                    if component_ids.contains(&asset.canister_id) =>
                {
                    return Err(inventory_error(
                        "terminal Component remains only claimed in the Root pool",
                    ));
                }
                CanisterPoolAssetStatus::Store | CanisterPoolAssetStatus::Claimed { .. } => {
                    return Err(inventory_error(
                        "Root pool role ownership conflicts with terminal Component identities",
                    ));
                }
                _ => {
                    if !seen.insert(asset.canister_id) {
                        return Err(inventory_error("Root pool duplicates a Fleet canister"));
                    }
                    entries.push(RegistryEntry {
                        pid: asset.canister_id.to_text(),
                        role: Some("canister_pool_asset".to_string()),
                        parent_pid: Some(root.fleet_subnet_root.to_text()),
                        module_hash: None,
                        protocol_binding: None,
                    });
                    insert_controlled_cycles(
                        controlled_cycles_by_principal,
                        asset.canister_id,
                        asset.cycles.to_u128(),
                    )?;
                }
            }
        }
        let next = page.next_start_after;
        if next.is_none() {
            break;
        }
        if next == start_after {
            return Err(inventory_error("Root pool page cursor did not advance"));
        }
        start_after = next;
    }
    if !store_seen {
        return Err(inventory_error(
            "Root pool does not exactly retain its Store",
        ));
    }
    Ok(())
}

fn maximum_descendant_page_from_result(result: &RootComponentProvisioningResult) -> u64 {
    result
        .placements
        .iter()
        .flat_map(|placement| placement.members.iter())
        .map(|member| u64::from(member.limits.maximum_descendants))
        .max()
        .unwrap_or(0)
}

fn maximum_workloads_from_result(
    result: &RootComponentProvisioningResult,
) -> Result<u64, CurrentProtocolError> {
    result
        .placements
        .iter()
        .flat_map(|placement| placement.members.iter())
        .try_fold(0_u64, |total, member| {
            total
                .checked_add(u64::from(member.limits.maximum_descendants))
                .and_then(|value| value.checked_add(1))
                .ok_or_else(|| inventory_error("Root Component workload bound overflowed"))
        })
}

const fn workload_authority(
    root: Principal,
    claim: &CanisterPoolClaim,
) -> TerminalWorkloadAuthority {
    TerminalWorkloadAuthority {
        component: claim.component,
        operation_id: claim.operation_id,
        root,
    }
}

fn insert_terminal_workload(
    workloads: &mut BTreeMap<Principal, TerminalWorkloadAuthority>,
    canister_id: Principal,
    authority: TerminalWorkloadAuthority,
    duplicate_reason: &'static str,
) -> Result<(), CurrentProtocolError> {
    if workloads.insert(canister_id, authority).is_some() {
        return Err(inventory_error(duplicate_reason));
    }
    Ok(())
}

fn validate_complete_workload_coverage(
    components: &BTreeMap<Principal, TerminalWorkloadAuthority>,
    pool: &BTreeMap<Principal, TerminalWorkloadAuthority>,
) -> Result<(), CurrentProtocolError> {
    if components != pool {
        return Err(inventory_error(
            "Root pool Workload ownership conflicts with terminal Component identities",
        ));
    }
    Ok(())
}

fn inspect_root_controlled_cycle_balance(
    icp: &IcpCli,
    root_candid_path: &Path,
    root: Principal,
    canister_id: Principal,
) -> Result<u128, CurrentProtocolError> {
    let response: RootInventoryCommandResponse = call_with_candid(
        icp,
        root_candid_path,
        root,
        protocol::CANIC_ROOT_COMMAND,
        &RootInventoryCommand::InspectCanister(CanisterInspectionRequest { canister_id }),
    )?;
    let RootInventoryCommandResponse::InspectCanister(status) = response;
    u128::try_from(status.cycles.0)
        .map_err(|_| inventory_error("management cycle balance exceeds u128"))
}

fn insert_controlled_cycles(
    balances: &mut BTreeMap<String, u128>,
    principal: Principal,
    cycles: u128,
) -> Result<(), CurrentProtocolError> {
    if balances.insert(principal.to_text(), cycles).is_some() {
        return Err(inventory_error(
            "one controlled Principal has more than one cycle observation",
        ));
    }
    Ok(())
}

fn query_all_children(
    icp: &IcpCli,
    candid_path: &Path,
    parent: Principal,
    maximum_children: u64,
) -> Result<Vec<CanisterInfo>, CurrentProtocolError> {
    let mut entries = Vec::new();
    let mut offset = 0_u64;
    let mut expected_total = None;
    loop {
        let response: ChildrenStatusResponse = query_with_candid(
            icp,
            candid_path,
            parent,
            protocol::CANIC_STATUS,
            &ChildrenStatusRequest::Children(PageRequest {
                limit: CHILD_PAGE_LIMIT,
                offset,
            }),
        )?;
        let ChildrenStatusResponse::Children(page) = response;
        let page_len = u64::try_from(page.entries.len())
            .map_err(|_| inventory_error("child page length does not fit u64"))?;
        if page_len > CHILD_PAGE_LIMIT
            || expected_total
                .replace(page.total)
                .is_some_and(|total| total != page.total)
            || page.total > maximum_children
        {
            return Err(inventory_error(format!(
                "Canister {parent} returned an invalid bounded child page"
            )));
        }
        if page.entries.is_empty() {
            if offset != page.total {
                return Err(inventory_error(format!(
                    "Canister {parent} returned an incomplete child page"
                )));
            }
            break;
        }
        offset = offset
            .checked_add(page_len)
            .ok_or_else(|| inventory_error("child page offset overflowed"))?;
        if offset > page.total {
            return Err(inventory_error(format!(
                "Canister {parent} returned more children than declared"
            )));
        }
        entries.extend(page.entries);
        if offset == page.total {
            break;
        }
    }
    Ok(entries)
}

fn registry_entry(
    child: &CanisterInfo,
    protocol_entry: &ProtocolEntry,
) -> Result<RegistryEntry, CurrentProtocolError> {
    if child
        .module_hash
        .as_ref()
        .is_none_or(|hash| hash.len() != 32 || hex_bytes(hash) != protocol_entry.module_hash)
    {
        return Err(inventory_error(format!(
            "Canister {} has no exact observed current-release module hash",
            child.pid
        )));
    }
    Ok(RegistryEntry {
        pid: child.pid.to_text(),
        role: Some(child.role.to_string()),
        parent_pid: child.parent_pid.map(|parent| parent.to_text()),
        module_hash: Some(protocol_entry.module_hash.clone()),
        protocol_binding: Some(protocol_entry.binding.clone()),
    })
}

fn validate_root_authority(
    registry: &FleetRegistry,
    authorities: &[canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority],
) -> Result<(), CurrentProtocolError> {
    let registered = registry
        .fleet_subnet_roots
        .iter()
        .filter(|root| root.status != FleetSubnetRootStatus::Removed)
        .map(|root| root.fleet_subnet_root)
        .collect::<BTreeSet<_>>();
    let retained = authorities
        .iter()
        .map(|authority| authority.binding.fleet_subnet_root)
        .collect::<BTreeSet<_>>();
    let every_root_is_exact = registry
        .fleet_subnet_roots
        .iter()
        .filter(|root| root.status != FleetSubnetRootStatus::Removed)
        .all(|root| {
            authorities
                .iter()
                .find(|authority| authority.binding.fleet_subnet_root == root.fleet_subnet_root)
                .is_some_and(|authority| root_authority_matches_registry(registry, root, authority))
        });
    if registered != retained || retained.len() != authorities.len() || !every_root_is_exact {
        return Err(inventory_error(
            "Root authority differs from terminal Registry",
        ));
    }
    Ok(())
}

fn root_authority_matches_registry(
    registry: &FleetRegistry,
    root: &FleetSubnetRootEntry,
    authority: &canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority,
) -> bool {
    let binding = &authority.binding;
    let registry_binding_matches = binding.authority == registry.authority
        && binding.placement_subnet == root.placement_subnet
        && binding.fleet_subnet_root == root.fleet_subnet_root;
    let topology_matches = binding.component_admissions == root.component_admissions
        && binding.component_topology_digest == root.component_topology_digest
        && binding.limits == root.limits
        && binding.funding == root.funding;
    let release_matches = authority.initial_release_set == root.active_release_set;
    let store = &authority.wasm_store_authority;
    let store_matches_root = store.authority == registry.authority
        && store.placement_subnet == root.placement_subnet
        && store.fleet_subnet_root == root.fleet_subnet_root
        && store.release_build_id == root.active_release_set.release_build_id;
    registry_binding_matches && topology_matches && release_matches && store_matches_root
}

fn common_release_set(
    authorities: &[canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority],
) -> Result<FleetSubnetRootReleaseSet, CurrentProtocolError> {
    let [first, rest @ ..] = authorities else {
        return Err(inventory_error(
            "terminal Fleet has no Root release authority",
        ));
    };
    let first_store_matches_root =
        first.wasm_store_authority.release_build_id == first.initial_release_set.release_build_id;
    let every_root_matches = rest
        .iter()
        .all(|authority| authority.initial_release_set == first.initial_release_set);
    let every_store_matches = rest.iter().all(|authority| {
        authority.wasm_store_authority.release_build_id
            == authority.initial_release_set.release_build_id
    });
    if !first_store_matches_root || !every_root_matches || !every_store_matches {
        return Err(inventory_error(
            "terminal Roots do not share one current release authority",
        ));
    }
    Ok(first.initial_release_set)
}

fn retained_principal(
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    name: &str,
) -> Result<Principal, CurrentProtocolError> {
    state
        .pending_principals
        .get(name)
        .or_else(|| state.principals.get(name))
        .cloned()
        .or_else(|| {
            desired
                .canisters
                .iter()
                .find(|canister| canister.name == name)
                .and_then(|canister| canister.principal.clone())
        })
        .and_then(|principal| Principal::from_text(principal).ok())
        .ok_or_else(|| inventory_error(format!("{name} has no exact retained Principal")))
}

fn maximum_descendant_page(topology: &ComponentTopology) -> u64 {
    topology
        .component_specs
        .iter()
        .map(|spec| u64::from(spec.limits.maximum_descendants))
        .max()
        .unwrap_or(0)
}

fn maximum_inventory_entries(
    registry: &FleetRegistry,
    topology: &ComponentTopology,
) -> Result<usize, CurrentProtocolError> {
    registry
        .fleet_subnet_roots
        .iter()
        .filter(|root| root.status != FleetSubnetRootStatus::Removed)
        .try_fold(1_u64, |total, root| {
            let components =
                root.component_admissions
                    .iter()
                    .try_fold(0_u64, |subtotal, admission| {
                        let spec = topology.get(&admission.component_spec).ok_or_else(|| {
                            inventory_error(format!(
                                "Root {} admits unknown Component Spec {}",
                                root.fleet_subnet_root, admission.component_spec
                            ))
                        })?;
                        let per_component = u64::from(spec.limits.maximum_descendants)
                            .checked_add(1)
                            .ok_or_else(|| inventory_error("Component bound overflowed"))?;
                        subtotal
                            .checked_add(
                                u64::from(admission.maximum_root_instances)
                                    .checked_mul(per_component)
                                    .ok_or_else(|| inventory_error("Component bound overflowed"))?,
                            )
                            .ok_or_else(|| inventory_error("Component bound overflowed"))
                    })?;
            total
                .checked_add(components)
                .and_then(|value| {
                    value.checked_add(u64::from(root.limits.canister_pool.maximum_size))
                })
                .and_then(|value| value.checked_add(2))
                .ok_or_else(|| inventory_error("inventory bound overflowed"))
        })
        .and_then(|bound| {
            usize::try_from(bound)
                .map_err(|_| inventory_error("inventory bound does not fit this host"))
        })
}

fn verify_protocol(
    path: &Path,
    binding: &RegistryProtocolBinding,
) -> Result<(), CurrentProtocolError> {
    let bytes = match read_optional_regular_bytes(path) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            return Err(CurrentProtocolError::ReadCandid {
                path: path.to_path_buf(),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "current protocol sidecar is missing",
                ),
            });
        }
        Err(RegularFileReadError::NotRegular) => {
            return Err(inventory_error(format!(
                "current protocol sidecar is not a regular no-follow file: {}",
                path.display()
            )));
        }
        Err(RegularFileReadError::Io(source)) => {
            return Err(CurrentProtocolError::ReadCandid {
                path: path.to_path_buf(),
                source,
            });
        }
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            return Err(inventory_error(
                "regular no-follow protocol sidecar reads are unsupported",
            ));
        }
    };
    let observed = derive_protocol_profile_hashes(
        &binding.release_identity,
        &binding.role,
        &binding.capabilities,
        &bytes,
    );
    if observed.candid_sha256 != binding.candid_sha256
        || observed.protocol_profile_digest != binding.protocol_profile_digest
    {
        return Err(inventory_error(format!(
            "current protocol sidecar changed: {}",
            path.display()
        )));
    }
    Ok(())
}

fn application_candid_sidecar(
    root: &Path,
    wasm_relative_path: &str,
) -> Result<PathBuf, CurrentProtocolError> {
    validate_release_artifact_relative_path(wasm_relative_path)
        .map_err(|error| inventory_error(error.to_string()))?;
    let mut candid_relative_path = PathBuf::from(wasm_relative_path);
    candid_relative_path.set_extension("did");
    let candid_path = root.join(&candid_relative_path);
    let canonical_root = fs::canonicalize(root).map_err(|source| {
        inventory_error(format!(
            "cannot resolve current protocol root {}: {source}",
            root.display()
        ))
    })?;
    let parent = candid_path
        .parent()
        .ok_or_else(|| inventory_error("current protocol sidecar has no parent"))?;
    let canonical_parent = fs::canonicalize(parent).map_err(|source| {
        inventory_error(format!(
            "cannot resolve current protocol sidecar parent {}: {source}",
            parent.display()
        ))
    })?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err(inventory_error(format!(
            "current protocol sidecar escapes the canonical ICP root: {}",
            candid_path.display()
        )));
    }
    Ok(candid_path)
}

fn resolve_path(root: &Path, configured: &str) -> PathBuf {
    let configured = Path::new(configured);
    if configured.is_absolute() {
        configured.to_path_buf()
    } else {
        root.join(configured)
    }
}

fn inventory_error(reason: impl Into<String>) -> CurrentProtocolError {
    CurrentProtocolError::Configuration(format!("terminal inventory: {}", reason.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::{
        cdk::types::Cycles,
        dto::component_deployment::{ComponentDeploymentLimits, ComponentDeploymentPurpose},
        dto::component_provisioning::{
            FleetComponentProvisioningOperation, RootComponentActivationEvidence,
            RootComponentPublicationEvidence, RootProvisionedGroupMember,
        },
        dto::component_registry::ComponentRegistryHead,
        ids::{
            AppId, CanonicalNetworkId, ComponentBinding, ComponentGroupDeploymentId,
            ComponentGroupMemberId, ComponentGroupMemberPath, ComponentGroupPlacementId,
            ComponentInstanceId, ComponentSpecId, ComponentTopologyDigest, CyclesFundingBudget,
            FleetAdmissionPolicy, FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey,
            FleetRegistryAuthority, FleetSubnetCanisterPoolConfig, FleetSubnetRootLimits,
            ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
        },
        role_contract::ProtocolProfileDigest,
    };

    fn protocol_entry(module_hash: &str) -> ProtocolEntry {
        ProtocolEntry {
            binding: RegistryProtocolBinding {
                release_identity: "0.109.test".to_string(),
                role: CanisterRole::from("managed_component"),
                capabilities: BTreeSet::new(),
                candid_sha256: [1; 32],
                protocol_profile_digest: ProtocolProfileDigest::from_bytes([2; 32]),
            },
            candid_path: PathBuf::from("managed_component.did"),
            module_hash: module_hash.to_string(),
        }
    }

    #[test]
    fn child_projection_requires_an_exact_observed_current_module() {
        let parent = Principal::from_slice(&[7; 29]);
        let child = Principal::from_slice(&[8; 29]);
        let expected = "11".repeat(32);
        let protocol = protocol_entry(&expected);
        let mut info = CanisterInfo {
            pid: child,
            role: protocol.binding.role.clone(),
            parent_pid: Some(parent),
            module_hash: None,
            created_at: 1,
        };
        assert!(matches!(
            registry_entry(&info, &protocol),
            Err(CurrentProtocolError::Configuration(reason))
                if reason.contains("no exact observed current-release module hash")
        ));

        info.module_hash = Some(vec![0x22; 32]);
        assert!(matches!(
            registry_entry(&info, &protocol),
            Err(CurrentProtocolError::Configuration(reason))
                if reason.contains("no exact observed current-release module hash")
        ));

        info.module_hash = Some(vec![0x11; 32]);
        let projected = registry_entry(&info, &protocol).expect("project exact Directory row");
        assert_eq!(projected.module_hash.as_deref(), Some(expected.as_str()));
        assert_eq!(projected.protocol_binding.as_ref(), Some(&protocol.binding));
    }

    #[test]
    fn root_pool_workloads_cover_top_level_and_descendant_component_canisters() {
        let (partition_authority, member, protocol, _) = component_partition_fixture();
        let root = member.binding.fleet_subnet_root;
        let parent = member.binding.canister_id;
        let child = Principal::from_slice(&[31; 29]);
        let operation_id = [32; 32];
        let workload = TerminalWorkloadAuthority {
            component: member.binding.component,
            operation_id,
            root,
        };
        let child_info = CanisterInfo {
            pid: child,
            role: protocol.binding.role.clone(),
            parent_pid: Some(parent),
            module_hash: Some(vec![0x11; 32]),
            created_at: 1,
        };
        let allocation = RootComponentChildAllocationResponse {
            operation_id,
            component: member.binding.component,
            parent_canister_id: parent,
            parent_role: member.binding.role.clone(),
            child_role: child_info.role.clone(),
            child_kind:
                canic_core::control_plane_support::config::schema::ComponentChildKind::Shard,
            maximum_instances_per_parent: 64,
            maximum_descendants: 64,
            maximum_registry_bytes: 1_048_576,
            reserved_against_registry: ComponentRegistryHead {
                component: member.binding.component,
                revision: 2,
                content_hash: [33; 32],
            },
            release_set: *partition_authority.active_release_set,
            phase: RootComponentAllocationPhase::Committed,
            creation: Some(
                canic_core::dto::component_registry::RootComponentCreationEvidence {
                    wasm_store: Principal::from_slice(&[34; 29]),
                    payload_hash: [35; 32],
                    payload_size_bytes: 128,
                    initial_cycles: Cycles::new(5_000_000_000_000),
                    controller: root,
                    canister: Some(child),
                },
            ),
            installation: Some(
                canic_core::dto::component_registry::RootComponentChildInstallEvidence {
                    raw_module_hash: [0x11; 32],
                    chunk_hashes: vec![vec![36; 32]],
                    binding: canic_core::ids::ComponentChildBinding {
                        component: member.binding.clone(),
                        parent_canister_id: parent,
                        role: child_info.role.clone(),
                        canister_id: child,
                    },
                },
            ),
        };

        let authority = TerminalDescendantAuthority {
            child: &child_info,
            component: &member.binding,
            parent_role: &member.binding.role,
            protocol: &protocol,
            release_set: partition_authority.active_release_set,
            root,
            workload: &workload,
        };
        validate_terminal_descendant_allocation_response(&authority, &allocation)
            .expect("exact Root-owned descendant allocation is authoritative");

        let top_level = TerminalWorkloadAuthority {
            component: member.binding.component,
            operation_id: member.member_operation_id,
            root,
        };
        let components = BTreeMap::from([(parent, top_level.clone()), (child, workload.clone())]);
        let pool = components.clone();
        validate_complete_workload_coverage(&components, &pool)
            .expect("top-level and descendant Workloads exactly cover the Component tree");

        let foreign = TerminalWorkloadAuthority {
            component: ComponentInstanceId::from_generated_bytes([37; 32]),
            ..workload.clone()
        };
        for invalid_pool in [
            BTreeMap::from([(parent, top_level.clone()), (child, foreign)]),
            BTreeMap::from([(parent, workload.clone()), (child, top_level.clone())]),
            BTreeMap::from([(parent, top_level.clone())]),
            BTreeMap::from([
                (parent, top_level.clone()),
                (child, workload.clone()),
                (Principal::from_slice(&[38; 29]), workload.clone()),
            ]),
        ] {
            assert!(validate_complete_workload_coverage(&components, &invalid_pool).is_err());
        }

        let mut duplicate = BTreeMap::new();
        insert_terminal_workload(&mut duplicate, child, workload.clone(), "duplicate")
            .expect("first Workload");
        assert!(insert_terminal_workload(&mut duplicate, child, workload, "duplicate").is_err());

        let expected_maximum = 1 + u64::from(member.limits.maximum_descendants);
        let result = RootComponentProvisioningResult {
            placements: vec![
                canic_core::dto::component_provisioning::RootProvisionedGroupPlacement {
                    group_placement: ComponentGroupPlacementId {
                        deployment: "pool_workload_bound"
                            .parse::<ComponentGroupDeploymentId>()
                            .expect("deployment ID"),
                        ordinal: 0,
                    },
                    component_group: "pool_workload_group".parse().expect("Component Group ID"),
                    members: vec![member],
                },
            ],
        };
        assert_eq!(
            maximum_workloads_from_result(&result).expect("bounded workload capacity"),
            expected_maximum
        );
    }

    #[test]
    fn terminal_component_partition_accepts_descendant_advanced_current_head() {
        let (authority, member, protocol, partition) = component_partition_fixture();

        validate_component_partition_response(&authority, &member, &protocol, &partition)
            .expect("Root-qualified activation receipt covers later descendant progress");
        assert_eq!(partition.prepared.head.revision, 1);
        assert_eq!(partition.activation.head.revision, 2);
        assert_eq!(partition.current.head.revision, 4);
        assert_eq!(partition.current.committed_descendants, 1);
    }

    #[test]
    fn terminal_component_partition_rejects_wrong_receipt_and_regressed_current_head() {
        let (authority, member, protocol, mut partition) = component_partition_fixture();
        partition.allocation_operation_id = [99; 32];
        assert!(matches!(
            validate_component_partition_response(&authority, &member, &protocol, &partition),
            Err(CurrentProtocolError::TerminalInventoryField {
                field: "component.allocation_operation_id",
                ..
            })
        ));

        let (_, _, _, mut partition) = component_partition_fixture();
        partition.current.head.revision = 1;
        assert!(matches!(
            validate_component_partition_response(&authority, &member, &protocol, &partition),
            Err(CurrentProtocolError::TerminalInventoryField {
                field: "component.current.revision",
                ..
            })
        ));

        let (_, _, _, mut partition) = component_partition_fixture();
        partition.activation.provisioning_origin =
            ComponentProvisioningOrigin::FleetAdministrator {
                caller: Principal::anonymous(),
            };
        assert!(matches!(
            validate_component_partition_response(&authority, &member, &protocol, &partition),
            Err(CurrentProtocolError::TerminalInventoryField {
                field: "component.activation.provisioning_origin",
                ..
            })
        ));
    }

    #[test]
    fn terminal_component_partition_rejects_every_changed_authority_dimension() {
        let (authority, member, protocol, mut partition) = component_partition_fixture();
        partition.current.binding.canister_id = Principal::anonymous();
        assert_terminal_component_partition_field(
            &authority,
            &member,
            &protocol,
            &partition,
            "component.current.binding",
        );

        let (mut authority, member, protocol, partition) = component_partition_fixture();
        authority.operation_id = [91; 32];
        assert_terminal_component_partition_field(
            &authority,
            &member,
            &protocol,
            &partition,
            "component.prepared.provisioning_origin",
        );

        let (mut authority, member, protocol, partition) = component_partition_fixture();
        authority.plan_hash = [92; 32];
        assert_terminal_component_partition_field(
            &authority,
            &member,
            &protocol,
            &partition,
            "component.prepared.provisioning_origin",
        );

        let (authority, member, protocol, mut partition) = component_partition_fixture();
        partition.activation.release_set.manifest_digest = ReleaseSetDigest::from_bytes([93; 32]);
        assert_terminal_component_partition_field(
            &authority,
            &member,
            &protocol,
            &partition,
            "component.activation.release_set",
        );

        let (authority, member, protocol, mut partition) = component_partition_fixture();
        partition.activation.head.content_hash = [0; 32];
        assert_terminal_component_partition_field(
            &authority,
            &member,
            &protocol,
            &partition,
            "component.activation.content_hash",
        );

        let (authority, member, protocol, mut partition) = component_partition_fixture();
        partition.current.status = ComponentLifecycleStatus::Prepared;
        assert_terminal_component_partition_field(
            &authority,
            &member,
            &protocol,
            &partition,
            "component.current.status",
        );
    }

    fn assert_terminal_component_partition_field(
        authority: &ComponentPartitionAuthority<'_>,
        member: &RootProvisionedGroupMember,
        protocol: &ProtocolEntry,
        partition: &ComponentRegistryActivePartitionResponse,
        expected_field: &'static str,
    ) {
        assert!(matches!(
            validate_component_partition_response(authority, member, protocol, partition),
            Err(CurrentProtocolError::TerminalInventoryField { field, .. })
                if field == expected_field
        ));
    }

    fn component_partition_fixture() -> (
        ComponentPartitionAuthority<'static>,
        RootProvisionedGroupMember,
        ProtocolEntry,
        ComponentRegistryActivePartitionResponse,
    ) {
        let (terminal, _) = empty_terminal_root_status();
        let group_placement = Box::leak(Box::new(ComponentGroupPlacementId {
            deployment: "terminal_group"
                .parse::<ComponentGroupDeploymentId>()
                .expect("deployment ID"),
            ordinal: 0,
        }));
        let member_path = ComponentGroupMemberPath::try_from(vec![
            "hub".parse::<ComponentGroupMemberId>().expect("member ID"),
        ])
        .expect("member path");
        let component_spec = "hub".parse::<ComponentSpecId>().expect("Component Spec ID");
        let component = ComponentInstanceId::from_generated_bytes([21; 32]);
        let binding = ComponentBinding {
            authority: terminal.registry.authority.clone(),
            component,
            component_spec: component_spec.clone(),
            spec_hash: [22; 32],
            role: CanisterRole::from("managed_component"),
            placement_subnet: terminal.root.placement_subnet,
            fleet_subnet_root: terminal.root.fleet_subnet_root,
            canister_id: Principal::from_slice(&[23; 29]),
        };
        let member = RootProvisionedGroupMember {
            member_operation_id: [24; 32],
            member_path: member_path.clone(),
            component_spec,
            purpose: ComponentDeploymentPurpose::Ordinary,
            limits: ComponentDeploymentLimits {
                maximum_descendants: 1,
                maximum_registry_bytes: 1_024,
                spawn_grant_reductions: Vec::new(),
            },
            binding: binding.clone(),
            component_registry_revision: 1,
            component_registry_content_hash: [25; 32],
        };
        let protocol = protocol_entry(&"11".repeat(32));
        let origin = ComponentProvisioningOrigin::ComponentGroup {
            operation_id: terminal.operation_id,
            plan_hash: terminal.plan_hash,
            group_placement: group_placement.clone(),
            member_path,
        };
        let snapshot = |revision, content_hash, status, committed_descendants| {
            ComponentRegistryPartitionResponse {
                head: ComponentRegistryHead {
                    component,
                    revision,
                    content_hash,
                },
                binding: binding.clone(),
                protocol_profile_digest: protocol.binding.protocol_profile_digest,
                provisioning_origin: origin.clone(),
                release_set: *terminal.active_release_set,
                status,
                reserved_descendants: 0,
                committed_descendants,
                encoded_bytes: 128 + u64::from(committed_descendants),
            }
        };
        let partition = ComponentRegistryActivePartitionResponse {
            allocation_operation_id: member.member_operation_id,
            prepared: snapshot(1, [25; 32], ComponentLifecycleStatus::Prepared, 0),
            activation: snapshot(2, [26; 32], ComponentLifecycleStatus::Active, 0),
            current: snapshot(4, [27; 32], ComponentLifecycleStatus::Active, 1),
        };
        let authority = ComponentPartitionAuthority {
            active_release_set: terminal.active_release_set,
            group_placement,
            operation_id: terminal.operation_id,
            plan_hash: terminal.plan_hash,
        };
        (authority, member, protocol, partition)
    }

    #[test]
    fn terminal_root_inventory_accepts_source_registry_before_active_publication() {
        let (authority, status) = empty_terminal_root_status();
        validate_terminal_root_component_status(
            &authority,
            &status,
            status.result.as_ref().expect("terminal result"),
        )
        .expect("source Registry and active publication are distinct authorities");
        assert_eq!(authority.source_fleet_registry.revision, 3);
        assert_eq!(authority.active_fleet_registry.revision, 4);
    }

    #[test]
    fn terminal_root_inventory_rejects_wrong_source_registry() {
        let (authority, mut status) = empty_terminal_root_status();
        status.fleet_registry = authority.active_fleet_registry.clone();

        assert!(matches!(
            validate_terminal_root_component_status(
                &authority,
                &status,
                status.result.as_ref().expect("terminal result"),
            ),
            Err(CurrentProtocolError::TerminalInventoryField {
                field: "fleet_registry",
                ..
            })
        ));
    }

    #[test]
    fn terminal_root_inventory_rejects_wrong_publication_registry() {
        let (authority, mut status) = empty_terminal_root_status();
        status
            .publication
            .as_mut()
            .expect("publication evidence")
            .fleet_registry = authority.source_fleet_registry.clone();

        assert!(matches!(
            validate_terminal_root_component_status(
                &authority,
                &status,
                status.result.as_ref().expect("terminal result"),
            ),
            Err(CurrentProtocolError::TerminalInventoryField {
                field: "publication.fleet_registry",
                ..
            })
        ));
    }

    #[test]
    fn terminal_root_inventory_requires_coordinator_plan_authority() {
        let (authority, mut status) = empty_terminal_root_status();
        status.configuration_digest = ComponentDeploymentConfigurationDigest::from_bytes([99; 32]);

        assert!(matches!(
            validate_terminal_root_component_status(
                &authority,
                &status,
                status.result.as_ref().expect("terminal result"),
            ),
            Err(CurrentProtocolError::TerminalInventoryField {
                field: "configuration_digest",
                ..
            })
        ));

        status.plan_hash = [0; 32];
        assert!(matches!(
            validate_terminal_root_component_status(
                &authority,
                &status,
                status.result.as_ref().expect("terminal result"),
            ),
            Err(CurrentProtocolError::TerminalInventoryField {
                field: "plan_hash",
                ..
            })
        ));
    }

    #[test]
    fn terminal_coordinator_inventory_requires_active_published_registry() {
        let (authority, root_status) = empty_terminal_root_status();
        let mut status = empty_terminal_coordinator_status(&authority, &root_status);
        validate_terminal_coordinator_component_status(
            authority.operation_id,
            authority.active_fleet_registry,
            &status,
        )
        .expect("Coordinator publishes the active successor Registry");

        status.published_fleet_registry = Some(authority.source_fleet_registry.clone());
        assert!(matches!(
            validate_terminal_coordinator_component_status(
                authority.operation_id,
                authority.active_fleet_registry,
                &status,
            ),
            Err(CurrentProtocolError::TerminalInventoryField {
                field: "coordinator.published_fleet_registry",
                ..
            })
        ));
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the focused fixture spells out the complete independently retained terminal Root wire"
    )]
    fn empty_terminal_root_status() -> (
        TerminalRootComponentAuthority<'static>,
        RootComponentProvisioningStatusResponse,
    ) {
        let fleet = FleetBinding {
            fleet: FleetKey {
                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                fleet_id: FleetId::from_generated_bytes([1; 32]),
            },
            app: AppId::from("terminal_inventory_test"),
        };
        let coordinator = Principal::from_slice(&[2; 29]);
        let root_principal = Principal::from_slice(&[3; 29]);
        let registry_authority = FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: fleet.clone(),
                coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[4; 29])),
                coordinator,
            },
            epoch: 1,
        };
        let release_set = FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [5; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([6; 32]),
        };
        let root = FleetSubnetRootEntry {
            placement_subnet: SubnetId::from_principal(Principal::from_slice(&[7; 29])),
            fleet_subnet_root: root_principal,
            component_admissions: Vec::new(),
            component_topology_digest: ComponentTopologyDigest::from_bytes([8; 32]),
            active_release_set: release_set,
            limits: FleetSubnetRootLimits {
                maximum_component_instances: 1,
                maximum_registry_bytes: 1,
                maximum_wasm_store_bytes: 1,
                canister_pool: FleetSubnetCanisterPoolConfig {
                    minimum_size: 0,
                    maximum_size: 1,
                    canister_cycles: Cycles::new(1),
                },
                cycles_funding: CyclesFundingBudget {
                    window_secs: 1,
                    maximum_cycles: Cycles::new(1),
                },
                maximum_group_placements: 1,
            },
            funding: crate::test_support::fleet_subnet_root_funding_authority(),
            status: FleetSubnetRootStatus::Active,
        };
        let registry = FleetRegistry {
            authority: registry_authority.clone(),
            revision: 4,
            admission: FleetAdmissionPolicy {
                schema_version: 1,
                fleet,
                generation: 1,
                fleet_principals: Vec::new(),
                rules: Vec::new(),
                policy_digest: [9; 32],
            },
            component_specs: Vec::new(),
            fleet_subnet_roots: vec![root.clone()],
            services: Vec::new(),
        };
        let source_fleet_registry = FleetRegistryVersion {
            authority: registry_authority.clone(),
            revision: 3,
            content_hash: [10; 32],
        };
        let active_fleet_registry = FleetRegistryVersion {
            authority: registry_authority,
            revision: 4,
            content_hash: [11; 32],
        };
        let operation_id = [11; 32];
        let plan_hash = [12; 32];
        let configuration_digest = ComponentDeploymentConfigurationDigest::from_bytes([17; 32]);
        let authority = TerminalRootComponentAuthority {
            active_release_set: Box::leak(Box::new(release_set)),
            active_fleet_registry: Box::leak(Box::new(active_fleet_registry.clone())),
            configuration_digest,
            operation_id,
            plan_hash,
            registry: Box::leak(Box::new(registry)),
            root: Box::leak(Box::new(root)),
            source_fleet_registry: Box::leak(Box::new(source_fleet_registry.clone())),
        };
        let status = RootComponentProvisioningStatusResponse {
            operation_id,
            plan_hash,
            fleet_registry: source_fleet_registry,
            configuration_digest,
            fleet_subnet_root: root_principal,
            phase: RootComponentProvisioningPhase::RuntimesActive,
            placement_count: 0,
            component_count: 0,
            reserved_component_count: 0,
            claimed_component_count: 0,
            installed_component_count: 0,
            registry_committed_component_count: 0,
            published_component_count: 0,
            activated_component_count: 0,
            root_runtime_active: true,
            result: Some(RootComponentProvisioningResult {
                placements: Vec::new(),
            }),
            publication: Some(RootComponentPublicationEvidence {
                fleet_registry: active_fleet_registry,
                fleet_directory_content_hash: [13; 32],
                component_directories: Vec::new(),
                component_group_directories: Vec::new(),
            }),
            activation: Some(RootComponentActivationEvidence {
                fleet_activation_operation_id: [14; 32],
                initial_inventory_hash: [15; 32],
                component_count: 0,
                root_activated_at_ns: 1,
            }),
            accepted_at_ns: 1,
            provisioned_at_ns: Some(2),
            published_at_ns: Some(3),
            activation_started_at_ns: Some(4),
            runtimes_activated_at_ns: Some(5),
            receipt_content_hash: [16; 32],
        };
        (authority, status)
    }

    fn empty_terminal_coordinator_status(
        authority: &TerminalRootComponentAuthority<'_>,
        root_status: &RootComponentProvisioningStatusResponse,
    ) -> FleetComponentProvisioningStatusResponse {
        FleetComponentProvisioningStatusResponse {
            operation_id: authority.operation_id,
            plan_hash: authority.plan_hash,
            fleet_registry: authority.source_fleet_registry.clone(),
            configuration_digest: authority.configuration_digest,
            operation: FleetComponentProvisioningOperation::FreshInstall,
            phase: FleetComponentProvisioningPhase::RuntimesActivated,
            directory_confirmation_root_count: 1,
            root_batch_count: 1,
            accepted_root_count: 1,
            acceptance_in_flight_root: None,
            provisioned_root_count: 1,
            current_root: None,
            provisioning_in_flight_root: None,
            directory_confirmed_root_count: 1,
            current_synchronization: None,
            current_publication: None,
            publication_in_flight_root: None,
            runtime_activated_root_count: 1,
            current_activation: None,
            activation_in_flight_root: None,
            pending_root_failure: None,
            group_placement_count: root_status.placement_count,
            component_count: root_status.component_count,
            planned_at_ns: 1,
            roots_accepted_at_ns: Some(2),
            components_provisioned_at_ns: Some(3),
            published_fleet_registry: Some(authority.active_fleet_registry.clone()),
            service_topology_published_at_ns: Some(4),
            directories_confirmed_at_ns: Some(5),
            runtimes_activated_at_ns: Some(6),
        }
    }

    #[test]
    fn protocol_sidecar_binds_hash_and_complete_profile() {
        let root = crate::test_support::temp_dir("terminal-protocol-sidecar");
        fs::create_dir_all(&root).expect("create sidecar fixture");
        let path = root.join("managed_component.did");
        let candid = b"service : { ping : () -> () query; };";
        fs::write(&path, candid).expect("write sidecar");
        let role = CanisterRole::from("managed_component");
        let capabilities = BTreeSet::new();
        let hashes = derive_protocol_profile_hashes("0.109.test", &role, &capabilities, candid);
        let binding = RegistryProtocolBinding {
            release_identity: "0.109.test".to_string(),
            role,
            capabilities,
            candid_sha256: hashes.candid_sha256,
            protocol_profile_digest: hashes.protocol_profile_digest,
        };
        verify_protocol(&path, &binding).expect("verify exact current sidecar");
        fs::write(&path, b"service : {};").expect("change sidecar");
        assert!(matches!(
            verify_protocol(&path, &binding),
            Err(CurrentProtocolError::Configuration(reason))
                if reason.contains("protocol sidecar changed")
        ));
    }

    #[test]
    fn toko_fresh_fleet_application_candid_comes_from_immutable_artifact_path() {
        let root = crate::test_support::temp_dir("terminal-artifact-candid");
        let artifact_parent = root.join(".icp/release/artifacts/managed_component");
        fs::create_dir_all(&artifact_parent).expect("create immutable artifact directory");
        let candid = application_candid_sidecar(
            &root,
            ".icp/release/artifacts/managed_component/release-bound-module.wasm",
        )
        .expect("derive immutable Candid sidecar");

        assert_eq!(candid, artifact_parent.join("release-bound-module.did"),);
    }

    #[cfg(unix)]
    #[test]
    fn toko_fresh_fleet_protocol_candid_rejects_final_symlink() {
        use std::os::unix::fs::symlink;

        let root = crate::test_support::temp_dir("terminal-protocol-sidecar-link");
        fs::create_dir_all(&root).expect("create sidecar fixture");
        let target = root.join("target.did");
        let link = root.join("managed_component.did");
        let candid = b"service : { ping : () -> () query; };";
        fs::write(&target, candid).expect("write target sidecar");
        symlink(&target, &link).expect("link mutable sidecar");
        let role = CanisterRole::from("managed_component");
        let capabilities = BTreeSet::new();
        let hashes = derive_protocol_profile_hashes("0.109.test", &role, &capabilities, candid);
        let binding = RegistryProtocolBinding {
            release_identity: "0.109.test".to_string(),
            role,
            capabilities,
            candid_sha256: hashes.candid_sha256,
            protocol_profile_digest: hashes.protocol_profile_digest,
        };

        assert!(matches!(
            verify_protocol(&link, &binding),
            Err(CurrentProtocolError::Configuration(reason))
                if reason.contains("not a regular no-follow file")
        ));
    }
}
