use super::super::external as deploy_external;
use super::fixtures::*;
use super::*;

#[test]
fn deploy_external_leaf_commands_default_to_json() {
    let external_plan = deploy_external::DeployExternalOptions::parse(
        [OsString::from("demo")],
        deploy_external::plan_command,
        deploy_external::plan_usage,
    )
    .expect("parse deploy external plan");
    let external_check = deploy_external::DeployExternalOptions::parse(
        [OsString::from("demo")],
        deploy_external::check_command,
        deploy_external::check_usage,
    )
    .expect("parse deploy external check");
    let external_handoff = deploy_external::DeployExternalOptions::parse(
        [OsString::from("demo")],
        deploy_external::handoff_command,
        deploy_external::handoff_usage,
    )
    .expect("parse deploy external handoff");
    let external_proposals = deploy_external::DeployExternalOptions::parse(
        [OsString::from("demo")],
        deploy_external::proposals_command,
        deploy_external::proposals_usage,
    )
    .expect("parse deploy external proposals");
    let external_pending = deploy_external::DeployExternalOptions::parse(
        [OsString::from("demo")],
        deploy_external::pending_command,
        deploy_external::pending_usage,
    )
    .expect("parse deploy external pending");

    for options in [
        external_plan,
        external_check,
        external_handoff,
        external_proposals,
        external_pending,
    ] {
        assert_eq!(options.truth.deployment, "demo");
        assert_eq!(options.format, output_format::ExternalOutputFormat::Json);
    }
    let critical_fix = deploy_external::DeployExternalCriticalFixOptions::parse(
        [
            OsString::from("--fix-id"),
            OsString::from("fix-2026-05"),
            OsString::from("--severity"),
            OsString::from("critical"),
            OsString::from("demo"),
        ],
        deploy_external::critical_fix_command,
        deploy_external::critical_fix_usage,
    )
    .expect("parse deploy external critical-fix");
    assert_eq!(critical_fix.truth.deployment, "demo");
    assert_eq!(
        critical_fix.format,
        output_format::ExternalOutputFormat::Json
    );
    assert_eq!(critical_fix.fix_id, "fix-2026-05");
    assert_eq!(critical_fix.severity, "critical");
    let verify = deploy_external::DeployExternalVerifyOptions::parse(
        [
            OsString::from("--request"),
            OsString::from("external-verification.json"),
        ],
        deploy_external::verify_command,
        deploy_external::verify_usage,
    )
    .expect("parse deploy external verify");
    assert_eq!(verify.request, PathBuf::from("external-verification.json"));
    assert_eq!(verify.format, output_format::ExternalOutputFormat::Json);
    let consent = deploy_external::DeployExternalInspectOptions::parse(
        [
            OsString::from("--request"),
            OsString::from("external-consent.json"),
        ],
        deploy_external::consent_command,
        deploy_external::consent_usage,
    )
    .expect("parse deploy external inspect consent");
    assert_eq!(consent.request, PathBuf::from("external-consent.json"));
    assert_eq!(consent.format, output_format::ExternalOutputFormat::Json);
}

#[test]
fn deploy_external_leaf_commands_parse_text_format() {
    let external_plan = deploy_external::DeployExternalOptions::parse(
        [
            OsString::from("--format"),
            OsString::from("text"),
            OsString::from("demo"),
        ],
        deploy_external::plan_command,
        deploy_external::plan_usage,
    )
    .expect("parse deploy external plan text");
    let external_check = deploy_external::DeployExternalOptions::parse(
        [
            OsString::from("--format"),
            OsString::from("text"),
            OsString::from("demo"),
        ],
        deploy_external::check_command,
        deploy_external::check_usage,
    )
    .expect("parse deploy external check text");
    let external_handoff = deploy_external::DeployExternalOptions::parse(
        [
            OsString::from("--format"),
            OsString::from("text"),
            OsString::from("demo"),
        ],
        deploy_external::handoff_command,
        deploy_external::handoff_usage,
    )
    .expect("parse deploy external handoff text");
    let external_proposals = deploy_external::DeployExternalOptions::parse(
        [
            OsString::from("--format"),
            OsString::from("text"),
            OsString::from("demo"),
        ],
        deploy_external::proposals_command,
        deploy_external::proposals_usage,
    )
    .expect("parse deploy external proposals text");
    let external_pending = deploy_external::DeployExternalOptions::parse(
        [
            OsString::from("--format"),
            OsString::from("text"),
            OsString::from("demo"),
        ],
        deploy_external::pending_command,
        deploy_external::pending_usage,
    )
    .expect("parse deploy external pending text");

    assert_eq!(external_plan.truth.deployment, "demo");
    assert_eq!(
        external_plan.format,
        output_format::ExternalOutputFormat::Text
    );
    assert_eq!(external_check.truth.deployment, "demo");
    assert_eq!(
        external_check.format,
        output_format::ExternalOutputFormat::Text
    );
    assert_eq!(external_handoff.truth.deployment, "demo");
    assert_eq!(
        external_handoff.format,
        output_format::ExternalOutputFormat::Text
    );
    assert_eq!(external_proposals.truth.deployment, "demo");
    assert_eq!(
        external_proposals.format,
        output_format::ExternalOutputFormat::Text
    );
    assert_eq!(external_pending.truth.deployment, "demo");
    assert_eq!(
        external_pending.format,
        output_format::ExternalOutputFormat::Text
    );
}

