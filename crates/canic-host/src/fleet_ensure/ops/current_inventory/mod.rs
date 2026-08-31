//! Module: fleet_ensure::ops::current_inventory
//!
//! Responsibility: derive one complete terminal live inventory from current protocol authority.
//! Does not own: topology decisions, convergence sequencing, or historical installation state.
//! Boundary: exact terminal Registry, Root children, and current release artifacts are required.

use super::TerminalFleetInventory;
use super::current_protocol::{
    CompiledCurrentComponentProvisioning, CurrentProtocolError,
    compile_current_component_provisioning, query_current_root_authorities, query_registry,
    resolve_placements,
};
use crate::{
    canister_protocol::query_with_candid,
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
        canister::CanisterInfo,
        component_provisioning::{
            FleetSubnetRootProvisioningBatch, RootComponentProvisioningPhase,
            RootComponentProvisioningStatusResponse,
        },
        component_registry::{
            ComponentLifecycleStatus, ComponentProvisioningOrigin,
            ComponentRegistryPartitionRequest, ComponentRegistryPartitionResponse,
        },
        fleet_registry::{FleetRegistry, FleetSubnetRootEntry, FleetSubnetRootStatus},
        page::{Page, PageRequest},
        pool::{CanisterPoolAssetStatus, CanisterPoolResponse, CanisterPoolStatusRequest},
        role::{CycleBalanceStatusResponse, OperationStatusRequest},
    },
    ids::{CanisterRole, FleetSubnetRootReleaseSet},
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
enum CycleStatusRequest {
    CycleBalance,
}

#[derive(CandidType, Deserialize)]
enum CycleStatusResponse {
    CycleBalance(CycleBalanceStatusResponse),
}

