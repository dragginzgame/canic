//! Module: fleet_install_plan::persistence
//!
//! Responsibility: compile, validate, and immutably publish pre-effect Fleet install authority.
//! Does not own: passive plan data, Subnet selection, Canister creation, or Registry mutation.
//! Boundary: finalized build evidence plus explicit placement/funding input becomes durable
//! before any external effect.

use crate::{
    component_topology::{
        PlannedFleetSubnetRootTopology, PlannedFleetSubnetRootTopologyInput,
        plan_initial_fleet_topology,
    },
    durable_io::{
        RegularFileLockError, RegularFileReadError, create_new_bytes_with_parents,
        lock_regular_file_with_parents, read_optional_regular_bytes,
    },
    fleet_install_plan::model::{
        FleetInstallPlan, FleetInstallPlanError, FleetInstallPlanRequest,
        PersistedFleetInstallPlan, PersistedFleetSubnetRootReleaseSet,
        PlannedCanisterCreationFunding, PlannedFleetCoordinator, PlannedFleetSubnetRoot,
    },
    release_build::load_finalized_release_build,
    release_set::{FleetSubnetRootReleaseSetManifest, load_persisted_application_artifact_union},
};
use std::{
    collections::BTreeMap,
    io,
    path::{Path, PathBuf},
};

use candid::Principal;
use canic_core::{
    bootstrap::compiled::ConfigModel,
    control_plane_support::config::{
        ComponentDeploymentPurpose, ComponentGroupDeploymentSpec, FleetServiceTopology,
    },
    ids::{
        ComponentGroupDeploymentId, ComponentSpecId, FleetBinding, FleetServiceId,
        FleetSubnetRootReleaseSet, ReleaseBuildId, SubnetId,
    },
};
use sha2::{Digest, Sha256};

pub(super) const FLEET_INSTALL_PLAN_FILE: &str = "plan.json";
const FLEET_INSTALL_PLAN_LOCK_FILE: &str = "plan.lock";
const ROOT_RELEASE_SET_DIRECTORY: &str = "root-release-sets";
const MAX_FLEET_INSTALL_PLAN_BYTES: usize = 16 * 1_024 * 1_024;
const MAX_ROOT_RELEASE_SET_BYTES: usize = 16 * 1_024 * 1_024;

struct CompiledFleetInstallPlan {
    plan: FleetInstallPlan,
    topology_roots: Vec<PlannedFleetSubnetRootTopology>,
    root_release_sets: Vec<FleetSubnetRootReleaseSetManifest>,
}

/// Compile and immutably publish one complete pre-effect Fleet install plan.
pub fn compile_and_persist_fleet_install_plan(
    request: FleetInstallPlanRequest<'_>,
) -> Result<PersistedFleetInstallPlan, FleetInstallPlanError> {
    load_finalized_release_build(request.root, request.release_build_id)?;
    let topology = request.config.compile_component_topology()?;
    let union = load_persisted_application_artifact_union(
        request.root,
        &topology,
        request.release_build_id,
    )?;
    let compiled = compile_plan(&request, union.digest, &union.union)?;
    let path = fleet_install_plan_path(request.root, &request.fleet, request.release_build_id);
    let lock_path = path.with_file_name(FLEET_INSTALL_PLAN_LOCK_FILE);
    let _lock = lock_plan(&lock_path)?;

    if read_optional_bytes(&path, FileKind::Plan)?.is_some() {
        let persisted = load_persisted_fleet_install_plan(
            request.root,
            request.config,
            &request.fleet,
            request.release_build_id,
        )?;
        return if persisted.plan == compiled.plan {
            Ok(persisted)
        } else {
            Err(FleetInstallPlanError::ConflictingPlan { path })
        };
    }

    for (root, manifest) in compiled
        .topology_roots
        .iter()
        .zip(&compiled.root_release_sets)
    {
        let manifest_path = root_release_set_path(&path, root.placement_subnet);
        let bytes = manifest.canonical_bytes_planned(&topology, root, &union.union)?;
        check_size(&bytes, FileKind::RootReleaseSet)?;
        publish_exact(
            &manifest_path,
            &bytes,
            FileKind::RootReleaseSet,
            FleetInstallPlanError::ConflictingRootReleaseSet {
                path: manifest_path.clone(),
            },
        )?;
    }

    let plan_bytes = canonical_plan_bytes(&compiled.plan, &topology, request.config, union.digest)?;
    publish_exact(
        &path,
        &plan_bytes,
        FileKind::Plan,
        FleetInstallPlanError::ConflictingPlan { path: path.clone() },
    )?;
    load_persisted_fleet_install_plan(
        request.root,
        request.config,
        &request.fleet,
        request.release_build_id,
    )
}