#[test]
fn deploy_external_request_commands_parse_text_format() {
    let critical_fix = deploy_external::DeployExternalCriticalFixOptions::parse(
        [
            OsString::from("--fix-id"),
            OsString::from("fix-2026-05"),
            OsString::from("--severity"),
            OsString::from("critical"),
            OsString::from("--format"),
            OsString::from("text"),
            OsString::from("demo"),
        ],
        deploy_external::critical_fix_command,
        deploy_external::critical_fix_usage,
    )
    .expect("parse deploy external critical-fix text");
    assert_eq!(critical_fix.truth.deployment, "demo");
    assert_eq!(
        critical_fix.format,
        output_format::ExternalOutputFormat::Text
    );
    assert_eq!(critical_fix.fix_id, "fix-2026-05");
    assert_eq!(critical_fix.severity, "critical");
    let verify = deploy_external::DeployExternalVerifyOptions::parse(
        [
            OsString::from("--request"),
            OsString::from("external-verification.json"),
            OsString::from("--format"),
            OsString::from("text"),
        ],
        deploy_external::verify_command,
        deploy_external::verify_usage,
    )
    .expect("parse deploy external verify text");
    assert_eq!(verify.request, PathBuf::from("external-verification.json"));
    assert_eq!(verify.format, output_format::ExternalOutputFormat::Text);
    let consent = deploy_external::DeployExternalInspectOptions::parse(
        [
            OsString::from("--request"),
            OsString::from("external-consent.json"),
            OsString::from("--format"),
            OsString::from("text"),
        ],
        deploy_external::consent_command,
        deploy_external::consent_usage,
    )
    .expect("parse deploy external inspect consent text");
    assert_eq!(consent.request, PathBuf::from("external-consent.json"));
    assert_eq!(consent.format, output_format::ExternalOutputFormat::Text);
}

