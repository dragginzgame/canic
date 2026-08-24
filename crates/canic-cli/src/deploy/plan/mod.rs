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
use canic_core::ids::FleetName;
#[cfg(test)]
use canic_host::deployment_truth::DeploymentAssumptionV1;
use canic_host::{
    deployment_truth::{DeploymentPlanV1, LocalDeploymentPlanRequest},
    fleet_install_input::{
        FleetInstallCatalogAcquisitionV1, FleetInstallInputError,
        SubnetCatalogLoadFailureEvidenceV1, load_and_resolve_fleet_install_input,
        load_and_resolve_fleet_install_input_for_preflight,
    },
    fleet_install_plan::{
        FreshFleetDecisionAuthorityRequest, FreshFleetDeploymentPlanRequest,
        FreshFleetDeploymentPlanV1, FreshFleetOperatorFundingEvidenceV1,
        FreshFleetPreflightEffectsV1, FreshFleetPreflightRequest, PlannedCanisterCreationFunding,
        compile_fresh_fleet_deployment_plan, compile_fresh_fleet_preflight,
        fresh_fleet_maximum_operator_debit, load_fresh_fleet_decision_authority,
        observe_fresh_fleet_operator_funding,
    },
    icp::IcpCli,
    install_root::{
        FreshFleetInstallRecoveryPlanV1, InspectFreshFleetInstallRecoveryRequest,
        inspect_fresh_fleet_install_recovery, require_supported_recovery_builder,
    },
    network::resolve_canonical_network_id_from_root,
    release_build::load_finalized_release_build,
    release_set::AppConfigSnapshot,
};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use command::REPORT_COMMAND;
pub(super) use command::{DeployPlanOptions, DeployPlanRoots, usage};
use diagnostics::{
    fresh_fleet_placement_warnings, fresh_fleet_plan_blocker, plan_assumptions, plan_blockers,
    plan_warnings, target_resolution_blockers,
};
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
use report::{DeploymentPlanReport, ProposedOperationLabel, REPORT_SCHEMA_VERSION};
use report::{
    PlanDiagnosticSource, SOURCE_APP_CONFIG, SOURCE_BUILD_PROFILE, SOURCE_DEPLOYMENT_CONFIG,
    SOURCE_FLEET_CATALOG, SOURCE_FLEET_INPUT, SOURCE_LOCAL_OBSERVATION,
};

const ASSUMPTION_PREFIX_LOCAL_ARTIFACTS: &str = "local_artifacts.";
const ASSUMPTION_PREFIX_LOCAL_CONFIG: &str = "local_config.";
const ASSUMPTION_PREFIX_FLEET_CATALOG: &str = "fleet_catalog.";
const ASSUMPTION_PREFIX_UNSUPPORTED: &str = "unsupported.";
const ASSUMPTION_KEY_LOCAL_CONFIG_POOLS: &str = "local_config.pools";
const ASSUMPTION_KEY_LOCAL_CONFIG_ROLES: &str = "local_config.roles";
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
    let icp = IcpCli::new(&options.icp, Some(options.environment.clone()))
        .with_cwd(roots.icp_root.clone());
    build_report_with_operator_observer(options, roots, &|expected_principal, maximum_debit| {
        observe_fresh_fleet_operator_funding(&icp, expected_principal, maximum_debit)
    })
}

