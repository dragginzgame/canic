use super::{DeployCommandError, value_arg};
use crate::{
    cli::{
        clap::{
            flag_arg, parse_matches, path_option, render_usage, required_string,
            string_option_or_else, typed_option,
        },
        defaults::local_network,
        globals::internal_network_arg,
        help::print_help_or_version,
    },
    version_text,
};
use canic_host::{
    canister_build::CanisterBuildProfile,
    deployment_truth::{
        DeploymentAssumptionV1, DeploymentPlanV1, LocalDeploymentPlanRequest, RoleArtifactV1,
    },
    release_set::{
        configured_fleet_name, icp_root as resolve_icp_root,
        workspace_root as resolve_workspace_root,
    },
};
use clap::Command as ClapCommand;
use serde::Serialize;
use std::{
    ffi::OsString,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

const REPORT_SCHEMA_VERSION: u16 = 1;
const REPORT_COMMAND: &str = "canic deploy plan";
const SEVERITY_INFO: &str = "info";
const SEVERITY_WARNING: &str = "warning";
const SEVERITY_BLOCKED: &str = "blocked";
const SEVERITY_UNSUPPORTED: &str = "unsupported";
const CATEGORY_ARTIFACT: &str = "artifact";
const CATEGORY_AUTHORITY: &str = "authority";
const CATEGORY_CONFIG: &str = "config";
const CATEGORY_DEPLOYMENT_IDENTITY: &str = "deployment_identity";
const CATEGORY_INVENTORY: &str = "inventory";
const CATEGORY_OBSERVATION: &str = "observation";
const CATEGORY_TOPOLOGY: &str = "topology";
const CATEGORY_TRUST_DOMAIN: &str = "trust_domain";
const CATEGORY_UNSUPPORTED_SHAPE: &str = "unsupported_shape";
const CATEGORY_VERIFIER_READINESS: &str = "verifier_readiness";
const SOURCE_CLI_ARG: &str = "cli_arg";
const SOURCE_BUILD_PROFILE: &str = "build_profile";
const SOURCE_DEPLOYMENT_CONFIG: &str = "deployment_config";
const SOURCE_DEPLOYMENT_PLAN_BUILDER: &str = "deployment_plan_builder";
const SOURCE_FLEET_CONFIG: &str = "fleet_config";
const SOURCE_INSTALLED_DEPLOYMENT: &str = "installed_deployment";
const SOURCE_LOCAL_OBSERVATION: &str = "local_observation";
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
const ASSUMPTION_KEY_LOCAL_STATE_ROOT_CANISTER_ID: &str = "local_state.root_canister_id";
const ASSUMPTION_KEY_LOCAL_STATE_UNVERIFIED_ROOT_CANISTER_ID: &str =
    "local_state.unverified_root_canister_id";
const DEPLOYMENT_ARG: &str = "deployment";
const JSON_ARG: &str = "json";
const OUT_ARG: &str = "out";
const CONFIG_ARG: &str = "config";
const BUILD_PROFILE_ARG: &str = "build-profile";

const DEPLOY_PLAN_HELP_AFTER: &str = "\
Examples:
  canic deploy plan demo-local
  canic deploy plan demo-local --json
  canic deploy plan demo-local --out deployment-plan.json
  canic deploy plan demo-local --config fleets/demo/canic.toml

Builds a deterministic planning report from local project config. The command
does not install, upgrade, create canisters, write deployment truth, update
installed deployment records, or call live IC state. Future-apply preview rows
are proposed operation labels only; they are not executed and are not apply
operation objects. JSON output is a DeploymentPlanReport, not an EvidenceEnvelope,
deployment truth, or authorization to mutate. --out writes JSON only and fails if
the requested path already exists or its parent directory is missing.";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeployPlanOptions {
    pub(super) deployment: String,
    pub(super) network: String,
    pub(super) json: bool,
    pub(super) out: Option<PathBuf>,
    pub(super) config: Option<PathBuf>,
    pub(super) build_profile: CanisterBuildProfile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeployPlanRoots {
    pub(super) workspace_root: PathBuf,
    pub(super) icp_root: PathBuf,
}

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
    category: &'static str,
    code: String,
    severity: &'static str,
    subject: String,
    detail: String,
    next: Option<String>,
    source: &'static str,
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

fn target_resolution_blockers(
    options: &DeployPlanOptions,
    config_path: &Path,
) -> Vec<PlanDiagnostic> {
    if let Err(err) = validate_deployment_target_name(&options.deployment) {
        return vec![PlanDiagnostic {
            category: CATEGORY_DEPLOYMENT_IDENTITY,
            code: "deployment_target_invalid".to_string(),
            severity: SEVERITY_BLOCKED,
            subject: options.deployment.clone(),
            detail: err,
            next: Some("use letters, numbers, '-' or '_' for deployment target names".to_string()),
            source: SOURCE_CLI_ARG,
        }];
    }

    match configured_fleet_name(config_path) {
        Ok(_) => Vec::new(),
        Err(err) => vec![PlanDiagnostic {
            category: CATEGORY_CONFIG,
            code: "deployment_target_unresolved".to_string(),
            severity: SEVERITY_BLOCKED,
            subject: options.deployment.clone(),
            detail: format!(
                "deployment target {} could not be resolved from {}: {err}",
                options.deployment,
                config_path.display()
            ),
            next: Some(
                "provide --config with a readable fleet config for this deployment".to_string(),
            ),
            source: SOURCE_DEPLOYMENT_CONFIG,
        }],
    }
}

fn validate_deployment_target_name(name: &str) -> Result<(), String> {
    let valid = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
    if valid {
        Ok(())
    } else {
        Err(format!(
            "invalid deployment target name {name:?}; use letters, numbers, '-' or '_'"
        ))
    }
}

fn verified_facts(
    options: &DeployPlanOptions,
    config_path: &Path,
    target_resolved: bool,
    plan: &DeploymentPlanV1,
) -> Vec<PlanDiagnostic> {
    if !target_resolved {
        return Vec::new();
    }

    let mut facts = vec![PlanDiagnostic {
        category: CATEGORY_CONFIG,
        code: "deployment_target_resolved".to_string(),
        severity: SEVERITY_INFO,
        subject: options.deployment.clone(),
        detail: format!(
            "deployment target {} resolved from {}",
            options.deployment,
            config_path.display()
        ),
        next: None,
        source: SOURCE_FLEET_CONFIG,
    }];

    facts.push(PlanDiagnostic {
        category: CATEGORY_CONFIG,
        code: "fleet_template_resolved".to_string(),
        severity: SEVERITY_INFO,
        subject: plan.deployment_identity.deployment_name.clone(),
        detail: format!("fleet template resolved: {}", plan.fleet_template),
        next: None,
        source: SOURCE_FLEET_CONFIG,
    });
    facts.extend(plan_context_facts(options, config_path, plan));
    facts.extend(plan_identity_facts(plan));
    facts.extend(authority_profile_facts(plan));
    facts.extend(expected_role_artifact_inventory_facts(plan));
    facts.extend(expected_canister_inventory_facts(plan));
    facts.extend(expected_pool_inventory_facts(plan));
    facts.extend(role_artifact_facts(&plan.role_artifacts));
    facts.extend(trust_domain_facts(plan));
    facts.extend(verifier_readiness_facts(plan));

    if let Some(root) = &plan.trust_domain.root_trust_anchor {
        facts.push(PlanDiagnostic {
            category: CATEGORY_OBSERVATION,
            code: "installed_root_canister_id_resolved".to_string(),
            severity: SEVERITY_INFO,
            subject: options.deployment.clone(),
            detail: format!("installed deployment state resolves root canister {root}"),
            next: None,
            source: SOURCE_INSTALLED_DEPLOYMENT,
        });
    }

    facts
}

fn plan_context_facts(
    options: &DeployPlanOptions,
    config_path: &Path,
    plan: &DeploymentPlanV1,
) -> Vec<PlanDiagnostic> {
    let subject = plan.deployment_identity.deployment_name.clone();
    let mut facts = vec![
        PlanDiagnostic {
            category: CATEGORY_ARTIFACT,
            code: "build_profile_resolved".to_string(),
            severity: SEVERITY_INFO,
            subject: subject.clone(),
            detail: format!("build profile resolved: {}", build_profile_name(options)),
            next: None,
            source: SOURCE_BUILD_PROFILE,
        },
        PlanDiagnostic {
            category: CATEGORY_CONFIG,
            code: "config_path_resolved".to_string(),
            severity: SEVERITY_INFO,
            subject: subject.clone(),
            detail: format!("config path resolved: {}", config_path.display()),
            next: None,
            source: SOURCE_DEPLOYMENT_CONFIG,
        },
        PlanDiagnostic {
            category: CATEGORY_CONFIG,
            code: "runtime_variant_resolved".to_string(),
            severity: SEVERITY_INFO,
            subject: subject.clone(),
            detail: format!("runtime variant resolved: {}", plan.runtime_variant),
            next: None,
            source: SOURCE_DEPLOYMENT_PLAN_BUILDER,
        },
        PlanDiagnostic {
            category: CATEGORY_DEPLOYMENT_IDENTITY,
            code: "network_resolved".to_string(),
            severity: SEVERITY_INFO,
            subject: subject.clone(),
            detail: format!("network resolved: {}", plan.deployment_identity.network),
            next: None,
            source: SOURCE_DEPLOYMENT_PLAN_BUILDER,
        },
        PlanDiagnostic {
            category: CATEGORY_DEPLOYMENT_IDENTITY,
            code: "plan_id_resolved".to_string(),
            severity: SEVERITY_INFO,
            subject: subject.clone(),
            detail: format!("plan id resolved: {}", plan.plan_id),
            next: None,
            source: SOURCE_DEPLOYMENT_PLAN_BUILDER,
        },
    ];

    if let Some(version) = &plan.deployment_identity.canic_version {
        facts.push(PlanDiagnostic {
            category: CATEGORY_DEPLOYMENT_IDENTITY,
            code: "planner_version_resolved".to_string(),
            severity: SEVERITY_INFO,
            subject,
            detail: format!("planner version resolved: {version}"),
            next: None,
            source: SOURCE_DEPLOYMENT_PLAN_BUILDER,
        });
    }

    facts
}

fn plan_identity_facts(plan: &DeploymentPlanV1) -> Vec<PlanDiagnostic> {
    let identity = &plan.deployment_identity;
    let subject = &identity.deployment_name;
    let mut facts = Vec::new();

    if !has_plan_assumption_prefix(plan, ASSUMPTION_PREFIX_LOCAL_ARTIFACTS)
        && !has_plan_assumption_key(plan, ASSUMPTION_KEY_LOCAL_CONFIG_ROLES)
    {
        push_digest_fact(
            &mut facts,
            DigestFact {
                category: CATEGORY_ARTIFACT,
                code: "artifact_set_resolved",
                subject,
                label: "artifact set digest",
                digest: identity.artifact_set_digest.as_deref(),
                source: SOURCE_DEPLOYMENT_PLAN_BUILDER,
            },
        );
    }
    push_digest_fact(
        &mut facts,
        DigestFact {
            category: CATEGORY_ARTIFACT,
            code: "deployment_manifest_resolved",
            subject,
            label: "deployment manifest digest",
            digest: identity.deployment_manifest_digest.as_deref(),
            source: SOURCE_DEPLOYMENT_PLAN_BUILDER,
        },
    );
    if !has_plan_assumption_key(plan, ASSUMPTION_KEY_LOCAL_CONFIG_CONTROLLERS) {
        push_digest_fact(
            &mut facts,
            DigestFact {
                category: CATEGORY_AUTHORITY,
                code: "authority_profile_resolved",
                subject,
                label: "authority profile hash",
                digest: identity.authority_profile_hash.as_deref(),
                source: SOURCE_DEPLOYMENT_PLAN_BUILDER,
            },
        );
    }
    push_digest_fact(
        &mut facts,
        DigestFact {
            category: CATEGORY_CONFIG,
            code: "canonical_runtime_config_resolved",
            subject,
            label: "canonical runtime config digest",
            digest: identity.canonical_runtime_config_digest.as_deref(),
            source: SOURCE_DEPLOYMENT_CONFIG,
        },
    );
    if !has_plan_assumption_key(plan, ASSUMPTION_KEY_LOCAL_CONFIG_POOLS) {
        push_digest_fact(
            &mut facts,
            DigestFact {
                category: CATEGORY_TOPOLOGY,
                code: "pool_identity_set_resolved",
                subject,
                label: "pool identity set digest",
                digest: identity.pool_identity_set_digest.as_deref(),
                source: SOURCE_DEPLOYMENT_PLAN_BUILDER,
            },
        );
    }
    if !has_plan_assumption_key(plan, ASSUMPTION_KEY_LOCAL_CONFIG_ROLES) {
        push_digest_fact(
            &mut facts,
            DigestFact {
                category: CATEGORY_TOPOLOGY,
                code: "role_topology_resolved",
                subject,
                label: "role topology hash",
                digest: identity.role_topology_hash.as_deref(),
                source: SOURCE_DEPLOYMENT_PLAN_BUILDER,
            },
        );
    }

    facts
}

fn has_plan_assumption_key(plan: &DeploymentPlanV1, key: &str) -> bool {
    plan.unresolved_assumptions
        .iter()
        .any(|assumption| assumption.key == key)
}

fn has_plan_assumption_prefix(plan: &DeploymentPlanV1, prefix: &str) -> bool {
    plan.unresolved_assumptions
        .iter()
        .any(|assumption| assumption.key.starts_with(prefix))
}

struct DigestFact<'a> {
    category: &'static str,
    code: &'static str,
    subject: &'a str,
    label: &'static str,
    digest: Option<&'a str>,
    source: &'static str,
}

