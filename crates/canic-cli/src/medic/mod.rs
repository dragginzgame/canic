//! Module: canic_cli::medic
//!
//! Responsibility: diagnose local workspace and installed-Fleet readiness.
//! Does not own: deployment mutation, recovery, Fleet catalog persistence, or
//! canister control-plane changes.
//! Boundary: reads local workspace/Fleet state and renders diagnostic-only
//! medic reports.

mod admission;
mod auth;
mod blob_storage;
mod command;
mod fleet;
mod package;
mod render;
mod report;
mod role_contract;
#[cfg(test)]
mod tests;
mod workspace;

use std::path::Path;

use canic_core::role_contract::RoleContractFinding;
use canic_host::{
    icp::{IcpCli, IcpCommandError},
    icp_config::resolve_current_canic_icp_root,
    install_root::discover_workspace_canic_config_choices,
    installed_fleet::{InstalledFleetError, read_installed_fleet_from_root},
    state_manifest::{StateManifestResolution, resolve_workspace_state_manifest},
};

use admission::check_fleet_admission;
use auth::check_auth_renewal;
use blob_storage::{check_blob_storage_billing, check_blob_storage_not_selected};
use command::MedicOptions;
pub use command::{MedicCommandError, run};
use fleet::{FleetMedicContext, deploy_plan_then, fleet_medic_context, installed_fleet_checks};
use report::{MedicCategory, MedicCheck, MedicReport, MedicScope, MedicSource};
use workspace::{state_audit_workspace_check, workspace_config_checks};

const ICP_SESSION_DETAIL: &str = "password-protected PEM identities can cache sessions";
const ICP_SESSION_NEXT: &str =
    "icp settings session-length 1h; icp identity reauth <name> --duration 1h";
const FLEET_NOT_SELECTED_CHECK_CODE: &str = "fleet_not_selected";

fn build_medic_report(options: &MedicOptions) -> MedicReport {
    match options.scope {
        MedicScope::Fleet => {
            let context = fleet_medic_context(options);
            let environment = Some(context.environment.clone());
            MedicReport::with_environment(options, environment, run_fleet_checks(options, &context))
        }
        MedicScope::Workspace => MedicReport::new(options, run_workspace_checks(options)),
    }
}

fn run_workspace_checks(options: &MedicOptions) -> Vec<MedicCheck> {
    let mut checks = vec![
        check_icp_cli(options),
        check_icp_identity_session_cache_hint(),
    ];

    match resolve_current_canic_icp_root() {
        Ok(root) => {
            checks.push(MedicCheck::pass(
                MedicCategory::Environment,
                "workspace_root_resolved",
                "workspace_root",
                format!("resolved {}", root.display()),
                "none",
                MedicSource::Command,
            ));
            let state_resolution = match discover_workspace_canic_config_choices(&root) {
                Ok(configs) => resolve_workspace_state_manifest(&root, &configs, None),
                Err(error) => StateManifestResolution::Rejected {
                    errors: vec![RoleContractFinding::DependencyShapeUnsupported {
                        reason: error.to_string(),
                    }],
                },
            };
            checks.push(state_audit_workspace_check(&state_resolution));
            checks.extend(workspace_config_checks(&root, options));
        }
        Err(err) => {
            checks.push(MedicCheck::fail(
                MedicCategory::Environment,
                "workspace_root_missing",
                "workspace_root",
                err.to_string(),
                "run from a Canic workspace root",
                MedicSource::Command,
            ));
            checks.push(MedicCheck::not_evaluated(
                MedicCategory::Runtime,
                "state_audit_not_evaluated",
                "state_manifest",
                "state audit requires a resolved Canic workspace root",
                "run from a Canic workspace root, then run canic state audit",
                MedicSource::StateManifest,
            ));
        }
    }

    checks.push(MedicCheck::not_evaluated(
        MedicCategory::FleetState,
        FLEET_NOT_SELECTED_CHECK_CODE,
        "fleet",
        "no Fleet was selected",
        "run canic medic fleet <fleet>",
        MedicSource::Command,
    ));
    checks
}

