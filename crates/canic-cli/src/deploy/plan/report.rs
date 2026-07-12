//! Module: canic_cli::deploy::plan::report
//!
//! Responsibility: define the serialized deployment-plan report model and stable labels.
//! Does not own: plan construction, diagnostic classification, outcome policy, or rendering.
//! Boundary: supplies one typed report contract to assembly and presentation owners.

use serde::Serialize;

use canic_host::deployment_truth::DeploymentPlanV1;

pub(super) const REPORT_SCHEMA_VERSION: u16 = 1;
pub(super) const SEVERITY_INFO: PlanDiagnosticSeverity = PlanDiagnosticSeverity::Info;
pub(super) const SEVERITY_WARNING: PlanDiagnosticSeverity = PlanDiagnosticSeverity::Warning;
pub(super) const SEVERITY_BLOCKED: PlanDiagnosticSeverity = PlanDiagnosticSeverity::Blocked;
pub(super) const SEVERITY_UNSUPPORTED: PlanDiagnosticSeverity = PlanDiagnosticSeverity::Unsupported;
pub(super) const CATEGORY_ARTIFACT: PlanDiagnosticCategory = PlanDiagnosticCategory::Artifact;
pub(super) const CATEGORY_AUTHORITY: PlanDiagnosticCategory = PlanDiagnosticCategory::Authority;
pub(super) const CATEGORY_CONFIG: PlanDiagnosticCategory = PlanDiagnosticCategory::Config;
pub(super) const CATEGORY_DEPLOYMENT_IDENTITY: PlanDiagnosticCategory =
    PlanDiagnosticCategory::DeploymentIdentity;
pub(super) const CATEGORY_INVENTORY: PlanDiagnosticCategory = PlanDiagnosticCategory::Inventory;
pub(super) const CATEGORY_OBSERVATION: PlanDiagnosticCategory = PlanDiagnosticCategory::Observation;
pub(super) const CATEGORY_TOPOLOGY: PlanDiagnosticCategory = PlanDiagnosticCategory::Topology;
pub(super) const CATEGORY_TRUST_DOMAIN: PlanDiagnosticCategory =
    PlanDiagnosticCategory::TrustDomain;
pub(super) const CATEGORY_UNSUPPORTED_SHAPE: PlanDiagnosticCategory =
    PlanDiagnosticCategory::UnsupportedShape;
pub(super) const CATEGORY_VERIFIER_READINESS: PlanDiagnosticCategory =
    PlanDiagnosticCategory::VerifierReadiness;
pub(super) const SOURCE_CLI_ARG: PlanDiagnosticSource = PlanDiagnosticSource::CliArg;
pub(super) const SOURCE_BUILD_PROFILE: PlanDiagnosticSource = PlanDiagnosticSource::BuildProfile;
pub(super) const SOURCE_DEPLOYMENT_CONFIG: PlanDiagnosticSource =
    PlanDiagnosticSource::DeploymentConfig;
pub(super) const SOURCE_DEPLOYMENT_PLAN_BUILDER: PlanDiagnosticSource =
    PlanDiagnosticSource::DeploymentPlanBuilder;
pub(super) const SOURCE_FLEET_CONFIG: PlanDiagnosticSource = PlanDiagnosticSource::FleetConfig;
pub(super) const SOURCE_INSTALLED_DEPLOYMENT: PlanDiagnosticSource =
    PlanDiagnosticSource::InstalledDeployment;
pub(super) const SOURCE_LOCAL_OBSERVATION: PlanDiagnosticSource =
    PlanDiagnosticSource::LocalObservation;
pub(super) const FUTURE_APPLY_PREVIEW_PHASE: ProposedOperationPhase =
    ProposedOperationPhase::FutureApplyPreview;
pub(super) const PROPOSED_OPERATION_NOT_EXECUTED: ProposedOperationStatus =
    ProposedOperationStatus::NotExecuted;
pub(super) const OP_CREATE_CANISTER: ProposedOperationKind = ProposedOperationKind::CreateCanister;
pub(super) const OP_INSTALL_WASM: ProposedOperationKind = ProposedOperationKind::InstallWasm;
pub(super) const OP_UPGRADE_WASM: ProposedOperationKind = ProposedOperationKind::UpgradeWasm;
pub(super) const OP_APPLY_POLICY: ProposedOperationKind = ProposedOperationKind::ApplyPolicy;
pub(super) const OP_SET_CONTROLLERS: ProposedOperationKind = ProposedOperationKind::SetControllers;
pub(super) const OP_REGISTER_CHILD: ProposedOperationKind = ProposedOperationKind::RegisterChild;
pub(super) const OP_REGISTER_ROOT: ProposedOperationKind = ProposedOperationKind::RegisterRoot;
pub(super) const OP_VERIFY_READINESS: ProposedOperationKind =
    ProposedOperationKind::VerifyReadiness;