#[test]
fn deploy_external_help_documents_passive_scope() {
    let help = deploy_external::usage();
    let plan_help = deploy_external::plan_usage();
    let check_help = deploy_external::check_usage();
    let handoff_help = deploy_external::handoff_usage();
    let proposals_help = deploy_external::proposals_usage();
    let pending_help = deploy_external::pending_usage();
    let critical_fix_help = deploy_external::critical_fix_usage();
    let inspect_help = deploy_external::inspect_usage();
    let consent_help = deploy_external::consent_usage();
    let verification_policy_help = deploy_external::verification_policy_usage();
    let verification_check_help = deploy_external::verification_check_usage();
    let completion_help = deploy_external::completion_usage();
    let verify_help = deploy_external::verify_usage();

    assert!(help.contains("Build passive external lifecycle reports"));
    assert!(help.contains("do not request"));
    assert!(help.contains("mutate deployment state"));
    assert!(help.contains("Build a passive external lifecycle check"));
    assert!(help.contains("Build a passive external lifecycle handoff packet"));
    assert!(help.contains("Build a passive external lifecycle pending report"));
    assert!(help.contains("Build a passive critical external fix report"));
    assert!(help.contains("Inspect passive external lifecycle internals"));
    assert!(help.contains("Build a passive external upgrade verification report"));
    assert!(plan_help.contains("ExternalLifecyclePlanV1 JSON"));
    assert!(plan_help.contains("No consent delivery"));
    assert!(check_help.contains("ExternalLifecycleCheckV1 JSON"));
    assert!(check_help.contains("summarize direct, pending"));
    assert!(handoff_help.contains("ExternalLifecycleHandoffV1 JSON"));
    assert!(handoff_help.contains("operator coordination instructions"));
    assert!(proposals_help.contains("ExternalUpgradeProposalReportV1 JSON"));
    assert!(proposals_help.contains("do not grant consent"));
    assert!(pending_help.contains("ExternalLifecyclePendingReportV1 JSON"));
    assert!(pending_help.contains("residual exposure"));
    assert!(critical_fix_help.contains("CriticalExternalFixReportV1 JSON"));
    assert!(critical_fix_help.contains("without claiming deployment completion"));
    assert!(inspect_help.contains("canic deploy external inspect consent"));
    assert!(inspect_help.contains("verification-policy"));
    assert!(inspect_help.contains("verification-check"));
    assert!(inspect_help.contains("completion"));
    assert!(inspect_help.contains("do not request consent"));
    assert!(consent_help.contains("ExternalUpgradeConsentEvidenceRequest-shaped JSON"));
    assert!(consent_help.contains("does not verify live completion"));
    assert!(
        verification_policy_help.contains("ExternalUpgradeVerificationPolicyRequest-shaped JSON")
    );
    assert!(verification_policy_help.contains("live-inventory"));
    assert!(verification_policy_help.contains("postconditions"));
    assert!(
        verification_check_help.contains("ExternalUpgradeVerificationCheckRequest-shaped JSON")
    );
    assert!(verification_check_help.contains("supplied observation facts"));
    assert!(verification_check_help.contains("DeploymentCheckV1 inventory artifact"));
    assert!(completion_help.contains("ExternalUpgradeCompletionReportRequest-shaped JSON"));
    assert!(completion_help.contains("proposal, consent evidence"));
    assert!(completion_help.contains("only deployment-truth inventory verification"));
    assert!(verify_help.contains("ExternalUpgradeVerificationReportRequest-shaped JSON"));
    assert!(verify_help.contains("live inventory remains the source of truth"));
}

#[test]
fn deploy_external_path_has_no_mutation_primitives() {
    let source = include_str!("../external.rs");
    let external_source =
        source_between(source, "pub(super) fn run<I>", "impl DeployExternalOptions");
    for forbidden in [
        "update_settings",
        "install_code",
        "create_canister",
        "delete_canister",
        "stop_canister",
        "uninstall_code",
        "provisional_create_canister",
        "dfx",
    ] {
        assert!(
            !external_source.contains(forbidden),
            "external lifecycle CLI path must stay passive; found forbidden token {forbidden}"
        );
    }
}

#[test]
fn deploy_external_command_dispatches_passive_leaf_commands() {
    for command in [
        "plan",
        "check",
        "handoff",
        "proposals",
        "pending",
        "critical-fix",
    ] {
        let parsed = parse_subcommand(
            deploy_command(),
            [
                OsString::from("external"),
                OsString::from(command),
                OsString::from("demo"),
            ],
        )
        .expect("parse deploy external")
        .expect("external command");

        assert_eq!(parsed.0, "external");

        let nested = parse_subcommand(deploy_external::command(), parsed.1)
            .expect("parse nested external")
            .expect("external leaf command");
        assert_eq!(nested.0, command);
        assert_eq!(nested.1, vec![OsString::from("demo")]);
    }

    let parsed = parse_subcommand(
        deploy_command(),
        [
            OsString::from("external"),
            OsString::from("verify"),
            OsString::from("--request"),
            OsString::from("external-verification.json"),
        ],
    )
    .expect("parse deploy external verify")
    .expect("external command");

    assert_eq!(parsed.0, "external");

    let nested = parse_subcommand(deploy_external::command(), parsed.1)
        .expect("parse nested external verify")
        .expect("external verify command");
    assert_eq!(nested.0, "verify");
    assert_eq!(
        nested.1,
        vec![
            OsString::from("--request"),
            OsString::from("external-verification.json")
        ]
    );
}

#[test]
fn deploy_external_inspect_dispatches_passive_leaf_commands() {
    for (command, request) in [
        ("consent", "external-consent.json"),
        ("verification-policy", "external-verification-policy.json"),
        ("verification-check", "external-verification-check.json"),
        ("completion", "external-completion.json"),
    ] {
        assert_external_inspect_dispatches(command, request);
    }
}

