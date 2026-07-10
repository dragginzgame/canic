//! Module: canic_cli::medic::report
//!
//! Responsibility: own the stable medic report model, ordering, and aggregate status.
//! Does not own: diagnostic collection, command parsing, or text/JSON rendering.
//! Boundary: project and deployment checks construct this private CLI report model.

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
    pub(super) network: Option<String>,
    pub(super) deployment: Option<String>,
    pub(super) status: MedicStatus,
    pub(super) checks: Vec<MedicCheck>,
}

impl MedicReport {
    pub(super) fn new(options: &MedicOptions, checks: Vec<MedicCheck>) -> Self {
        let network = match options.scope {
            MedicScope::Project => options.network.clone(),
            MedicScope::Deployment => Some(options.deployment_network()),
        };
        Self::with_network(options, network, checks)
    }

    pub(super) fn with_network(
        options: &MedicOptions,
        network: Option<String>,
        checks: Vec<MedicCheck>,
    ) -> Self {
        let status = aggregate_status(&checks);
        Self {
            schema_version: SCHEMA_VERSION,
            command: options.command_label(),
            scope: options.scope,
            network,
            deployment: options.deployment.clone(),
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
#[serde(rename_all = "snake_case")]
pub(super) enum MedicScope {
    Project,
    Deployment,
}

///
/// MedicStatus
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum MedicStatus {
    Pass,
    Warn,
    Fail,
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
#[serde(rename_all = "snake_case")]
pub(super) enum MedicCategory {
    Environment,
    ProjectConfig,
    Network,
    DeploymentState,
    Topology,
    Auth,
    BlobStorage,
    Runtime,
}

impl MedicCategory {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Environment => "environment",
            Self::ProjectConfig => "project_config",
            Self::Network => "network",
            Self::DeploymentState => "deployment_state",
            Self::Topology => "topology",
            Self::Auth => "auth",
            Self::BlobStorage => "blob_storage",
            Self::Runtime => "runtime",
        }
    }

    const fn order(self) -> usize {
        match self {
            Self::Environment => 0,
            Self::ProjectConfig => 1,
            Self::Network => 2,
            Self::DeploymentState => 3,
            Self::Topology => 4,
            Self::Auth => 5,
            Self::BlobStorage => 6,
            Self::Runtime => 7,
        }
    }
}

///
/// MedicSource
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum MedicSource {
    Command,
    IcpCli,
    IcpConfig,
    FleetConfig,
    InstalledDeployment,
    DeploymentTruth,
    LocalReplica,
    BlobStorageReadiness,
    AuthRenewal,
    StateManifest,
}

impl MedicSource {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::IcpCli => "icp_cli",
            Self::IcpConfig => "icp_config",
            Self::FleetConfig => "fleet_config",
            Self::InstalledDeployment => "installed_deployment",
            Self::DeploymentTruth => "deployment_truth",
            Self::LocalReplica => "local_replica",
            Self::BlobStorageReadiness => "blob_storage_readiness",
            Self::AuthRenewal => "auth_renewal",
            Self::StateManifest => "state_manifest",
        }
    }
}