fn push_digest_fact(facts: &mut Vec<PlanDiagnostic>, fact: DigestFact<'_>) {
    if let Some(digest) = fact.digest {
        facts.push(PlanDiagnostic {
            category: fact.category,
            code: fact.code.to_string(),
            severity: SEVERITY_INFO,
            subject: fact.subject.to_string(),
            detail: format!("{} resolved: {digest}", fact.label),
            next: None,
            source: fact.source,
        });
    }
}

fn authority_profile_facts(plan: &DeploymentPlanV1) -> Vec<PlanDiagnostic> {
    if has_plan_assumption_key(plan, ASSUMPTION_KEY_LOCAL_CONFIG_CONTROLLERS) {
        return Vec::new();
    }

    let expected_count = plan.authority_profile.expected_controllers.len();
    vec![PlanDiagnostic {
        category: CATEGORY_AUTHORITY,
        code: "expected_controller_set_resolved".to_string(),
        severity: SEVERITY_INFO,
        subject: plan.deployment_identity.deployment_name.clone(),
        detail: format!("expected controller set resolved: {expected_count} controller(s)"),
        next: None,
        source: SOURCE_DEPLOYMENT_PLAN_BUILDER,
    }]
}

fn expected_role_artifact_inventory_facts(plan: &DeploymentPlanV1) -> Vec<PlanDiagnostic> {
    if has_plan_assumption_key(plan, ASSUMPTION_KEY_LOCAL_CONFIG_ROLES) {
        return Vec::new();
    }

    let expected_count = plan.role_artifacts.len();
    vec![PlanDiagnostic {
        category: CATEGORY_ARTIFACT,
        code: "expected_role_artifact_inventory_resolved".to_string(),
        severity: SEVERITY_INFO,
        subject: plan.deployment_identity.deployment_name.clone(),
        detail: format!("expected role artifact inventory resolved: {expected_count} role(s)"),
        next: None,
        source: SOURCE_DEPLOYMENT_PLAN_BUILDER,
    }]
}

