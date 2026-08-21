//! Module: canic_cli::medic::fleet
//!
//! Responsibility: construct installed-Fleet, Registry, and Coordinator checks.
//! Does not own: deployment mutation, check ordering, or report rendering.
//! Boundary: maps local and runtime Fleet evidence into Medic checks.

use crate::{
    cli::defaults::local_environment,
    medic::{
        command::MedicOptions,
        display_medic_path,
        report::{MedicCategory, MedicCheck, MedicSource, MedicStatus},
    },
};
use std::path::{Path, PathBuf};

use canic_host::{
    fleet_catalog::FleetCatalogEntryV1,
    icp_config::resolve_current_canic_icp_root,
    installed_fleet::{
        InstalledFleetError, InstalledFleetRequest, InstalledFleetResolution, InstalledFleetSource,
        resolve_installed_fleet_from_root,
    },
};

///
/// FleetMedicContext
///

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

pub(super) fn installed_fleet_checks(
    icp_root: Option<&Path>,
    fleet: &FleetCatalogEntryV1,
    environment: &str,
) -> Vec<MedicCheck> {
    let coordinator = check_coordinator_canister_id(fleet);
    let coordinator_present = coordinator.status != MedicStatus::Fail;

    vec![
        check_config_path(icp_root, fleet),
        coordinator,
        check_fleet_registry_observation(icp_root, fleet, environment, coordinator_present),
        check_coordinator_readiness_not_evaluated(coordinator_present),
    ]
}

fn check_config_path(icp_root: Option<&Path>, fleet: &FleetCatalogEntryV1) -> MedicCheck {
    let Some(root) = icp_root else {
        return MedicCheck::not_evaluated(
            MedicCategory::WorkspaceConfig,
            "app_config_not_evaluated",
            "config",
            "App config lookup skipped because the workspace root was not resolved",
            "run from a Canic workspace root",
            MedicSource::AppConfig,
        );
    };
    let config_path = root
        .join("apps")
        .join(fleet.app.as_str())
        .join("canic.toml");
    if config_path.is_file() {
        MedicCheck::pass(
            MedicCategory::WorkspaceConfig,
            "app_config_found",
            "config",
            display_medic_path(root, &config_path),
            "none",
            MedicSource::AppConfig,
        )
    } else {
        MedicCheck::fail(
            MedicCategory::WorkspaceConfig,
            "app_config_missing",
            "config",
            format!("missing {}", display_medic_path(root, &config_path)),
            "restore the source App config",
            MedicSource::AppConfig,
        )
    }
}

fn check_fleet_registry_observation(
    icp_root: Option<&Path>,
    fleet: &FleetCatalogEntryV1,
    environment: &str,
    coordinator_present: bool,
) -> MedicCheck {
    if !coordinator_present {
        return check_fleet_registry_not_evaluated(coordinator_present);
    }

    let Some(root) = icp_root else {
        return MedicCheck::not_evaluated(
            MedicCategory::Topology,
            "fleet_registry_not_evaluated",
            "registry",
            "Fleet registry observation skipped because the workspace root was not resolved",
            "run from a Canic workspace root",
            MedicSource::InstalledFleet,
        );
    };

    let request = InstalledFleetRequest {
        fleet: fleet.fleet_name.to_string(),
        environment: environment.to_string(),
    };

    match resolve_installed_fleet_from_root(&request, root) {
        Ok(resolution) => fleet_registry_observed_check(&resolution),
        Err(err) => fleet_registry_error_check(err),
    }
}

pub(super) fn check_fleet_registry_not_evaluated(coordinator_present: bool) -> MedicCheck {
    let detail = if coordinator_present {
        "Fleet registry observation was not evaluated"
    } else {
        "Fleet Registry observation skipped because the Fleet catalog row has no Coordinator principal"
    };

    MedicCheck::not_evaluated(
        MedicCategory::Topology,
        "fleet_registry_not_evaluated",
        "registry",
        detail,
        "repair the blocking Fleet-state check, then rerun canic medic fleet <fleet>",
        MedicSource::InstalledFleet,
    )
}