fn assert_external_inspect_dispatches(command: &str, request: &str) {
    let parsed = parse_subcommand(
        deploy_command(),
        [
            OsString::from("external"),
            OsString::from("inspect"),
            OsString::from(command),
            OsString::from("--request"),
            OsString::from(request),
        ],
    )
    .expect("parse deploy external inspect")
    .expect("external command");

    assert_eq!(parsed.0, "external");

    let external = parse_subcommand(deploy_external::command(), parsed.1)
        .expect("parse nested external inspect")
        .expect("external inspect command");
    assert_eq!(external.0, "inspect");

    let inspect = parse_subcommand(deploy_external::inspect_command(), external.1)
        .expect("parse nested inspect command")
        .expect("external inspect leaf command");
    assert_eq!(inspect.0, command);
    assert_eq!(
        inspect.1,
        vec![OsString::from("--request"), OsString::from(request)]
    );
}

#[test]
fn external_lifecycle_plan_builder_uses_stable_local_ids() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let plan = deploy_external::build_lifecycle_plan(&check);

    assert_eq!(
        plan.lifecycle_plan_id,
        "local:local:demo:external-lifecycle-plan"
    );
    assert_eq!(
        plan.lifecycle_authority_report_id,
        "local:local:demo:lifecycle-authority-report"
    );
    assert_eq!(plan.deployment_plan_id, "plan-1");
    assert_eq!(plan.inventory_id, "inventory-1");
    assert_eq!(plan.proposed_external_role_upgrades.len(), 1);
    assert!(plan.directly_executable_role_upgrades.is_empty());
    assert_eq!(plan.lifecycle_plan_digest.len(), 64);
}

#[test]
fn external_lifecycle_check_builder_links_pending_report() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let pending_report = deploy_external::build_lifecycle_pending_report(&check);
    let lifecycle_check = deploy_external::build_lifecycle_check(&check);

    assert_eq!(
        lifecycle_check.check_id,
        "local:local:demo:external-lifecycle-check"
    );
    assert_eq!(lifecycle_check.pending_report_id, pending_report.report_id);
    assert_eq!(
        lifecycle_check.pending_report_digest,
        pending_report.report_digest
    );
    assert_eq!(lifecycle_check.pending_external_count, 1);
    assert_eq!(lifecycle_check.direct_upgrade_count, 0);
    assert_eq!(lifecycle_check.blocked_count, 0);
    assert_eq!(lifecycle_check.check_digest.len(), 64);
}

#[test]
fn external_lifecycle_handoff_builder_packages_pending_actions() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let lifecycle_check = deploy_external::build_lifecycle_check(&check);
    let handoff = deploy_external::build_lifecycle_handoff(&check);

    assert_eq!(
        handoff.handoff_id,
        "local:local:demo:external-lifecycle-handoff"
    );
    assert_eq!(handoff.lifecycle_check_id, lifecycle_check.check_id);
    assert_eq!(handoff.lifecycle_check_digest, lifecycle_check.check_digest);
    assert_eq!(handoff.handoff_actions.len(), 1);
    assert_eq!(
        handoff.handoff_actions[0].consent_channel_kind,
        ConsentChannelKindV1::GeneratedCommand
    );
    assert_eq!(handoff.handoff_digest.len(), 64);
}

#[test]
fn external_proposal_report_builder_delegates_to_lifecycle_plan() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let lifecycle_plan = deploy_external::build_lifecycle_plan(&check);
    let report = deploy_external::build_upgrade_proposal_report(&check);

    assert_eq!(
        report.report_id,
        "local:local:demo:external-upgrade-proposals"
    );
    assert_eq!(report.report_digest.len(), 64);
    assert_eq!(report.lifecycle_plan_id, lifecycle_plan.lifecycle_plan_id);
    assert_eq!(
        report.lifecycle_plan_digest,
        lifecycle_plan.lifecycle_plan_digest
    );
    assert_eq!(report.deployment_plan_id, "plan-1");
    assert_eq!(report.inventory_id, "inventory-1");
    assert_eq!(report.proposals.len(), 1);
    assert_eq!(
        report.proposals[0].lifecycle_plan_digest,
        lifecycle_plan.lifecycle_plan_digest
    );
    assert_eq!(
        report.proposals[0].required_external_action,
        "external_controller_execution"
    );
}