pub(super) fn build_report_with_operator_observer<E>(
    options: &DeployPlanOptions,
    roots: &DeployPlanRoots,
    observe_operator: &impl Fn(
        &str,
        &PlannedCanisterCreationFunding,
    ) -> Result<FreshFleetOperatorFundingEvidenceV1, E>,
) -> DeploymentPlanReport
where
    E: std::fmt::Display,
{
    let config_path = plan_config_path(&roots.workspace_root, options);
    let fleet_input_path = plan_fleet_input_path(&roots.icp_root, options);
    let mut blockers = target_resolution_blockers(options, &config_path, &roots.icp_root);
    let target_resolved = blockers.is_empty();
    let mut catalog_acquisition = None;
    let mut catalog_failure = None;
    let mut install_recovery = None;
    let fresh_fleet_plan = if target_resolved {
        match build_fresh_fleet_plan(
            options,
            roots,
            &config_path,
            &fleet_input_path,
            observe_operator,
        ) {
            Ok(build) => {
                catalog_acquisition = Some(build.catalog_acquisition);
                install_recovery = build.install_recovery;
                Some(build.plan)
            }
            Err(error) => {
                catalog_failure = error.catalog_failure.map(|failure| *failure);
                install_recovery = error.install_recovery.map(|recovery| *recovery);
                blockers.push(fresh_fleet_plan_blocker(
                    &options.fleet,
                    error.detail,
                    error.source,
                    options.refresh_catalog,
                ));
                None
            }
        }
    } else {
        None
    };
    let resolved_build_profile = fresh_fleet_plan.as_ref().map_or_else(
        || build_profile_name(options),
        |plan| plan.preflight.build_profile.clone(),
    );
    let resolved_release_build_id = fresh_fleet_plan
        .as_ref()
        .and_then(|plan| plan.preflight.release_build_id)
        .or(options.release_build_id)
        .or_else(|| install_recovery.as_ref().map(recovery_release_build_id));
    let mut plan = build_plan(options, roots, &config_path, &resolved_build_profile);
    plan.plan_digest = fresh_fleet_plan
        .as_ref()
        .map(|fresh_fleet_plan| fresh_fleet_plan.plan_digest.clone());
    if target_resolved {
        blockers.extend(plan_blockers(&plan));
    }
    let mut assumptions = plan_assumptions(&plan);
    let mut warnings = plan_warnings(&plan);
    if let Some(fresh_fleet_plan) = fresh_fleet_plan.as_ref() {
        warnings.extend(fresh_fleet_placement_warnings(fresh_fleet_plan));
    }
    let mut verified_facts = verified_facts(
        options,
        &config_path,
        target_resolved,
        &resolved_build_profile,
        &plan,
    );
    let proposed_operations = report_proposed_operations(&plan, install_recovery.as_ref());
    let mut next_actions = next_actions(options, &blockers, &warnings, &assumptions);
    append_recovery_next_actions(&mut next_actions, install_recovery.as_ref());
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
        fleet: options.fleet.clone(),
        app: options.app.clone(),
        environment: options.environment.clone(),
        fleet_input_path: display_path(&fleet_input_path),
        build_profile: resolved_build_profile,
        release_build_id: resolved_release_build_id.map(|identity| identity.to_string()),
        config_path: display_path(&config_path),
        status,
        comparison_status,
        catalog_acquisition,
        catalog_failure,
        fresh_fleet_plan,
        install_recovery,
        plan,
        blockers,
        warnings,
        assumptions,
        verified_facts,
        proposed_operations,
        next_actions,
    }
}

fn report_proposed_operations(
    plan: &DeploymentPlanV1,
    recovery: Option<&FreshFleetInstallRecoveryPlanV1>,
) -> Vec<ProposedOperationLabel> {
    if recovery.is_some_and(|recovery| recovery.effects_started) {
        Vec::new()
    } else {
        proposed_operations(plan)
    }
}

fn append_recovery_next_actions(
    next_actions: &mut Vec<String>,
    recovery: Option<&FreshFleetInstallRecoveryPlanV1>,
) {
    if !recovery.is_some_and(|recovery| recovery.effects_started) {
        return;
    }
    next_actions.push(
        "review install_recovery before explicitly authorizing the retained-session resume"
            .to_string(),
    );
    next_actions.push("do not start a replacement fresh Fleet install".to_string());
}

const fn recovery_release_build_id(
    recovery: &FreshFleetInstallRecoveryPlanV1,
) -> canic_core::ids::ReleaseBuildId {
    recovery.release_build_id
}

