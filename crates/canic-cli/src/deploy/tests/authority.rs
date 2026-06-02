use super::super::authority as deploy_authority;
use super::fixtures::*;
use super::*;

#[test]
fn deploy_authority_leaf_commands_default_to_json() {
    let authority_check = deploy_authority::DeployAuthorityOptions::parse(
        [OsString::from("demo")],
        deploy_authority::check_command,
        deploy_authority::check_usage,
    )
    .expect("parse deploy authority check");
    let authority_evidence = deploy_authority::DeployAuthorityOptions::parse(
        [OsString::from("demo")],
        deploy_authority::evidence_command,
        deploy_authority::evidence_usage,
    )
    .expect("parse deploy authority evidence");
    let authority_report = deploy_authority::DeployAuthorityOptions::parse(
        [OsString::from("demo")],
        deploy_authority::report_command,
        deploy_authority::report_usage,
    )
    .expect("parse deploy authority report");
    let authority_receipt = deploy_authority::DeployAuthorityOptions::parse(
        [OsString::from("demo")],
        deploy_authority::receipt_command,
        deploy_authority::receipt_usage,
    )
    .expect("parse deploy authority receipt");

    for options in [
        authority_check,
        authority_evidence,
        authority_report,
        authority_receipt,
    ] {
        assert_eq!(options.truth.deployment, "demo");
        assert_eq!(options.format, output_format::AuthorityOutputFormat::Json);
    }
}

#[test]
fn deploy_authority_leaf_commands_parse_text_format() {
    let authority_check = deploy_authority::DeployAuthorityOptions::parse(
        [
            OsString::from("--format"),
            OsString::from("text"),
            OsString::from("demo"),
        ],
        deploy_authority::check_command,
        deploy_authority::check_usage,
    )
    .expect("parse deploy authority check text");
    let authority_evidence = deploy_authority::DeployAuthorityOptions::parse(
        [
            OsString::from("--format"),
            OsString::from("text"),
            OsString::from("demo"),
        ],
        deploy_authority::evidence_command,
        deploy_authority::evidence_usage,
    )
    .expect("parse deploy authority evidence text");
    let authority_report = deploy_authority::DeployAuthorityOptions::parse(
        [
            OsString::from("--format"),
            OsString::from("text"),
            OsString::from("demo"),
        ],
        deploy_authority::report_command,
        deploy_authority::report_usage,
    )
    .expect("parse deploy authority report text");
    let authority_receipt = deploy_authority::DeployAuthorityOptions::parse(
        [
            OsString::from("--format"),
            OsString::from("text"),
            OsString::from("demo"),
        ],
        deploy_authority::receipt_command,
        deploy_authority::receipt_usage,
    )
    .expect("parse deploy authority receipt text");

    assert_eq!(authority_check.truth.deployment, "demo");
    assert_eq!(
        authority_check.format,
        output_format::AuthorityOutputFormat::Text
    );
    assert_eq!(authority_evidence.truth.deployment, "demo");
    assert_eq!(
        authority_evidence.format,
        output_format::AuthorityOutputFormat::Text
    );
    assert_eq!(authority_report.truth.deployment, "demo");
    assert_eq!(
        authority_report.format,
        output_format::AuthorityOutputFormat::Text
    );
    assert_eq!(authority_receipt.truth.deployment, "demo");
    assert_eq!(
        authority_receipt.format,
        output_format::AuthorityOutputFormat::Text
    );
}

#[test]
fn deploy_authority_command_help_does_not_claim_json_only_output() {
    let help = deploy_authority::usage();

    assert!(help.contains("Print the local authority reconciliation plan"));
    assert!(help.contains("Print the local authority dry-run evidence"));
    assert!(help.contains("Print the local authority reconciliation report"));
    assert!(help.contains("Print the local authority dry-run receipt"));
    assert!(help.contains("A successful command means the local authority artifact was produced"));
    assert!(help.contains("not that the deployment is globally safe"));
    assert!(help.contains("controller state"));
    assert!(help.contains("was changed"));
    assert!(!help.contains("authority reconciliation plan JSON"));
    assert!(!help.contains("authority dry-run evidence JSON"));
    assert!(!help.contains("authority reconciliation report JSON"));
    assert!(!help.contains("authority dry-run receipt JSON"));
}