/// Load one exact plan and all root manifests under their durable path identity.
pub fn load_persisted_fleet_install_plan(
    root: &Path,
    config: &ConfigModel,
    fleet: &FleetBinding,
    release_build_id: ReleaseBuildId,
) -> Result<PersistedFleetInstallPlan, FleetInstallPlanError> {
    load_finalized_release_build(root, release_build_id)?;
    let topology = config.compile_component_topology()?;
    let union = load_persisted_application_artifact_union(root, &topology, release_build_id)?;
    let path = fleet_install_plan_path(root, fleet, release_build_id);
    let bytes = read_optional_bytes(&path, FileKind::Plan)?
        .ok_or_else(|| FleetInstallPlanError::MissingPlan { path: path.clone() })?;
    check_size(&bytes, FileKind::Plan)?;
    let plan: FleetInstallPlan =
        serde_json::from_slice(&bytes).map_err(|error| invalid_plan(&path, error.to_string()))?;
    if &plan.fleet != fleet || plan.release_build_id != release_build_id {
        return Err(invalid_plan(
            &path,
            "document identity does not match its Fleet/release-build path",
        ));
    }
    let canonical = canonical_plan_bytes(&plan, &topology, config, union.digest)?;
    if canonical != bytes {
        return Err(invalid_plan(&path, "plan bytes are not canonical"));
    }

    let mut root_release_sets = Vec::with_capacity(plan.fleet_subnet_roots.len());
    for planned_root in &plan.fleet_subnet_roots {
        let topology_root = topology_root(planned_root);
        let manifest_path = root_release_set_path(&path, planned_root.placement_subnet);
        let manifest_bytes = read_optional_bytes(&manifest_path, FileKind::RootReleaseSet)?
            .ok_or_else(|| FleetInstallPlanError::MissingRootReleaseSet {
                path: manifest_path.clone(),
            })?;
        check_size(&manifest_bytes, FileKind::RootReleaseSet)?;
        let manifest: FleetSubnetRootReleaseSetManifest =
            serde_json::from_slice(&manifest_bytes)
                .map_err(|error| invalid_root_release_set(&manifest_path, error.to_string()))?;
        let canonical =
            manifest.canonical_bytes_planned(&topology, &topology_root, &union.union)?;
        if canonical != manifest_bytes {
            return Err(invalid_root_release_set(
                &manifest_path,
                "manifest bytes are not canonical",
            ));
        }
        let digest = manifest.digest_planned(&topology, &topology_root, &union.union)?;
        if planned_root.initial_release_set
            != (FleetSubnetRootReleaseSet {
                release_build_id,
                manifest_digest: digest,
            })
        {
            return Err(invalid_plan(
                &path,
                format!(
                    "root {} release-set identity does not match its manifest",
                    planned_root.placement_subnet
                ),
            ));
        }
        root_release_sets.push(PersistedFleetSubnetRootReleaseSet {
            placement_subnet: planned_root.placement_subnet,
            manifest,
            digest,
            path: manifest_path,
        });
    }

    Ok(PersistedFleetInstallPlan {
        digest: Sha256::digest(&bytes).into(),
        plan,
        path,
        root_release_sets,
    })
}

