//! Module: canic_cli::medic::fleet
//!
//! Responsibility: construct terminal current-Fleet ensure checks.
//! Does not own: Fleet convergence, live mutation, or retained compatibility.
//! Boundary: accepts only the current ensure inventory and its exact reviewed plan.

use crate::{
    cli::defaults::local_environment,
    medic::{
        command::MedicOptions,
        report::{MedicCategory, MedicCheck, MedicSource},
    },
};
use std::path::{Path, PathBuf};

use canic_host::{
    fleet_ensure::{CurrentFleetResolution, load_desired_fleet},
    icp_config::resolve_current_canic_icp_root,
};

/// Local target information shared by Fleet-level checks.
pub(super) struct FleetMedicContext {
    pub(super) icp_root: Option<PathBuf>,
    pub(super) environment: String,
    pub(super) environment_check: MedicCheck,
}

pub(super) fn fleet_medic_context(options: &MedicOptions) -> FleetMedicContext {
    let icp_root = resolve_current_canic_icp_root().ok();
    let (environment, environment_check) = fleet_environment_selection(options);
    FleetMedicContext {
        icp_root,
        environment,
        environment_check,
    }
}

pub(super) fn fleet_environment_selection(options: &MedicOptions) -> (String, MedicCheck) {
    if let Some(environment) = &options.environment {
        return (
            environment.clone(),
            MedicCheck::pass(
                MedicCategory::TargetEnvironment,
                "local_environment_explicit",
                "environment",
                environment.clone(),
                "none",
                MedicSource::Command,
            ),
        );
    }

    let environment = local_environment();
    (
        environment.clone(),
        MedicCheck::pass(
            MedicCategory::TargetEnvironment,
            "local_environment_implicit",
            "environment",
            environment,
            "override with top-level --environment <name>",
            MedicSource::Command,
        ),
    )
}

pub(super) fn current_fleet_checks(
    root: &Path,
    fleet: &str,
    resolution: &CurrentFleetResolution,
) -> Vec<MedicCheck> {
    vec![
        current_desired_check(root, fleet, resolution),
        current_topology_check(fleet, resolution),
        current_protocol_authority_check(fleet, resolution),
        current_conservation_check(resolution),
    ]
}

fn current_desired_check(
    root: &Path,
    fleet: &str,
    resolution: &CurrentFleetResolution,
) -> MedicCheck {
    let path = root.join("fleets").join(format!("{fleet}.toml"));
    match load_desired_fleet(&path) {
        Ok(loaded)
            if loaded.sha256 == resolution.plan.desired_sha256
                && loaded.desired.environment == resolution.plan.environment
                && loaded.desired.fleet == resolution.plan.fleet =>
        {
            MedicCheck::pass(
                MedicCategory::WorkspaceConfig,
                "current_fleet_desired_matches",
                "desired_fleet",
                format!(
                    "{} matches desired_sha256={}",
                    path.display(),
                    resolution.plan.desired_sha256
                ),
                "none",
                MedicSource::CurrentEnsure,
            )
        }
        Ok(_) => MedicCheck::fail(
            MedicCategory::WorkspaceConfig,
            "current_fleet_desired_drift",
            "desired_fleet",
            format!(
                "{} no longer matches terminal desired_sha256={}",
                path.display(),
                resolution.plan.desired_sha256
            ),
            ensure_plan_next(fleet),
            MedicSource::CurrentEnsure,
        ),
        Err(error) => MedicCheck::fail(
            MedicCategory::WorkspaceConfig,
            "current_fleet_desired_unavailable",
            "desired_fleet",
            error.to_string(),
            format!(
                "restore {} or pass its replacement explicitly to {}",
                path.display(),
                ensure_plan_next(fleet)
            ),
            MedicSource::CurrentEnsure,
        ),
    }
}

fn current_topology_check(fleet: &str, resolution: &CurrentFleetResolution) -> MedicCheck {
    MedicCheck::pass(
        MedicCategory::Topology,
        "current_fleet_topology_bound",
        "topology",
        format!(
            "coordinator={}; roots={}; canisters={}; operation={}",
            resolution.topology.coordinator_canister_id,
            resolution.topology.fleet_subnet_root_canister_ids.len(),
            resolution.registry.entries.len(),
            resolution.plan.operation_id,
        ),
        format!("run canic info list {fleet}"),
        MedicSource::CurrentEnsure,
    )
}

fn current_protocol_authority_check(
    fleet: &str,
    resolution: &CurrentFleetResolution,
) -> MedicCheck {
    match resolution.initial_active_registry(fleet) {
        Ok(registry) => MedicCheck::pass(
            MedicCategory::Topology,
            "current_fleet_protocol_authority_bound",
            "registry",
            format!(
                "registry_version={}; roots={}; coordinator={}",
                registry.revision,
                registry.fleet_subnet_roots.len(),
                registry.authority.binding.coordinator,
            ),
            "none",
            MedicSource::CurrentEnsure,
        ),
        Err(error) => MedicCheck::fail(
            MedicCategory::Topology,
            "current_fleet_protocol_authority_invalid",
            "registry",
            error.to_string(),
            ensure_plan_next(fleet),
            MedicSource::CurrentEnsure,
        ),
    }
}

fn current_conservation_check(resolution: &CurrentFleetResolution) -> MedicCheck {
    let conservation = &resolution.plan.conservation;
    MedicCheck::pass(
        MedicCategory::Funding,
        "current_fleet_conservation_bound",
        "cycles",
        format!(
            "observed={}; retained={}; transfers={}; maximum_fees={}; maximum_burn={}; maximum_new_funding={}; maximum_operator_debit={}; expected_post={}",
            conservation.observed_controlled_cycles,
            conservation.retained_in_reused_canisters_cycles,
            conservation.scheduled_transfer_cycles,
            conservation.maximum_unavoidable_fee_cycles,
            conservation.maximum_execution_burn_cycles,
            conservation.maximum_new_funding_cycles,
            conservation.maximum_operator_debit_cycles,
            conservation.expected_post_operation_cycles,
        ),
        "re-plan before any new paid effect",
        MedicSource::CurrentEnsure,
    )
}

pub(super) fn ensure_plan_next(fleet: &str) -> String {
    format!("run canic fleet ensure {fleet} --desired fleets/{fleet}.toml and review plan_sha256")
}