fn build_fresh_fleet_plan<E>(
    options: &DeployPlanOptions,
    roots: &DeployPlanRoots,
    config_path: &Path,
    fleet_input_path: &Path,
    observe_operator: &impl Fn(
        &str,
        &PlannedCanisterCreationFunding,
    ) -> Result<FreshFleetOperatorFundingEvidenceV1, E>,
) -> Result<FreshFleetPlanBuild, FreshFleetPreflightBuildError>
where
    E: std::fmt::Display,
{
    let config = AppConfigSnapshot::load(config_path)
        .map_err(|error| preflight_build_error(SOURCE_APP_CONFIG, error))?;
    let fleet_name = options
        .fleet
        .parse::<FleetName>()
        .map_err(|error| preflight_build_error(SOURCE_FLEET_INPUT, error))?;
    let canonical_network_id =
        resolve_canonical_network_id_from_root(&roots.icp_root, &options.environment)
            .map_err(|error| preflight_build_error(SOURCE_DEPLOYMENT_CONFIG, error))?;
    let app = canic_core::ids::AppId::from(options.app.as_str());
    let install_recovery =
        inspect_fresh_fleet_install_recovery(InspectFreshFleetInstallRecoveryRequest {
            root: &roots.icp_root,
            canonical_network_id,
            fleet_name: &fleet_name,
            app: &app,
            config: config.model(),
        })
        .map_err(|error| preflight_build_error(SOURCE_LOCAL_OBSERVATION, error))?;
    let build = (|| {
        let input = if options.refresh_catalog {
            load_and_resolve_fleet_install_input(
                &roots.icp_root,
                &options.environment,
                fleet_input_path,
            )
        } else {
            load_and_resolve_fleet_install_input_for_preflight(
                &roots.icp_root,
                &options.environment,
                fleet_input_path,
            )
        }
        .map_err(fleet_input_preflight_build_error)?;
        let catalog_acquisition = input.catalog_acquisition.clone();
        let (build_profile, release_build_id) =
            resolve_plan_release_source(options, roots, install_recovery.as_ref())
                .map_err(|error| preflight_build_error(SOURCE_BUILD_PROFILE, error))?;
        let preflight = compile_fresh_fleet_preflight(FreshFleetPreflightRequest {
            config: config.model(),
            app: &options.app,
            fleet_name: &fleet_name,
            coordinator: &input.coordinator,
            admission: &input.admission,
            fleet_subnet_roots: &input.fleet_subnet_roots,
            build_profile,
            release_build_id,
            effects: FreshFleetPreflightEffectsV1::none_started(),
        })
        .map_err(|error| preflight_build_error(SOURCE_FLEET_INPUT, error))?;
        let maximum_operator_debit = fresh_fleet_maximum_operator_debit(&preflight)
            .map_err(|error| preflight_build_error(SOURCE_FLEET_INPUT, error))?;
        let required_operator_debit = install_recovery
            .as_ref()
            .map_or(&maximum_operator_debit, |recovery| {
                &recovery.remaining_operator_debit
            });
        let operator = observe_operator(&input.operator_principal, required_operator_debit)
            .map_err(|error| preflight_build_error(SOURCE_LOCAL_OBSERVATION, error))?;
        let authority_request = FreshFleetDecisionAuthorityRequest {
            workspace_root: &roots.workspace_root,
            icp_root: &roots.icp_root,
            config: &config,
            requested_environment: &options.environment,
            canonical_network_id,
            release_build_id,
            fleet_input: &input,
            operator: &operator,
        };
        let authority = match install_recovery.as_ref() {
            Some(recovery) => recovery.load_decision_authority(authority_request),
            None => load_fresh_fleet_decision_authority(authority_request).map_err(Into::into),
        }
        .map_err(|error| preflight_build_error(SOURCE_BUILD_PROFILE, error))?;
        let decision_request = FreshFleetDeploymentPlanRequest {
            preflight,
            authority,
        };
        let plan = match install_recovery.as_ref() {
            Some(recovery) => recovery
                .compile_decision(decision_request)
                .map_err(|error| preflight_build_error(SOURCE_LOCAL_OBSERVATION, error))?,
            None => compile_fresh_fleet_deployment_plan(decision_request)
                .map_err(|error| preflight_build_error(SOURCE_FLEET_INPUT, error))?,
        };
        Ok(FreshFleetPlanBuild {
            plan,
            catalog_acquisition,
            install_recovery: install_recovery.clone(),
        })
    })();
    build.map_err(|mut error: FreshFleetPreflightBuildError| {
        error.install_recovery = install_recovery.map(Box::new);
        error
    })
}

struct FreshFleetPlanBuild {
    plan: FreshFleetDeploymentPlanV1,
    catalog_acquisition: FleetInstallCatalogAcquisitionV1,
    install_recovery: Option<FreshFleetInstallRecoveryPlanV1>,
}

struct FreshFleetPreflightBuildError {
    detail: String,
    source: PlanDiagnosticSource,
    catalog_failure: Option<Box<SubnetCatalogLoadFailureEvidenceV1>>,
    install_recovery: Option<Box<FreshFleetInstallRecoveryPlanV1>>,
}

fn preflight_build_error(
    source: PlanDiagnosticSource,
    error: impl std::fmt::Display,
) -> FreshFleetPreflightBuildError {
    FreshFleetPreflightBuildError {
        detail: error.to_string(),
        source,
        catalog_failure: None,
        install_recovery: None,
    }
}