#[derive(CandidType)]
enum RootInventoryStatusRequest {
    ComponentRegistryPartition(ComponentRegistryPartitionRequest),
    Operation(OperationStatusRequest),
    Pool(CanisterPoolStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum RootInventoryStatusResponse {
    ComponentRegistryPartition(Box<ComponentRegistryPartitionResponse>),
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
        desired,
        state,
        operation_id,
        &registry,
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
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "one terminal walk keeps the exact desired, Registry, release, and bounded output owners explicit"
)]
fn query_entries(
    icp: &IcpCli,
    desired: &DesiredFleet,
    state: &FleetEnsureStateRecord,
    operation_id: &str,
    registry: &FleetRegistry,
    config: &AppConfigSnapshot,
    authorities: &[canic_core::dto::fleet_subnet_root::FleetSubnetRootAuthority],
    protocols: &ProtocolCatalog,
) -> Result<(Vec<RegistryEntry>, BTreeMap<String, u128>), CurrentProtocolError> {
    let operation_id = canic_core::cdk::utils::hash::decode_hex(operation_id)
        .ok()
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .ok_or(CurrentProtocolError::InvalidOperationIdentity)?;
    let configuration = config
        .model()
        .compile_component_deployment_configuration()
        .map_err(|error| inventory_error(error.to_string()))?;
    let placements = resolve_placements(desired, state, registry)?;
    let compiled = compile_current_component_provisioning(
        &configuration,
        registry,
        operation_id,
        &placements,
    )?;
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
        let batch = compiled
            .request
            .plan
            .batches
            .iter()
            .find(|batch| batch.root.fleet_subnet_root == root.fleet_subnet_root)
            .ok_or_else(|| inventory_error("terminal Root has no compiled Component batch"))?;
        let status = query_root_component_status(
            icp,
            &protocols.root.candid_path,
            root.fleet_subnet_root,
            operation_id,
        )?;
        let component_ids = append_root_components(
            icp,
            registry,
            root,
            batch,
            &compiled,
            &status,
            protocols,
            &mut seen,
            &mut entries,
            &mut parents,
            &mut controlled_cycles_by_principal,
        )?;
        append_pool_assets(
            icp,
            &protocols.root.candid_path,
            root,
            store,
            &component_ids,
            &mut seen,
            &mut entries,
            &mut controlled_cycles_by_principal,
        )?;
    }
    let maximum_entries = maximum_inventory_entries(registry, config.component_topology())?;
    let descendant_bound = maximum_descendant_page(config.component_topology());
    while let Some((parent, candid_path, bound)) = parents.pop_front() {
        for child in query_all_children(icp, &candid_path, parent, bound)? {
            if child.parent_pid != Some(parent) || !seen.insert(child.pid) {
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
            let entry = registry_entry(&child, protocol)?;
            insert_controlled_cycles(
                &mut controlled_cycles_by_principal,
                child.pid,
                query_cycle_balance(icp, &protocol.candid_path, child.pid)?,
            )?;
            if protocol
                .binding
                .capabilities
                .contains(&RoleCapabilityKey::Sharding)
            {
                parents.push_back((child.pid, protocol.candid_path.clone(), descendant_bound));
            }
            entries.push(entry);
            if entries.len() > maximum_entries {
                return Err(inventory_error(format!(
                    "current Fleet exceeds authority-derived inventory bound {maximum_entries}"
                )));
            }
        }
    }
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
    registry: &FleetRegistry,
    root: &FleetSubnetRootEntry,
    batch: &FleetSubnetRootProvisioningBatch,
    compiled: &CompiledCurrentComponentProvisioning,
    status: &RootComponentProvisioningStatusResponse,
    protocols: &ProtocolCatalog,
    seen: &mut BTreeSet<Principal>,
    entries: &mut Vec<RegistryEntry>,
    parents: &mut VecDeque<(Principal, PathBuf, u64)>,
    controlled_cycles_by_principal: &mut BTreeMap<String, u128>,
) -> Result<BTreeSet<Principal>, CurrentProtocolError> {
    let expected_count = batch
        .placements
        .iter()
        .try_fold(0_u32, |total, placement| {
            total
                .checked_add(u32::try_from(placement.entries.len()).map_err(|_| {
                    inventory_error("compiled Component count does not fit the Root protocol")
                })?)
                .ok_or_else(|| inventory_error("compiled Component count overflowed"))
        })?;
    if status.operation_id != compiled.request.operation_id
        || status.plan_hash != compiled.plan_hash
        || status.fleet_registry != compiled.request.plan.fleet_registry
        || status.configuration_digest != compiled.request.plan.configuration_digest
        || status.fleet_subnet_root != root.fleet_subnet_root
        || status.phase != RootComponentProvisioningPhase::RuntimesActive
        || status.component_count != expected_count
        || status.reserved_component_count != expected_count
        || status.claimed_component_count != expected_count
        || status.installed_component_count != expected_count
        || status.registry_committed_component_count != expected_count
        || status.published_component_count != expected_count
        || status.activated_component_count != expected_count
        || !status.root_runtime_active
        || status.activation.is_none()
        || status.publication.is_none()
    {
        return Err(inventory_error(format!(
            "Root {} Component result is not terminal and exact",
            root.fleet_subnet_root
        )));
    }
    let result = status
        .result
        .as_ref()
        .ok_or_else(|| inventory_error("terminal Root Component result is missing"))?;
    if result.placements.len() != batch.placements.len() {
        return Err(inventory_error(
            "terminal Root placement result differs from the compiled batch",
        ));
    }
    let mut component_ids = BTreeSet::new();
    let descendant_bound = maximum_descendant_page_from_batch(batch);
    for placement in &result.placements {
        let expected_placement = batch
            .placements
            .iter()
            .find(|expected| {
                expected.group_placement == placement.group_placement
                    && expected.component_group == placement.component_group
            })
            .ok_or_else(|| inventory_error("Root returned an unknown Component placement"))?;
        if placement.members.len() != expected_placement.entries.len() {
            return Err(inventory_error(
                "Root returned an incomplete Component placement",
            ));
        }
        for member in &placement.members {
            let expected = expected_placement
                .entries
                .iter()
                .find(|entry| entry.member_path == member.member_path)
                .ok_or_else(|| inventory_error("Root returned an unknown Component member"))?;
            let spec = registry
                .component_specs
                .iter()
                .find(|spec| spec.component_spec == expected.component_spec)
                .ok_or_else(|| inventory_error("Component member names an unknown Spec"))?;
            let binding = &member.binding;
            let member_matches_plan = member.component_spec == expected.component_spec
                && member.purpose == expected.purpose;
            let authority_matches = binding.authority == registry.authority;
            let component_spec_matches = binding.component_spec == expected.component_spec;
            let spec_hash_matches = binding.spec_hash == expected.spec_hash;
            let role_matches = binding.role == spec.component_role;
            let binding_matches_registry =
                authority_matches && component_spec_matches && spec_hash_matches && role_matches;
            let binding_matches_root = binding.placement_subnet == root.placement_subnet
                && binding.fleet_subnet_root == root.fleet_subnet_root;
            let identity_is_new =
                component_ids.insert(binding.canister_id) && seen.insert(binding.canister_id);
            if !member_matches_plan
                || !binding_matches_registry
                || !binding_matches_root
                || !identity_is_new
            {
                return Err(inventory_error(
                    "Root Component result conflicts with current Registry authority",
                ));
            }
            let protocol = protocols.child(&binding.role).ok_or_else(|| {
                inventory_error(format!(
                    "Component role {} has no current protocol authority",
                    binding.role
                ))
            })?;
            let partition_authority = ComponentPartitionAuthority {
                active_release_set: &batch.active_release_set,
                group_placement: &expected_placement.group_placement,
                operation_id: compiled.request.operation_id,
                plan_hash: compiled.plan_hash,
            };
            validate_component_partition(
                icp,
                &protocols.root.candid_path,
                root.fleet_subnet_root,
                &partition_authority,
                member,
                protocol,
            )?;
            entries.push(RegistryEntry {
                pid: binding.canister_id.to_text(),
                role: Some(binding.role.to_string()),
                parent_pid: Some(root.fleet_subnet_root.to_text()),
                module_hash: Some(protocol.module_hash.clone()),
                protocol_binding: Some(protocol.binding.clone()),
            });
            insert_controlled_cycles(
                controlled_cycles_by_principal,
                binding.canister_id,
                query_cycle_balance(icp, &protocol.candid_path, binding.canister_id)?,
            )?;
            if protocol
                .binding
                .capabilities
                .contains(&RoleCapabilityKey::Sharding)
            {
                parents.push_back((
                    binding.canister_id,
                    protocol.candid_path.clone(),
                    descendant_bound,
                ));
            }
        }
    }
    if component_ids.len() != usize::try_from(expected_count).unwrap_or(usize::MAX) {
        return Err(inventory_error(
            "Root Component result contains duplicate member identities",
        ));
    }
    Ok(component_ids)
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
        &RootInventoryStatusRequest::ComponentRegistryPartition(
            ComponentRegistryPartitionRequest {
                component: member.binding.component,
            },
        ),
    )?;
    let RootInventoryStatusResponse::ComponentRegistryPartition(partition) = response else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
    let expected_origin = ComponentProvisioningOrigin::ComponentGroup {
        operation_id: authority.operation_id,
        plan_hash: authority.plan_hash,
        group_placement: (*authority.group_placement).clone(),
        member_path: member.member_path.clone(),
    };
    if partition.head.component != member.binding.component
        || !component_partition_head_is_exact_activation(
            partition.head.revision,
            partition.status,
            member.component_registry_revision,
        )
        || partition.binding != member.binding
        || partition.protocol_profile_digest != protocol_entry.binding.protocol_profile_digest
        || partition.provisioning_origin != expected_origin
        || partition.release_set != *authority.active_release_set
    {
        return Err(inventory_error(
            "Root Component partition conflicts with its terminal result",
        ));
    }
    Ok(())
}

