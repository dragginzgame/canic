use crate::{
    CanisterProtocolError,
    fleet_catalog::{FleetCatalogEntryV1, FleetCatalogError, read_fleet_catalog_entry_from_root},
    fleet_install_plan::load_persisted_fleet_install_plan,
    icp::IcpCli,
    install_root::{
        discover_workspace_canic_config_choices, load_verified_installed_fleet_registry,
        select_discovered_app_config_path,
    },
    protocol_binding::{
        RegistryProtocolBinding, ResolvedProtocolBinding, resolve_infrastructure_protocol_binding,
        resolve_registry_protocol_binding,
    },
    registry::RegistryEntry,
    release_build::load_finalized_release_build,
    release_set::{
        AppConfigSnapshot, CanicInfrastructureArtifactEntry, CanicInfrastructureRole,
        load_persisted_application_artifact_union,
        load_persisted_canic_infrastructure_artifact_manifest,
    },
    role_contract::{PackageValidationMode, resolve_declared_role_contract},
};
use candid::{CandidType, Principal};
use canic_control_plane::dto::fleet_coordinator::{
    CoordinatorStatusRequest, CoordinatorStatusResponse,
};
use canic_core::{
    cdk::utils::hash::hex_bytes,
    control_plane_support::{config::ComponentTopology, ops::fleet_registry::FleetRegistryOps},
    dto::{
        canister::CanisterInfo,
        fleet_registry::{
            FleetRegistry, FleetRegistryManifest, FleetRegistryVersion, FleetSubnetRootStatus,
        },
        page::{Page, PageRequest},
    },
    ids::{
        CanisterRole, FleetBinding, FleetCoordinatorRootFundingPolicy, FleetKey,
        FleetSubnetRootFundingAuthority, SubnetId,
    },
    protocol,
    role_contract::{RoleCapabilityKey, RoleContractResolution},
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const CHILD_PAGE_LIMIT: u64 = 1_000;

///
/// InstalledFleetRequest
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledFleetRequest {
    pub fleet: String,
    pub environment: String,
}

///
/// InstalledFleetResolution
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledFleetResolution {
    pub fleet: FleetCatalogEntryV1,
    pub registry: InstalledFleetRegistry,
    pub topology: ResolvedFleetTopology,
}

/// Exact selected Root from authenticated installed Fleet authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledFleetRootResolution {
    pub fleet: FleetCatalogEntryV1,
    pub root_canister_id: Principal,
}

/// Exact Coordinator from authenticated installed Fleet authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledFleetCoordinatorResolution {
    pub fleet: FleetCatalogEntryV1,
    pub coordinator_canister_id: Principal,
}

/// Authenticated installed placement and funding authority for one Root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledFleetRootFundingResolution {
    pub fleet_subnet_root: Principal,
    pub placement_subnet: SubnetId,
    pub status: FleetSubnetRootStatus,
    pub funding: FleetSubnetRootFundingAuthority,
    pub placement_cost: crate::fleet_install_plan::PlannedSubnetPlacementCostEvidence,
}

/// Authenticated installed infrastructure funding authority retained for diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledFleetFundingResolution {
    pub fleet: FleetCatalogEntryV1,
    pub coordinator_canister_id: Principal,
    pub coordinator_root_funding: Option<FleetCoordinatorRootFundingPolicy>,
    pub coordinator_placement_cost: crate::fleet_install_plan::PlannedSubnetPlacementCostEvidence,
    pub roots: Vec<InstalledFleetRootFundingResolution>,
}

struct InstalledFleetAuthority {
    fleet: FleetCatalogEntryV1,
    registry: FleetRegistry,
    plan: crate::fleet_install_plan::FleetInstallPlan,
    config_path: PathBuf,
    config: AppConfigSnapshot,
}

///
/// InstalledFleetRegistry
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledFleetRegistry {
    pub entries: Vec<RegistryEntry>,
}

///
/// ResolvedFleetTopology
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedFleetTopology {
    pub coordinator_canister_id: String,
    pub fleet_subnet_root_canister_ids: Vec<String>,
    pub children_by_parent: BTreeMap<Option<String>, Vec<String>>,
    pub roles_by_canister: BTreeMap<String, String>,
}

