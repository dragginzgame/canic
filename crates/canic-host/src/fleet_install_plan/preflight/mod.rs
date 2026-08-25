//! Module: fleet_install_plan::preflight
//!
//! Responsibility: purely compile canonical fresh-Fleet placement, admission, and funding input.
//! Does not own: input loading, catalog collection, release artifacts, persistence, or effects.
//! Boundary: both plan and install must pass this compiler before any effect boundary.

use super::{initial_placement_policy, model::*};
use crate::component_topology::{PlannedFleetSubnetRootTopologyInput, plan_initial_fleet_topology};

use candid::Principal;
use canic_core::{
    ids::{FleetAdmissionSelector, FleetAdmissionTarget},
    shared_support::fleet_admission_policy::{
        effective_fleet_admission_template_principals, fleet_admission_template_projection_digest,
        validate_fleet_admission_policy_template,
    },
};

pub const FRESH_FLEET_PREFLIGHT_SCHEMA_VERSION: u16 = 1;

/// Compile one canonical effect-free fresh-Fleet preflight result.
pub fn compile_fresh_fleet_preflight(
    request: FreshFleetPreflightRequest<'_>,
) -> Result<FreshFleetPreflightV1, FreshFleetPreflightError> {
    compile_fleet_preflight(request, false)
}

pub fn compile_retained_fleet_preflight(
    request: FreshFleetPreflightRequest<'_>,
) -> Result<FreshFleetPreflightV1, FreshFleetPreflightError> {
    compile_fleet_preflight(request, true)
}

fn compile_fleet_preflight(
    request: FreshFleetPreflightRequest<'_>,
    retain_historical_pool_policy: bool,
) -> Result<FreshFleetPreflightV1, FreshFleetPreflightError> {
    validate_effect_boundary(request.effects)?;
    if request.config.app_id().as_str() != request.app {
        return Err(FreshFleetPreflightError::AppMismatch {
            configured_app: request.config.app_id().to_string(),
            requested_app: request.app.to_string(),
        });
    }
    validate_coordinator(request.coordinator)?;
    validate_admission(
        request.config,
        request.fleet_subnet_roots,
        request.admission,
    )?;

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
    let mut fleet_subnet_roots = Vec::with_capacity(topology_plan.fleet_subnet_roots.len());

    for topology_root in &topology_plan.fleet_subnet_roots {
        let input = request
            .fleet_subnet_roots
            .iter()
            .find(|input| input.placement_subnet == topology_root.placement_subnet)
            .ok_or(FreshFleetPreflightError::MissingResolvedRoot {
                placement_subnet: topology_root.placement_subnet,
            })?;
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
        fleet_subnet_roots.push(FreshFleetSubnetRootPlanV1 {
            placement_subnet: topology_root.placement_subnet,
            placement_cost: input.placement_cost.clone(),
            component_group_placements: input.component_group_placements.clone(),
            component_admissions: topology_root.component_admissions.clone(),
            component_topology_digest: topology_root.component_topology_digest,
            admission_projections: compile_admission_projections(
                request.config,
                request.admission,
                topology_root.placement_subnet,
                &topology_root.component_admissions,
            )?,
            limits: topology_root.limits.clone(),
            funding: input.funding.clone(),
            canister_pool_imports: input.canister_pool_imports.clone(),
            root_creation_funding: input.root_creation_funding.clone(),
            wasm_store_creation_funding: input.wasm_store_creation_funding.clone(),
            initial_component_canisters: 0,
            initial_pool_canisters: 0,
            pool_canister_creations: 0,
            remaining_pool_canisters: 0,
        });
    }

    let component_counts = if retain_historical_pool_policy {
        initial_placement_policy::validate_historical_component_group_assignments(
            request.config,
            &fleet_subnet_roots,
        )
    } else {
        initial_placement_policy::validate_initial_component_group_assignments(
            request.config,
            &fleet_subnet_roots,
        )
    }
    .map_err(
        |error| FreshFleetPreflightError::InvalidComponentGroupPlacementAssignments {
            reason: error.to_string(),
        },
    )?;
    record_root_counts(&mut fleet_subnet_roots, &component_counts)?;

    Ok(FreshFleetPreflightV1 {
        schema_version: FRESH_FLEET_PREFLIGHT_SCHEMA_VERSION,
        app: request.app.to_string(),
        fleet_name: request.fleet_name.clone(),
        funding_profile: request.coordinator.root_funding.as_ref().map_or(
            canic_core::ids::FleetFundingProfile::SingleSubnet,
            |policy| policy.funding_profile,
        ),
        coordinator: request.coordinator.clone(),
        admission: request.admission.clone(),
        fleet_subnet_roots,
        build_profile: request.build_profile.target_dir_name().to_string(),
        release_build_id: request.release_build_id,
        effects: request.effects,
        component_topology: topology_plan.component_topology,
    })
}