fn expected_canister_inventory_facts(plan: &DeploymentPlanV1) -> Vec<PlanDiagnostic> {
    if has_plan_assumption_key(plan, ASSUMPTION_KEY_LOCAL_CONFIG_ROLES) {
        return Vec::new();
    }

    let expected_count = plan.expected_canisters.len();
    vec![PlanDiagnostic {
        category: CATEGORY_INVENTORY,
        code: "expected_canister_inventory_resolved".to_string(),
        severity: SEVERITY_INFO,
        subject: plan.deployment_identity.deployment_name.clone(),
        detail: format!("expected canister inventory resolved: {expected_count} canister role(s)"),
        next: None,
        source: SOURCE_DEPLOYMENT_PLAN_BUILDER,
    }]
}

fn expected_pool_inventory_facts(plan: &DeploymentPlanV1) -> Vec<PlanDiagnostic> {
    if has_plan_assumption_key(plan, ASSUMPTION_KEY_LOCAL_CONFIG_POOLS) {
        return Vec::new();
    }

    let expected_count = plan.expected_pool.len();
    vec![PlanDiagnostic {
        category: CATEGORY_INVENTORY,
        code: "expected_pool_inventory_resolved".to_string(),
        severity: SEVERITY_INFO,
        subject: plan.deployment_identity.deployment_name.clone(),
        detail: format!(
            "expected pool inventory resolved: {expected_count} pool canister expectation(s)"
        ),
        next: None,
        source: SOURCE_DEPLOYMENT_PLAN_BUILDER,
    }]
}