///
/// InstalledFleetError
///

#[derive(Debug, ThisError)]
pub enum InstalledFleetError {
    #[error("Fleet {fleet} is not installed on environment {environment}")]
    NoInstalledFleet { environment: String, fleet: String },

    #[error("failed to read the canonical-network Fleet catalog: {0}")]
    FleetCatalog(#[from] FleetCatalogError),

    #[error("installed Fleet authority is invalid: {0}")]
    InstalledAuthority(String),

    #[error("installed Fleet live Registry is invalid: {0}")]
    LiveRegistry(String),

    #[error("installed Fleet child inventory is invalid: {0}")]
    ChildInventory(String),

    #[error(transparent)]
    Protocol(#[from] CanisterProtocolError),

    #[error("installed Fleet {fleet} has no current root {root}")]
    RootNotInFleet { fleet: String, root: Principal },

    #[error(
        "installed Fleet {fleet} has {root_count} current Fleet Subnet Roots; select one exact Root principal"
    )]
    AmbiguousFleetSubnetRoot { fleet: String, root_count: usize },
}

pub fn resolve_installed_fleet_from_root(
    request: &InstalledFleetRequest,
    icp_binary: &str,
    icp_root: &Path,
) -> Result<InstalledFleetResolution, InstalledFleetError> {
    let installed = load_installed_fleet_authority(request, icp_root)?;
    let icp = IcpCli::new(icp_binary, Some(request.environment.clone())).with_cwd(icp_root);
    let protocols = InstalledProtocolCatalog::load(&installed, icp_root)?;
    let registry = query_current_registry(&icp, &protocols.coordinator, &installed)?;
    let entries = query_installed_entries(
        &icp,
        icp_root,
        &request.environment,
        &installed.fleet,
        &registry,
        installed.config.component_topology(),
        &protocols,
    )?;
    let topology = ResolvedFleetTopology::from_registry(
        installed.registry.authority.binding.coordinator,
        &registry,
        &entries,
    );
    Ok(InstalledFleetResolution {
        fleet: installed.fleet,
        registry: InstalledFleetRegistry { entries },
        topology,
    })
}

pub fn read_installed_fleet_from_root(
    environment: &str,
    fleet: &str,
    icp_root: &Path,
) -> Result<FleetCatalogEntryV1, InstalledFleetError> {
    read_fleet_catalog_entry_from_root(icp_root, environment, fleet)
        .map_err(InstalledFleetError::FleetCatalog)?
        .ok_or_else(|| InstalledFleetError::NoInstalledFleet {
            environment: environment.to_string(),
            fleet: fleet.to_string(),
        })
}

/// Resolve one explicit non-Removed Root through the Coordinator-anchored install authority.
pub fn resolve_installed_fleet_root_from_root(
    request: &InstalledFleetRequest,
    selected_root: Principal,
    icp_root: &Path,
) -> Result<InstalledFleetRootResolution, InstalledFleetError> {
    let installed = load_installed_fleet_authority(request, icp_root)?;
    let selected = select_current_root(
        installed
            .registry
            .fleet_subnet_roots
            .iter()
            .map(|entry| (entry.fleet_subnet_root, entry.status)),
        selected_root,
    )
    .ok_or_else(|| InstalledFleetError::RootNotInFleet {
        fleet: request.fleet.clone(),
        root: selected_root,
    })?;

    Ok(InstalledFleetRootResolution {
        fleet: installed.fleet,
        root_canister_id: selected,
    })
}

/// Resolve the unique Coordinator through the same verified installed authority.
pub fn resolve_installed_fleet_coordinator_from_root(
    request: &InstalledFleetRequest,
    icp_root: &Path,
) -> Result<InstalledFleetCoordinatorResolution, InstalledFleetError> {
    let installed = load_installed_fleet_authority(request, icp_root)?;
    Ok(InstalledFleetCoordinatorResolution {
        coordinator_canister_id: installed.registry.authority.binding.coordinator,
        fleet: installed.fleet,
    })
}

