//! Module: canic_cli::medic::fleet
//!
//! Responsibility: construct installed-Fleet, registry, and root checks.
//! Does not own: deployment mutation, check ordering, or report rendering.
//! Boundary: maps local and runtime Fleet evidence into Medic checks.

use crate::{
    cli::defaults::local_environment,
    medic::{
        command::MedicOptions,
        display_medic_path,
        report::{MedicCategory, MedicCheck, MedicSource, MedicStatus},
    },
    support::candid::role_candid_path,
};
use std::path::{Path, PathBuf};

use canic_host::{
    canister_ready::query_canister_ready,
    fleet_catalog::FleetCatalogEntryV1,
    icp::IcpCli,
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
    options: &MedicOptions,
    icp_root: Option<&Path>,
    fleet: &FleetCatalogEntryV1,
    environment: &str,
) -> Vec<MedicCheck> {
    let root_canister = check_root_canister_id(fleet);
    let root_canister_present = root_canister.status != MedicStatus::Fail;
    let root_readiness = if root_canister_present {
        check_root_ready(options, icp_root, fleet, environment)
    } else {
        check_root_readiness_not_evaluated(root_canister_present)
    };

    vec![
        check_config_path(icp_root, fleet),
        root_canister,
        check_fleet_registry_observation(
            options,
            icp_root,
            fleet,
            environment,
            root_canister_present,
        ),
        root_readiness,
    ]
}

fn check_config_path(icp_root: Option<&Path>, fleet: &FleetCatalogEntryV1) -> MedicCheck {
    let Some(root) = icp_root else {
        return MedicCheck::not_evaluated(
            MedicCategory::ProjectConfig,
            "app_config_not_evaluated",
            "config",
            "App config lookup skipped because the project root was not resolved",
            "run from a Canic project root",
            MedicSource::AppConfig,
        );
    };
    let config_path = root
        .join("apps")
        .join(fleet.app.as_str())
        .join("canic.toml");
    if config_path.is_file() {
        MedicCheck::pass(
            MedicCategory::ProjectConfig,
            "app_config_found",
            "config",
            display_medic_path(root, &config_path),
            "none",
            MedicSource::AppConfig,
        )
    } else {
        MedicCheck::fail(
            MedicCategory::ProjectConfig,
            "app_config_missing",
            "config",
            format!("missing {}", display_medic_path(root, &config_path)),
            "restore the source App config",
            MedicSource::AppConfig,
        )
    }
}

fn check_fleet_registry_observation(
    options: &MedicOptions,
    icp_root: Option<&Path>,
    fleet: &FleetCatalogEntryV1,
    environment: &str,
    root_canister_present: bool,
) -> MedicCheck {
    if !root_canister_present {
        return check_fleet_registry_not_evaluated(root_canister_present);
    }

    let Some(root) = icp_root else {
        return MedicCheck::not_evaluated(
            MedicCategory::Topology,
            "fleet_registry_not_evaluated",
            "registry",
            "Fleet registry observation skipped because the project root was not resolved",
            "run from a Canic project root",
            MedicSource::InstalledFleet,
        );
    };

    let request = InstalledFleetRequest {
        fleet: fleet.fleet_name.to_string(),
        environment: environment.to_string(),
        icp: options.icp.clone(),
        detect_lost_local_root: true,
    };

    match resolve_installed_fleet_from_root(&request, root) {
        Ok(resolution) => fleet_registry_observed_check(&resolution),
        Err(err) => fleet_registry_error_check(err),
    }
}

pub(super) fn check_fleet_registry_not_evaluated(root_canister_present: bool) -> MedicCheck {
    let detail = if root_canister_present {
        "Fleet registry observation was not evaluated"
    } else {
        "Fleet registry observation skipped because the Fleet catalog row has no root principal"
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

pub(super) fn fleet_registry_observed_check(resolution: &InstalledFleetResolution) -> MedicCheck {
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
    format!("run canic deploy plan {fleet} --app {app} to inspect desired Fleet shape")
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
    let source = match error {
        InstalledFleetError::ReplicaQuery(_) | InstalledFleetError::LostLocalFleet { .. } => {
            MedicSource::LocalReplica
        }
        InstalledFleetError::Icp(_) => MedicSource::IcpCli,
        InstalledFleetError::NoInstalledFleet { .. }
        | InstalledFleetError::FleetCatalog(_)
        | InstalledFleetError::Registry(_)
        | InstalledFleetError::Io(_) => MedicSource::InstalledFleet,
    };

    MedicCheck::fail(
        MedicCategory::Topology,
        "fleet_registry_unavailable",
        "registry",
        error.to_string(),
        "run canic status, then rerun canic medic fleet <fleet>",
        source,
    )
}

pub(super) fn check_root_canister_id(fleet: &FleetCatalogEntryV1) -> MedicCheck {
    if fleet.root_principal.trim().is_empty() {
        MedicCheck::fail(
            MedicCategory::Topology,
            "root_canister_id_missing",
            "root",
            "Fleet catalog row does not record a root principal",
            "reinstall the Fleet with canic install <app> <fleet> --fleet-input <path>",
            MedicSource::InstalledFleet,
        )
    } else {
        MedicCheck::pass(
            MedicCategory::Topology,
            "root_canister_id_present",
            "root",
            fleet.root_principal.clone(),
            "none",
            MedicSource::InstalledFleet,
        )
    }
}

pub(super) fn check_root_readiness_not_evaluated(root_canister_present: bool) -> MedicCheck {
    let detail = if root_canister_present {
        "root readiness was not evaluated"
    } else {
        "root readiness skipped because the Fleet catalog row has no root principal"
    };

    MedicCheck::not_evaluated(
        MedicCategory::Topology,
        "root_readiness_not_evaluated",
        "root",
        detail,
        "repair the blocking Fleet-state check, then rerun canic medic fleet <fleet>",
        MedicSource::InstalledFleet,
    )
}

fn check_root_ready(
    options: &MedicOptions,
    icp_root: Option<&Path>,
    fleet: &FleetCatalogEntryV1,
    environment: &str,
) -> MedicCheck {
    let source = root_readiness_source(environment);
    let mut icp = IcpCli::new(&options.icp, Some(environment.to_string()));
    if let Some(root) = icp_root {
        icp = icp.with_cwd(root);
    }
    let candid_path = role_candid_path(icp_root, environment, "root");
    let ready = query_canister_ready(
        &icp,
        &fleet.root_principal,
        environment,
        icp_root,
        candid_path.as_deref(),
    )
    .map_err(|err| err.to_string());

    match ready {
        Ok(true) => MedicCheck::pass(
            MedicCategory::Topology,
            "root_readiness_pass",
            "root",
            "canic_ready=true",
            "none",
            source,
        ),
        Ok(false) => MedicCheck::warn(
            MedicCategory::Topology,
            "root_readiness_fail",
            "root",
            "canic_ready=false",
            "wait briefly, then run canic medic fleet <fleet>",
            source,
        ),
        Err(err) => MedicCheck::fail(
            MedicCategory::Topology,
            "root_readiness_fail",
            "root",
            err,
            "run canic install",
            source,
        ),
    }
}

pub(super) fn root_readiness_source(environment: &str) -> MedicSource {
    if environment == local_environment() {
        MedicSource::LocalReplica
    } else {
        MedicSource::IcpCli
    }
}