fn role_artifact_facts(artifacts: &[RoleArtifactV1]) -> Vec<PlanDiagnostic> {
    artifacts
        .iter()
        .filter_map(|artifact| {
            artifact
                .observed_wasm_gz_file_sha256
                .as_ref()
                .map(|digest| PlanDiagnostic {
                    category: CATEGORY_ARTIFACT,
                    code: "role_artifact_observed".to_string(),
                    severity: SEVERITY_INFO,
                    subject: artifact.role.clone(),
                    detail: role_artifact_fact_detail(artifact, digest),
                    next: None,
                    source: SOURCE_LOCAL_OBSERVATION,
                })
        })
        .collect()
}

fn role_artifact_fact_detail(artifact: &RoleArtifactV1, digest: &str) -> String {
    match &artifact.wasm_gz_path {
        Some(path) => format!("observed wasm artifact {path} with sha256 {digest}"),
        None => format!("observed wasm artifact sha256 {digest}"),
    }
}

fn trust_domain_facts(plan: &DeploymentPlanV1) -> Vec<PlanDiagnostic> {
    let mut facts = Vec::new();
    let subject = plan.deployment_identity.deployment_name.clone();

    if let Some(root) = &plan.trust_domain.root_trust_anchor {
        facts.push(PlanDiagnostic {
            category: CATEGORY_TRUST_DOMAIN,
            code: "root_trust_anchor_resolved".to_string(),
            severity: SEVERITY_INFO,
            subject: subject.clone(),
            detail: format!("root trust anchor resolved: {root}"),
            next: None,
            source: SOURCE_INSTALLED_DEPLOYMENT,
        });
    }

    if let Some(migration_from) = &plan.trust_domain.migration_from {
        facts.push(PlanDiagnostic {
            category: CATEGORY_TRUST_DOMAIN,
            code: "migration_trust_anchor_resolved".to_string(),
            severity: SEVERITY_INFO,
            subject,
            detail: format!("migration trust anchor resolved: {migration_from}"),
            next: None,
            source: SOURCE_DEPLOYMENT_PLAN_BUILDER,
        });
    }

    facts
}