/// Resolve funding profile, placement cost and current Roots from verified install authority.
pub fn resolve_installed_fleet_funding_from_root(
    request: &InstalledFleetRequest,
    icp_root: &Path,
) -> Result<InstalledFleetFundingResolution, InstalledFleetError> {
    let installed = load_installed_fleet_authority(request, icp_root)?;
    let mut roots = Vec::with_capacity(installed.registry.fleet_subnet_roots.len());
    for root in &installed.registry.fleet_subnet_roots {
        let mut placements = installed
            .plan
            .fleet_subnet_roots
            .iter()
            .filter(|planned| planned.placement_subnet == root.placement_subnet);
        let planned = placements.next().ok_or_else(|| {
            InstalledFleetError::InstalledAuthority(format!(
                "Registry Root {} has no exact planned placement {}",
                root.fleet_subnet_root, root.placement_subnet
            ))
        })?;
        if placements.next().is_some() || planned.funding != root.funding {
            return Err(InstalledFleetError::InstalledAuthority(format!(
                "Registry Root {} conflicts with planned placement funding authority",
                root.fleet_subnet_root
            )));
        }
        roots.push(InstalledFleetRootFundingResolution {
            fleet_subnet_root: root.fleet_subnet_root,
            placement_subnet: root.placement_subnet,
            status: root.status,
            funding: root.funding.clone(),
            placement_cost: planned.placement_cost.clone(),
        });
    }
    Ok(InstalledFleetFundingResolution {
        coordinator_canister_id: installed.registry.authority.binding.coordinator,
        coordinator_root_funding: installed.plan.coordinator.root_funding.clone(),
        coordinator_placement_cost: installed.plan.coordinator.placement_cost.clone(),
        roots,
        fleet: installed.fleet,
    })
}

fn load_installed_fleet_authority(
    request: &InstalledFleetRequest,
    icp_root: &Path,
) -> Result<InstalledFleetAuthority, InstalledFleetError> {
    let fleet = read_installed_fleet_from_root(&request.environment, &request.fleet, icp_root)?;
    let choices = discover_workspace_canic_config_choices(icp_root)
        .map_err(|error| InstalledFleetError::InstalledAuthority(error.to_string()))?;
    let config_path = select_discovered_app_config_path(&choices, fleet.app.as_str())
        .map_err(|error| InstalledFleetError::InstalledAuthority(error.to_string()))?
        .ok_or_else(|| {
            InstalledFleetError::InstalledAuthority(format!(
                "no discovered canic.toml declares catalog App {}",
                fleet.app
            ))
        })?;
    let config = AppConfigSnapshot::load(&config_path)
        .map_err(|error| InstalledFleetError::InstalledAuthority(error.to_string()))?;
    let binding = FleetBinding {
        fleet: FleetKey {
            canonical_network_id: fleet.canonical_network_id,
            fleet_id: fleet.fleet_id,
        },
        app: fleet.app.clone(),
    };
    let plan = load_persisted_fleet_install_plan(
        icp_root,
        config.model(),
        &binding,
        fleet.release_build_id,
    )
    .map_err(|error| InstalledFleetError::InstalledAuthority(error.to_string()))?;
    let registry = load_verified_installed_fleet_registry(&plan)
        .map_err(InstalledFleetError::InstalledAuthority)?;
    let catalog_coordinator = fleet
        .coordinator_principal
        .parse::<Principal>()
        .map_err(|error| InstalledFleetError::InstalledAuthority(error.to_string()))?;
    if registry.authority.binding.coordinator != catalog_coordinator {
        return Err(InstalledFleetError::InstalledAuthority(
            "catalog Coordinator differs from verified installed Registry".to_string(),
        ));
    }
    Ok(InstalledFleetAuthority {
        fleet,
        registry,
        plan: plan.plan,
        config_path,
        config,
    })
}

impl ResolvedFleetTopology {
    fn from_registry(
        coordinator: Principal,
        registry: &FleetRegistry,
        entries: &[RegistryEntry],
    ) -> Self {
        let mut children_by_parent = BTreeMap::<Option<String>, Vec<String>>::new();
        let mut roles_by_canister = BTreeMap::new();
        for entry in entries {
            children_by_parent
                .entry(entry.parent_pid.clone())
                .or_default()
                .push(entry.pid.clone());
            if let Some(role) = &entry.role {
                roles_by_canister.insert(entry.pid.clone(), role.clone());
            }
        }
        for children in children_by_parent.values_mut() {
            children.sort_unstable();
        }
        Self {
            coordinator_canister_id: coordinator.to_text(),
            fleet_subnet_root_canister_ids: registry
                .fleet_subnet_roots
                .iter()
                .filter(|root| root.status != FleetSubnetRootStatus::Removed)
                .map(|root| root.fleet_subnet_root.to_text())
                .collect(),
            children_by_parent,
            roles_by_canister,
        }
    }

