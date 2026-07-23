//! Module: canic_cli::deploy::plan::evidence
//!
//! Responsibility: construct verified deployment-plan evidence.
//! Does not own: blocker classification, comparison, proposed operations, or rendering.
//! Boundary: maps resolved plan inputs into informational report diagnostics.

use crate::deploy::plan::{
    ASSUMPTION_KEY_LOCAL_CONFIG_CONTROLLERS, ASSUMPTION_KEY_LOCAL_CONFIG_POOLS,
    ASSUMPTION_KEY_LOCAL_CONFIG_ROLES, ASSUMPTION_PREFIX_LOCAL_ARTIFACTS, build_profile_name,
    command::DeployPlanOptions,
    report::{
        CATEGORY_ARTIFACT, CATEGORY_AUTHORITY, CATEGORY_CONFIG, CATEGORY_DEPLOYMENT_IDENTITY,
        CATEGORY_INVENTORY, CATEGORY_OBSERVATION, CATEGORY_TOPOLOGY, CATEGORY_TRUST_DOMAIN,
        CATEGORY_VERIFIER_READINESS, PlanDiagnostic, PlanDiagnosticCategory, PlanDiagnosticSource,
        SEVERITY_INFO, SOURCE_APP_CONFIG, SOURCE_BUILD_PROFILE, SOURCE_DEPLOYMENT_CONFIG,
        SOURCE_DEPLOYMENT_PLAN_BUILDER, SOURCE_INSTALLED_DEPLOYMENT, SOURCE_LOCAL_OBSERVATION,
    },
};
use std::path::Path;

use canic_host::deployment_truth::{DeploymentPlanV1, RoleArtifactV1};

pub(super) fn verified_facts(
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
        source: SOURCE_APP_CONFIG,
    }];

    facts.push(PlanDiagnostic {
        category: CATEGORY_CONFIG,
        code: "fleet_template_resolved".to_string(),
        severity: SEVERITY_INFO,
        subject: plan.deployment_identity.deployment_name.clone(),
        detail: format!("fleet template resolved: {}", plan.fleet_template),
        next: None,
        source: SOURCE_APP_CONFIG,
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
            code: "environment_resolved".to_string(),
            severity: SEVERITY_INFO,
            subject: subject.clone(),
            detail: format!(
                "environment resolved: {}",
                plan.deployment_identity.environment
            ),
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
    category: PlanDiagnosticCategory,
    code: &'static str,
    subject: &'a str,
    label: &'static str,
    digest: Option<&'a str>,
    source: PlanDiagnosticSource,
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

pub(super) fn verifier_readiness_facts(plan: &DeploymentPlanV1) -> Vec<PlanDiagnostic> {
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

pub(super) const fn verifier_readiness_required(plan: &DeploymentPlanV1) -> bool {
    plan.expected_verifier_readiness.required
        || !plan
            .expected_verifier_readiness
            .expected_role_epochs
            .is_empty()
}