#[test]
fn external_pending_report_builder_links_plan_and_proposals() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let lifecycle_plan = deploy_external::build_lifecycle_plan(&check);
    let proposal_report = deploy_external::build_upgrade_proposal_report(&check);
    let pending_report = deploy_external::build_lifecycle_pending_report(&check);

    assert_eq!(
        pending_report.report_id,
        "local:local:demo:external-lifecycle-pending"
    );
    assert_eq!(
        pending_report.lifecycle_plan_id,
        lifecycle_plan.lifecycle_plan_id
    );
    assert_eq!(
        pending_report.lifecycle_plan_digest,
        lifecycle_plan.lifecycle_plan_digest
    );
    assert_eq!(pending_report.proposal_report_id, proposal_report.report_id);
    assert_eq!(
        pending_report.proposal_report_digest,
        proposal_report.report_digest
    );
    assert_eq!(pending_report.pending_external_count, 1);
    assert_eq!(pending_report.direct_upgrade_count, 0);
    assert_eq!(pending_report.blocked_count, 0);
    assert_eq!(
        pending_report.pending_external_actions[0].proposal_id,
        proposal_report.proposals[0].proposal_id
    );
    assert_eq!(pending_report.report_digest.len(), 64);
}

#[test]
fn external_critical_fix_report_builder_links_pending_report() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let pending_report = deploy_external::build_lifecycle_pending_report(&check);
    let critical_fix =
        deploy_external::build_critical_fix_report(&check, "fix-2026-05", "critical");

    assert_eq!(
        critical_fix.report_id,
        "local:local:demo:critical-external-fix"
    );
    assert_eq!(critical_fix.fix_id, "fix-2026-05");
    assert_eq!(critical_fix.severity, "critical");
    assert_eq!(critical_fix.pending_report_id, pending_report.report_id);
    assert_eq!(
        critical_fix.pending_report_digest,
        pending_report.report_digest
    );
    assert_eq!(critical_fix.externally_blocked_roles, vec!["root"]);
    assert_eq!(critical_fix.required_external_actions.len(), 1);
    assert!(!critical_fix.residual_exposure.is_empty());
    assert_eq!(critical_fix.report_digest.len(), 64);
}

#[test]
fn external_consent_evidence_builder_delegates_to_receipt_validation() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let proposal_report = deploy_external::build_upgrade_proposal_report(&check);
    let proposal = proposal_report.proposals[0].clone();
    let receipt = external_upgrade_receipt_from_observation(
        "external-upgrade-receipt-1",
        &proposal,
        ExternalUpgradeConsentStateV1::Pending,
        None,
        None,
    );
    let evidence =
        deploy_external::build_upgrade_consent_evidence(ExternalUpgradeConsentEvidenceRequest {
            evidence_id: "external-upgrade-consent-1".to_string(),
            proposal,
            receipt,
        })
        .expect("consent evidence should build");

    assert_eq!(evidence.evidence_id, "external-upgrade-consent-1");
    assert_eq!(evidence.receipt_id, "external-upgrade-receipt-1");
    assert!(!evidence.status_summary.is_empty());
    assert_eq!(evidence.evidence_digest.len(), 64);
}

#[test]
fn external_verification_policy_builder_uses_proposal_requirements() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let proposal_report = deploy_external::build_upgrade_proposal_report(&check);
    let proposal = proposal_report.proposals[0].clone();
    let policy = deploy_external::build_upgrade_verification_policy(
        ExternalUpgradeVerificationPolicyRequest {
            policy_id: "external-upgrade-verification-policy-1".to_string(),
            proposal,
        },
    );

    assert_eq!(policy.policy_id, "external-upgrade-verification-policy-1");
    assert_eq!(policy.verification_requirements.len(), 5);
    assert!(policy.verification_requirements.iter().any(|row| {
        row.requirement == LifecycleVerificationRequirementV1::LiveInventory
            && row.status == ExternalUpgradeVerificationRequirementStatusV1::Required
    }));
    assert!(!policy.status_summary.is_empty());
    assert_eq!(policy.policy_digest.len(), 64);
}