    /// Return the only current Fleet Subnet Root when an operation still has singular scope.
    pub fn unique_fleet_subnet_root<'a>(
        &'a self,
        fleet: &str,
    ) -> Result<&'a str, InstalledFleetError> {
        match self.fleet_subnet_root_canister_ids.as_slice() {
            [root] => Ok(root),
            roots => Err(InstalledFleetError::AmbiguousFleetSubnetRoot {
                fleet: fleet.to_string(),
                root_count: roots.len(),
            }),
        }
    }
}

struct InstalledProtocolCatalog {
    coordinator: ResolvedProtocolBinding,
    root: RegistryProtocolBinding,
    by_role: BTreeMap<CanisterRole, RegistryProtocolBinding>,
}

impl InstalledProtocolCatalog {
    fn load(
        installed: &InstalledFleetAuthority,
        icp_root: &Path,
    ) -> Result<Self, InstalledFleetError> {
        let infrastructure = load_persisted_canic_infrastructure_artifact_manifest(
            icp_root,
            installed.plan.release_build_id,
        )
        .map_err(|error| InstalledFleetError::InstalledAuthority(error.to_string()))?;
        let infrastructure_entry = |role| {
            infrastructure
                .manifest
                .entries
                .iter()
                .find(|entry| entry.role == role)
                .ok_or_else(|| {
                    InstalledFleetError::InstalledAuthority(format!(
                        "finalized release is missing {} protocol authority",
                        role.as_str()
                    ))
                })
        };
        let coordinator_artifact = infrastructure_entry(CanicInfrastructureRole::FleetCoordinator)?;
        let coordinator = resolve_infrastructure_protocol_binding(
            icp_root,
            &installed.fleet.environment,
            coordinator_artifact,
        )
        .map_err(|error| InstalledFleetError::InstalledAuthority(error.to_string()))?;
        let root = infrastructure_protocol_binding(infrastructure_entry(
            CanicInfrastructureRole::FleetSubnetRoot,
        )?);
        let store = infrastructure_protocol_binding(infrastructure_entry(
            CanicInfrastructureRole::WasmStore,
        )?);

        let finalized = load_finalized_release_build(icp_root, installed.plan.release_build_id)
            .map_err(|error| InstalledFleetError::InstalledAuthority(error.to_string()))?;
        let application = load_persisted_application_artifact_union(
            icp_root,
            installed.config.component_topology(),
            installed.plan.release_build_id,
        )
        .map_err(|error| InstalledFleetError::InstalledAuthority(error.to_string()))?;
        let mut by_role = BTreeMap::from([(store.role.clone(), store)]);
        for artifact in application.union.entries {
            let contract = match resolve_declared_role_contract(
                &installed.config_path,
                installed.config.model(),
                &artifact.role,
                PackageValidationMode::Passive,
            ) {
                RoleContractResolution::Resolved { contract } => contract,
                RoleContractResolution::Rejected { errors } => {
                    return Err(InstalledFleetError::InstalledAuthority(format!(
                        "role {} protocol authority is unavailable: {errors:?}",
                        artifact.role
                    )));
                }
            };
            let binding = RegistryProtocolBinding {
                release_identity: finalized.record.builder_version.clone(),
                role: artifact.role.clone(),
                capabilities: contract.capabilities,
                candid_sha256: artifact.candid_sha256,
                protocol_profile_digest: artifact.protocol_profile_digest,
            };
            if by_role.insert(artifact.role.clone(), binding).is_some() {
                return Err(InstalledFleetError::InstalledAuthority(format!(
                    "role {} has more than one finalized protocol authority",
                    artifact.role
                )));
            }
        }
        Ok(Self {
            coordinator,
            root,
            by_role,
        })
    }

    fn binding_for_child(&self, role: &CanisterRole) -> Option<RegistryProtocolBinding> {
        self.by_role.get(role).cloned()
    }
}