fn compile_plan(
    request: &FleetInstallPlanRequest<'_>,
    union_digest: [u8; 32],
    union: &crate::release_set::ApplicationArtifactUnion,
) -> Result<CompiledFleetInstallPlan, FleetInstallPlanError> {
    if request.config.app_id() != &request.fleet.app {
        return Err(FleetInstallPlanError::AppMismatch {
            configured_app: request.config.app_id().to_string(),
            fleet_app: request.fleet.app.to_string(),
        });
    }
    validate_coordinator(&request.coordinator)?;
    let topology_inputs = request
        .fleet_subnet_roots
        .iter()
        .map(|root| PlannedFleetSubnetRootTopologyInput {
            placement_subnet: root.placement_subnet,
            component_admissions: root.component_admissions.clone(),
            limits: root.limits.clone(),
        })
        .collect();
    let topology_plan = plan_initial_fleet_topology(request.config, topology_inputs)?;

    let mut planned_roots = Vec::with_capacity(topology_plan.fleet_subnet_roots.len());
    let mut root_release_sets = Vec::with_capacity(topology_plan.fleet_subnet_roots.len());
    for topology_root in &topology_plan.fleet_subnet_roots {
        let input = request
            .fleet_subnet_roots
            .iter()
            .find(|input| input.placement_subnet == topology_root.placement_subnet)
            .expect("validated topology root must come from one exact input");
        validate_funding(
            &format!("Fleet Subnet Root {}", topology_root.placement_subnet),
            &input.root_creation_funding,
        )?;
        validate_funding(
            &format!(
                "Wasm Store for Fleet Subnet Root {}",
                topology_root.placement_subnet
            ),
            &input.wasm_store_creation_funding,
        )?;
        let manifest = FleetSubnetRootReleaseSetManifest::project_planned(
            &topology_plan.component_topology,
            topology_root,
            union,
        )?;
        let manifest_digest =
            manifest.digest_planned(&topology_plan.component_topology, topology_root, union)?;
        planned_roots.push(PlannedFleetSubnetRoot {
            placement_subnet: topology_root.placement_subnet,
            component_group_placements: input.component_group_placements.clone(),
            component_admissions: topology_root.component_admissions.clone(),
            component_topology_digest: topology_root.component_topology_digest,
            initial_release_set: FleetSubnetRootReleaseSet {
                release_build_id: request.release_build_id,
                manifest_digest,
            },
            limits: topology_root.limits.clone(),
            canister_pool_imports: input.canister_pool_imports.clone(),
            root_creation_funding: input.root_creation_funding.clone(),
            wasm_store_creation_funding: input.wasm_store_creation_funding.clone(),
        });
        root_release_sets.push(manifest);
    }

    let plan = FleetInstallPlan {
        fleet: request.fleet.clone(),
        release_build_id: request.release_build_id,
        application_artifact_union_digest: union_digest,
        coordinator: request.coordinator.clone(),
        fleet_subnet_roots: planned_roots,
    };
    validate_initial_component_group_assignments(request.config, &plan.fleet_subnet_roots)?;
    canonical_plan_bytes(
        &plan,
        &topology_plan.component_topology,
        request.config,
        union_digest,
    )?;
    Ok(CompiledFleetInstallPlan {
        plan,
        topology_roots: topology_plan.fleet_subnet_roots,
        root_release_sets,
    })
}

