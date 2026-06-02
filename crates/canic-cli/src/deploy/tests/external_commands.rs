use super::super::external as deploy_external;
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