fn infrastructure_protocol_binding(
    artifact: &CanicInfrastructureArtifactEntry,
) -> RegistryProtocolBinding {
    RegistryProtocolBinding {
        release_identity: artifact.protocol_release_identity.clone(),
        role: artifact.protocol_role.clone(),
        capabilities: artifact.protocol_capabilities.clone(),
        candid_sha256: artifact.candid_sha256,
        protocol_profile_digest: artifact.protocol_profile_digest,
    }
}

fn query_current_registry(
    icp: &IcpCli,
    binding: &ResolvedProtocolBinding,
    installed: &InstalledFleetAuthority,
) -> Result<FleetRegistry, InstalledFleetError> {
    let coordinator = installed.registry.authority.binding.coordinator;
    let CoordinatorStatusResponse::Registry(registry) =
        crate::query_canister_with_arg::<_, CoordinatorStatusResponse>(
            icp,
            binding,
            coordinator,
            protocol::CANIC_STATUS,
            &CoordinatorStatusRequest::Registry,
        )?
    else {
        return Err(live_registry_error(
            "Coordinator returned an unrelated Registry",
        ));
    };
    let CoordinatorStatusResponse::RegistryManifest(manifest) =
        crate::query_canister_with_arg::<_, CoordinatorStatusResponse>(
            icp,
            binding,
            coordinator,
            protocol::CANIC_STATUS,
            &CoordinatorStatusRequest::RegistryManifest,
        )?
    else {
        return Err(live_registry_error(
            "Coordinator returned an unrelated Registry manifest",
        ));
    };
    let CoordinatorStatusResponse::RegistryVersion(version) =
        crate::query_canister_with_arg::<_, CoordinatorStatusResponse>(
            icp,
            binding,
            coordinator,
            protocol::CANIC_STATUS,
            &CoordinatorStatusRequest::RegistryVersion,
        )?
    else {
        return Err(live_registry_error(
            "Coordinator returned an unrelated Registry version",
        ));
    };
    validate_live_registry(installed, &registry, &manifest, &version)?;
    Ok(registry)
}

fn validate_live_registry(
    installed: &InstalledFleetAuthority,
    registry: &FleetRegistry,
    manifest: &FleetRegistryManifest,
    version: &FleetRegistryVersion,
) -> Result<(), InstalledFleetError> {
    let authority = &installed.registry.authority;
    if &registry.authority != authority {
        return Err(live_registry_error(
            "current authority differs from the terminal installed Fleet",
        ));
    }
    FleetRegistryOps::validate(authority, installed.config.component_topology(), registry)
        .map_err(|error| live_registry_error(error.to_string()))?;
    let expected_manifest =
        FleetRegistryOps::manifest(authority, installed.config.component_topology(), registry)
            .map_err(|error| live_registry_error(error.to_string()))?;
    let expected_version =
        FleetRegistryOps::version(authority, installed.config.component_topology(), registry)
            .map_err(|error| live_registry_error(error.to_string()))?;
    if manifest != &expected_manifest || version != &expected_version {
        return Err(live_registry_error(
            "snapshot, manifest and version do not describe one exact Registry",
        ));
    }
    Ok(())
}

fn live_registry_error(reason: impl Into<String>) -> InstalledFleetError {
    InstalledFleetError::LiveRegistry(reason.into())
}

#[derive(CandidType)]
enum ChildrenStatusRequestFragment {
    Children(PageRequest),
}

#[derive(CandidType, Deserialize)]
enum ChildrenStatusResponseFragment {
    Children(Page<CanisterInfo>),
}

