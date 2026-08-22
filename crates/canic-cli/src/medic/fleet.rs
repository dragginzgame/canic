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
        InstalledFleetError, InstalledFleetFundingResolution, InstalledFleetRequest,
        resolve_installed_fleet_funding_from_root,
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
        check_funding_profile_observation(icp_root, fleet, environment, coordinator_present),
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

    match resolve_installed_fleet_funding_from_root(&request, root) {
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

fn fleet_registry_observed_check(resolution: &InstalledFleetFundingResolution) -> MedicCheck {
    let current_roots = resolution
        .roots
        .iter()
        .filter(|root| {
            root.status != canic_core::dto::fleet_registry::FleetSubnetRootStatus::Removed
        })
        .count();
    let detail = format!(
        "coordinator={}; roots={}; current_roots={current_roots}",
        resolution.coordinator_canister_id,
        resolution.roots.len(),
    );

    if current_roots == 0 {
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
            MedicSource::InstalledFleet,
        );
    }

    MedicCheck::pass(
        MedicCategory::Topology,
        "fleet_registry_observed",
        "registry",
        detail,
        format!(
            "run canic cycles topup {} coordinator <amount> for Coordinator recovery",
            resolution.fleet.fleet_name
        ),
        MedicSource::InstalledFleet,
    )
}

fn check_funding_profile_observation(
    icp_root: Option<&Path>,
    fleet: &FleetCatalogEntryV1,
    environment: &str,
    coordinator_present: bool,
) -> MedicCheck {
    if !coordinator_present {
        return MedicCheck::not_evaluated(
            MedicCategory::Funding,
            "funding_profile_not_evaluated",
            "funding",
            "funding authority was not evaluated because the Coordinator principal is missing",
            "repair the installed Fleet catalog authority",
            MedicSource::InstalledFleet,
        );
    }
    let Some(root) = icp_root else {
        return MedicCheck::not_evaluated(
            MedicCategory::Funding,
            "funding_profile_not_evaluated",
            "funding",
            "funding authority lookup skipped because the workspace root was not resolved",
            "run from a Canic workspace root",
            MedicSource::InstalledFleet,
        );
    };
    let request = InstalledFleetRequest {
        fleet: fleet.fleet_name.to_string(),
        environment: environment.to_string(),
    };
    match resolve_installed_fleet_funding_from_root(&request, root) {
        Ok(resolution) => funding_profile_check(&resolution),
        Err(error) => MedicCheck::fail(
            MedicCategory::Funding,
            "funding_profile_invalid",
            "funding",
            error.to_string(),
            "restore the digest-bound Fleet plan and verified activation journal",
            MedicSource::InstalledFleet,
        ),
    }
}

fn funding_profile_check(resolution: &InstalledFleetFundingResolution) -> MedicCheck {
    let Some(policy) = resolution.coordinator_root_funding.as_ref() else {
        return MedicCheck::fail(
            MedicCategory::Funding,
            "funding_policy_missing",
            "funding",
            "installed Fleet authority has no Coordinator Root-funding policy",
            "reinstall from an exact protected 0.108 Fleet input",
            MedicSource::InstalledFleet,
        );
    };
    if resolution
        .roots
        .iter()
        .any(|root| root.funding.root_funding.funding_profile != policy.funding_profile)
    {
        return MedicCheck::fail(
            MedicCategory::Funding,
            "funding_profile_mismatch",
            "funding",
            "Coordinator and Root funding profiles disagree",
            "do not deploy; rebuild the fresh Fleet plan from protected input",
            MedicSource::InstalledFleet,
        );
    }
    let automatic_icp_roots = resolution
        .roots
        .iter()
        .filter(|root| {
            root.funding
                .icp_refill
                .as_ref()
                .is_some_and(|refill| refill.automatic.is_some())
        })
        .count();
    let detail = format!(
        "profile={}; roots={}; automatic_icp_roots={automatic_icp_roots}; fleet_auto_grants={}; fleet_auto_cycles={}",
        funding_profile_label(policy.funding_profile),
        resolution.roots.len(),
        policy.maximum_automatic_grants,
        policy.maximum_automatic_cycles,
    );
    let mut fiduciary_warnings = resolution
        .coordinator_placement_cost
        .warning
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    fiduciary_warnings.extend(
        resolution
            .roots
            .iter()
            .filter_map(|root| root.placement_cost.warning.clone()),
    );
    if fiduciary_warnings.is_empty() {
        MedicCheck::pass(
            MedicCategory::Funding,
            "funding_profile_verified",
            "funding",
            detail,
            format!(
                "use canic cycles topup {} coordinator <amount> or an explicit current Root Principal for break-glass recovery",
                resolution.fleet.fleet_name
            ),
            MedicSource::InstalledFleet,
        )
    } else {
        MedicCheck::warn(
            MedicCategory::Funding,
            "fiduciary_funding_cost_acknowledged",
            "funding",
            format!("{detail}; {}", fiduciary_warnings.join("; ")),
            "retain explicit Fiduciary cost acknowledgement and monitor treasury headroom",
            MedicSource::InstalledFleet,
        )
    }
}

const fn funding_profile_label(profile: canic_core::ids::FleetFundingProfile) -> &'static str {
    match profile {
        canic_core::ids::FleetFundingProfile::SingleSubnet => "single_subnet",
        canic_core::ids::FleetFundingProfile::PreviewMultiSubnet => "preview_multi_subnet",
        canic_core::ids::FleetFundingProfile::MultiSubnet => "multi_subnet",
    }
}

fn deploy_plan_next(fleet: &str, app: &str) -> String {
    format!(
        "run canic deploy plan {fleet} --app {app} --fleet-input <path> to inspect desired Fleet shape"
    )
}

pub(super) fn deploy_plan_then(fleet: &str, next: impl AsRef<str>) -> String {
    format!("{}; {}", deploy_plan_next(fleet, "<app>"), next.as_ref())
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