fn verifier_readiness_facts(plan: &DeploymentPlanV1) -> Vec<PlanDiagnostic> {
    if !verifier_readiness_required(plan) {
        return Vec::new();
    }

    let role_epoch_count = plan.expected_verifier_readiness.expected_role_epochs.len();
    let detail = if role_epoch_count == 0 {
        "verifier readiness is required by the deployment plan".to_string()
    } else {
        format!("verifier readiness is required for {role_epoch_count} role epoch expectation(s)")
    };

    vec![PlanDiagnostic {
        category: CATEGORY_VERIFIER_READINESS,
        code: "verifier_readiness_expectation_resolved".to_string(),
        severity: SEVERITY_INFO,
        subject: plan.deployment_identity.deployment_name.clone(),
        detail,
        next: None,
        source: SOURCE_DEPLOYMENT_PLAN_BUILDER,
    }]
}

fn plan_assumptions(plan: &DeploymentPlanV1) -> Vec<PlanDiagnostic> {
    plan.unresolved_assumptions
        .iter()
        .filter(|assumption| !is_unsupported_plan_assumption(&assumption.key))
        .filter(|assumption| !is_blocking_plan_assumption(&assumption.key))
        .filter(|assumption| !is_warning_plan_assumption(&assumption.key))
        .map(assumption_diagnostic)
        .collect()
}

fn plan_blockers(plan: &DeploymentPlanV1) -> Vec<PlanDiagnostic> {
    plan.unresolved_assumptions
        .iter()
        .filter(|assumption| {
            is_unsupported_plan_assumption(&assumption.key)
                || is_blocking_plan_assumption(&assumption.key)
        })
        .map(blocking_assumption_diagnostic)
        .collect()
}

fn is_unsupported_plan_assumption(key: &str) -> bool {
    key.starts_with(ASSUMPTION_PREFIX_UNSUPPORTED)
}

fn is_blocking_plan_assumption(key: &str) -> bool {
    key.starts_with(ASSUMPTION_PREFIX_LOCAL_CONFIG)
        || key == ASSUMPTION_KEY_LOCAL_STATE_UNVERIFIED_ROOT_CANISTER_ID
}

