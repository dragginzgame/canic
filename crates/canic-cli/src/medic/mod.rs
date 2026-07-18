//! Module: canic_cli::medic
//!
//! Responsibility: diagnose Canic project and installed-deployment readiness.
//! Does not own: deployment mutation, recovery, install-state persistence, or
//! canister control-plane changes.
//! Boundary: reads local project/deployment state and renders diagnostic-only
//! medic reports.

mod auth;
mod blob_storage;
mod command;
mod deployment;
mod package;
mod project;
mod render;
mod report;
mod role_contract;
#[cfg(test)]
mod tests;

use std::path::Path;

use canic_core::role_contract::RoleContractFinding;
use canic_host::{
    icp::{IcpCli, IcpCommandError},
    icp_config::resolve_current_canic_icp_root,
    install_root::discover_project_canic_config_choices,
    installed_deployment::{InstalledDeploymentError, read_installed_deployment_state_from_root},
    state_manifest::{StateManifestResolution, resolve_project_state_manifest},
};

use auth::check_auth_renewal;
use blob_storage::{check_blob_storage_billing, check_blob_storage_not_selected};
use command::MedicOptions;
pub use command::{MedicCommandError, run};
use deployment::{
    DeploymentMedicContext, deploy_plan_then, deployment_medic_context,
    deployment_name_conflation_checks, installed_deployment_state_checks,
};
use project::{project_config_checks, state_audit_project_check};
use report::{MedicCategory, MedicCheck, MedicReport, MedicScope, MedicSource};

const ICP_SESSION_DETAIL: &str = "password-protected PEM identities can cache sessions";
const ICP_SESSION_NEXT: &str =
    "icp settings session-length 1h; icp identity reauth <name> --duration 1h";
const DEPLOYMENT_NOT_SELECTED_CHECK_CODE: &str = "deployment_not_selected";

fn build_medic_report(options: &MedicOptions) -> MedicReport {
    match options.scope {
        MedicScope::Project => MedicReport::new(options, run_project_checks(options)),
        MedicScope::Deployment => {
            let context = deployment_medic_context(options);
            let network = Some(context.network.clone());
            MedicReport::with_network(options, network, run_deployment_checks(options, &context))
        }
    }
}

fn run_project_checks(options: &MedicOptions) -> Vec<MedicCheck> {
    let mut checks = vec![
        check_icp_cli(options),
        check_icp_identity_session_cache_hint(),
    ];

    match resolve_current_canic_icp_root() {
        Ok(root) => {
            checks.push(MedicCheck::pass(
                MedicCategory::Environment,
                "project_root_resolved",
                "project_root",
                format!("resolved {}", root.display()),
                "none",
                MedicSource::Command,
            ));
            let state_resolution = match discover_project_canic_config_choices(&root) {
                Ok(configs) => resolve_project_state_manifest(&root, &configs, None),
                Err(error) => StateManifestResolution::Rejected {
                    errors: vec![RoleContractFinding::DependencyShapeUnsupported {
                        reason: error.to_string(),
                    }],
                },
            };
            checks.push(state_audit_project_check(&state_resolution));
            checks.extend(project_config_checks(&root, options));
        }
        Err(err) => {
            checks.push(MedicCheck::fail(
                MedicCategory::Environment,
                "project_root_missing",
                "project_root",
                err.to_string(),
                "run from a Canic project root",
                MedicSource::Command,
            ));
            checks.push(MedicCheck::not_evaluated(
                MedicCategory::Runtime,
                "state_audit_not_evaluated",
                "state_manifest",
                "state audit requires a resolved Canic project root",
                "run from a Canic project root, then run canic state audit",
                MedicSource::StateManifest,
            ));
        }
    }

    checks.push(MedicCheck::not_evaluated(
        MedicCategory::DeploymentState,
        DEPLOYMENT_NOT_SELECTED_CHECK_CODE,
        "deployment",
        "no deployment target was selected",
        "run canic medic deployment <deployment>",
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

fn run_deployment_checks(
    options: &MedicOptions,
    context: &DeploymentMedicContext,
) -> Vec<MedicCheck> {
    let mut checks = run_project_checks(options)
        .into_iter()
        .filter(|check| check.code != DEPLOYMENT_NOT_SELECTED_CHECK_CODE)
        .collect::<Vec<_>>();
    let network = &context.network;
    let icp_root = context.icp_root.as_deref();

    checks.push(context.network_check.clone());

    let state_result = match icp_root {
        Some(root) => {
            read_installed_deployment_state_from_root(network, options.deployment_name(), root)
                .map_err(Some)
        }
        None => Err(None),
    };
    let state = match state_result {
        Ok(state) => {
            checks.push(MedicCheck::pass(
                MedicCategory::DeploymentState,
                "deployment_target_found",
                "deployment",
                format!("{} installed", state.deployment_name),
                "run canic info list",
                MedicSource::InstalledDeployment,
            ));
            Some(state)
        }
        Err(Some(InstalledDeploymentError::NoInstalledDeployment { .. })) => {
            checks.push(MedicCheck::fail(
                MedicCategory::DeploymentState,
                "deployment_target_missing",
                "deployment",
                "no installed deployment found",
                deploy_plan_then(
                    options.deployment_name(),
                    "then run canic install <fleet-template> or canic deploy register <deployment> --fleet-template <fleet-template> --root <principal> --allow-unverified",
                ),
                MedicSource::InstalledDeployment,
            ));
            if let Some(root) = icp_root {
                checks.extend(deployment_name_conflation_checks(
                    root,
                    options.deployment_name(),
                ));
            }
            None
        }
        Err(err) => {
            let detail = err.map_or_else(
                || "could not resolve ICP project root".to_string(),
                |err| err.to_string(),
            );
            checks.push(MedicCheck::fail(
                MedicCategory::DeploymentState,
                "deployment_target_missing",
                "deployment",
                detail,
                deploy_plan_then(
                    options.deployment_name(),
                    "then reinstall from the owning fleet template or re-register the deployment target with --allow-unverified",
                ),
                MedicSource::InstalledDeployment,
            ));
            None
        }
    };

    if let Some(state) = state.as_ref() {
        checks.extend(installed_deployment_state_checks(
            options, icp_root, state, network,
        ));
    }

    if let Some(canister) = &options.blob_storage {
        checks.push(check_blob_storage_billing(options, canister, network));
    } else {
        checks.push(check_blob_storage_not_selected(options, icp_root, network));
    }

    if let Some(issuer) = &options.auth_renewal {
        checks.push(check_auth_renewal(options, issuer, network));
    } else {
        checks.push(MedicCheck::not_evaluated(
            MedicCategory::Auth,
            "auth_renewal_not_selected",
            "auth_renewal",
            "no auth-renewal issuer was selected",
            "run canic medic deployment <deployment> --auth-renewal <issuer-principal>",
            MedicSource::Command,
        ));
    }

    checks
}

fn check_icp_cli(options: &MedicOptions) -> MedicCheck {
    let network = options.network.clone();
    match IcpCli::new(&options.icp, network).compatible_version() {
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