fn display_medic_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn run_fleet_checks(options: &MedicOptions, context: &FleetMedicContext) -> Vec<MedicCheck> {
    let mut checks = run_workspace_checks(options)
        .into_iter()
        .filter(|check| check.code != FLEET_NOT_SELECTED_CHECK_CODE)
        .collect::<Vec<_>>();
    let environment = &context.environment;
    let icp_root = context.icp_root.as_deref();

    checks.push(context.environment_check.clone());

    let state_result = match icp_root {
        Some(root) => {
            read_installed_fleet_from_root(environment, options.fleet_name(), root).map_err(Some)
        }
        None => Err(None),
    };
    let state = match state_result {
        Ok(state) => {
            checks.push(MedicCheck::pass(
                MedicCategory::FleetState,
                "fleet_found",
                "fleet",
                format!("{} installed", state.fleet_name),
                "run canic info list",
                MedicSource::InstalledFleet,
            ));
            Some(state)
        }
        Err(Some(InstalledFleetError::NoInstalledFleet { .. })) => {
            checks.push(MedicCheck::fail(
                MedicCategory::FleetState,
                "fleet_missing",
                "fleet",
                "no installed Fleet found",
                deploy_plan_then(
                    options.fleet_name(),
                    "then run canic install <app> <fleet> --fleet-input <path>",
                ),
                MedicSource::InstalledFleet,
            ));
            None
        }
        Err(err) => {
            let detail = err.map_or_else(
                || "could not resolve ICP project root".to_string(),
                |err| err.to_string(),
            );
            checks.push(MedicCheck::fail(
                MedicCategory::FleetState,
                "fleet_missing",
                "fleet",
                detail,
                deploy_plan_then(
                    options.fleet_name(),
                    "then reinstall the Fleet with canic install <app> <fleet> --fleet-input <path>",
                ),
                MedicSource::InstalledFleet,
            ));
            None
        }
    };

    if let Some(state) = state.as_ref() {
        checks.extend(installed_fleet_checks(icp_root, state, environment));
        checks.push(check_fleet_admission(options, context));
    }

    if let Some(canister) = &options.blob_storage {
        checks.push(check_blob_storage_billing(options, canister, environment));
    } else {
        checks.push(check_blob_storage_not_selected(
            options,
            icp_root,
            environment,
        ));
    }

    if let Some(issuer) = &options.auth_renewal {
        checks.push(check_auth_renewal(options, issuer, environment));
    } else {
        checks.push(MedicCheck::not_evaluated(
            MedicCategory::Auth,
            "auth_renewal_not_selected",
            "auth_renewal",
            "no auth-renewal issuer was selected",
            "run canic medic fleet <fleet> --auth-renewal <issuer-principal>",
            MedicSource::Command,
        ));
    }

    checks
}

fn check_icp_cli(options: &MedicOptions) -> MedicCheck {
    let environment = options.environment.clone();
    match IcpCli::new(&options.icp, environment).compatible_version() {
        Ok(version) => MedicCheck::pass(
            MedicCategory::Environment,
            "icp_cli_ok",
            "icp",
            version,
            "none",
            MedicSource::IcpCli,
        ),
        Err(err) => icp_cli_error_check(err),
    }
}

fn icp_cli_error_check(error: IcpCommandError) -> MedicCheck {
    let code = match error {
        IcpCommandError::MissingCli { .. } => "icp_cli_missing",
        IcpCommandError::IncompatibleCliVersion { .. }
        | IcpCommandError::Io(_)
        | IcpCommandError::Failed { .. }
        | IcpCommandError::Json { .. } => "icp_cli_incompatible",
    };

    MedicCheck::fail(
        MedicCategory::Environment,
        code,
        "icp",
        error.to_string(),
        "install supported icp-cli or pass top-level --icp <path>",
        MedicSource::IcpCli,
    )
}

fn check_icp_identity_session_cache_hint() -> MedicCheck {
    MedicCheck::pass(
        MedicCategory::Environment,
        "icp_identity_session_hint",
        "icp_identity",
        ICP_SESSION_DETAIL,
        ICP_SESSION_NEXT,
        MedicSource::IcpCli,
    )
}