#[test]
fn deploy_authority_leaf_help_documents_exit_status_scope() {
    let report_help = deploy_authority::report_usage();
    let receipt_help = deploy_authority::receipt_usage();
    let evidence_help = deploy_authority::evidence_usage();

    assert!(report_help.contains("Authority status is authority-scoped"));
    assert!(report_help.contains("whole-deployment safety"));
    assert!(receipt_help.contains("zero attempted"));
    assert!(receipt_help.contains("actions."));
    assert!(evidence_help.contains("evidence generation succeeded"));
}

#[test]
fn deploy_authority_command_dispatches_check() {
    let parsed = parse_subcommand(
        deploy_command(),
        [
            OsString::from("authority"),
            OsString::from("check"),
            OsString::from("demo"),
        ],
    )
    .expect("parse deploy authority")
    .expect("authority command");

    assert_eq!(parsed.0, "authority");
    assert_eq!(
        parsed.1,
        vec![OsString::from("check"), OsString::from("demo")]
    );

    let nested = parse_subcommand(deploy_authority::command(), parsed.1)
        .expect("parse nested authority")
        .expect("authority check command");
    assert_eq!(nested.0, "check");
    assert_eq!(nested.1, vec![OsString::from("demo")]);
}

#[test]
fn deploy_authority_command_dispatches_evidence() {
    let parsed = parse_subcommand(
        deploy_command(),
        [
            OsString::from("authority"),
            OsString::from("evidence"),
            OsString::from("demo"),
        ],
    )
    .expect("parse deploy authority")
    .expect("authority command");

    assert_eq!(parsed.0, "authority");
    assert_eq!(
        parsed.1,
        vec![OsString::from("evidence"), OsString::from("demo")]
    );

    let nested = parse_subcommand(deploy_authority::command(), parsed.1)
        .expect("parse nested authority")
        .expect("authority evidence command");
    assert_eq!(nested.0, "evidence");
    assert_eq!(nested.1, vec![OsString::from("demo")]);
}

#[test]
fn deploy_authority_command_dispatches_report() {
    let parsed = parse_subcommand(
        deploy_command(),
        [
            OsString::from("authority"),
            OsString::from("report"),
            OsString::from("demo"),
        ],
    )
    .expect("parse deploy authority")
    .expect("authority command");

    assert_eq!(parsed.0, "authority");
    assert_eq!(
        parsed.1,
        vec![OsString::from("report"), OsString::from("demo")]
    );

    let nested = parse_subcommand(deploy_authority::command(), parsed.1)
        .expect("parse nested authority")
        .expect("authority report command");
    assert_eq!(nested.0, "report");
    assert_eq!(nested.1, vec![OsString::from("demo")]);
}

#[test]
fn deploy_authority_command_dispatches_receipt() {
    let parsed = parse_subcommand(
        deploy_command(),
        [
            OsString::from("authority"),
            OsString::from("receipt"),
            OsString::from("demo"),
        ],
    )
    .expect("parse deploy authority")
    .expect("authority command");

    assert_eq!(parsed.0, "authority");
    assert_eq!(
        parsed.1,
        vec![OsString::from("receipt"), OsString::from("demo")]
    );

    let nested = parse_subcommand(deploy_authority::command(), parsed.1)
        .expect("parse nested authority")
        .expect("authority receipt command");
    assert_eq!(nested.0, "receipt");
    assert_eq!(nested.1, vec![OsString::from("demo")]);
}

