//! Module: canic_cli::deploy::plan
//!
//! Responsibility: orchestrate deterministic deployment planning and report assembly.
//! Does not own: deployment mutation, report rendering, or output persistence.
//! Boundary: resolves local planning evidence and delegates report output to its owner.

mod command;
mod diagnostics;
mod evidence;
mod render;

use super::DeployCommandError;
use crate::{cli::help::print_help_or_version, version_text};
#[cfg(test)]
use canic_host::deployment_truth::DeploymentAssumptionV1;
use canic_host::deployment_truth::{
    DeploymentAssumptionKindV1, DeploymentPlanV1, LocalDeploymentPlanRequest,
};
use serde::Serialize;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use command::REPORT_COMMAND;
pub(super) use command::{DeployPlanOptions, DeployPlanRoots, usage};
use diagnostics::{
    is_observed_state_drift_assumption, plan_assumptions, plan_blockers, plan_warnings,
    target_resolution_blockers,
};
#[cfg(test)]
use evidence::verifier_readiness_facts;
use evidence::{verified_facts, verifier_readiness_required};
pub(super) use render::{command_exit_result, write_report};
#[cfg(test)]
pub(super) use render::{render_json, render_text};

const REPORT_SCHEMA_VERSION: u16 = 1;
const SEVERITY_INFO: PlanDiagnosticSeverity = PlanDiagnosticSeverity::Info;
const SEVERITY_WARNING: PlanDiagnosticSeverity = PlanDiagnosticSeverity::Warning;
const SEVERITY_BLOCKED: PlanDiagnosticSeverity = PlanDiagnosticSeverity::Blocked;
const SEVERITY_UNSUPPORTED: PlanDiagnosticSeverity = PlanDiagnosticSeverity::Unsupported;
const CATEGORY_ARTIFACT: PlanDiagnosticCategory = PlanDiagnosticCategory::Artifact;
const CATEGORY_AUTHORITY: PlanDiagnosticCategory = PlanDiagnosticCategory::Authority;
const CATEGORY_CONFIG: PlanDiagnosticCategory = PlanDiagnosticCategory::Config;
const CATEGORY_DEPLOYMENT_IDENTITY: PlanDiagnosticCategory =
    PlanDiagnosticCategory::DeploymentIdentity;
const CATEGORY_INVENTORY: PlanDiagnosticCategory = PlanDiagnosticCategory::Inventory;
const CATEGORY_OBSERVATION: PlanDiagnosticCategory = PlanDiagnosticCategory::Observation;
const CATEGORY_TOPOLOGY: PlanDiagnosticCategory = PlanDiagnosticCategory::Topology;
const CATEGORY_TRUST_DOMAIN: PlanDiagnosticCategory = PlanDiagnosticCategory::TrustDomain;
const CATEGORY_UNSUPPORTED_SHAPE: PlanDiagnosticCategory = PlanDiagnosticCategory::UnsupportedShape;
const CATEGORY_VERIFIER_READINESS: PlanDiagnosticCategory =
    PlanDiagnosticCategory::VerifierReadiness;
const SOURCE_CLI_ARG: PlanDiagnosticSource = PlanDiagnosticSource::CliArg;
const SOURCE_BUILD_PROFILE: PlanDiagnosticSource = PlanDiagnosticSource::BuildProfile;
const SOURCE_DEPLOYMENT_CONFIG: PlanDiagnosticSource = PlanDiagnosticSource::DeploymentConfig;
const SOURCE_DEPLOYMENT_PLAN_BUILDER: PlanDiagnosticSource =
    PlanDiagnosticSource::DeploymentPlanBuilder;
const SOURCE_FLEET_CONFIG: PlanDiagnosticSource = PlanDiagnosticSource::FleetConfig;
const SOURCE_INSTALLED_DEPLOYMENT: PlanDiagnosticSource = PlanDiagnosticSource::InstalledDeployment;
const SOURCE_LOCAL_OBSERVATION: PlanDiagnosticSource = PlanDiagnosticSource::LocalObservation;
const FUTURE_APPLY_PREVIEW_PHASE: ProposedOperationPhase =
    ProposedOperationPhase::FutureApplyPreview;
const PROPOSED_OPERATION_NOT_EXECUTED: ProposedOperationStatus =
    ProposedOperationStatus::NotExecuted;