fn fleet_registry_observed_check(resolution: &InstalledFleetResolution) -> MedicCheck {
    let entries = resolution.registry.entries.len();
    let roles = resolution.topology.roles_by_canister.len();
    let detail = format!(
        "root={}; entries={entries}; roles={roles}",
        resolution.registry.root_canister_id
    );
    let source = installed_fleet_source_for_medic(resolution.source);

    if entries == 0 {
        return MedicCheck::warn(
            MedicCategory::Topology,
            "fleet_registry_empty",
            "registry",
            detail,
            format!(
                "{}; then run canic deploy check {}",
                deploy_plan_next(
                    resolution.fleet.fleet_name.as_str(),
                    resolution.fleet.app.as_str(),
                ),
                resolution.fleet.fleet_name
            ),
            source,
        );
    }

    MedicCheck::pass(
        MedicCategory::Topology,
        "fleet_registry_observed",
        "registry",
        detail,
        runtime_inspection_next(resolution),
        source,
    )
}

fn deploy_plan_next(fleet: &str, app: &str) -> String {
    format!(
        "run canic deploy plan {fleet} --app {app} --fleet-input <path> to inspect desired Fleet shape"
    )
}

fn runtime_inspection_next(resolution: &InstalledFleetResolution) -> String {
    let fleet = &resolution.fleet.fleet_name;
    let mut roles = resolution
        .topology
        .roles_by_canister
        .values()
        .cloned()
        .collect::<Vec<_>>();
    roles.sort();
    roles.dedup();

    if let Some(role) = roles
        .iter()
        .find(|role| role.as_str() == "root")
        .or_else(|| roles.first())
    {
        return format!(
            "run canic inspect fleet {fleet} --role {role} to inspect runtime-observed status for one explicit role"
        );
    }

    let mut canisters = resolution
        .registry
        .entries
        .iter()
        .map(|entry| entry.pid.clone())
        .collect::<Vec<_>>();
    canisters.sort();
    canisters.dedup();

    canisters.first().map_or_else(
        || "none".to_string(),
        |canister| {
            format!(
                "run canic inspect canister {canister} to inspect runtime-observed status for one explicit canister"
            )
        },
    )
}

pub(super) fn deploy_plan_then(fleet: &str, next: impl AsRef<str>) -> String {
    format!("{}; {}", deploy_plan_next(fleet, "<app>"), next.as_ref())
}

const fn installed_fleet_source_for_medic(source: InstalledFleetSource) -> MedicSource {
    match source {
        InstalledFleetSource::LocalReplica => MedicSource::LocalReplica,
        InstalledFleetSource::IcpCli => MedicSource::IcpCli,
    }
}

fn fleet_registry_error_check(error: InstalledFleetError) -> MedicCheck {
    MedicCheck::fail(
        MedicCategory::Topology,
        "fleet_registry_unavailable",
        "registry",
        error.to_string(),
        "run canic status, then rerun canic medic fleet <fleet>",
        MedicSource::InstalledFleet,
    )
}

pub(super) fn check_coordinator_canister_id(fleet: &FleetCatalogEntryV1) -> MedicCheck {
    if fleet.coordinator_principal.trim().is_empty() {
        MedicCheck::fail(
            MedicCategory::Topology,
            "coordinator_canister_id_missing",
            "coordinator",
            "Fleet catalog row does not record a Coordinator principal",
            "reinstall the Fleet with canic install <app> <fleet> --fleet-input <path>",
            MedicSource::InstalledFleet,
        )
    } else {
        MedicCheck::pass(
            MedicCategory::Topology,
            "coordinator_canister_id_present",
            "coordinator",
            fleet.coordinator_principal.clone(),
            "none",
            MedicSource::InstalledFleet,
        )
    }
}

pub(super) fn check_coordinator_readiness_not_evaluated(coordinator_present: bool) -> MedicCheck {
    let detail = if coordinator_present {
        "Coordinator readiness was not evaluated"
    } else {
        "Coordinator readiness skipped because the Fleet catalog row has no Coordinator principal"
    };

    MedicCheck::not_evaluated(
        MedicCategory::Topology,
        "coordinator_readiness_not_evaluated",
        "coordinator",
        detail,
        "run canic info subnets <fleet> for live Coordinator/root evidence",
        MedicSource::InstalledFleet,
    )
}