fn canonical_plan_bytes(
    plan: &FleetInstallPlan,
    topology: &canic_core::bootstrap::compiled::ComponentTopology,
    config: &ConfigModel,
    union_digest: [u8; 32],
) -> Result<Vec<u8>, FleetInstallPlanError> {
    let configured_app = config.app_id();
    if &plan.fleet.app != configured_app {
        return Err(FleetInstallPlanError::AppMismatch {
            configured_app: configured_app.to_string(),
            fleet_app: plan.fleet.app.to_string(),
        });
    }
    if plan.application_artifact_union_digest != union_digest {
        return Err(FleetInstallPlanError::ApplicationArtifactUnionDigestMismatch);
    }
    validate_coordinator(&plan.coordinator)?;
    let mut previous = None;
    let mut admissions = Vec::with_capacity(plan.fleet_subnet_roots.len());
    for root in &plan.fleet_subnet_roots {
        if root.placement_subnet.as_principal() == &Principal::anonymous() {
            return Err(FleetInstallPlanError::AnonymousRootSubnet);
        }
        if previous.is_some_and(|previous| previous >= root.placement_subnet) {
            return Err(FleetInstallPlanError::NonCanonicalRootOrder);
        }
        previous = Some(root.placement_subnet);
        validate_funding(
            &format!("Fleet Subnet Root {}", root.placement_subnet),
            &root.root_creation_funding,
        )?;
        validate_funding(
            &format!("Wasm Store for Fleet Subnet Root {}", root.placement_subnet),
            &root.wasm_store_creation_funding,
        )?;
        topology.validate_planned_root(
            &root.component_admissions,
            root.component_topology_digest,
            &root.limits,
        )?;
        if root.initial_release_set.release_build_id != plan.release_build_id {
            return Err(FleetInstallPlanError::RootReleaseBuildMismatch {
                placement_subnet: root.placement_subnet,
            });
        }
        admissions.push(root.component_admissions.as_slice());
    }
    topology.validate_fleet_admissions(&admissions)?;
    validate_initial_component_group_assignments(config, &plan.fleet_subnet_roots)?;
    let bytes = serde_json::to_vec(plan).map_err(FleetInstallPlanError::PlanSerialization)?;
    check_size(&bytes, FileKind::Plan)?;
    Ok(bytes)
}

fn validate_initial_component_group_assignments(
    config: &ConfigModel,
    roots: &[PlannedFleetSubnetRoot],
) -> Result<(), FleetInstallPlanError> {
    let configuration = config
        .compile_component_deployment_configuration()
        .map_err(|error| invalid_assignments(error.to_string()))?;
    let deployments = &configuration
        .deployment_topology
        .component_group_deployments;
    let mut assignments = BTreeMap::<
        (ComponentGroupDeploymentId, u32),
        (SubnetId, &ComponentGroupDeploymentSpec),
    >::new();
    let mut service_roots = BTreeMap::<FleetServiceId, BTreeMap<SubnetId, u32>>::new();

    for root in roots {
        let assignments_are_sorted = root.component_group_placements.is_sorted();
        let assignments_are_unique = root
            .component_group_placements
            .windows(2)
            .all(|window| window[0] != window[1]);
        let assignments_are_canonical = [assignments_are_sorted, assignments_are_unique]
            .into_iter()
            .all(std::convert::identity);
        if !assignments_are_canonical {
            return Err(invalid_assignments(format!(
                "root {} assignments are not strictly canonical",
                root.placement_subnet
            )));
        }
        if root.component_group_placements.len() > root.limits.maximum_group_placements as usize {
            return Err(invalid_assignments(format!(
                "root {} exceeds maximum_group_placements",
                root.placement_subnet
            )));
        }
        validate_root_initial_assignment_capacity(
            root,
            deployments,
            &mut assignments,
            &mut service_roots,
        )?;
    }

    for deployment in deployments {
        validate_deployment_assignment(deployment, &assignments)?;
    }
    validate_service_assignments(&configuration.fleet_service_topology, &service_roots)
}

