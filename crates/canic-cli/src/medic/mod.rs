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

use std::{fmt::Write as _, path::Path};

use canic_core::{ids::AppId, role_contract::RoleContractFinding};
use canic_host::{
    icp::{IcpCli, IcpCommandError},
    icp_config::resolve_current_canic_icp_root,
    install_root::{
        FreshFleetInstallRecoveryPlanV1, InspectFreshFleetInstallRecoveryRequest,
        RetainedFleetInstallSessionSummaryV1, discover_workspace_canic_config_choices,
        inspect_fresh_fleet_install_recovery, inspect_incomplete_fleet_install_session,
    },
    installed_fleet::{InstalledFleetError, read_installed_fleet_from_root},
    network::resolve_canonical_network_id_from_root,
    release_set::AppConfigSnapshot,
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
    let recovery = context.icp_root.as_deref().map(|root| {
        inspect_retained_fleet_recovery(root, &context.environment, options.fleet_name())
    });
    let mut checks = run_workspace_checks(options)
        .into_iter()
        .filter(|check| check.code != FLEET_NOT_SELECTED_CHECK_CODE)
        .collect::<Vec<_>>();
    if matches!(recovery, Some(RetainedFleetRecoveryInspection::Found(_))) {
        defer_workspace_checks_for_recovery(&mut checks);
    }
    let environment = &context.environment;
    let icp_root = context.icp_root.as_deref();

    checks.push(context.environment_check.clone());

    let state_result = match (&recovery, icp_root) {
        (Some(RetainedFleetRecoveryInspection::Found(recovery)), _) => {
            checks.push(retained_recovery_check(
                options.fleet_name(),
                &recovery.summary,
                recovery.plan.as_ref(),
                recovery.plan_error.as_deref(),
            ));
            return finish_optional_fleet_checks(options, icp_root, environment, checks);
        }
        (Some(RetainedFleetRecoveryInspection::Invalid { detail }), _) => {
            checks.push(MedicCheck::fail(
                MedicCategory::FleetState,
                "fleet_recovery_invalid",
                "fleet_recovery",
                detail,
                "preserve the retained session and inspect its exact plan, release-build, and journal evidence; do not start a fresh or replacement Fleet",
                MedicSource::InstalledFleet,
            ));
            return finish_optional_fleet_checks(options, icp_root, environment, checks);
        }
        (_, Some(root)) => {
            read_installed_fleet_from_root(environment, options.fleet_name(), root).map_err(Some)
        }
        (_, None) => Err(None),
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

    finish_optional_fleet_checks(options, icp_root, environment, checks)
}

fn defer_workspace_checks_for_recovery(checks: &mut Vec<MedicCheck>) {
    checks.retain(|check| check.category == MedicCategory::Environment);
    checks.push(MedicCheck::not_evaluated(
        MedicCategory::WorkspaceConfig,
        "workspace_role_contract_deferred_for_recovery",
        "workspace_role_contract",
        "an authoritative retained installation session owns release and artifact selection",
        "finish the exact retained session before evaluating the current workspace as a fresh install",
        MedicSource::InstalledFleet,
    ));
}

fn finish_optional_fleet_checks(
    options: &MedicOptions,
    icp_root: Option<&Path>,
    environment: &str,
    mut checks: Vec<MedicCheck>,
) -> Vec<MedicCheck> {
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

enum RetainedFleetRecoveryInspection {
    Found(Box<RetainedFleetRecoveryEvidence>),
    Invalid { detail: String },
    None,
}

struct RetainedFleetRecoveryEvidence {
    summary: RetainedFleetInstallSessionSummaryV1,
    plan: Option<FreshFleetInstallRecoveryPlanV1>,
    plan_error: Option<String>,
}

fn inspect_retained_fleet_recovery(
    root: &Path,
    environment: &str,
    fleet_name: &str,
) -> RetainedFleetRecoveryInspection {
    if !has_retained_fleet_install_sessions(root) {
        return RetainedFleetRecoveryInspection::None;
    }
    let canonical_network_id = match resolve_canonical_network_id_from_root(root, environment) {
        Ok(network) => network,
        Err(error) => {
            return RetainedFleetRecoveryInspection::Invalid {
                detail: format!("could not resolve retained Fleet network identity: {error}"),
            };
        }
    };
    let fleet_name = match fleet_name.parse() {
        Ok(fleet) => fleet,
        Err(error) => {
            return RetainedFleetRecoveryInspection::Invalid {
                detail: format!("invalid Fleet name for recovery inspection: {error}"),
            };
        }
    };
    let summary =
        match inspect_incomplete_fleet_install_session(root, canonical_network_id, &fleet_name) {
            Ok(Some(summary)) => summary,
            Ok(None) => return RetainedFleetRecoveryInspection::None,
            Err(error) => {
                return RetainedFleetRecoveryInspection::Invalid {
                    detail: format!("could not read retained Fleet session authority: {error}"),
                };
            }
        };
    let choices = match discover_workspace_canic_config_choices(root) {
        Ok(choices) => choices,
        Err(error) => {
            return RetainedFleetRecoveryInspection::Found(Box::new(
                RetainedFleetRecoveryEvidence {
                    summary,
                    plan: None,
                    plan_error: Some(format!(
                        "current workspace App discovery could not enrich the retained session: {error}"
                    )),
                },
            ));
        }
    };
    let mut matching_snapshots = Vec::new();
    for path in choices {
        let Ok(snapshot) = AppConfigSnapshot::load(&path) else {
            continue;
        };
        if AppId::from(snapshot.app_id()) == summary.app {
            matching_snapshots.push(snapshot);
        }
    }
    if matching_snapshots.len() != 1 {
        return RetainedFleetRecoveryInspection::Found(Box::new(RetainedFleetRecoveryEvidence {
            summary,
            plan: None,
            plan_error: Some(format!(
                "retained App authority matched {} current workspace configurations",
                matching_snapshots.len()
            )),
        }));
    }
    let snapshot = matching_snapshots
        .pop()
        .expect("exactly one matching App configuration");
    match inspect_fresh_fleet_install_recovery(InspectFreshFleetInstallRecoveryRequest {
        root,
        canonical_network_id,
        fleet_name: &fleet_name,
        app: &summary.app,
        config: snapshot.model(),
    }) {
        Ok(Some(plan)) => {
            RetainedFleetRecoveryInspection::Found(Box::new(RetainedFleetRecoveryEvidence {
                summary,
                plan: Some(plan),
                plan_error: None,
            }))
        }
        Ok(None) => {
            RetainedFleetRecoveryInspection::Found(Box::new(RetainedFleetRecoveryEvidence {
                summary,
                plan: None,
                plan_error: Some(
                    "retained session exists but its detailed recovery plan was unavailable"
                        .to_string(),
                ),
            }))
        }
        Err(error) => {
            RetainedFleetRecoveryInspection::Found(Box::new(RetainedFleetRecoveryEvidence {
                summary,
                plan: None,
                plan_error: Some(format!(
                    "retained session is authoritative, but current-source plan enrichment failed: {error}"
                )),
            }))
        }
    }
}

fn has_retained_fleet_install_sessions(root: &Path) -> bool {
    root.join(".canic")
        .join("recovery")
        .join("fleet-install-sessions")
        .is_dir()
}

fn operation_id_text(operation_id: [u8; 32]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in operation_id {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

fn retained_recovery_check(
    fleet: &str,
    summary: &RetainedFleetInstallSessionSummaryV1,
    plan: Option<&FreshFleetInstallRecoveryPlanV1>,
    plan_error: Option<&str>,
) -> MedicCheck {
    let detail = match plan {
        Some(plan) => format!(
            "app={} operation={} classification={} retained_builder={} current_builder={} release_build={} plan_digest={} fenced_creations={}/{} next_replay_phase={} remaining_debit={:?}",
            summary.app,
            operation_id_text(summary.operation_id),
            plan.classification.as_str(),
            plan.retained_builder_version,
            env!("CARGO_PKG_VERSION"),
            plan.release_build_id,
            plan.fresh_fleet_plan_digest,
            plan.fenced_operator_creations,
            plan.total_operator_creations,
            plan.next_replay_phase,
            plan.remaining_operator_debit,
        ),
        None => format!(
            "app={} operation={} release_build={} plan_digest={} detailed_plan_unavailable={}",
            summary.app,
            operation_id_text(summary.operation_id),
            summary.release_build_id,
            summary.fresh_fleet_plan_digest,
            plan_error.unwrap_or("unknown"),
        ),
    };
    let next = format!(
        "resume only this retained session with canic install {} {fleet} --fleet-input <original-path> --expected-plan-digest {} --release-build {}; do not start a fresh or replacement Fleet",
        summary.app, summary.fresh_fleet_plan_digest, summary.release_build_id,
    );
    MedicCheck::warn(
        MedicCategory::FleetState,
        "fleet_recovery_pending",
        "fleet_recovery",
        detail,
        next,
        MedicSource::InstalledFleet,
    )
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