fn fleet_input_preflight_build_error(
    error: FleetInstallInputError,
) -> FreshFleetPreflightBuildError {
    let catalog_failure = error
        .subnet_catalog_failure()
        .map(SubnetCatalogLoadFailureEvidenceV1::from_preflight_failure);
    let source = if catalog_failure.is_some() {
        SOURCE_FLEET_CATALOG
    } else {
        SOURCE_FLEET_INPUT
    };
    FreshFleetPreflightBuildError {
        detail: error.to_string(),
        source,
        catalog_failure: catalog_failure.map(Box::new),
        install_recovery: None,
    }
}

fn resolve_plan_release_source(
    options: &DeployPlanOptions,
    roots: &DeployPlanRoots,
    recovery: Option<&FreshFleetInstallRecoveryPlanV1>,
) -> Result<
    (
        canic_host::canister_build::CanisterBuildProfile,
        Option<canic_core::ids::ReleaseBuildId>,
    ),
    String,
> {
    let release_build_id = options
        .release_build_id
        .or_else(|| recovery.map(|recovery| recovery.release_build_id));
    let Some(release_build_id) = release_build_id else {
        return Ok((
            options
                .profile
                .unwrap_or(canic_host::canister_build::CanisterBuildProfile::Release),
            None,
        ));
    };
    let finalized = load_finalized_release_build(&roots.icp_root, release_build_id)
        .map_err(|error| error.to_string())?;
    if let Some(recovery) = recovery {
        if release_build_id != recovery.release_build_id {
            return Err(
                "requested release build differs from the interrupted Fleet install session"
                    .to_string(),
            );
        }
        require_supported_recovery_builder(
            &finalized.record.builder_version,
            env!("CARGO_PKG_VERSION"),
        )
        .map_err(|error| error.to_string())?;
    } else if finalized.record.builder_version != env!("CARGO_PKG_VERSION") {
        return Err(format!(
            "finalized release build belongs to Canic {}, not current Canic {}",
            finalized.record.builder_version,
            env!("CARGO_PKG_VERSION")
        ));
    }
    if options
        .profile
        .is_some_and(|requested| requested != finalized.record.build_profile)
    {
        return Err(format!(
            "requested build profile differs from finalized release build profile {}",
            finalized.record.build_profile.target_dir_name()
        ));
    }
    let decision_release_build_id = recovery.map_or(Some(release_build_id), |recovery| {
        recovery.decision_release_build_id
    });
    Ok((finalized.record.build_profile, decision_release_build_id))
}

fn build_plan(
    options: &DeployPlanOptions,
    roots: &DeployPlanRoots,
    config_path: &Path,
    build_profile: &str,
) -> DeploymentPlanV1 {
    canic_host::deployment_truth::build_local_deployment_plan(&LocalDeploymentPlanRequest {
        fleet_name: options.fleet.clone(),
        app: options.app.clone(),
        environment: options.environment.clone(),
        artifact_environment: options.environment.clone(),
        workspace_root: roots.workspace_root.clone(),
        icp_root: roots.icp_root.clone(),
        config_path: Some(config_path.to_path_buf()),
        runtime_variant: options.environment.clone(),
        build_profile: build_profile.to_string(),
    })
}

fn plan_config_path(workspace_root: &Path, options: &DeployPlanOptions) -> PathBuf {
    workspace_root
        .join("apps")
        .join(&options.app)
        .join("canic.toml")
}