fn validate_root_initial_assignment_capacity<'a>(
    root: &PlannedFleetSubnetRoot,
    deployments: &'a [ComponentGroupDeploymentSpec],
    assignments: &mut BTreeMap<
        (ComponentGroupDeploymentId, u32),
        (SubnetId, &'a ComponentGroupDeploymentSpec),
    >,
    service_roots: &mut BTreeMap<FleetServiceId, BTreeMap<SubnetId, u32>>,
) -> Result<(), FleetInstallPlanError> {
    let mut component_counts = BTreeMap::<ComponentSpecId, u32>::new();
    let mut component_count = 0_u32;
    for assignment in &root.component_group_placements {
        let deployment = deployments
            .binary_search_by(|candidate| candidate.deployment.cmp(&assignment.deployment))
            .ok()
            .map(|index| &deployments[index])
            .ok_or_else(|| {
                invalid_assignments(format!(
                    "root {} references unknown deployment '{}'",
                    root.placement_subnet, assignment.deployment
                ))
            })?;
        if assignment.ordinal >= deployment.initial_placements {
            return Err(invalid_assignments(format!(
                "deployment '{}' ordinal {} is outside its initial placement set",
                assignment.deployment, assignment.ordinal
            )));
        }
        if assignments
            .insert(
                (assignment.deployment.clone(), assignment.ordinal),
                (root.placement_subnet, deployment),
            )
            .is_some()
        {
            return Err(invalid_assignments(format!(
                "deployment '{}' ordinal {} is assigned more than once",
                assignment.deployment, assignment.ordinal
            )));
        }
        component_count = component_count
            .checked_add(
                u32::try_from(deployment.members.len())
                    .map_err(|_| invalid_assignments("deployment member count does not fit u32"))?,
            )
            .ok_or_else(|| invalid_assignments("root Component count overflowed"))?;
        for member in &deployment.members {
            let count = component_counts
                .entry(member.component_spec.clone())
                .or_default();
            *count = count
                .checked_add(1)
                .ok_or_else(|| invalid_assignments("root Component Spec count overflowed"))?;
            if let ComponentDeploymentPurpose::FleetServiceMember { service, .. } = &member.purpose
            {
                let count = service_roots
                    .entry(service.clone())
                    .or_default()
                    .entry(root.placement_subnet)
                    .or_default();
                *count = count
                    .checked_add(1)
                    .ok_or_else(|| invalid_assignments("Fleet-service member count overflowed"))?;
            }
        }
    }
    if component_count > root.limits.maximum_component_instances {
        return Err(invalid_assignments(format!(
            "root {} initial Components exceed protected root capacity",
            root.placement_subnet
        )));
    }
    validate_initial_pool_capacity(root, component_count)?;
    for (component_spec, count) in component_counts {
        let admission = root
            .component_admissions
            .binary_search_by(|admission| admission.component_spec.cmp(&component_spec))
            .ok()
            .map(|index| &root.component_admissions[index])
            .ok_or_else(|| {
                invalid_assignments(format!(
                    "root {} does not admit Component Spec '{}'",
                    root.placement_subnet, component_spec
                ))
            })?;
        if count > admission.maximum_root_instances {
            return Err(invalid_assignments(format!(
                "root {} exceeds admission for Component Spec '{}'",
                root.placement_subnet, component_spec
            )));
        }
    }
    Ok(())
}

fn validate_initial_pool_capacity(
    root: &PlannedFleetSubnetRoot,
    component_count: u32,
) -> Result<(), FleetInstallPlanError> {
    let imported_assets = u32::try_from(root.canister_pool_imports.len())
        .map_err(|_| invalid_assignments("root Canister pool import count does not fit u32"))?;
    let automatic_ready_target = root.limits.canister_pool.minimum_size.max(imported_assets);
    if component_count > automatic_ready_target {
        return Err(invalid_assignments(format!(
            "root {} initial atomic Component batch requires {component_count} Ready prepaid Canisters but its configured minimum/import target is {automatic_ready_target}",
            root.placement_subnet
        )));
    }
    Ok(())
}