fn is_warning_plan_assumption(key: &str) -> bool {
    key.starts_with(ASSUMPTION_PREFIX_LOCAL_STATE) && !is_blocking_plan_assumption(key)
}

fn blocking_assumption_diagnostic(assumption: &DeploymentAssumptionV1) -> PlanDiagnostic {
    let unsupported = is_unsupported_plan_assumption(&assumption.key);
    PlanDiagnostic {
        category: if unsupported {
            CATEGORY_UNSUPPORTED_SHAPE
        } else {
            assumption_category(&assumption.key)
        },
        code: diagnostic_code(&assumption.key),
        severity: if unsupported {
            SEVERITY_UNSUPPORTED
        } else {
            SEVERITY_BLOCKED
        },
        subject: assumption.key.clone(),
        detail: assumption.description.clone(),
        next: Some(blocking_assumption_next(&assumption.key)),
        source: SOURCE_DEPLOYMENT_PLAN_BUILDER,
    }
}

fn blocking_assumption_next(key: &str) -> String {
    if is_unsupported_plan_assumption(key) {
        "change the desired deployment shape to one supported by canic deploy plan".to_string()
    } else if key == ASSUMPTION_KEY_LOCAL_STATE_UNVERIFIED_ROOT_CANISTER_ID {
        "run canic deploy check and verify the registered root before planning apply".to_string()
    } else {
        "repair the local fleet config before planning apply".to_string()
    }
}

fn plan_warnings(plan: &DeploymentPlanV1) -> Vec<PlanDiagnostic> {
    plan.unresolved_assumptions
        .iter()
        .filter(|assumption| is_warning_plan_assumption(&assumption.key))
        .map(|assumption| PlanDiagnostic {
            category: CATEGORY_OBSERVATION,
            code: local_state_warning_code(assumption),
            severity: SEVERITY_WARNING,
            subject: plan.deployment_identity.deployment_name.clone(),
            detail: assumption.description.clone(),
            next: Some(
                "run canic deploy check after installation or provide saved evidence".to_string(),
            ),
            source: SOURCE_INSTALLED_DEPLOYMENT,
        })
        .collect()
}

fn local_state_warning_code(assumption: &DeploymentAssumptionV1) -> String {
    if is_observed_state_drift_assumption(assumption) {
        "observed_inventory_drift".to_string()
    } else if assumption.key == ASSUMPTION_KEY_LOCAL_STATE_ROOT_CANISTER_ID {
        "observed_inventory_unavailable".to_string()
    } else {
        diagnostic_code(&assumption.key)
    }
}

fn assumption_diagnostic(assumption: &DeploymentAssumptionV1) -> PlanDiagnostic {
    PlanDiagnostic {
        category: assumption_category(&assumption.key),
        code: diagnostic_code(&assumption.key),
        severity: SEVERITY_WARNING,
        subject: assumption.key.clone(),
        detail: assumption.description.clone(),
        next: assumption_next(&assumption.key),
        source: SOURCE_DEPLOYMENT_PLAN_BUILDER,
    }
}

fn assumption_category(key: &str) -> &'static str {
    if key.contains("artifact") || key.contains("manifest") {
        CATEGORY_ARTIFACT
    } else if key.contains("state") {
        CATEGORY_OBSERVATION
    } else if key.contains("controller") {
        CATEGORY_AUTHORITY
    } else if key.contains("pool") {
        CATEGORY_TOPOLOGY
    } else {
        CATEGORY_CONFIG
    }
}

fn assumption_next(key: &str) -> Option<String> {
    if key.contains("artifact") || key.contains("manifest") {
        Some("run canic build or provide a build profile with resolved artifacts".to_string())
    } else if key.contains("local_state") {
        Some("compare after first deployment or provide deployment-check evidence".to_string())
    } else {
        None
    }
}

fn diagnostic_code(key: &str) -> String {
    let mut code = String::new();
    for ch in key.chars() {
        if ch.is_ascii_alphanumeric() {
            code.push(ch.to_ascii_lowercase());
        } else if !code.ends_with('_') {
            code.push('_');
        }
    }
    code.trim_matches('_').to_string()
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

const fn verifier_readiness_required(plan: &DeploymentPlanV1) -> bool {
    plan.expected_verifier_readiness.required
        || !plan
            .expected_verifier_readiness
            .expected_role_epochs
            .is_empty()
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
        assumption.key == ASSUMPTION_KEY_LOCAL_STATE_ROOT_CANISTER_ID
            && !is_observed_state_drift_assumption(assumption)
    })
}