#[test]
fn external_verification_check_builder_evaluates_supplied_observation() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let proposal_report = deploy_external::build_upgrade_proposal_report(&check);
    let proposal = proposal_report.proposals[0].clone();
    let policy = deploy_external::build_upgrade_verification_policy(
        ExternalUpgradeVerificationPolicyRequest {
            policy_id: "external-upgrade-verification-policy-1".to_string(),
            proposal: proposal.clone(),
        },
    );
    let verification_check = deploy_external::build_upgrade_verification_check(
        ExternalUpgradeVerificationCheckRequest {
            check_id: "external-upgrade-verification-check-1".to_string(),
            policy,
            observation: Some(ExternalUpgradeVerificationObservationV1 {
                source: ExternalVerificationObservationSourceV1::SuppliedObservation,
                deployment_check_id: None,
                deployment_check_digest: None,
                inventory_id: Some("inventory-verified".to_string()),
                observed_at: Some("2026-05-26T00:00:00Z".to_string()),
                live_inventory_observed: true,
                controller_observation_present: true,
                observed_control_class: Some(proposal.control_class),
                observed_module_hash: proposal.target_installed_module_hash,
                observed_canonical_embedded_config_sha256: proposal
                    .target_canonical_embedded_config_sha256,
                protected_call_ready: Some(true),
            }),
            deployment_check: None,
        },
    )
    .expect("verification check should build");

    assert_eq!(
        verification_check.check_id,
        "external-upgrade-verification-check-1"
    );
    assert_eq!(
        verification_check.verification_result,
        ExternalUpgradeVerificationResultV1::Pending
    );
    assert_eq!(verification_check.check_digest.len(), 64);
}

#[test]
fn external_verification_check_builder_verifies_deployment_truth_inventory() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    check.inventory.observed_canisters[0].module_hash = Some("module".to_string());
    check.inventory.observed_canisters[0].canonical_embedded_config_digest =
        Some(sample_sha256("c"));
    check.inventory.observed_verifier_readiness.status = ObservationStatusV1::Observed;

    let proposal_report = deploy_external::build_upgrade_proposal_report(&check);
    let proposal = proposal_report.proposals[0].clone();
    let policy = deploy_external::build_upgrade_verification_policy(
        ExternalUpgradeVerificationPolicyRequest {
            policy_id: "external-upgrade-verification-policy-1".to_string(),
            proposal,
        },
    );
    let verification_check = deploy_external::build_upgrade_verification_check(
        ExternalUpgradeVerificationCheckRequest {
            check_id: "external-upgrade-verification-check-1".to_string(),
            policy,
            observation: None,
            deployment_check: Some(check.clone()),
        },
    )
    .expect("inventory-backed verification check should build");

    assert_eq!(
        verification_check.observation.source,
        ExternalVerificationObservationSourceV1::DeploymentTruthInventory
    );
    assert_eq!(
        verification_check
            .observation
            .deployment_check_id
            .as_deref(),
        Some(check.check_id.as_str())
    );
    assert!(
        verification_check.requirement_results.iter().all(|row| {
            row.status == ExternalUpgradeVerificationRequirementStatusV1::NotRequired
                || row.satisfied == Some(true)
        }),
        "{:?}",
        verification_check.requirement_results
    );
    assert_eq!(
        verification_check.verification_result,
        ExternalUpgradeVerificationResultV1::Verified
    );
}

#[test]
fn external_verification_check_builder_rejects_ambiguous_observation_sources() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let proposal_report = deploy_external::build_upgrade_proposal_report(&check);
    let proposal = proposal_report.proposals[0].clone();
    let policy = deploy_external::build_upgrade_verification_policy(
        ExternalUpgradeVerificationPolicyRequest {
            policy_id: "external-upgrade-verification-policy-1".to_string(),
            proposal: proposal.clone(),
        },
    );
    let observation = ExternalUpgradeVerificationObservationV1 {
        source: ExternalVerificationObservationSourceV1::SuppliedObservation,
        deployment_check_id: None,
        deployment_check_digest: None,
        inventory_id: Some("inventory-verified".to_string()),
        observed_at: Some("2026-05-26T00:00:00Z".to_string()),
        live_inventory_observed: true,
        controller_observation_present: true,
        observed_control_class: Some(proposal.control_class),
        observed_module_hash: proposal.target_installed_module_hash,
        observed_canonical_embedded_config_sha256: proposal.target_canonical_embedded_config_sha256,
        protected_call_ready: Some(true),
    };

    let both_err = deploy_external::build_upgrade_verification_check(
        ExternalUpgradeVerificationCheckRequest {
            check_id: "external-upgrade-verification-check-1".to_string(),
            policy: policy.clone(),
            observation: Some(observation),
            deployment_check: Some(check.clone()),
        },
    )
    .expect_err("both observation sources should be rejected");
    std::assert_matches!(both_err, DeployCommandError::Blocked(_));

    let neither_err = deploy_external::build_upgrade_verification_check(
        ExternalUpgradeVerificationCheckRequest {
            check_id: "external-upgrade-verification-check-1".to_string(),
            policy,
            observation: None,
            deployment_check: None,
        },
    )
    .expect_err("missing observation source should be rejected");
    std::assert_matches!(neither_err, DeployCommandError::Blocked(_));
}