fn validate_deployment_assignment(
    deployment: &ComponentGroupDeploymentSpec,
    assignments: &BTreeMap<
        (ComponentGroupDeploymentId, u32),
        (SubnetId, &ComponentGroupDeploymentSpec),
    >,
) -> Result<(), FleetInstallPlanError> {
    let matching = assignments
        .iter()
        .filter(|((candidate, _), _)| candidate == &deployment.deployment)
        .collect::<Vec<_>>();
    let ordinals_are_exact = matching
        .iter()
        .map(|((_, ordinal), _)| *ordinal)
        .eq(0..deployment.initial_placements);
    let assignment_count_is_exact = matching.len() == deployment.initial_placements as usize;
    let assignment_set_is_complete = [assignment_count_is_exact, ordinals_are_exact]
        .into_iter()
        .all(std::convert::identity);
    if !assignment_set_is_complete {
        return Err(invalid_assignments(format!(
            "deployment '{}' does not assign every initial ordinal exactly once",
            deployment.deployment
        )));
    }
    let mut root_counts = BTreeMap::<SubnetId, u32>::new();
    for (_, (root, _)) in matching {
        let count = root_counts.entry(*root).or_default();
        *count = count
            .checked_add(1)
            .ok_or_else(|| invalid_assignments("deployment root count overflowed"))?;
    }
    let density_is_valid = root_counts
        .values()
        .all(|count| *count <= deployment.placement.maximum_per_root);
    let required_roots = deployment
        .initial_placements
        .min(deployment.placement.minimum_distinct_roots) as usize;
    let spread_is_valid = root_counts.len() >= required_roots;
    let placement_is_valid = [density_is_valid, spread_is_valid]
        .into_iter()
        .all(std::convert::identity);
    if !placement_is_valid {
        return Err(invalid_assignments(format!(
            "deployment '{}' violates its root density or spread policy",
            deployment.deployment
        )));
    }
    Ok(())
}

fn validate_service_assignments(
    topology: &FleetServiceTopology,
    service_roots: &BTreeMap<FleetServiceId, BTreeMap<SubnetId, u32>>,
) -> Result<(), FleetInstallPlanError> {
    for target in &topology.targets {
        let roots = service_roots
            .get(&target.service)
            .cloned()
            .unwrap_or_default();
        let density_is_valid = roots
            .values()
            .all(|count| *count <= target.placement.maximum_members_per_root);
        let members = roots.values().try_fold(0_u32, |total, count| {
            total
                .checked_add(*count)
                .ok_or_else(|| invalid_assignments("Fleet-service member count overflowed"))
        })?;
        let required_roots = members.min(target.placement.minimum_distinct_roots) as usize;
        let spread_is_valid = roots.len() >= required_roots;
        let placement_is_valid = [density_is_valid, spread_is_valid]
            .into_iter()
            .all(std::convert::identity);
        if !placement_is_valid {
            return Err(invalid_assignments(format!(
                "Fleet service '{}' violates its root density or spread policy",
                target.service
            )));
        }
    }
    Ok(())
}

fn invalid_assignments(reason: impl Into<String>) -> FleetInstallPlanError {
    FleetInstallPlanError::InvalidComponentGroupPlacementAssignments {
        reason: reason.into(),
    }
}

fn topology_root(root: &PlannedFleetSubnetRoot) -> PlannedFleetSubnetRootTopology {
    PlannedFleetSubnetRootTopology {
        placement_subnet: root.placement_subnet,
        component_admissions: root.component_admissions.clone(),
        component_topology_digest: root.component_topology_digest,
        limits: root.limits.clone(),
    }
}

fn validate_coordinator(
    coordinator: &PlannedFleetCoordinator,
) -> Result<(), FleetInstallPlanError> {
    if coordinator.coordinator_subnet.as_principal() == &Principal::anonymous() {
        return Err(FleetInstallPlanError::AnonymousCoordinatorSubnet);
    }
    validate_funding("Fleet Coordinator", &coordinator.creation_funding)
}

fn validate_funding(
    owner: &str,
    funding: &PlannedCanisterCreationFunding,
) -> Result<(), FleetInstallPlanError> {
    let positive = match funding {
        PlannedCanisterCreationFunding::Cycles { cycles } => *cycles > 0,
        PlannedCanisterCreationFunding::Icp { e8s } => *e8s > 0,
    };
    if positive {
        Ok(())
    } else {
        Err(FleetInstallPlanError::NonPositiveCreationFunding {
            owner: owner.to_string(),
        })
    }
}