fn is_observed_state_drift_assumption(assumption: &DeploymentAssumptionV1) -> bool {
    assumption.key == ASSUMPTION_KEY_LOCAL_STATE_ROOT_CANISTER_ID
        && assumption.description.contains(" has network ")
}

fn sort_diagnostics(diagnostics: &mut [PlanDiagnostic]) {
    diagnostics.sort_by(|left, right| {
        diagnostic_severity_rank(left.severity)
            .cmp(&diagnostic_severity_rank(right.severity))
            .then_with(|| left.severity.cmp(right.severity))
            .then_with(|| left.category.cmp(right.category))
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.subject.cmp(&right.subject))
            .then_with(|| left.source.cmp(right.source))
    });
}

fn diagnostic_severity_rank(severity: &str) -> u8 {
    match severity {
        SEVERITY_BLOCKED => 0,
        SEVERITY_UNSUPPORTED => 1,
        SEVERITY_WARNING => 2,
        SEVERITY_INFO => 3,
        _ => 4,
    }
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

pub(super) fn write_report(
    options: &DeployPlanOptions,
    report: &DeploymentPlanReport,
) -> Result<(), DeployCommandError> {
    if let Some(out) = &options.out {
        write_json_new(out, report)?;
    }

    if options.json {
        print_json(report)
    } else {
        println!("{}", render_text(report));
        Ok(())
    }
}

pub(super) fn command_exit_result(report: &DeploymentPlanReport) -> Result<(), DeployCommandError> {
    match report.status {
        PlanStatus::Planned | PlanStatus::Warning => Ok(()),
        PlanStatus::Blocked | PlanStatus::Unsupported => Err(DeployCommandError::PlanBlocked(
            report.status.as_str().to_string(),
        )),
    }
}

fn write_json_new(path: &Path, report: &DeploymentPlanReport) -> Result<(), DeployCommandError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(plan_output_error)?;
    let data = render_json(report)?;
    file.write_all(data.as_bytes()).map_err(plan_output_error)?;
    file.write_all(b"\n").map_err(plan_output_error)?;
    Ok(())
}

fn print_json(report: &DeploymentPlanReport) -> Result<(), DeployCommandError> {
    let json = render_json(report)?;
    println!("{json}");
    Ok(())
}

pub(super) fn render_json(report: &DeploymentPlanReport) -> Result<String, DeployCommandError> {
    serde_json::to_string_pretty(report).map_err(plan_output_error)
}

fn plan_output_error(err: impl std::error::Error + 'static) -> DeployCommandError {
    DeployCommandError::PlanOutput(Box::new(err))
}

pub(super) fn render_text(report: &DeploymentPlanReport) -> String {
    let mut lines = vec![
        "Deployment plan".to_string(),
        format!("schema_version: {}", report.schema_version),
        format!("command: {}", report.command),
        format!("status: {}", report.status.as_str()),
        format!("comparison: {}", report.comparison_status.as_str()),
        format!("target: {}", report.target),
        format!("network: {}", report.network),
        format!("config: {}", report.config_path),
        format!("build_profile: {}", report.build_profile),
        String::new(),
    ];

    append_diagnostics(&mut lines, "blockers", &report.blockers);
    append_diagnostics(&mut lines, "warnings", &report.warnings);
    append_diagnostics(&mut lines, "assumptions", &report.assumptions);
    append_diagnostics(&mut lines, "verified facts", &report.verified_facts);
    append_operations(&mut lines, &report.proposed_operations);
    append_next_actions(&mut lines, &report.next_actions);

    lines.join("\n")
}

fn append_diagnostics(lines: &mut Vec<String>, label: &str, diagnostics: &[PlanDiagnostic]) {
    if diagnostics.is_empty() {
        return;
    }

    lines.push(label.to_string());
    for diagnostic in diagnostics {
        lines.push(format!(
            "  [{}] {} {}",
            diagnostic.severity, diagnostic.category, diagnostic.code
        ));
        lines.push(format!("    subject: {}", diagnostic.subject));
        lines.push(format!("    detail: {}", diagnostic.detail));
        lines.push(format!("    source: {}", diagnostic.source));
        if let Some(next) = &diagnostic.next {
            lines.push(format!("    next: {next}"));
        }
    }
    lines.push(String::new());
}