pub(super) const OP_VERIFY_TOPOLOGY: ProposedOperationKind = ProposedOperationKind::VerifyTopology;
pub(super) const OP_UPLOAD_ARTIFACT: ProposedOperationKind = ProposedOperationKind::UploadArtifact;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(in crate::deploy) struct DeploymentPlanReport {
    pub(super) schema_version: u16,
    pub(super) command: &'static str,
    pub(super) target: String,
    pub(super) network: String,
    pub(super) build_profile: String,
    pub(super) config_path: String,
    pub(super) status: PlanStatus,
    pub(super) comparison_status: ComparisonStatus,
    pub(super) plan: DeploymentPlanV1,
    pub(super) blockers: Vec<PlanDiagnostic>,
    pub(super) warnings: Vec<PlanDiagnostic>,
    pub(super) assumptions: Vec<PlanDiagnostic>,
    pub(super) verified_facts: Vec<PlanDiagnostic>,
    pub(super) proposed_operations: Vec<ProposedOperationLabel>,
    pub(super) next_actions: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PlanStatus {
    Planned,
    Warning,
    Blocked,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ComparisonStatus {
    NotRequested,
    NotAvailable,
    Compared,
    ComparedWithWarnings,
    ComparedWithDrift,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct PlanDiagnostic {
    pub(super) category: PlanDiagnosticCategory,
    pub(super) code: String,
    pub(super) severity: PlanDiagnosticSeverity,
    pub(super) subject: String,
    pub(super) detail: String,
    pub(super) next: Option<String>,
    pub(super) source: PlanDiagnosticSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PlanDiagnosticCategory {
    Artifact,
    Authority,
    Config,
    DeploymentIdentity,
    Inventory,
    Observation,
    Topology,
    TrustDomain,
    UnsupportedShape,
    VerifierReadiness,
}

impl PlanDiagnosticCategory {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Artifact => "artifact",
            Self::Authority => "authority",
            Self::Config => "config",
            Self::DeploymentIdentity => "deployment_identity",
            Self::Inventory => "inventory",
            Self::Observation => "observation",
            Self::Topology => "topology",
            Self::TrustDomain => "trust_domain",
            Self::UnsupportedShape => "unsupported_shape",
            Self::VerifierReadiness => "verifier_readiness",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PlanDiagnosticSeverity {
    Blocked,
    Info,
    Unsupported,
    Warning,
}

impl PlanDiagnosticSeverity {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Blocked => "blocked",
            Self::Info => "info",
            Self::Unsupported => "unsupported",
            Self::Warning => "warning",
        }
    }

    pub(super) const fn sort_rank(self) -> u8 {
        match self {
            Self::Blocked => 0,
            Self::Unsupported => 1,
            Self::Warning => 2,
            Self::Info => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PlanDiagnosticSource {
    BuildProfile,
    CliArg,
    DeploymentConfig,
    DeploymentPlanBuilder,
    FleetConfig,
    InstalledDeployment,
    LocalObservation,
}

impl PlanDiagnosticSource {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::BuildProfile => "build_profile",
            Self::CliArg => "cli_arg",
            Self::DeploymentConfig => "deployment_config",
            Self::DeploymentPlanBuilder => "deployment_plan_builder",
            Self::FleetConfig => "fleet_config",
            Self::InstalledDeployment => "installed_deployment",
            Self::LocalObservation => "local_observation",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct ProposedOperationLabel {
    pub(super) phase: ProposedOperationPhase,
    pub(super) label: ProposedOperationKind,
    pub(super) subject: String,
    pub(super) status: ProposedOperationStatus,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ProposedOperationPhase {
    FutureApplyPreview,
}

impl ProposedOperationPhase {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::FutureApplyPreview => "future_apply_preview",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ProposedOperationKind {
    ApplyPolicy,
    CreateCanister,
    InstallWasm,
    RegisterChild,
    RegisterRoot,
    SetControllers,
    UpgradeWasm,
    UploadArtifact,
    VerifyReadiness,
    VerifyTopology,
}

impl ProposedOperationKind {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::ApplyPolicy => "apply_policy",
            Self::CreateCanister => "create_canister",
            Self::InstallWasm => "install_wasm",
            Self::RegisterChild => "register_child",
            Self::RegisterRoot => "register_root",
            Self::SetControllers => "set_controllers",
            Self::UpgradeWasm => "upgrade_wasm",
            Self::UploadArtifact => "upload_artifact",
            Self::VerifyReadiness => "verify_readiness",
            Self::VerifyTopology => "verify_topology",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ProposedOperationStatus {
    NotExecuted,
}

impl ProposedOperationStatus {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::NotExecuted => "not_executed",
        }
    }
}

impl PlanStatus {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Warning => SEVERITY_WARNING.label(),
            Self::Blocked => SEVERITY_BLOCKED.label(),
            Self::Unsupported => SEVERITY_UNSUPPORTED.label(),
        }
    }
}

impl ComparisonStatus {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::NotRequested => "not_requested",
            Self::NotAvailable => "not_available",
            Self::Compared => "compared",
            Self::ComparedWithWarnings => "compared_with_warnings",
            Self::ComparedWithDrift => "compared_with_drift",
        }
    }
}
