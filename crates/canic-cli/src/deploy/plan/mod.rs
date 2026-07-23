//! Module: canic_cli::deploy::plan
//!
//! Responsibility: orchestrate deterministic deployment planning and report assembly.
//! Does not own: deployment mutation, report rendering, or output persistence.
//! Boundary: resolves local planning evidence and delegates report output to its owner.

mod command;
mod diagnostics;
mod evidence;
mod outcome;
mod render;
mod report;

use super::DeployCommandError;
use crate::{cli::help::print_help_or_version, version_text};
#[cfg(test)]
use canic_host::deployment_truth::DeploymentAssumptionV1;
use canic_host::deployment_truth::{DeploymentPlanV1, LocalDeploymentPlanRequest};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use command::REPORT_COMMAND;
pub(super) use command::{DeployPlanOptions, DeployPlanRoots, usage};
use diagnostics::{plan_assumptions, plan_blockers, plan_warnings, target_resolution_blockers};
use evidence::verified_facts;
#[cfg(test)]
use evidence::verifier_readiness_facts;
use outcome::{
    aggregate_status, comparison_status, next_actions, proposed_operations, sort_diagnostics,
};
#[cfg(test)]
use outcome::{operation, sort_proposed_operations};
pub(super) use render::{command_exit_result, write_report};
#[cfg(test)]
pub(super) use render::{render_json, render_text};
use report::{DeploymentPlanReport, REPORT_SCHEMA_VERSION};

const ASSUMPTION_PREFIX_LOCAL_ARTIFACTS: &str = "local_artifacts.";
const ASSUMPTION_PREFIX_LOCAL_CONFIG: &str = "local_config.";
const ASSUMPTION_PREFIX_LOCAL_STATE: &str = "local_state.";
const ASSUMPTION_PREFIX_UNSUPPORTED: &str = "unsupported.";
const ASSUMPTION_KEY_LOCAL_CONFIG_CONTROLLERS: &str = "local_config.controllers";
const ASSUMPTION_KEY_LOCAL_CONFIG_POOLS: &str = "local_config.pools";
const ASSUMPTION_KEY_LOCAL_CONFIG_ROLES: &str = "local_config.roles";
const ASSUMPTION_KEY_LOCAL_STATE_UNVERIFIED_ROOT_CANISTER_ID: &str =
    "local_state.unverified_root_canister_id";
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
        environment: options.environment.clone(),
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
        environment: options.environment.clone(),
        artifact_environment: options.environment.clone(),
        workspace_root: roots.workspace_root.clone(),
        icp_root: roots.icp_root.clone(),
        config_path: Some(config_path.to_path_buf()),
        runtime_variant: options.environment.clone(),
        build_profile: build_profile_name(options),
    })
}

fn plan_config_path(workspace_root: &Path, options: &DeployPlanOptions) -> PathBuf {
    let config = options.config.clone().unwrap_or_else(|| {
        PathBuf::from("apps")
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

#[cfg(test)]
mod tests {
    use super::report::*;
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
            "warning|artifact|artifact_gap|beta|app_config",
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
                "warning|artifact|artifact_gap|beta|app_config",
                "warning|artifact|artifact_gap|beta|deployment_plan_builder",
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
            "app_config" => SOURCE_APP_CONFIG,
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
            environment: "local".to_string(),
            build_profile: "debug".to_string(),
            config_path: "apps/demo/canic.toml".to_string(),
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
                environment: "local".to_string(),
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