#[test]
fn external_completion_report_builder_links_consent_and_verification() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let proposal_report = deploy_external::build_upgrade_proposal_report(&check);
    let proposal = proposal_report.proposals[0].clone();
    let receipt = external_upgrade_receipt_from_observation(
        "external-upgrade-receipt-1",
        &proposal,
        ExternalUpgradeConsentStateV1::Pending,
        None,
        None,
    );
    let consent_evidence =
        deploy_external::build_upgrade_consent_evidence(ExternalUpgradeConsentEvidenceRequest {
            evidence_id: "external-upgrade-consent-1".to_string(),
            proposal: proposal.clone(),
            receipt,
        })
        .expect("consent evidence should build");
    let policy = deploy_external::build_upgrade_verification_policy(
        ExternalUpgradeVerificationPolicyRequest {
            policy_id: "external-upgrade-verification-policy-1".to_string(),
            proposal: proposal.clone(),
        },
    );
    let verification_check = deploy_external::build_upgrade_verification_check(
        ExternalUpgradeVerificationCheckRequest {
            check_id: "external-upgrade-verification-check-1".to_string(),
            policy,
            observation: Some(ExternalUpgradeVerificationObservationV1 {
                source: ExternalVerificationObservationSourceV1::SuppliedObservation,
                deployment_check_id: None,
                deployment_check_digest: None,
                inventory_id: Some("inventory-verified".to_string()),
                observed_at: Some("2026-05-26T00:00:00Z".to_string()),
                live_inventory_observed: true,
                controller_observation_present: true,
                observed_control_class: Some(proposal.control_class),
                observed_module_hash: proposal.target_installed_module_hash.clone(),
                observed_canonical_embedded_config_sha256: proposal
                    .target_canonical_embedded_config_sha256
                    .clone(),
                protected_call_ready: Some(true),
            }),
            deployment_check: None,
        },
    )
    .expect("verification check should build");

    let completion =
        deploy_external::build_upgrade_completion_report(ExternalUpgradeCompletionReportRequest {
            report_id: "external-upgrade-completion-1".to_string(),
            proposal,
            consent_evidence,
            verification_check,
        })
        .expect("completion report should build");

    assert_eq!(completion.report_id, "external-upgrade-completion-1");
    assert_eq!(
        completion.completion_status,
        ExternalUpgradeCompletionStatusV1::AwaitingConsent
    );
    assert_eq!(completion.report_digest.len(), 64);
}

#[test]
fn external_verification_report_builder_delegates_to_receipt_validation() {
    let mut check = sample_authority_check();
    check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

    let proposal_report = deploy_external::build_upgrade_proposal_report(&check);
    let proposal = proposal_report.proposals[0].clone();
    let receipt = external_upgrade_receipt_from_observation(
        "external-upgrade-receipt-1",
        &proposal,
        ExternalUpgradeConsentStateV1::Pending,
        None,
        None,
    );
    let report = deploy_external::build_upgrade_verification_report(
        ExternalUpgradeVerificationReportRequest {
            report_id: "external-upgrade-verification-1".to_string(),
            proposal,
            receipt,
        },
    )
    .expect("verification report should build");

    assert_eq!(report.report_id, "external-upgrade-verification-1");
    assert_eq!(report.receipt_id, "external-upgrade-receipt-1");
    assert!(!report.status_summary.is_empty());
    assert_eq!(report.report_digest.len(), 64);
}

#[test]
fn external_plan_rejects_unknown_format() {
    let result = deploy_external::DeployExternalOptions::parse(
        [
            OsString::from("--format"),
            OsString::from("yaml"),
            OsString::from("demo"),
        ],
        deploy_external::plan_command,
        deploy_external::plan_usage,
    );

    std::assert_matches!(
        result,
        Err(DeployCommandError::Usage(message))
            if message.contains("invalid external lifecycle output format: yaml")
    );
}