fn publish_exact(
    path: &Path,
    bytes: &[u8],
    kind: FileKind,
    conflict: FleetInstallPlanError,
) -> Result<(), FleetInstallPlanError> {
    match create_new_bytes_with_parents(path, bytes) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
            match read_optional_bytes(path, kind)? {
                Some(existing) if existing == bytes => Ok(()),
                Some(_) => Err(conflict),
                None => Err(FleetInstallPlanError::Io {
                    path: path.to_path_buf(),
                    source,
                }),
            }
        }
        Err(source) => Err(FleetInstallPlanError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

#[derive(Clone, Copy)]
enum FileKind {
    Plan,
    RootReleaseSet,
}

fn read_optional_bytes(
    path: &Path,
    kind: FileKind,
) -> Result<Option<Vec<u8>>, FleetInstallPlanError> {
    match read_optional_regular_bytes(path) {
        Ok(bytes) => Ok(bytes),
        Err(RegularFileReadError::NotRegular) => Err(match kind {
            FileKind::Plan => FleetInstallPlanError::UnsafePlan {
                path: path.to_path_buf(),
            },
            FileKind::RootReleaseSet => FleetInstallPlanError::UnsafeRootReleaseSet {
                path: path.to_path_buf(),
            },
        }),
        Err(RegularFileReadError::Io(source)) => Err(FleetInstallPlanError::Io {
            path: path.to_path_buf(),
            source,
        }),
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => Err(FleetInstallPlanError::Io {
            path: path.to_path_buf(),
            source: io::Error::new(
                io::ErrorKind::Unsupported,
                "regular no-follow Fleet install authority reads are unsupported",
            ),
        }),
    }
}

fn lock_plan(path: &Path) -> Result<std::fs::File, FleetInstallPlanError> {
    match lock_regular_file_with_parents(path) {
        Ok(lock) => Ok(lock),
        Err(RegularFileLockError::NotRegular) => Err(FleetInstallPlanError::UnsafePlanLock {
            path: path.to_path_buf(),
        }),
        Err(RegularFileLockError::Io(source)) => Err(FleetInstallPlanError::Io {
            path: path.to_path_buf(),
            source,
        }),
        #[cfg(windows)]
        Err(RegularFileLockError::UnsupportedPlatform) => Err(FleetInstallPlanError::Io {
            path: path.to_path_buf(),
            source: io::Error::new(
                io::ErrorKind::Unsupported,
                "Fleet install plan locking is unsupported",
            ),
        }),
    }
}

fn check_size(bytes: &[u8], kind: FileKind) -> Result<(), FleetInstallPlanError> {
    let (maximum_bytes, error) = match kind {
        FileKind::Plan => (
            MAX_FLEET_INSTALL_PLAN_BYTES,
            FleetInstallPlanError::PlanTooLarge {
                maximum_bytes: MAX_FLEET_INSTALL_PLAN_BYTES,
                actual_bytes: bytes.len(),
            },
        ),
        FileKind::RootReleaseSet => (
            MAX_ROOT_RELEASE_SET_BYTES,
            FleetInstallPlanError::RootReleaseSetTooLarge {
                maximum_bytes: MAX_ROOT_RELEASE_SET_BYTES,
                actual_bytes: bytes.len(),
            },
        ),
    };
    if bytes.len() > maximum_bytes {
        Err(error)
    } else {
        Ok(())
    }
}

fn invalid_plan(path: &Path, reason: impl Into<String>) -> FleetInstallPlanError {
    FleetInstallPlanError::InvalidPlanDocument {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}

fn invalid_root_release_set(path: &Path, reason: impl Into<String>) -> FleetInstallPlanError {
    FleetInstallPlanError::InvalidRootReleaseSetDocument {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}

pub(super) fn fleet_install_plan_path(
    root: &Path,
    fleet: &FleetBinding,
    release_build_id: ReleaseBuildId,
) -> PathBuf {
    root.join(".canic")
        .join("recovery")
        .join("fleet-install-plans")
        .join(fleet.fleet.canonical_network_id.to_string())
        .join(fleet.fleet.fleet_id.to_string())
        .join(release_build_id.to_string())
        .join(FLEET_INSTALL_PLAN_FILE)
}

pub(super) fn root_release_set_path(plan_path: &Path, placement_subnet: SubnetId) -> PathBuf {
    plan_path
        .parent()
        .expect("Fleet install plan path has one identity directory")
        .join(ROOT_RELEASE_SET_DIRECTORY)
        .join(format!("{placement_subnet}.json"))
}
