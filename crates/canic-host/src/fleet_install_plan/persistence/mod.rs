//! Module: fleet_install_plan::persistence
//!
//! Responsibility: compile, validate, and immutably publish pre-effect Fleet install authority.
//! Does not own: passive plan data, Subnet selection, Canister creation, or Registry mutation.
//! Boundary: finalized build evidence plus explicit placement/funding input becomes durable
//! before any external effect.

use crate::{
    component_topology::PlannedFleetSubnetRootTopology,
    durable_io::{
        RegularFileLockError, RegularFileReadError, create_new_bytes_with_parents,
        lock_regular_file_with_parents, read_optional_regular_bytes,
    },
    fleet_install_plan::initial_placement_policy,
    fleet_install_plan::model::{
        FleetInstallPlan, FleetInstallPlanError, FleetInstallPlanRequest,
        FreshFleetPreflightEffectsV1, FreshFleetPreflightError, FreshFleetPreflightRequest,
        FreshFleetSubnetRootPlanV1, PersistedFleetInstallPlan, PersistedFleetSubnetRootReleaseSet,
        PlannedCanisterCreationFunding, PlannedFleetAdmissionProjection, PlannedFleetCoordinator,
        PlannedFleetSubnetRoot,
    },
    fleet_install_plan::preflight::compile_fresh_fleet_preflight,
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
    ids::{
        FleetAdmissionTarget, FleetBinding, FleetSubnetRootReleaseSet, ReleaseBuildId, SubnetId,
    },
    shared_support::fleet_admission_policy::{
        bind_initial_fleet_admission_policy, compile_fleet_admission_policy_template,
        effective_fleet_admission_template_principals, fleet_admission_template_projection_digest,
        validate_installed_fleet_admission_policy,
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
    let finalized = load_finalized_release_build(request.root, request.release_build_id)?;
    let topology = request.config.compile_component_topology()?;
    let union = load_persisted_application_artifact_union(
        request.root,
        &topology,
        request.release_build_id,
    )?;
    let compiled = compile_plan(
        &request,
        finalized.record.build_profile,
        union.digest,
        &union.union,
    )?;
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
    build_profile: crate::canister_build::CanisterBuildProfile,
    union_digest: [u8; 32],
    union: &crate::release_set::ApplicationArtifactUnion,
) -> Result<CompiledFleetInstallPlan, FleetInstallPlanError> {
    let preflight = compile_fresh_fleet_preflight(FreshFleetPreflightRequest {
        config: request.config,
        app: request.fleet.app.as_str(),
        fleet_name: &request.fleet_name,
        coordinator: &request.coordinator,
        admission: &request.admission,
        fleet_subnet_roots: &request.fleet_subnet_roots,
        build_profile,
        release_build_id: Some(request.release_build_id),
        effects: FreshFleetPreflightEffectsV1::none_started(),
    })
    .map_err(fleet_install_plan_error)?;

    let mut planned_roots = Vec::with_capacity(preflight.fleet_subnet_roots.len());
    let mut root_release_sets = Vec::with_capacity(preflight.fleet_subnet_roots.len());
    let mut topology_roots = Vec::with_capacity(preflight.fleet_subnet_roots.len());
    for root in &preflight.fleet_subnet_roots {
        let topology_root = fresh_topology_root(root);
        let manifest = FleetSubnetRootReleaseSetManifest::project_planned(
            &preflight.component_topology,
            &topology_root,
            union,
        )?;
        let manifest_digest =
            manifest.digest_planned(&preflight.component_topology, &topology_root, union)?;
        planned_roots.push(PlannedFleetSubnetRoot {
            placement_subnet: root.placement_subnet,
            placement_cost: root.placement_cost.clone(),
            component_group_placements: root.component_group_placements.clone(),
            component_admissions: root.component_admissions.clone(),
            component_topology_digest: root.component_topology_digest,
            admission_projections: root.admission_projections.clone(),
            initial_release_set: FleetSubnetRootReleaseSet {
                release_build_id: request.release_build_id,
                manifest_digest,
            },
            limits: root.limits.clone(),
            funding: root.funding.clone(),
            canister_pool_imports: root.canister_pool_imports.clone(),
            root_creation_funding: root.root_creation_funding.clone(),
            wasm_store_creation_funding: root.wasm_store_creation_funding.clone(),
        });
        root_release_sets.push(manifest);
        topology_roots.push(topology_root);
    }

    let admission = bind_initial_fleet_admission_policy(request.fleet.clone(), &request.admission)
        .map_err(|error| FleetInstallPlanError::InvalidAdmissionPolicy {
            reason: error.to_string(),
        })?;
    let plan = FleetInstallPlan {
        fleet: request.fleet.clone(),
        fresh_fleet_plan_digest: request.fresh_fleet_plan_digest.clone(),
        release_build_id: request.release_build_id,
        application_artifact_union_digest: union_digest,
        admission,
        coordinator: request.coordinator.clone(),
        fleet_subnet_roots: planned_roots,
    };
    canonical_plan_bytes(
        &plan,
        &preflight.component_topology,
        request.config,
        union_digest,
    )?;
    Ok(CompiledFleetInstallPlan {
        plan,
        topology_roots,
        root_release_sets,
    })
}

fn fleet_install_plan_error(error: FreshFleetPreflightError) -> FleetInstallPlanError {
    match error {
        FreshFleetPreflightError::AppMismatch {
            configured_app,
            requested_app,
        } => FleetInstallPlanError::AppMismatch {
            configured_app,
            fleet_app: requested_app,
        },
        FreshFleetPreflightError::EffectsAlreadyStarted {
            build_started,
            workspace_mutation_started,
            ic_mutation_started,
        } => FleetInstallPlanError::EffectsAlreadyStarted {
            build_started,
            workspace_mutation_started,
            ic_mutation_started,
        },
        FreshFleetPreflightError::AnonymousCoordinatorSubnet => {
            FleetInstallPlanError::AnonymousCoordinatorSubnet
        }
        FreshFleetPreflightError::NonPositiveCreationFunding { owner } => {
            FleetInstallPlanError::NonPositiveCreationFunding { owner }
        }
        FreshFleetPreflightError::MissingResolvedRoot { placement_subnet } => {
            FleetInstallPlanError::MissingResolvedRoot { placement_subnet }
        }
        FreshFleetPreflightError::InvalidComponentGroupPlacementAssignments { reason } => {
            invalid_assignments(reason)
        }
        FreshFleetPreflightError::CountDoesNotFitU32 { subject } => {
            invalid_assignments(format!("{subject} count does not fit u32"))
        }
        FreshFleetPreflightError::InvalidAdmissionPolicy { reason } => {
            FleetInstallPlanError::InvalidAdmissionPolicy { reason }
        }
        FreshFleetPreflightError::UnknownAdmissionComponentSpec { component_spec } => {
            FleetInstallPlanError::InvalidAdmissionPolicy {
                reason: format!("unknown Component Spec '{component_spec}'"),
            }
        }
        FreshFleetPreflightError::UnknownAdmissionFleetSubnetRoot { placement_subnet } => {
            FleetInstallPlanError::InvalidAdmissionPolicy {
                reason: format!("unknown Fleet Subnet Root {placement_subnet}"),
            }
        }
        FreshFleetPreflightError::UnsupportedAdmissionSelector => {
            FleetInstallPlanError::InvalidAdmissionPolicy {
                reason: "unsupported generation-one selector".to_string(),
            }
        }
        FreshFleetPreflightError::Topology(error) => FleetInstallPlanError::Topology(error),
    }
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
    if !is_canonical_sha256(&plan.fresh_fleet_plan_digest) {
        return Err(FleetInstallPlanError::InvalidFreshFleetPlanDigest);
    }
    validate_installed_fleet_admission_policy(&plan.admission).map_err(|error| {
        FleetInstallPlanError::InvalidAdmissionPolicy {
            reason: error.to_string(),
        }
    })?;
    if plan.admission.fleet != plan.fleet {
        return Err(FleetInstallPlanError::InvalidAdmissionPolicy {
            reason: "policy Fleet binding does not match its install plan".to_string(),
        });
    }
    if plan.admission.generation != canic_core::ids::FLEET_ADMISSION_INITIAL_GENERATION {
        return Err(FleetInstallPlanError::InvalidAdmissionPolicy {
            reason: "fresh install policy is not generation one".to_string(),
        });
    }
    validate_installed_admission_selectors(plan, config)?;
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

fn validate_installed_admission_selectors(
    plan: &FleetInstallPlan,
    config: &ConfigModel,
) -> Result<(), FleetInstallPlanError> {
    for rule in &plan.admission.rules {
        match &rule.selector {
            canic_core::ids::FleetAdmissionSelector::ComponentSpec(component_spec) => {
                let configured = config.component_specs.contains_key(component_spec);
                let admitted = plan.fleet_subnet_roots.iter().any(|root| {
                    root.component_admissions
                        .iter()
                        .any(|admission| &admission.component_spec == component_spec)
                });
                if !configured || !admitted {
                    return Err(FleetInstallPlanError::InvalidAdmissionPolicy {
                        reason: format!("unknown Component Spec '{component_spec}'"),
                    });
                }
            }
            canic_core::ids::FleetAdmissionSelector::FleetSubnetRoot(placement_subnet) => {
                if !plan
                    .fleet_subnet_roots
                    .iter()
                    .any(|root| &root.placement_subnet == placement_subnet)
                {
                    return Err(FleetInstallPlanError::InvalidAdmissionPolicy {
                        reason: format!("unknown Fleet Subnet Root {placement_subnet}"),
                    });
                }
            }
            canic_core::ids::FleetAdmissionSelector::Fleet
            | canic_core::ids::FleetAdmissionSelector::ComponentInstance(_) => {
                return Err(FleetInstallPlanError::InvalidAdmissionPolicy {
                    reason: "unsupported generation-one selector".to_string(),
                });
            }
        }
    }
    let template = compile_fleet_admission_policy_template(
        plan.admission.fleet_principals.clone(),
        plan.admission.rules.clone(),
    )
    .map_err(|error| FleetInstallPlanError::InvalidAdmissionPolicy {
        reason: error.to_string(),
    })?;
    for root in &plan.fleet_subnet_roots {
        let expected = root
            .component_admissions
            .iter()
            .filter_map(|component| {
                let participant_roles =
                    config.component_spec_fleet_admission_roles(&component.component_spec)?;
                (!participant_roles.is_empty()).then_some((component, participant_roles))
            })
            .map(|(component, participant_roles)| {
                let target = FleetAdmissionTarget {
                    component_spec: component.component_spec.clone(),
                    component_instance: None,
                    fleet_subnet_root: root.placement_subnet,
                };
                let effective = effective_fleet_admission_template_principals(&template, &target);
                Ok(PlannedFleetAdmissionProjection {
                    component_spec: component.component_spec.clone(),
                    participant_roles,
                    effective_principal_count: u32::try_from(effective.len()).map_err(|_| {
                        FleetInstallPlanError::InvalidAdmissionPolicy {
                            reason: "projection Principal count does not fit u32".to_string(),
                        }
                    })?,
                    template_projection_digest: fleet_admission_template_projection_digest(
                        template.template_digest,
                        &target,
                        &effective,
                    ),
                })
            })
            .collect::<Result<Vec<_>, FleetInstallPlanError>>()?;
        if root.admission_projections != expected {
            return Err(FleetInstallPlanError::InvalidAdmissionPolicy {
                reason: format!(
                    "Fleet Subnet Root {} admission projections do not match policy and topology",
                    root.placement_subnet
                ),
            });
        }
    }
    Ok(())
}

fn is_canonical_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn validate_initial_component_group_assignments(
    config: &ConfigModel,
    roots: &[PlannedFleetSubnetRoot],
) -> Result<(), FleetInstallPlanError> {
    let roots = roots.iter().map(fresh_root).collect::<Vec<_>>();
    initial_placement_policy::validate_initial_component_group_assignments(config, &roots)
        .map(|_| ())
        .map_err(|error| invalid_assignments(error.to_string()))
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

fn fresh_root(root: &PlannedFleetSubnetRoot) -> FreshFleetSubnetRootPlanV1 {
    FreshFleetSubnetRootPlanV1 {
        placement_subnet: root.placement_subnet,
        placement_cost: root.placement_cost.clone(),
        component_group_placements: root.component_group_placements.clone(),
        component_admissions: root.component_admissions.clone(),
        component_topology_digest: root.component_topology_digest,
        admission_projections: root.admission_projections.clone(),
        limits: root.limits.clone(),
        funding: root.funding.clone(),
        canister_pool_imports: root.canister_pool_imports.clone(),
        root_creation_funding: root.root_creation_funding.clone(),
        wasm_store_creation_funding: root.wasm_store_creation_funding.clone(),
        initial_component_canisters: 0,
        initial_pool_canisters: 0,
        pool_canister_creations: 0,
        remaining_pool_canisters: 0,
    }
}

fn fresh_topology_root(root: &FreshFleetSubnetRootPlanV1) -> PlannedFleetSubnetRootTopology {
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