#[test]
fn authority_evidence_builder_delegates_to_host_local_ids() {
    let check = sample_authority_check();

    let evidence =
        deploy_authority::build_dry_run_evidence(&check).expect("build authority dry-run evidence");

    assert_eq!(evidence.evidence_id, "local:local:demo:authority-evidence");
    assert_eq!(evidence.check_id, "check-1");
    assert_eq!(
        evidence.authority_report.check_id.as_deref(),
        Some("check-1")
    );
    assert_eq!(
        evidence.authority_report.report_id,
        "local:local:demo:authority-report"
    );
    assert_eq!(evidence.authority_report.inventory_id, "inventory-1");
    assert_eq!(
        evidence.authority_report.authority_profile_hash.as_deref(),
        Some("authority")
    );
    assert_eq!(
        evidence.authority_receipt.check_id.as_deref(),
        Some("check-1")
    );
    assert_eq!(
        evidence.authority_receipt.operation_id,
        "local:local:demo:authority-dry-run-receipt"
    );
    assert_eq!(evidence.authority_receipt.inventory_id, "inventory-1");
    assert_eq!(
        evidence.authority_receipt.authority_profile_hash.as_deref(),
        Some("authority")
    );
}

#[test]
fn authority_receipt_builder_delegates_to_host_local_ids() {
    let check = sample_authority_check();

    let receipt =
        deploy_authority::build_dry_run_receipt(&check).expect("build authority dry-run receipt");

    assert_eq!(
        receipt.operation_id,
        "local:local:demo:authority-dry-run-receipt"
    );
    assert_eq!(receipt.check_id.as_deref(), Some("check-1"));
    assert_eq!(receipt.reconciliation_plan_id, "plan-1");
    assert_eq!(
        receipt.authority_report_id,
        "local:local:demo:authority-report"
    );
    assert_eq!(receipt.inventory_id, "inventory-1");
    assert_eq!(receipt.authority_profile_hash.as_deref(), Some("authority"));
    assert!(receipt.attempted_actions.is_empty());
}

#[test]
fn authority_check_rejects_unknown_format() {
    let result = deploy_authority::DeployAuthorityOptions::parse(
        [
            OsString::from("--format"),
            OsString::from("csv"),
            OsString::from("demo"),
        ],
        deploy_authority::check_command,
        deploy_authority::check_usage,
    );

    std::assert_matches!(
        result,
        Err(DeployCommandError::Usage(message))
            if message.contains("invalid authority output format: csv")
    );
}

#[test]
fn authority_evidence_rejects_unknown_format() {
    let result = deploy_authority::DeployAuthorityOptions::parse(
        [
            OsString::from("--format"),
            OsString::from("xml"),
            OsString::from("demo"),
        ],
        deploy_authority::evidence_command,
        deploy_authority::evidence_usage,
    );

    std::assert_matches!(
        result,
        Err(DeployCommandError::Usage(message))
            if message.contains("invalid authority output format: xml")
    );
}

#[test]
fn authority_report_rejects_unknown_format() {
    let result = deploy_authority::DeployAuthorityOptions::parse(
        [
            OsString::from("--format"),
            OsString::from("yaml"),
            OsString::from("demo"),
        ],
        deploy_authority::report_command,
        deploy_authority::report_usage,
    );

    std::assert_matches!(
        result,
        Err(DeployCommandError::Usage(message))
            if message.contains("invalid authority output format: yaml")
    );
}

#[test]
fn authority_receipt_rejects_unknown_format() {
    let result = deploy_authority::DeployAuthorityOptions::parse(
        [
            OsString::from("--format"),
            OsString::from("toml"),
            OsString::from("demo"),
        ],
        deploy_authority::receipt_command,
        deploy_authority::receipt_usage,
    );

    std::assert_matches!(
        result,
        Err(DeployCommandError::Usage(message))
            if message.contains("invalid authority output format: toml")
    );
}