fn query_installed_entries(
    icp: &IcpCli,
    icp_root: &Path,
    environment: &str,
    fleet: &FleetCatalogEntryV1,
    registry: &FleetRegistry,
    topology: &ComponentTopology,
    protocols: &InstalledProtocolCatalog,
) -> Result<Vec<RegistryEntry>, InstalledFleetError> {
    let coordinator = registry.authority.binding.coordinator;
    let coordinator_text = coordinator.to_text();
    let mut entries = vec![RegistryEntry {
        pid: coordinator_text.clone(),
        role: Some(CanisterRole::FLEET_COORDINATOR.to_string()),
        parent_pid: None,
        module_hash: None,
        protocol_binding: Some(protocols.coordinator.binding().clone()),
    }];
    let mut seen = BTreeSet::from([coordinator]);
    let mut parents = VecDeque::new();
    for root in registry
        .fleet_subnet_roots
        .iter()
        .filter(|root| root.status != FleetSubnetRootStatus::Removed)
    {
        if !seen.insert(root.fleet_subnet_root) {
            return Err(child_inventory_error(format!(
                "Root {} is duplicated",
                root.fleet_subnet_root
            )));
        }
        let entry = RegistryEntry {
            pid: root.fleet_subnet_root.to_text(),
            role: Some(CanisterRole::ROOT.to_string()),
            parent_pid: Some(coordinator_text.clone()),
            module_hash: None,
            protocol_binding: Some(protocols.root.clone()),
        };
        let binding = resolve_registry_protocol_binding(icp_root, environment, &entry)
            .map_err(|error| InstalledFleetError::InstalledAuthority(error.to_string()))?;
        let direct_child_bound = u64::from(root.limits.maximum_component_instances)
            .checked_add(u64::from(root.limits.canister_pool.maximum_size))
            .and_then(|count| count.checked_add(1))
            .ok_or_else(|| child_inventory_error("Root child bound overflowed"))?;
        parents.push_back((root.fleet_subnet_root, binding, direct_child_bound));
        entries.push(entry);
    }

    let maximum_entries = maximum_inventory_entries(registry, topology)?;
    let descendant_page_bound = maximum_descendant_page(topology);
    while let Some((parent, binding, direct_child_bound)) = parents.pop_front() {
        for child in query_all_children(icp, &binding, parent, direct_child_bound)? {
            if child.parent_pid != Some(parent) {
                return Err(child_inventory_error(format!(
                    "Canister {} does not name queried parent {parent}",
                    child.pid
                )));
            }
            if !seen.insert(child.pid) {
                return Err(child_inventory_error(format!(
                    "Canister {} appears more than once",
                    child.pid
                )));
            }
            let protocol_binding = protocols.binding_for_child(&child.role);
            if child.module_hash.is_some() && protocol_binding.is_none() {
                return Err(child_inventory_error(format!(
                    "installed Canister {} role {} has no finalized protocol authority",
                    child.pid, child.role
                )));
            }
            let entry = registry_entry_from_child(child, protocol_binding)?;
            if entry
                .protocol_binding
                .as_ref()
                .is_some_and(|binding| binding.capabilities.contains(&RoleCapabilityKey::Sharding))
                && entry.module_hash.is_some()
            {
                let child_binding =
                    resolve_registry_protocol_binding(icp_root, environment, &entry).map_err(
                        |error| InstalledFleetError::InstalledAuthority(error.to_string()),
                    )?;
                let child_principal = Principal::from_text(&entry.pid).map_err(|error| {
                    child_inventory_error(format!(
                        "Canister {} has an invalid Principal: {error}",
                        entry.pid
                    ))
                })?;
                parents.push_back((child_principal, child_binding, descendant_page_bound));
            }
            entries.push(entry);
            if entries.len() > maximum_entries {
                return Err(child_inventory_error(format!(
                    "Fleet {} exceeds its authority-derived inventory bound {maximum_entries}",
                    fleet.fleet_name
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
    Ok(entries)
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
) -> Result<usize, InstalledFleetError> {
    registry
        .fleet_subnet_roots
        .iter()
        .filter(|root| root.status != FleetSubnetRootStatus::Removed)
        .try_fold(1_u64, |total, root| {
            let component_bound = root.component_admissions.iter().try_fold(
                0_u64,
                |component_total, admission| {
                    let spec = topology.get(&admission.component_spec).ok_or_else(|| {
                        child_inventory_error(format!(
                            "Root {} admits unknown Component Spec {}",
                            root.fleet_subnet_root, admission.component_spec
                        ))
                    })?;
                    let admission_bound = maximum_admission_canisters(
                        admission.maximum_root_instances,
                        spec.limits.maximum_descendants,
                    )?;
                    component_total
                        .checked_add(admission_bound)
                        .ok_or_else(|| child_inventory_error("Component bound overflowed"))
                },
            )?;
            let root_bound = component_bound
                .checked_add(u64::from(root.limits.canister_pool.maximum_size))
                .and_then(|count| count.checked_add(2))
                .ok_or_else(|| child_inventory_error("inventory bound overflowed"))?;
            total
                .checked_add(root_bound)
                .ok_or_else(|| child_inventory_error("inventory bound overflowed"))
        })
        .and_then(|bound| {
            usize::try_from(bound)
                .map_err(|_| child_inventory_error("inventory bound does not fit this host"))
        })
}

fn maximum_admission_canisters(
    maximum_root_instances: u32,
    maximum_descendants: u32,
) -> Result<u64, InstalledFleetError> {
    u64::from(maximum_descendants)
        .checked_add(1)
        .and_then(|per_component| u64::from(maximum_root_instances).checked_mul(per_component))
        .ok_or_else(|| child_inventory_error("Component bound overflowed"))
}

fn query_all_children(
    icp: &IcpCli,
    binding: &ResolvedProtocolBinding,
    parent: Principal,
    maximum_children: u64,
) -> Result<Vec<CanisterInfo>, InstalledFleetError> {
    let mut entries = Vec::new();
    let mut offset = 0_u64;
    let mut expected_total = None;
    loop {
        let response = crate::query_canister_with_arg::<_, ChildrenStatusResponseFragment>(
            icp,
            binding,
            parent,
            protocol::CANIC_STATUS,
            &ChildrenStatusRequestFragment::Children(PageRequest {
                limit: CHILD_PAGE_LIMIT,
                offset,
            }),
        )?;
        let ChildrenStatusResponseFragment::Children(page) = response;
        let page_len = u64::try_from(page.entries.len())
            .map_err(|_| child_inventory_error("child page length does not fit u64"))?;
        if page_len > CHILD_PAGE_LIMIT {
            return Err(child_inventory_error(format!(
                "Canister {parent} returned {} children above requested page limit {CHILD_PAGE_LIMIT}",
                page.entries.len()
            )));
        }
        if expected_total
            .replace(page.total)
            .is_some_and(|total| total != page.total)
        {
            return Err(child_inventory_error(format!(
                "Canister {parent} child total changed during pagination"
            )));
        }
        if page.total > maximum_children {
            return Err(child_inventory_error(format!(
                "Canister {parent} reports child total {} above authority bound {maximum_children}",
                page.total
            )));
        }
        if page.entries.is_empty() {
            if offset != page.total {
                return Err(child_inventory_error(format!(
                    "Canister {parent} returned an incomplete child page at offset {offset}"
                )));
            }
            break;
        }
        offset = offset
            .checked_add(page_len)
            .ok_or_else(|| child_inventory_error("child page offset overflowed"))?;
        if offset > page.total {
            return Err(child_inventory_error(format!(
                "Canister {parent} returned more children than its declared total"
            )));
        }
        entries.extend(page.entries);
        if offset == page.total {
            break;
        }
    }
    Ok(entries)
}

fn registry_entry_from_child(
    child: CanisterInfo,
    protocol_binding: Option<RegistryProtocolBinding>,
) -> Result<RegistryEntry, InstalledFleetError> {
    let module_hash = child
        .module_hash
        .map(|hash| {
            if hash.len() != 32 {
                return Err(child_inventory_error(format!(
                    "Canister {} module hash is not 32 bytes",
                    child.pid
                )));
            }
            Ok(hex_bytes(hash))
        })
        .transpose()?;
    Ok(RegistryEntry {
        pid: child.pid.to_text(),
        role: Some(child.role.to_string()),
        parent_pid: child.parent_pid.map(|parent| parent.to_text()),
        module_hash,
        protocol_binding,
    })
}

fn child_inventory_error(reason: impl Into<String>) -> InstalledFleetError {
    InstalledFleetError::ChildInventory(reason.into())
}

fn select_current_root(
    roots: impl IntoIterator<Item = (Principal, FleetSubnetRootStatus)>,
    selected_root: Principal,
) -> Option<Principal> {
    roots
        .into_iter()
        .find(|(root, status)| *root == selected_root && *status != FleetSubnetRootStatus::Removed)
        .map(|(root, _)| root)
}

#[cfg(test)]
mod tests;
