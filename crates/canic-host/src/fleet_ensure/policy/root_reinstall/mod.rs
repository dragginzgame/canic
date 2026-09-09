//! Module: fleet_ensure::policy::root_reinstall
//!
//! Responsibility: compile exact reviewed Root resets before current protected observation.
//! Does not own: management calls, persistence, or Fleet protocol continuation.
//! Boundary: permits only replacement installation on already controlled running Roots.

use super::*;

/// Compile a management-only reset prerequisite when a Root module differs.
#[expect(
    clippy::too_many_lines,
    reason = "one pure boundary binds the complete reset plan and its conservation budget"
)]
pub(in crate::fleet_ensure) fn compile(
    input: RootStartPlanInput<'_>,
    artifacts: &DesiredFleetArtifacts,
) -> Result<Option<FleetEnsurePlan>, EnsurePolicyError> {
    let RootStartPlanInput {
        state,
        desired,
        desired_sha256,
        observation,
        requested_fleet,
        created_at_time,
        ..
    } = input;
    validate_authority(desired, requested_fleet)?;
    let bounds = cycle_bounds(desired)?;
    let mut canisters = Vec::new();
    let mut bindings = Vec::new();
    let mut observed_cycles = 0_u128;
    for configured in desired.canisters.iter().filter(|configured| {
        configured.kind == DesiredCanisterKind::Root
            && configured.presence == DesiredPresence::Present
    }) {
        let observed = observation.roots.get(&configured.name).ok_or_else(|| {
            EnsurePolicyError::MissingRootManagementObservation {
                name: configured.name.clone(),
            }
        })?;
        let live = &observed.live;
        let expected_hash = wasm_sha256(artifacts, &configured.name)?;
        if live.module_sha256.as_deref() == Some(expected_hash.as_str()) {
            continue;
        }
        let mut controllers = live.controllers.clone();
        controllers.sort();
        let mut expected_controllers = configured.controllers.clone();
        expected_controllers.sort();
        let module_sha256 = live.module_sha256.clone().ok_or_else(|| {
            EnsurePolicyError::RootManagementAuthorityMismatch {
                field: "installed module",
                name: configured.name.clone(),
            }
        })?;
        let binding = RootManagementBinding {
            controllers,
            module_sha256,
            name: observed.name.clone(),
            principal: live.principal.clone(),
            subnet: observed.subnet.clone(),
        };
        let expected = RootManagementBinding {
            controllers: vec![desired.operator.clone()],
            module_sha256: binding.module_sha256.clone(),
            name: configured.name.clone(),
            principal: root_management_principal(configured, state, requested_fleet)?.to_string(),
            subnet: configured.subnet.clone(),
        };
        let desired_controllers_are_exact = expected_controllers == binding.controllers
            && configured.controller_canisters.is_empty();
        if binding != expected
            || !desired_controllers_are_exact
            || live.root_owned_lifecycle.is_some()
        {
            return Err(EnsurePolicyError::RootManagementAuthorityMismatch {
                field: "reinstall authority",
                name: configured.name.clone(),
            });
        }
        if live.status != CanisterRuntimeStatus::Running {
            return Ok(None);
        }
        require_install_initializer(desired, configured)?;
        require_complete_estate(state, desired, &configured.name)?;
        bindings.push(binding);
        observed_cycles = checked_add(observed_cycles, live.cycles, "Root reinstall cycles")?;
        canisters.push(CanisterPlan {
            actions: vec![
                EnsureAction::Stop {
                    name: configured.name.clone(),
                    principal: live.principal.clone(),
                },
                EnsureAction::Install {
                    canic_init: configured.canic_init.clone(),
                    reinstall_witness: None,
                    init_arg: configured.init_arg.clone(),
                    init_arg_sha256: optional_init_arg_sha256(artifacts, configured)?,
                    init_candid: configured.init_candid.clone(),
                    init_candid_sha256: optional_init_candid_sha256(artifacts, configured)?,
                    mode: InstallMode::Reinstall,
                    name: configured.name.clone(),
                    principal: live.principal.clone(),
                    wasm: configured.wasm.clone().ok_or_else(|| {
                        EnsurePolicyError::RootManagementAuthorityMismatch {
                            field: "replacement Wasm",
                            name: configured.name.clone(),
                        }
                    })?,
                    wasm_sha256: expected_hash,
                },
                EnsureAction::Start {
                    name: configured.name.clone(),
                    principal: live.principal.clone(),
                },
            ],
            disposition: CanisterDisposition::Reinstall,
            name: configured.name.clone(),
            observed_cycles: live.cycles,
            principal: Some(live.principal.clone()),
        });
    }
    if canisters.is_empty() {
        return Ok(None);
    }
    let burn = bounds
        .observation_burn
        .checked_mul(3)
        .and_then(|burn| {
            bounds
                .update_burn
                .checked_mul(3)
                .and_then(|updates| burn.checked_add(updates))
        })
        .and_then(|burn| burn.checked_mul(canisters.len() as u128))
        .ok_or(EnsurePolicyError::ArithmeticOverflow {
            field: "Root reinstall burn",
        })?;
    let expected_post_operation_cycles = observed_cycles.checked_sub(burn).ok_or_else(|| {
        EnsurePolicyError::InsufficientCycleConservation {
            action_count: canisters.len(),
            available: observed_cycles,
            required: burn,
            shortfall: burn.saturating_sub(observed_cycles),
        }
    })?;
    let mut plan = FleetEnsurePlan {
        canisters,
        continuation: None,
        conservation: CycleConservation {
            estate_funding_domains: Vec::new(),
            expected_post_operation_cycles,
            maximum_execution_burn_cycles: burn,
            maximum_new_funding_cycles: 0,
            maximum_operator_debit_cycles: 0,
            maximum_unavoidable_fee_cycles: 0,
            observed_controlled_cycles: observed_cycles,
            retained_in_reused_canisters_cycles: observed_cycles,
            scheduled_transfer_cycles: 0,
        },
        desired_sha256: desired_sha256.to_string(),
        environment: desired.environment.clone(),
        fleet: requested_fleet.to_string(),
        operation_id: operation_id(desired_sha256, &desired.environment, requested_fleet),
        plan_sha256: String::new(),
        planned_at_time: created_at_time,
        protocol_actions: Vec::new(),
        recovery_review: Some(Box::new(crate::fleet_ensure::model::FleetRecoveryReview {
            base_execution_burn_cycles: burn,
            continuation_reserve_cycles: 0,
            whole_continuation_ceiling_cycles: 0,
            known_pool_funding: Vec::new(),
            discovery: crate::fleet_ensure::model::RecoveryDiscovery::PendingCurrentProtocol,
        })),
        reinstall: None,
        root_reinstall_bindings: bindings,
        root_start_authority: None,
        reviewed_desired: Some(Box::new(
            crate::fleet_ensure::model::ReviewedDesiredFleetRecord::capture(desired),
        )),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        scope: FleetEnsurePlanScope::RootReinstallPrerequisite,
        terminal_inventory_operation_id: None,
    };
    plan.plan_sha256 = expected_plan_sha256(&plan);
    Ok(Some(plan))
}