fn plan_fleet_input_path(icp_root: &Path, options: &DeployPlanOptions) -> PathBuf {
    if options.fleet_input.is_absolute() {
        options.fleet_input.clone()
    } else {
        icp_root.join(&options.fleet_input)
    }
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

fn build_profile_name(options: &DeployPlanOptions) -> String {
    options
        .profile
        .unwrap_or(canic_host::canister_build::CanisterBuildProfile::Release)
        .target_dir_name()
        .to_string()
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
    use canic_host::fleet_install_input::{
        SubnetCatalogFailureCacheDispositionV1, SubnetCatalogFailureEffectsV1,
        SubnetCatalogLoadStageV1, SubnetCatalogRefreshTriggerV1,
        SubnetCatalogRegistryRecordEvidenceV1, SubnetCatalogRegistryRecordKindV1,
        SubnetCatalogRegistryValueEncodingV1, SubnetCatalogRetryabilityV1,
        SubnetCatalogSourceKindV1, SubnetCatalogSubjectV1, SubnetCatalogUnknownRetryReasonV1,
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
            ASSUMPTION_KEY_LOCAL_CONFIG_ROLES,
            "could not resolve configured roles",
        )]);

        let blockers = plan_blockers(&plan);
        let assumptions = plan_assumptions(&plan);
        let warnings = plan_warnings(&plan);

        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].category, CATEGORY_CONFIG);
        assert_eq!(blockers[0].severity, SEVERITY_BLOCKED);
        assert!(assumptions.is_empty());
        assert!(warnings.is_empty());
        assert_eq!(
            aggregate_status(&blockers, &warnings, &assumptions),
            PlanStatus::Blocked
        );
    }

    #[test]
    fn retained_install_recovery_renders_exact_session_phase_and_remaining_debit() {
        let mut report = report_with_status(PlanStatus::Blocked);
        let release_build_id = canic_core::ids::ReleaseBuildId::from_nonce(
            canic_core::ids::ReleaseBuildNonce::from_random_bytes([7; 32]),
        );
        report.install_recovery = Some(FreshFleetInstallRecoveryPlanV1 {
            schema_version: 1,
            classification:
                canic_host::install_root::FreshFleetInstallRecoveryClassificationV1::PaidEffectRecovery,
            fleet_install_operation_id: "ab".repeat(32),
            release_build_id,
            decision_release_build_id: None,
            retained_builder_version: "0.109.1".to_string(),
            fresh_fleet_plan_digest: "cd".repeat(32),
            effects_started: true,
            original_maximum_operator_debit: PlannedCanisterCreationFunding::Cycles {
                cycles: 310_000_300_000_000,
            },
            remaining_operator_debit: PlannedCanisterCreationFunding::Cycles { cycles: 0 },
            fenced_operator_creations: 3,
            total_operator_creations: 3,
            uncertain_creation_outcomes: Vec::new(),
            next_replay_phase: "fleet_subnet_root:subnet:store_bootstrap_verification".to_string(),
        });

        let text = render_text(&report);
        assert!(text.contains("status: blocked"));
        assert!(text.contains("no_effects_started: false"));
        assert!(text.contains("classification: paid_effect_recovery"));
        assert!(text.contains("decision_release_build: workspace"));
        assert!(text.contains("remaining_operator_debit: 0 cycles"));
        assert!(
            text.contains(
                "next_replay_phase: fleet_subnet_root:subnet:store_bootstrap_verification"
            )
        );
        let json = serde_json::from_str::<serde_json::Value>(
            &render_json(&report).expect("render recovery report JSON"),
        )
        .expect("valid recovery report JSON");
        assert_eq!(
            json["install_recovery"]["release_build_id"],
            release_build_id.to_string()
        );
        assert_eq!(
            json["install_recovery"]["remaining_operator_debit"]["cycles"],
            "0"
        );
        assert!(
            report_proposed_operations(&report.plan, report.install_recovery.as_ref()).is_empty(),
            "paid recovery must not relabel fenced creations as fresh proposals"
        );
        let mut next_actions = Vec::new();
        append_recovery_next_actions(&mut next_actions, report.install_recovery.as_ref());
        assert!(
            next_actions
                .iter()
                .any(|action| action.contains("retained-session resume"))
        );
        assert!(
            next_actions
                .iter()
                .any(|action| action.contains("do not start a replacement"))
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
    fn catalog_failure_rendering_preserves_typed_unknown_provenance() {
        let mut report = report_with_status(PlanStatus::Blocked);
        report.catalog_failure = Some(SubnetCatalogLoadFailureEvidenceV1 {
            schema_version: 1,
            network: "ic".to_string(),
            source_kind: Some(SubnetCatalogSourceKindV1::UncertifiedQuery),
            source_endpoints: vec!["https://ic0.app".to_string()],
            source_assurance: Some("uncertified_query".to_string()),
            minimum_assurance: "uncertified_query".to_string(),
            stage: SubnetCatalogLoadStageV1::RefreshFailed,
            registry_version: Some(881_337),
            returned_registry_value_version: Some(881_336),
            source_endpoint: Some("https://ic0.app".to_string()),
            assurance: Some("uncertified_query".to_string()),
            registry_records: vec![SubnetCatalogRegistryRecordEvidenceV1 {
                record_kind: SubnetCatalogRegistryRecordKindV1::RoutingTable,
                key: "canister_ranges_test".to_string(),
                subnet: None,
                canister_range_start: Some("aaaaa-aa".to_string()),
                requested_registry_version: 881_337,
                returned_registry_version: 881_330,
                timestamp_nanoseconds: 42,
                source_endpoint: "https://ic0.app".to_string(),
                assurance: "uncertified_query".to_string(),
                value_encoding: SubnetCatalogRegistryValueEncodingV1::Chunked,
            }],
            cache_disposition: SubnetCatalogFailureCacheDispositionV1::RefreshFailed {
                trigger: SubnetCatalogRefreshTriggerV1::Missing,
            },
            subject: Some(SubnetCatalogSubjectV1::RegistryRecord {
                record_kind: SubnetCatalogRegistryRecordKindV1::SubnetList,
                key: "subnet_list".to_string(),
                subnet: None,
                canister_range_start: None,
            }),
            code: "registry_refresh".to_string(),
            category: "network".to_string(),
            retryability: SubnetCatalogRetryabilityV1::Unknown {
                reason: SubnetCatalogUnknownRetryReasonV1::RegistryResponse,
            },
            source_message: "typed source failure".to_string(),
            effects: SubnetCatalogFailureEffectsV1 {
                build_started: false,
                workspace_mutation_started: false,
                ic_mutation_started: false,
            },
        });

        let json = render_json(&report).expect("render typed catalog failure JSON");
        let text = render_text(&report);

        assert!(json.contains("\"registry_version\": 881337"));
        assert!(json.contains("\"returned_registry_value_version\": 881336"));
        assert!(json.contains("\"value_encoding\": \"chunked\""));
        assert!(json.contains("\"kind\": \"unknown\""));
        assert!(json.contains("\"reason\": \"registry_response\""));
        assert!(text.contains("registry_version: 881337"));
        assert!(text.contains("returned_registry_value_version: 881336"));
        assert!(text.contains("source_endpoint: https://ic0.app"));
        assert!(text.contains("completed_registry_record_count: 1"));
        assert!(text.contains("value_encoding=chunked"));
        assert!(text.contains("cache_disposition: refresh_failed"));
        assert!(text.contains("refresh_trigger: missing"));
        assert!(text.contains("retryability: unknown"));
        assert!(text.contains("unknown_retry_reason: registry_response"));
        assert!(!text.contains("transient"));
    }

    #[test]
    fn catalog_acquisition_rendering_preserves_transient_provenance() {
        let mut report = report_with_status(PlanStatus::Planned);
        report.catalog_acquisition = Some(FleetInstallCatalogAcquisitionV1::ValidatedCache {
            cache_path: ".canic/ic-query/subnet-catalog.json".to_string(),
            cache_disposition: "refreshed_missing".to_string(),
            collected_at: "2026-08-21T12:00:00Z".to_string(),
        });

        let json = render_json(&report).expect("render catalog acquisition JSON");
        let text = render_text(&report);

        assert!(json.contains("\"cache_disposition\": \"refreshed_missing\""));
        assert!(text.contains("catalog acquisition provenance"));
        assert!(text.contains("cache_path: .canic/ic-query/subnet-catalog.json"));
        assert!(text.contains("cache_disposition: refreshed_missing"));
        assert!(text.contains("collected_at: 2026-08-21T12:00:00Z"));
        assert!(!json.contains("\"fresh_fleet_plan\": {"));
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
            "fleet_catalog" => SOURCE_FLEET_CATALOG,
            "fleet_input" => SOURCE_FLEET_INPUT,
            "local_observation" => SOURCE_LOCAL_OBSERVATION,
            _ => panic!("unknown diagnostic source fixture {value}"),
        }
    }

    fn report_with_status(status: PlanStatus) -> DeploymentPlanReport {
        DeploymentPlanReport {
            schema_version: REPORT_SCHEMA_VERSION,
            command: REPORT_COMMAND,
            fleet: "demo-local".to_string(),
            app: "demo".to_string(),
            environment: "local".to_string(),
            fleet_input_path: "deployments/demo-local.toml".to_string(),
            build_profile: "debug".to_string(),
            release_build_id: None,
            config_path: "apps/demo/canic.toml".to_string(),
            status,
            comparison_status: ComparisonStatus::NotRequested,
            catalog_acquisition: None,
            catalog_failure: None,
            fresh_fleet_plan: None,
            install_recovery: None,
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
            plan_digest: None,
            deployment_identity: DeploymentIdentityV1 {
                canonical_network_id: None,
                fleet_id: None,
                fleet_name: "demo-local".to_string(),
                app: "demo".to_string(),
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
            },
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
            protocol_profile_digest: None,
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