const OP_CREATE_CANISTER: ProposedOperationKind = ProposedOperationKind::CreateCanister;
const OP_INSTALL_WASM: ProposedOperationKind = ProposedOperationKind::InstallWasm;
const OP_UPGRADE_WASM: ProposedOperationKind = ProposedOperationKind::UpgradeWasm;
const OP_APPLY_POLICY: ProposedOperationKind = ProposedOperationKind::ApplyPolicy;
const OP_SET_CONTROLLERS: ProposedOperationKind = ProposedOperationKind::SetControllers;
const OP_REGISTER_CHILD: ProposedOperationKind = ProposedOperationKind::RegisterChild;
const OP_REGISTER_ROOT: ProposedOperationKind = ProposedOperationKind::RegisterRoot;
const OP_VERIFY_READINESS: ProposedOperationKind = ProposedOperationKind::VerifyReadiness;
const OP_VERIFY_TOPOLOGY: ProposedOperationKind = ProposedOperationKind::VerifyTopology;
const OP_UPLOAD_ARTIFACT: ProposedOperationKind = ProposedOperationKind::UploadArtifact;
const ASSUMPTION_PREFIX_LOCAL_ARTIFACTS: &str = "local_artifacts.";
const ASSUMPTION_PREFIX_LOCAL_CONFIG: &str = "local_config.";
const ASSUMPTION_PREFIX_LOCAL_STATE: &str = "local_state.";
const ASSUMPTION_PREFIX_UNSUPPORTED: &str = "unsupported.";
const ASSUMPTION_KEY_LOCAL_CONFIG_CONTROLLERS: &str = "local_config.controllers";
const ASSUMPTION_KEY_LOCAL_CONFIG_POOLS: &str = "local_config.pools";
const ASSUMPTION_KEY_LOCAL_CONFIG_ROLES: &str = "local_config.roles";
const ASSUMPTION_KEY_LOCAL_STATE_UNVERIFIED_ROOT_CANISTER_ID: &str =
    "local_state.unverified_root_canister_id";
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct DeploymentPlanReport {
    schema_version: u16,
    command: &'static str,
    target: String,
    network: String,
    build_profile: String,
    config_path: String,
    status: PlanStatus,
    comparison_status: ComparisonStatus,
    plan: DeploymentPlanV1,
    blockers: Vec<PlanDiagnostic>,
    warnings: Vec<PlanDiagnostic>,
    assumptions: Vec<PlanDiagnostic>,
    verified_facts: Vec<PlanDiagnostic>,
    proposed_operations: Vec<ProposedOperationLabel>,
    next_actions: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
enum PlanStatus {
    Planned,
    Warning,
    Blocked,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ComparisonStatus {
    NotRequested,
    NotAvailable,
    Compared,
    ComparedWithWarnings,
    ComparedWithDrift,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct PlanDiagnostic {
    category: PlanDiagnosticCategory,
    code: String,
    severity: PlanDiagnosticSeverity,
    subject: String,
    detail: String,
    next: Option<String>,
    source: PlanDiagnosticSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum PlanDiagnosticCategory {
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
    const fn label(self) -> &'static str {
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
enum PlanDiagnosticSeverity {
    Blocked,
    Info,
    Unsupported,
    Warning,
}

impl PlanDiagnosticSeverity {
    const fn label(self) -> &'static str {
        match self {
            Self::Blocked => "blocked",
            Self::Info => "info",
            Self::Unsupported => "unsupported",
            Self::Warning => "warning",
        }
    }

    const fn sort_rank(self) -> u8 {
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
enum PlanDiagnosticSource {
    BuildProfile,
    CliArg,
    DeploymentConfig,
    DeploymentPlanBuilder,
    FleetConfig,
    InstalledDeployment,
    LocalObservation,
}

impl PlanDiagnosticSource {
    const fn label(self) -> &'static str {
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
struct ProposedOperationLabel {
    phase: ProposedOperationPhase,
    label: ProposedOperationKind,
    subject: String,
    status: ProposedOperationStatus,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
enum ProposedOperationPhase {
    FutureApplyPreview,
}

impl ProposedOperationPhase {
    const fn label(self) -> &'static str {
        match self {
            Self::FutureApplyPreview => "future_apply_preview",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
enum ProposedOperationKind {
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
    const fn label(self) -> &'static str {
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
enum ProposedOperationStatus {
    NotExecuted,
}

impl ProposedOperationStatus {
    const fn label(self) -> &'static str {
        match self {
            Self::NotExecuted => "not_executed",
        }
    }
}

pub(super) fn run<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = DeployPlanOptions::parse(args)?;
    let roots = DeployPlanRoots::discover()?;
    let report = build_report(&options, &roots);
    write_report(&options, &report)?;
    command_exit_result(&report)
}

pub(super) fn build_report(
    options: &DeployPlanOptions,
    roots: &DeployPlanRoots,
) -> DeploymentPlanReport {
    let config_path = plan_config_path(&roots.workspace_root, options);
    let plan = build_plan(options, roots, &config_path);
    let mut blockers = target_resolution_blockers(options, &config_path);
    let target_resolved = blockers.is_empty();
    if target_resolved {
        blockers.extend(plan_blockers(&plan));
    }
    let mut assumptions = plan_assumptions(&plan);
    let mut warnings = plan_warnings(&plan);
    let mut verified_facts = verified_facts(options, &config_path, target_resolved, &plan);
    let proposed_operations = proposed_operations(&plan);
    let mut next_actions = next_actions(options, &blockers, &warnings, &assumptions);

    sort_diagnostics(&mut blockers);
    sort_diagnostics(&mut warnings);
    sort_diagnostics(&mut assumptions);
    sort_diagnostics(&mut verified_facts);
    next_actions.sort();
    next_actions.dedup();

    let status = aggregate_status(&blockers, &warnings, &assumptions);
    let comparison_status = comparison_status(&plan, &blockers, &warnings, &assumptions);

    DeploymentPlanReport {
        schema_version: REPORT_SCHEMA_VERSION,
        command: REPORT_COMMAND,
        target: options.deployment.clone(),
        network: options.network.clone(),
        build_profile: build_profile_name(options),
        config_path: display_path(&config_path),
        status,
        comparison_status,
        plan,
        blockers,
        warnings,
        assumptions,
        verified_facts,
        proposed_operations,
        next_actions,
    }
}

fn build_plan(
    options: &DeployPlanOptions,
    roots: &DeployPlanRoots,
    config_path: &Path,
) -> DeploymentPlanV1 {
    canic_host::deployment_truth::build_local_deployment_plan(&LocalDeploymentPlanRequest {
        deployment_name: options.deployment.clone(),
        network: options.network.clone(),
        workspace_root: roots.workspace_root.clone(),
        icp_root: roots.icp_root.clone(),
        config_path: Some(config_path.to_path_buf()),
        runtime_variant: options.network.clone(),
        build_profile: build_profile_name(options),
    })
}

fn proposed_operations(plan: &DeploymentPlanV1) -> Vec<ProposedOperationLabel> {
    let mut operations = Vec::new();
    for canister in &plan.expected_canisters {
        if canister.canister_id.is_none() {
            operations.push(operation(OP_CREATE_CANISTER, &canister.role));
        }
    }
    for canister in &plan.expected_pool {
        if canister.canister_id.is_none() {
            let subject = pool_operation_subject(&canister.pool, canister.role.as_deref());
            operations.push(operation(OP_CREATE_CANISTER, &subject));
        }
    }
    for canister in &plan.expected_canisters {
        if canister.canister_id.is_none() {
            operations.push(operation(
                registration_operation_label(&canister.role),
                &canister.role,
            ));
        }
    }
    for canister in &plan.expected_pool {
        if canister.canister_id.is_none() {
            let subject = pool_operation_subject(&canister.pool, canister.role.as_deref());
            operations.push(operation(OP_REGISTER_CHILD, &subject));
        }
    }
    for artifact in &plan.role_artifacts {
        operations.push(operation(OP_UPLOAD_ARTIFACT, &artifact.role));
        operations.push(operation(
            wasm_operation_label(plan, &artifact.role),
            &artifact.role,
        ));
    }
    if !plan.authority_profile.expected_controllers.is_empty() {
        operations.push(operation(
            OP_APPLY_POLICY,
            &plan.deployment_identity.deployment_name,
        ));
        operations.push(operation(
            OP_SET_CONTROLLERS,
            &plan.deployment_identity.deployment_name,
        ));
    }
    if verifier_readiness_required(plan) {
        operations.push(operation(
            OP_VERIFY_READINESS,
            &plan.deployment_identity.deployment_name,
        ));
    }
    operations.push(operation(
        OP_VERIFY_TOPOLOGY,
        &plan.deployment_identity.deployment_name,
    ));
    sort_proposed_operations(&mut operations);
    operations
}

fn registration_operation_label(role: &str) -> ProposedOperationKind {
    if role == "root" {
        OP_REGISTER_ROOT
    } else {
        OP_REGISTER_CHILD
    }
}

fn pool_operation_subject(pool: &str, role: Option<&str>) -> String {
    match role {
        Some(role) => format!("{pool}:{role}"),
        None => pool.to_string(),
    }
}

fn wasm_operation_label(plan: &DeploymentPlanV1, role: &str) -> ProposedOperationKind {
    if plan
        .expected_canisters
        .iter()
        .any(|canister| canister.role == role && canister.canister_id.is_some())
    {
        OP_UPGRADE_WASM
    } else {
        OP_INSTALL_WASM
    }
}

fn operation(label: ProposedOperationKind, subject: &str) -> ProposedOperationLabel {
    ProposedOperationLabel {
        phase: FUTURE_APPLY_PREVIEW_PHASE,
        label,
        subject: subject.to_string(),
        status: PROPOSED_OPERATION_NOT_EXECUTED,
    }
}

fn next_actions(
    options: &DeployPlanOptions,
    blockers: &[PlanDiagnostic],
    warnings: &[PlanDiagnostic],
    assumptions: &[PlanDiagnostic],
) -> Vec<String> {
    let mut actions = Vec::new();
    if !blockers.is_empty() {
        actions.push("fix blocker diagnostics before designing apply".to_string());
    }
    if !warnings.is_empty() || !assumptions.is_empty() {
        actions.push("resolve warnings before designing apply".to_string());
    }
    if has_artifact_diagnostics(blockers)
        || has_artifact_diagnostics(warnings)
        || has_artifact_diagnostics(assumptions)
    {
        actions
            .push("run canic build or provide a build profile with resolved artifacts".to_string());
    }
    actions.push(format!(
        "run canic medic deployment {} if operator readiness is uncertain",
        options.deployment
    ));
    actions
}

fn has_artifact_diagnostics(diagnostics: &[PlanDiagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.category == CATEGORY_ARTIFACT)
}

fn aggregate_status(
    blockers: &[PlanDiagnostic],
    warnings: &[PlanDiagnostic],
    assumptions: &[PlanDiagnostic],
) -> PlanStatus {
    if blockers
        .iter()
        .any(|diagnostic| diagnostic.severity == SEVERITY_UNSUPPORTED)
    {
        return PlanStatus::Unsupported;
    }
    if !blockers.is_empty() {
        return PlanStatus::Blocked;
    }
    if !warnings.is_empty() || !assumptions.is_empty() {
        return PlanStatus::Warning;
    }
    PlanStatus::Planned
}

fn comparison_status(
    plan: &DeploymentPlanV1,
    blockers: &[PlanDiagnostic],
    warnings: &[PlanDiagnostic],
    assumptions: &[PlanDiagnostic],
) -> ComparisonStatus {
    if !blockers.is_empty() {
        return ComparisonStatus::NotRequested;
    }

    if has_observed_state_drift(plan) {
        return ComparisonStatus::ComparedWithDrift;
    }

    if has_missing_observed_state(plan) {
        return ComparisonStatus::NotAvailable;
    }

    if plan.trust_domain.root_trust_anchor.is_none() {
        return ComparisonStatus::NotRequested;
    }

    if !warnings.is_empty() || !assumptions.is_empty() {
        ComparisonStatus::ComparedWithWarnings
    } else {
        ComparisonStatus::Compared
    }
}

fn has_observed_state_drift(plan: &DeploymentPlanV1) -> bool {
    plan.unresolved_assumptions
        .iter()
        .any(is_observed_state_drift_assumption)
}

fn has_missing_observed_state(plan: &DeploymentPlanV1) -> bool {
    plan.unresolved_assumptions.iter().any(|assumption| {
        assumption.has_kind(DeploymentAssumptionKindV1::LocalStateMissing)
            || assumption.has_kind(DeploymentAssumptionKindV1::LocalStateReadFailed)
    })
}

fn sort_diagnostics(diagnostics: &mut [PlanDiagnostic]) {
    diagnostics.sort_by(|left, right| {
        diagnostic_severity_rank(left.severity)
            .cmp(&diagnostic_severity_rank(right.severity))
            .then_with(|| left.severity.label().cmp(right.severity.label()))
            .then_with(|| left.category.label().cmp(right.category.label()))
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.subject.cmp(&right.subject))
            .then_with(|| left.source.label().cmp(right.source.label()))
    });
}

const fn diagnostic_severity_rank(severity: PlanDiagnosticSeverity) -> u8 {
    severity.sort_rank()
}

fn sort_proposed_operations(operations: &mut Vec<ProposedOperationLabel>) {
    operations.sort_by(|left, right| {
        left.phase
            .cmp(&right.phase)
            .then_with(|| left.label.cmp(&right.label))
            .then_with(|| left.subject.cmp(&right.subject))
            .then_with(|| left.status.cmp(&right.status))
    });
    operations.dedup();
}

fn plan_config_path(workspace_root: &Path, options: &DeployPlanOptions) -> PathBuf {
    let config = options.config.clone().unwrap_or_else(|| {
        PathBuf::from("fleets")
            .join(&options.deployment)
            .join("canic.toml")
    });
    if config.is_absolute() {
        config
    } else {
        workspace_root.join(config)
    }
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

fn build_profile_name(options: &DeployPlanOptions) -> String {
    options.build_profile.target_dir_name().to_string()
}

impl PlanStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Warning => SEVERITY_WARNING.label(),
            Self::Blocked => SEVERITY_BLOCKED.label(),
            Self::Unsupported => SEVERITY_UNSUPPORTED.label(),
        }
    }
}

impl ComparisonStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::NotRequested => "not_requested",
            Self::NotAvailable => "not_available",
            Self::Compared => "compared",
            Self::ComparedWithWarnings => "compared_with_warnings",
            Self::ComparedWithDrift => "compared_with_drift",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_host::deployment_truth::{
        ArtifactSourceV1, AuthorityProfileV1, CanisterControlClassV1, DeploymentIdentityV1,
        ExpectedCanisterV1, RoleArtifactV1, RoleEpochExpectationV1, TrustDomainV1,
        VerifierReadinessExpectationV1,
    };

    #[test]
    fn unsupported_plan_assumptions_become_unsupported_blockers() {
        let unsupported_key = format!("{ASSUMPTION_PREFIX_UNSUPPORTED}pool_relationship");
        let plan = plan_with_assumptions([assumption(
            &unsupported_key,
            "pool relationship is outside the deploy-plan planner contract",
        )]);

        let blockers = plan_blockers(&plan);
        let assumptions = plan_assumptions(&plan);
        let warnings = plan_warnings(&plan);

        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].category, CATEGORY_UNSUPPORTED_SHAPE);
        assert_eq!(blockers[0].code, "unsupported_pool_relationship");
        assert_eq!(blockers[0].severity, SEVERITY_UNSUPPORTED);
        assert_eq!(blockers[0].subject, unsupported_key);
        assert!(
            blockers[0]
                .next
                .as_deref()
                .is_some_and(|next| { next.contains("desired deployment shape") })
        );
        assert!(assumptions.is_empty());
        assert!(warnings.is_empty());
        assert_eq!(
            aggregate_status(&blockers, &warnings, &assumptions),
            PlanStatus::Unsupported
        );
    }

    #[test]
    fn blocked_status_wins_when_no_unsupported_assumption_exists() {
        let plan = plan_with_assumptions([assumption(
            ASSUMPTION_KEY_LOCAL_CONFIG_CONTROLLERS,
            "could not resolve configured controllers",
        )]);

        let blockers = plan_blockers(&plan);
        let assumptions = plan_assumptions(&plan);
        let warnings = plan_warnings(&plan);

        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].category, CATEGORY_AUTHORITY);
        assert_eq!(blockers[0].severity, SEVERITY_BLOCKED);
        assert!(assumptions.is_empty());
        assert!(warnings.is_empty());
        assert_eq!(
            aggregate_status(&blockers, &warnings, &assumptions),
            PlanStatus::Blocked
        );
    }

    #[test]
    fn verifier_readiness_expectations_emit_preview_label() {
        let mut required_plan = plan_with_assumptions([]);
        required_plan.expected_verifier_readiness.required = true;

        assert_proposed_operation(&required_plan, OP_VERIFY_READINESS, "demo-local");

        let mut epoch_plan = plan_with_assumptions([]);
        epoch_plan
            .expected_verifier_readiness
            .expected_role_epochs
            .push(RoleEpochExpectationV1 {
                role: "user_hub".to_string(),
                minimum_epoch: 42,
            });

        assert_proposed_operation(&epoch_plan, OP_VERIFY_READINESS, "demo-local");
    }

    #[test]
    fn verifier_readiness_preview_label_is_omitted_without_expectation() {
        let plan = plan_with_assumptions([]);

        assert!(
            proposed_operations(&plan)
                .iter()
                .all(|operation| operation.label != OP_VERIFY_READINESS)
        );
    }

    #[test]
    fn verifier_readiness_expectations_emit_verified_fact() {
        let mut plan = plan_with_assumptions([]);
        plan.expected_verifier_readiness.expected_role_epochs = vec![RoleEpochExpectationV1 {
            role: "user_hub".to_string(),
            minimum_epoch: 42,
        }];

        let facts = verifier_readiness_facts(&plan);

        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].category, CATEGORY_VERIFIER_READINESS);
        assert_eq!(facts[0].code, "verifier_readiness_expectation_resolved");
        assert_eq!(facts[0].severity, SEVERITY_INFO);
        assert_eq!(facts[0].subject, "demo-local");
        assert_eq!(facts[0].source, SOURCE_DEPLOYMENT_PLAN_BUILDER);
        assert!(facts[0].detail.contains("1 role epoch"));
    }

    #[test]
    fn verifier_readiness_fact_is_omitted_without_expectation() {
        let plan = plan_with_assumptions([]);

        assert!(verifier_readiness_facts(&plan).is_empty());
    }

    #[test]
    fn command_exit_contract_matches_plan_status() {
        for status in [PlanStatus::Planned, PlanStatus::Warning] {
            let report = report_with_status(status);

            assert!(command_exit_result(&report).is_ok());
        }

        for status in [PlanStatus::Blocked, PlanStatus::Unsupported] {
            let report = report_with_status(status);
            let err = command_exit_result(&report).expect_err("blocked status should fail");

            assert!(matches!(err, DeployCommandError::PlanBlocked(_)));
            assert_eq!(err.exit_code(), 1);
            assert!(err.suppress_stderr());
        }
    }

    #[test]
    fn diagnostic_sort_order_is_deterministic() {
        let mut diagnostics = diagnostic_fixtures([
            "warning|config|z_config_gap|demo|deployment_plan_builder",
            "warning|artifact|artifact_gap|beta|fleet_config",
            "warning|artifact|artifact_gap|alpha|deployment_plan_builder",
            "blocked|config|plan_blocker|demo|deployment_plan_builder",
            "unsupported|unsupported_shape|unsupported_pool|demo|deployment_plan_builder",
            "warning|artifact|artifact_gap|beta|deployment_plan_builder",
            "info|config|resolved_fact|demo|deployment_plan_builder",
        ]);

        sort_diagnostics(&mut diagnostics);

        let ordered = diagnostics.iter().map(diagnostic_key).collect::<Vec<_>>();
        assert_eq!(
            ordered,
            vec![
                "blocked|config|plan_blocker|demo|deployment_plan_builder",
                "unsupported|unsupported_shape|unsupported_pool|demo|deployment_plan_builder",
                "warning|artifact|artifact_gap|alpha|deployment_plan_builder",
                "warning|artifact|artifact_gap|beta|deployment_plan_builder",
                "warning|artifact|artifact_gap|beta|fleet_config",
                "warning|config|z_config_gap|demo|deployment_plan_builder",
                "info|config|resolved_fact|demo|deployment_plan_builder",
            ]
        );
    }

    #[test]
    fn proposed_operation_sort_order_deduplicates_repeated_labels() {
        let mut operations = vec![
            operation(OP_VERIFY_TOPOLOGY, "demo-local"),
            operation(OP_INSTALL_WASM, "root"),
            operation(OP_INSTALL_WASM, "root"),
            operation(OP_REGISTER_CHILD, "user_hub"),
            operation(OP_REGISTER_CHILD, "user_hub"),
        ];

        sort_proposed_operations(&mut operations);

        assert_eq!(
            operation_keys(&operations),
            vec![
                "future_apply_preview|install_wasm|root|not_executed",
                "future_apply_preview|register_child|user_hub|not_executed",
                "future_apply_preview|verify_topology|demo-local|not_executed",
            ]
        );
    }

    #[test]
    fn proposed_operations_returns_sorted_deduplicated_preview() {
        let mut plan = plan_with_assumptions([]);
        plan.expected_canisters = vec![expected_canister("root"), expected_canister("root")];

        assert_eq!(
            operation_keys(&proposed_operations(&plan)),
            vec![
                "future_apply_preview|create_canister|root|not_executed",
                "future_apply_preview|register_root|root|not_executed",
                "future_apply_preview|verify_topology|demo-local|not_executed",
            ]
        );
    }

    #[test]
    fn proposed_operations_include_artifact_upload_preview_labels() {
        let mut plan = plan_with_assumptions([]);
        plan.role_artifacts = vec![role_artifact("root"), role_artifact("user_hub")];

        assert_eq!(
            operation_keys(&proposed_operations(&plan)),
            vec![
                "future_apply_preview|install_wasm|root|not_executed",
                "future_apply_preview|install_wasm|user_hub|not_executed",
                "future_apply_preview|upload_artifact|root|not_executed",
                "future_apply_preview|upload_artifact|user_hub|not_executed",
                "future_apply_preview|verify_topology|demo-local|not_executed",
            ]
        );
    }

    #[test]
    fn proposed_operations_include_authority_policy_preview_labels() {
        let mut plan = plan_with_assumptions([]);
        plan.authority_profile.expected_controllers = vec!["aaaaa-aa".to_string()];

        assert_eq!(
            operation_keys(&proposed_operations(&plan)),
            vec![
                "future_apply_preview|apply_policy|demo-local|not_executed",
                "future_apply_preview|set_controllers|demo-local|not_executed",
                "future_apply_preview|verify_topology|demo-local|not_executed",
            ]
        );
    }

    fn operation_keys(operations: &[ProposedOperationLabel]) -> Vec<String> {
        operations.iter().map(operation_key).collect()
    }

    fn operation_key(operation: &ProposedOperationLabel) -> String {
        format!(
            "{}|{}|{}|{}",
            operation.phase.label(),
            operation.label.label(),
            operation.subject,
            operation.status.label()
        )
    }

    fn diagnostic_fixtures(keys: impl IntoIterator<Item = &'static str>) -> Vec<PlanDiagnostic> {
        keys.into_iter().map(diagnostic_fixture).collect()
    }

    fn diagnostic_key(diagnostic: &PlanDiagnostic) -> String {
        format!(
            "{}|{}|{}|{}|{}",
            diagnostic.severity.label(),
            diagnostic.category.label(),
            diagnostic.code,
            diagnostic.subject,
            diagnostic.source.label()
        )
    }

    fn diagnostic_fixture(key: &'static str) -> PlanDiagnostic {
        let [severity, category, code, subject, source] = key
            .split('|')
            .collect::<Vec<_>>()
            .try_into()
            .expect("diagnostic fixture keys contain five fields");
        PlanDiagnostic {
            category: diagnostic_category_fixture(category),
            code: code.to_string(),
            severity: diagnostic_severity_fixture(severity),
            subject: subject.to_string(),
            detail: "diagnostic detail".to_string(),
            next: None,
            source: diagnostic_source_fixture(source),
        }
    }

    fn diagnostic_category_fixture(value: &str) -> PlanDiagnosticCategory {
        match value {
            "artifact" => CATEGORY_ARTIFACT,
            "authority" => CATEGORY_AUTHORITY,
            "config" => CATEGORY_CONFIG,
            "deployment_identity" => CATEGORY_DEPLOYMENT_IDENTITY,
            "inventory" => CATEGORY_INVENTORY,
            "observation" => CATEGORY_OBSERVATION,
            "topology" => CATEGORY_TOPOLOGY,
            "trust_domain" => CATEGORY_TRUST_DOMAIN,
            "unsupported_shape" => CATEGORY_UNSUPPORTED_SHAPE,
            "verifier_readiness" => CATEGORY_VERIFIER_READINESS,
            _ => panic!("unknown diagnostic category fixture {value}"),
        }
    }

    fn diagnostic_severity_fixture(value: &str) -> PlanDiagnosticSeverity {
        match value {
            "blocked" => SEVERITY_BLOCKED,
            "info" => SEVERITY_INFO,
            "unsupported" => SEVERITY_UNSUPPORTED,
            "warning" => SEVERITY_WARNING,
            _ => panic!("unknown diagnostic severity fixture {value}"),
        }
    }

    fn diagnostic_source_fixture(value: &str) -> PlanDiagnosticSource {
        match value {
            "build_profile" => SOURCE_BUILD_PROFILE,
            "cli_arg" => SOURCE_CLI_ARG,
            "deployment_config" => SOURCE_DEPLOYMENT_CONFIG,
            "deployment_plan_builder" => SOURCE_DEPLOYMENT_PLAN_BUILDER,
            "fleet_config" => SOURCE_FLEET_CONFIG,
            "installed_deployment" => SOURCE_INSTALLED_DEPLOYMENT,
            "local_observation" => SOURCE_LOCAL_OBSERVATION,
            _ => panic!("unknown diagnostic source fixture {value}"),
        }
    }

    fn report_with_status(status: PlanStatus) -> DeploymentPlanReport {
        DeploymentPlanReport {
            schema_version: REPORT_SCHEMA_VERSION,
            command: REPORT_COMMAND,
            target: "demo-local".to_string(),
            network: "local".to_string(),
            build_profile: "debug".to_string(),
            config_path: "fleets/demo/canic.toml".to_string(),
            status,
            comparison_status: ComparisonStatus::NotRequested,
            plan: plan_with_assumptions([]),
            blockers: Vec::new(),
            warnings: Vec::new(),
            assumptions: Vec::new(),
            verified_facts: Vec::new(),
            proposed_operations: Vec::new(),
            next_actions: Vec::new(),
        }
    }

    fn plan_with_assumptions(
        assumptions: impl IntoIterator<Item = DeploymentAssumptionV1>,
    ) -> DeploymentPlanV1 {
        DeploymentPlanV1 {
            schema_version: 1,
            plan_id: "local:demo-local:plan".to_string(),
            deployment_identity: DeploymentIdentityV1 {
                deployment_name: "demo-local".to_string(),
                network: "local".to_string(),
                root_principal: None,
                authority_profile_hash: None,
                role_topology_hash: None,
                deployment_manifest_digest: None,
                canonical_runtime_config_digest: None,
                role_embedded_config_set_digest: None,
                artifact_set_digest: None,
                pool_identity_set_digest: None,
                canic_version: None,
                ic_memory_version: None,
            },
            trust_domain: TrustDomainV1 {
                root_trust_anchor: None,
                migration_from: None,
            },
            fleet_template: "demo".to_string(),
            runtime_variant: "local".to_string(),
            authority_profile: AuthorityProfileV1 {
                profile_id: "local:demo-local:authority".to_string(),
                expected_controllers: Vec::new(),
                staging_controllers: Vec::new(),
                emergency_controllers: Vec::new(),
            },
            role_artifacts: Vec::new(),
            expected_canisters: Vec::new(),
            expected_pool: Vec::new(),
            expected_verifier_readiness: VerifierReadinessExpectationV1 {
                required: false,
                expected_role_epochs: Vec::new(),
            },
            unresolved_assumptions: assumptions.into_iter().collect(),
        }
    }

    fn assumption(key: &str, description: &str) -> DeploymentAssumptionV1 {
        DeploymentAssumptionV1 {
            key: key.to_string(),
            description: description.to_string(),
        }
    }

    fn expected_canister(role: &str) -> ExpectedCanisterV1 {
        ExpectedCanisterV1 {
            role: role.to_string(),
            canister_id: None,
            control_class: CanisterControlClassV1::DeploymentControlled,
        }
    }

    fn role_artifact(role: &str) -> RoleArtifactV1 {
        RoleArtifactV1 {
            role: role.to_string(),
            source: ArtifactSourceV1::LocalBuild,
            build_profile: "debug".to_string(),
            wasm_path: None,
            wasm_gz_path: None,
            wasm_gz_size_bytes: None,
            wasm_sha256: None,
            wasm_gz_sha256: None,
            wasm_gz_sha256_source: None,
            observed_wasm_gz_file_sha256: None,
            observed_wasm_gz_file_sha256_source: None,
            installed_module_hash: None,
            candid_path: None,
            candid_sha256: None,
            raw_config_sha256: None,
            canonical_embedded_config_sha256: None,
            embedded_topology_sha256: None,
            builder_version: None,
            rust_toolchain: None,
            package_version: None,
        }
    }

    fn assert_proposed_operation(
        plan: &DeploymentPlanV1,
        label: ProposedOperationKind,
        subject: &str,
    ) {
        assert!(
            proposed_operations(plan).iter().any(|operation| {
                operation.phase == FUTURE_APPLY_PREVIEW_PHASE
                    && operation.label == label
                    && operation.subject == subject
                    && operation.status == PROPOSED_OPERATION_NOT_EXECUTED
            }),
            "missing proposed operation {} for {subject}",
            label.label()
        );
    }
}