/// Retained identities are omission evidence, never authority to add an unverified asset.
fn require_complete_estate(
    state: &FleetEnsureStateRecord,
    desired: &DesiredFleet,
    root: &str,
) -> Result<(), EnsurePolicyError> {
    let imports = desired
        .bootstrap
        .as_ref()
        .and_then(|bootstrap| bootstrap.roots.iter().find(|r| r.root == root));
    let selected = imports
        .into_iter()
        .flat_map(|r| &r.canister_pool_imports)
        .filter_map(|name| {
            desired
                .canisters
                .iter()
                .find(|c| &c.name == name)
                .and_then(|c| c.principal.as_ref())
                .or_else(|| state.principals.get(name))
        })
        .collect::<BTreeSet<_>>();
    let mut missing_principals = Vec::new();
    for (name, principal) in &state.principals {
        let Some(topology) = state.topology.get(name) else {
            continue;
        };
        if topology.kind == DesiredCanisterKind::Store
            || name == root
            || selected.contains(principal)
        {
            continue;
        }
        let mut parent = topology.parent.as_deref();
        let mut visited = BTreeSet::new();
        while let Some(name) = parent {
            if name == root {
                missing_principals.push(principal.clone());
                break;
            }
            if !visited.insert(name) {
                break;
            }
            parent = state.topology.get(name).and_then(|t| t.parent.as_deref());
        }
    }
    missing_principals.sort();
    missing_principals.dedup();
    if missing_principals.is_empty() {
        Ok(())
    } else {
        Err(EnsurePolicyError::IncompleteRootEstate {
            root: root.into(),
            missing_principals,
        })
    }
}