fn append_operations(lines: &mut Vec<String>, operations: &[ProposedOperationLabel]) {
    if operations.is_empty() {
        return;
    }

    lines.push("future apply preview (proposed operation labels; not executed)".to_string());
    for operation in operations {
        lines.push(format!(
            "  - phase: {} label: {} subject: {} status: {}",
            operation.phase.label(),
            operation.label.label(),
            operation.subject,
            operation.status.label()
        ));
    }
    lines.push(String::new());
}

fn append_next_actions(lines: &mut Vec<String>, actions: &[String]) {
    if actions.is_empty() {
        return;
    }

    lines.push("next actions".to_string());
    for action in actions {
        lines.push(format!("  - {action}"));
    }
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

impl DeployPlanOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| DeployCommandError::Usage(usage()))?;
        Ok(Self {
            deployment: required_string(&matches, DEPLOYMENT_ARG),
            network: string_option_or_else(&matches, "network", local_network),
            json: matches.get_flag(JSON_ARG),
            out: path_option(&matches, OUT_ARG),
            config: path_option(&matches, CONFIG_ARG),
            build_profile: typed_option(&matches, BUILD_PROFILE_ARG)
                .unwrap_or_else(CanisterBuildProfile::current),
        })
    }
}

impl DeployPlanRoots {
    fn discover() -> Result<Self, DeployCommandError> {
        Ok(Self {
            workspace_root: resolve_workspace_root().map_err(DeployCommandError::from)?,
            icp_root: resolve_icp_root().map_err(DeployCommandError::from)?,
        })
    }
}

pub(super) fn command() -> ClapCommand {
    ClapCommand::new("plan")
        .bin_name(REPORT_COMMAND)
        .about("Explain the deterministic deployment plan without mutation")
        .disable_help_flag(true)
        .override_usage("canic deploy plan <deployment>")
        .arg(deployment_arg())
        .arg(json_arg())
        .arg(out_arg())
        .arg(config_arg())
        .arg(build_profile_arg())
        .arg(internal_network_arg())
        .after_help(DEPLOY_PLAN_HELP_AFTER)
}

fn deployment_arg() -> clap::Arg {
    value_arg(DEPLOYMENT_ARG)
        .value_name(DEPLOYMENT_ARG)
        .required(true)
        .help("Deployment target name to plan")
}

fn json_arg() -> clap::Arg {
    flag_arg(JSON_ARG)
        .long(JSON_ARG)
        .help("Print JSON DeploymentPlanReport to stdout")
}

fn out_arg() -> clap::Arg {
    value_arg(OUT_ARG)
        .long(OUT_ARG)
        .value_name("path")
        .num_args(1)
        .help("Write JSON DeploymentPlanReport to a new file")
}

fn config_arg() -> clap::Arg {
    value_arg(CONFIG_ARG)
        .long(CONFIG_ARG)
        .value_name("path")
        .num_args(1)
        .help("Fleet config path used to build the desired plan")
}

fn build_profile_arg() -> clap::Arg {
    value_arg(BUILD_PROFILE_ARG)
        .long(BUILD_PROFILE_ARG)
        .value_name("debug|fast|release")
        .num_args(1)
        .value_parser(clap::value_parser!(CanisterBuildProfile))
        .help("Expected canister wasm build profile")
}

pub(super) fn usage() -> String {
    render_usage(command)
}

impl PlanStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Warning => SEVERITY_WARNING,
            Self::Blocked => SEVERITY_BLOCKED,
            Self::Unsupported => SEVERITY_UNSUPPORTED,
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
            diagnostic.severity,
            diagnostic.category,
            diagnostic.code,
            diagnostic.subject,
            diagnostic.source
        )
    }

    fn diagnostic_fixture(key: &'static str) -> PlanDiagnostic {
        let [severity, category, code, subject, source] = key
            .split('|')
            .collect::<Vec<_>>()
            .try_into()
            .expect("diagnostic fixture keys contain five fields");
        PlanDiagnostic {
            category,
            code: code.to_string(),
            severity,
            subject: subject.to_string(),
            detail: "diagnostic detail".to_string(),
            next: None,
            source,
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