fn validate_admission(
    config: &canic_core::bootstrap::compiled::ConfigModel,
    roots: &[PlannedFleetSubnetRootInput],
    admission: &canic_core::ids::FleetAdmissionPolicyTemplate,
) -> Result<(), FreshFleetPreflightError> {
    validate_fleet_admission_policy_template(admission).map_err(|error| {
        FreshFleetPreflightError::InvalidAdmissionPolicy {
            reason: error.to_string(),
        }
    })?;
    for rule in &admission.rules {
        match &rule.selector {
            FleetAdmissionSelector::ComponentSpec(component_spec) => {
                let configured = config.component_specs.contains_key(component_spec);
                let admitted = roots.iter().any(|root| {
                    root.component_admissions
                        .iter()
                        .any(|admission| &admission.component_spec == component_spec)
                });
                if !configured || !admitted {
                    return Err(FreshFleetPreflightError::UnknownAdmissionComponentSpec {
                        component_spec: component_spec.to_string(),
                    });
                }
            }
            FleetAdmissionSelector::FleetSubnetRoot(placement_subnet) => {
                if !roots
                    .iter()
                    .any(|root| &root.placement_subnet == placement_subnet)
                {
                    return Err(FreshFleetPreflightError::UnknownAdmissionFleetSubnetRoot {
                        placement_subnet: *placement_subnet,
                    });
                }
            }
            FleetAdmissionSelector::Fleet | FleetAdmissionSelector::ComponentInstance(_) => {
                return Err(FreshFleetPreflightError::UnsupportedAdmissionSelector);
            }
        }
    }
    Ok(())
}

fn compile_admission_projections(
    config: &canic_core::bootstrap::compiled::ConfigModel,
    admission: &canic_core::ids::FleetAdmissionPolicyTemplate,
    fleet_subnet_root: canic_core::ids::SubnetId,
    components: &[canic_core::ids::ComponentSpecAdmission],
) -> Result<Vec<PlannedFleetAdmissionProjection>, FreshFleetPreflightError> {
    components
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
                fleet_subnet_root,
            };
            let effective = effective_fleet_admission_template_principals(admission, &target);
            Ok(PlannedFleetAdmissionProjection {
                component_spec: component.component_spec.clone(),
                participant_roles,
                effective_principal_count: u32::try_from(effective.len()).map_err(|_| {
                    FreshFleetPreflightError::CountDoesNotFitU32 {
                        subject: "Fleet admission projection Principal",
                    }
                })?,
                template_projection_digest: fleet_admission_template_projection_digest(
                    admission.template_digest,
                    &target,
                    &effective,
                ),
            })
        })
        .collect()
}

fn record_root_counts(
    roots: &mut [FreshFleetSubnetRootPlanV1],
    component_counts: &std::collections::BTreeMap<canic_core::ids::SubnetId, u32>,
) -> Result<(), FreshFleetPreflightError> {
    for root in roots {
        let imported = u32::try_from(root.canister_pool_imports.len()).map_err(|_| {
            FreshFleetPreflightError::CountDoesNotFitU32 {
                subject: "root Canister pool import",
            }
        })?;
        let initial_pool = root.limits.canister_pool.minimum_size.max(imported);
        let components = component_counts
            .get(&root.placement_subnet)
            .copied()
            .unwrap_or_default();
        root.initial_component_canisters = components;
        root.initial_pool_canisters = initial_pool;
        root.pool_canister_creations = initial_pool.saturating_sub(imported);
        root.remaining_pool_canisters = initial_pool.saturating_sub(components);
    }
    Ok(())
}

const fn validate_effect_boundary(
    effects: FreshFleetPreflightEffectsV1,
) -> Result<(), FreshFleetPreflightError> {
    if effects.no_effects_started() {
        return Ok(());
    }
    Err(FreshFleetPreflightError::EffectsAlreadyStarted {
        build_started: effects.build_started,
        workspace_mutation_started: effects.workspace_mutation_started,
        ic_mutation_started: effects.ic_mutation_started,
    })
}

pub(super) fn validate_coordinator(
    coordinator: &PlannedFleetCoordinator,
) -> Result<(), FreshFleetPreflightError> {
    if coordinator.coordinator_subnet.as_principal() == &Principal::anonymous() {
        return Err(FreshFleetPreflightError::AnonymousCoordinatorSubnet);
    }
    validate_funding("Fleet Coordinator", &coordinator.creation_funding)
}

pub(super) fn validate_funding(
    owner: &str,
    funding: &PlannedCanisterCreationFunding,
) -> Result<(), FreshFleetPreflightError> {
    let positive = match funding {
        PlannedCanisterCreationFunding::Cycles { cycles } => *cycles > 0,
        PlannedCanisterCreationFunding::Icp { e8s } => *e8s > 0,
    };
    if positive {
        Ok(())
    } else {
        Err(FreshFleetPreflightError::NonPositiveCreationFunding {
            owner: owner.to_string(),
        })
    }
}