fn component_partition_head_is_exact_activation(
    observed_revision: u64,
    observed_status: ComponentLifecycleStatus,
    provisioned_revision: u64,
) -> bool {
    observed_status == ComponentLifecycleStatus::Active
        && provisioned_revision
            .checked_add(1)
            .is_some_and(|revision| observed_revision == revision)
}

#[expect(
    clippy::too_many_arguments,
    reason = "Root pool reconciliation keeps its exact Root, Store, workload, and output owners explicit"
)]
fn append_pool_assets(
    icp: &IcpCli,
    candid_path: &Path,
    root: &FleetSubnetRootEntry,
    store: Principal,
    component_ids: &BTreeSet<Principal>,
    seen: &mut BTreeSet<Principal>,
    entries: &mut Vec<RegistryEntry>,
    controlled_cycles_by_principal: &mut BTreeMap<String, u128>,
) -> Result<(), CurrentProtocolError> {
    let maximum_assets = u64::from(root.limits.maximum_component_instances)
        .checked_add(u64::from(root.limits.canister_pool.maximum_size))
        .and_then(|count| count.checked_add(1))
        .ok_or_else(|| inventory_error("Root pool inventory bound overflowed"))?;
    let mut start_after = None;
    let mut asset_count = 0_u64;
    let mut store_seen = false;
    let mut workloads = BTreeSet::new();
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
                CanisterPoolAssetStatus::Workload { .. }
                    if component_ids.contains(&asset.canister_id) =>
                {
                    if !workloads.insert(asset.canister_id) {
                        return Err(inventory_error("Root pool duplicates a Component workload"));
                    }
                }
                CanisterPoolAssetStatus::Claimed { .. }
                    if component_ids.contains(&asset.canister_id) =>
                {
                    return Err(inventory_error(
                        "terminal Component remains only claimed in the Root pool",
                    ));
                }
                CanisterPoolAssetStatus::Store
                | CanisterPoolAssetStatus::Workload { .. }
                | CanisterPoolAssetStatus::Claimed { .. } => {
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
    if !store_seen || workloads != *component_ids {
        return Err(inventory_error(
            "Root pool does not exactly retain its Store and terminal Component workloads",
        ));
    }
    Ok(())
}

fn maximum_descendant_page_from_batch(batch: &FleetSubnetRootProvisioningBatch) -> u64 {
    batch
        .placements
        .iter()
        .flat_map(|placement| placement.entries.iter())
        .map(|entry| u64::from(entry.limits.maximum_descendants))
        .max()
        .unwrap_or(0)
}

fn query_cycle_balance(
    icp: &IcpCli,
    candid_path: &Path,
    canister: Principal,
) -> Result<u128, CurrentProtocolError> {
    let response: CycleStatusResponse = query_with_candid(
        icp,
        candid_path,
        canister,
        protocol::CANIC_STATUS,
        &CycleStatusRequest::CycleBalance,
    )?;
    let CycleStatusResponse::CycleBalance(balance) = response;
    Ok(balance.cycles)
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
        .is_some_and(|hash| hash.len() != 32 || hex_bytes(hash) != protocol_entry.module_hash)
    {
        return Err(inventory_error(format!(
            "Canister {} module hash conflicts with current release authority",
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
    if registered != retained || retained.len() != authorities.len() {
        return Err(inventory_error(
            "Root authority differs from terminal Registry",
        ));
    }
    Ok(())
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
    use canic_core::role_contract::ProtocolProfileDigest;

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
    fn child_projection_requires_the_exact_current_module_when_observed() {
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
        let projected = registry_entry(&info, &protocol).expect("project retained Directory row");
        assert_eq!(projected.module_hash.as_deref(), Some(expected.as_str()));
        assert_eq!(projected.protocol_binding.as_ref(), Some(&protocol.binding));

        info.module_hash = Some(vec![0x22; 32]);
        assert!(matches!(
            registry_entry(&info, &protocol),
            Err(CurrentProtocolError::Configuration(reason))
                if reason.contains("module hash conflicts")
        ));
    }

    #[test]
    fn toko_fresh_fleet_component_partition_accepts_only_exact_activation_successor() {
        assert!(component_partition_head_is_exact_activation(
            5,
            ComponentLifecycleStatus::Active,
            4,
        ));
        assert!(!component_partition_head_is_exact_activation(
            4,
            ComponentLifecycleStatus::Active,
            4,
        ));
        assert!(!component_partition_head_is_exact_activation(
            6,
            ComponentLifecycleStatus::Active,
            4,
        ));
        assert!(!component_partition_head_is_exact_activation(
            5,
            ComponentLifecycleStatus::Prepared,
            4,
        ));
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
