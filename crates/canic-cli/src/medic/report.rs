//! Module: canic_cli::medic::report
//!
//! Responsibility: own the stable medic report model, ordering, and aggregate status.
//! Does not own: diagnostic collection, command parsing, or text/JSON rendering.
//! Boundary: workspace and Fleet checks construct this private CLI report model.

use super::MedicOptions;
use serde::Serialize;

const SCHEMA_VERSION: u8 = 1;

///
/// MedicReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct MedicReport {
    pub(super) schema_version: u8,
    pub(super) command: String,
    pub(super) scope: MedicScope,
    pub(super) environment: Option<String>,
    pub(super) fleet: Option<String>,
    pub(super) status: MedicStatus,
    pub(super) checks: Vec<MedicCheck>,
}

impl MedicReport {
    pub(super) fn new(options: &MedicOptions, checks: Vec<MedicCheck>) -> Self {
        let environment = match options.scope {
            MedicScope::Workspace => options.environment.clone(),
            MedicScope::Fleet => Some(options.fleet_environment()),
        };
        Self::with_environment(options, environment, checks)
    }

    pub(super) fn with_environment(
        options: &MedicOptions,
        environment: Option<String>,
        checks: Vec<MedicCheck>,
    ) -> Self {
        let status = aggregate_status(&checks);
        Self {
            schema_version: SCHEMA_VERSION,
            command: options.command_label(),
            scope: options.scope,
            environment,
            fleet: options.fleet.clone(),
            status,
            checks: ordered_checks(&checks).into_iter().cloned().collect(),
        }
    }
}

pub(super) fn aggregate_status(checks: &[MedicCheck]) -> MedicStatus {
    if checks.is_empty()
        || checks
            .iter()
            .all(|check| check.status == MedicStatus::NotEvaluated)
    {
        return MedicStatus::NotEvaluated;
    }
    if checks.iter().any(|check| check.status == MedicStatus::Fail) {
        return MedicStatus::Fail;
    }
    if checks.iter().any(|check| check.status == MedicStatus::Warn) {
        return MedicStatus::Warn;
    }
    MedicStatus::Pass
}

pub(super) fn ordered_checks(checks: &[MedicCheck]) -> Vec<&MedicCheck> {
    let mut checks = checks.iter().collect::<Vec<_>>();
    checks.sort_by_key(|check| check.category.order());
    checks
}

///
/// MedicCheck
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct MedicCheck {
    pub(super) category: MedicCategory,
    pub(super) code: String,
    pub(super) status: MedicStatus,
    pub(super) subject: String,
    pub(super) detail: String,
    pub(super) next: String,
    pub(super) source: MedicSource,
}

impl MedicCheck {
    pub(super) fn pass(
        category: MedicCategory,
        code: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
        next: impl Into<String>,
        source: MedicSource,
    ) -> Self {
        Self::new(
            category,
            code,
            MedicStatus::Pass,
            subject,
            detail,
            next,
            source,
        )
    }

    pub(super) fn warn(
        category: MedicCategory,
        code: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
        next: impl Into<String>,
        source: MedicSource,
    ) -> Self {
        Self::new(
            category,
            code,
            MedicStatus::Warn,
            subject,
            detail,
            next,
            source,
        )
    }

    pub(super) fn fail(
        category: MedicCategory,
        code: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
        next: impl Into<String>,
        source: MedicSource,
    ) -> Self {
        Self::new(
            category,
            code,
            MedicStatus::Fail,
            subject,
            detail,
            next,
            source,
        )
    }

    pub(super) fn not_evaluated(
        category: MedicCategory,
        code: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
        next: impl Into<String>,
        source: MedicSource,
    ) -> Self {
        Self::new(
            category,
            code,
            MedicStatus::NotEvaluated,
            subject,
            detail,
            next,
            source,
        )
    }

    pub(super) fn new(
        category: MedicCategory,
        code: impl Into<String>,
        status: MedicStatus,
        subject: impl Into<String>,
        detail: impl Into<String>,
        next: impl Into<String>,
        source: MedicSource,
    ) -> Self {
        Self {
            category,
            code: code.into(),
            status,
            subject: subject.into(),
            detail: detail.into(),
            next: next.into(),
            source,
        }
    }
}

///
/// MedicScope
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(super) enum MedicScope {
    #[serde(rename = "fleet")]
    Fleet,
    #[serde(rename = "workspace")]
    Workspace,
}

///
/// MedicStatus
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(super) enum MedicStatus {
    #[serde(rename = "pass")]
    Pass,
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "fail")]
    Fail,
    #[serde(rename = "not_evaluated")]
    NotEvaluated,
}

impl MedicStatus {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Warn => "warn",
            Self::Fail => "fail",
            Self::NotEvaluated => "not_evaluated",
        }
    }
}

///
/// MedicCategory
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(super) enum MedicCategory {
    #[serde(rename = "environment")]
    Environment,
    #[serde(rename = "workspace_config")]
    WorkspaceConfig,
    #[serde(rename = "target_environment")]
    TargetEnvironment,
    #[serde(rename = "fleet_state")]
    FleetState,
    #[serde(rename = "topology")]
    Topology,
    #[serde(rename = "funding")]
    Funding,
    #[serde(rename = "auth")]
    Auth,
    #[serde(rename = "blob_storage")]
    BlobStorage,
    #[serde(rename = "runtime")]
    Runtime,
}

impl MedicCategory {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Environment => "environment",
            Self::WorkspaceConfig => "workspace_config",
            Self::TargetEnvironment => "target_environment",
            Self::FleetState => "fleet_state",
            Self::Topology => "topology",
            Self::Funding => "funding",
            Self::Auth => "auth",
            Self::BlobStorage => "blob_storage",
            Self::Runtime => "runtime",
        }
    }

    const fn order(self) -> usize {
        match self {
            Self::Environment => 0,
            Self::WorkspaceConfig => 1,
            Self::TargetEnvironment => 2,
            Self::FleetState => 3,
            Self::Topology => 4,
            Self::Funding => 5,
            Self::Auth => 6,
            Self::BlobStorage => 7,
            Self::Runtime => 8,
        }
    }
}

///
/// MedicSource
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(super) enum MedicSource {
    #[serde(rename = "admission_status")]
    AdmissionStatus,
    #[serde(rename = "command")]
    Command,
    #[serde(rename = "icp_cli")]
    IcpCli,
    #[serde(rename = "icp_config")]
    IcpConfig,
    #[serde(rename = "app_config")]
    AppConfig,
    #[serde(rename = "current_ensure")]
    CurrentEnsure,
    #[serde(rename = "blob_storage_readiness")]
    BlobStorageReadiness,
    #[serde(rename = "auth_renewal")]
    AuthRenewal,
    #[serde(rename = "state_manifest")]
    StateManifest,
}

impl MedicSource {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::AdmissionStatus => "admission_status",
            Self::Command => "command",
            Self::IcpCli => "icp_cli",
            Self::IcpConfig => "icp_config",
            Self::AppConfig => "app_config",
            Self::CurrentEnsure => "current_ensure",
            Self::BlobStorageReadiness => "blob_storage_readiness",
            Self::AuthRenewal => "auth_renewal",
            Self::StateManifest => "state_manifest",
        }
    }
}
