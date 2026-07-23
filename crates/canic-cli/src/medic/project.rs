//! Module: canic_cli::medic::project
//!
//! Responsibility: construct project configuration and state-audit Medic checks.
//! Does not own: project check ordering, role-contract policy, or report rendering.
//! Boundary: maps local project metadata and state-manifest results into Medic checks.

use crate::medic::{
    command::MedicOptions,
    report::{MedicCategory, MedicCheck, MedicScope, MedicSource},
    role_contract::project_config_quality_checks,
};
use std::path::Path;

use canic_host::{
    icp_config::inspect_canic_icp_yaml_from_root,
    install_root::discover_project_canic_config_choices,
    state_manifest::{StateAuditStatus, StateManifestResolution, build_state_audit_report},
};

pub(super) fn state_audit_project_check(resolution: &StateManifestResolution) -> MedicCheck {
    let report = build_state_audit_report(resolution, None);
    let detail = format!(
        "state audit status {} with {} check(s)",
        report.status.label(),
        report.checks.len()
    );

    match report.status {
        StateAuditStatus::Pass => MedicCheck::pass(
            MedicCategory::Runtime,
            "state_audit_pass",
            "state_manifest",
            detail,
            "none",
            MedicSource::StateManifest,
        ),
        StateAuditStatus::Warn => MedicCheck::warn(
            MedicCategory::Runtime,
            "state_audit_warn",
            "state_manifest",
            detail,
            "run canic state audit",
            MedicSource::StateManifest,
        ),
        StateAuditStatus::Fail => MedicCheck::fail(
            MedicCategory::Runtime,
            "state_audit_fail",
            "state_manifest",
            detail,
            "run canic state audit and fix failing state metadata checks",
            MedicSource::StateManifest,
        ),
        StateAuditStatus::NotEvaluated => MedicCheck::not_evaluated(
            MedicCategory::Runtime,
            "state_audit_not_evaluated",
            "state_manifest",
            detail,
            "declare state metadata, then run canic state audit",
            MedicSource::StateManifest,
        ),
    }
}

pub(super) fn project_config_checks(root: &Path, options: &MedicOptions) -> Vec<MedicCheck> {
    let mut checks = Vec::new();
    match discover_project_canic_config_choices(root) {
        Ok(configs) if configs.is_empty() => checks.push(MedicCheck::fail(
            MedicCategory::ProjectConfig,
            "app_config_missing",
            "apps",
            "no Canic App configs found",
            "create apps/<app>/canic.toml or run canic app create <app>",
            MedicSource::AppConfig,
        )),
        Ok(configs) => {
            checks.push(MedicCheck::pass(
                MedicCategory::ProjectConfig,
                "app_config_discovered",
                "apps",
                format!("found {} Canic App config(s)", configs.len()),
                "none",
                MedicSource::AppConfig,
            ));
            checks.extend(project_config_quality_checks(root, &configs));
        }
        Err(err) => checks.push(MedicCheck::fail(
            MedicCategory::ProjectConfig,
            "app_config_missing",
            "apps",
            err.to_string(),
            "repair Canic App config discovery",
            MedicSource::AppConfig,
        )),
    }

    match inspect_canic_icp_yaml_from_root(root, None) {
        Ok(report) if report.icp_yaml_present => checks.push(MedicCheck::pass(
            MedicCategory::ProjectConfig,
            "icp_yaml_present",
            "icp.yaml",
            format!("found {}", report.path.display()),
            "none",
            MedicSource::IcpConfig,
        )),
        Ok(report) => checks.push(MedicCheck::fail(
            MedicCategory::ProjectConfig,
            "icp_yaml_missing",
            "icp.yaml",
            format!("missing {}", report.path.display()),
            "create or repair icp.yaml from the project root",
            MedicSource::IcpConfig,
        )),
        Err(err) => checks.push(MedicCheck::fail(
            MedicCategory::ProjectConfig,
            "icp_yaml_missing",
            "icp.yaml",
            err.to_string(),
            "create or repair icp.yaml from the project root",
            MedicSource::IcpConfig,
        )),
    }

    if let Some(environment) = project_environment_selection_check(options) {
        checks.push(environment);
    }

    checks
}

pub(super) fn project_environment_selection_check(options: &MedicOptions) -> Option<MedicCheck> {
    if options.scope != MedicScope::Project {
        return None;
    }

    Some(if options.environment.is_some() {
        MedicCheck::pass(
            MedicCategory::ProjectConfig,
            "local_environment_explicit",
            "environment",
            "environment selected explicitly",
            "none",
            MedicSource::IcpConfig,
        )
    } else {
        MedicCheck::warn(
            MedicCategory::ProjectConfig,
            "local_environment_implicit",
            "environment",
            "no environment was selected for project-level checks",
            "select an explicit environment before deployment checks",
            MedicSource::IcpConfig,
        )
    })
}
