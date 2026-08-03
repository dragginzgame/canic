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
    io,
    path::{Path, PathBuf},
};

use candid::Principal;
use canic_core::{
    bootstrap::compiled::ConfigModel,
    ids::{AppId, FleetBinding, FleetSubnetRootReleaseSet, ReleaseBuildId, SubnetId},
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

    let plan_bytes = canonical_plan_bytes(
        &compiled.plan,
        &topology,
        request.config.app_id(),
        union.digest,
    )?;
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
    let canonical = canonical_plan_bytes(&plan, &topology, config.app_id(), union.digest)?;
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
            &input.creation_funding,
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
            component_admissions: topology_root.component_admissions.clone(),
            component_topology_digest: topology_root.component_topology_digest,
            initial_release_set: FleetSubnetRootReleaseSet {
                release_build_id: request.release_build_id,
                manifest_digest,
            },
            limits: topology_root.limits.clone(),
            canister_pool_imports: input.canister_pool_imports.clone(),
            creation_funding: input.creation_funding.clone(),
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
    canonical_plan_bytes(
        &plan,
        &topology_plan.component_topology,
        request.config.app_id(),
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
    configured_app: &AppId,
    union_digest: [u8; 32],
) -> Result<Vec<u8>, FleetInstallPlanError> {
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
            &root.creation_funding,
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
    let bytes = serde_json::to_vec(plan).map_err(FleetInstallPlanError::PlanSerialization)?;
    check_size(&bytes, FileKind::Plan)?;
    Ok(bytes)
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
