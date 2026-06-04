use super::super::authority as deploy_authority;
use super::fixtures::*;
use super::*;

#[test]
fn deploy_authority_leaf_commands_default_to_json() {
    for (command, usage, name) in authority_leaf_commands() {
        let options = deploy_authority::DeployAuthorityOptions::parse(
            [OsString::from("demo")],
            command,
            usage,
        )
        .unwrap_or_else(|_| panic!("parse deploy authority {name}"));

        assert_eq!(options.truth.deployment, "demo");
        assert_eq!(options.format, output_format::AuthorityOutputFormat::Json);
    }
}

#[test]
fn deploy_authority_leaf_commands_parse_text_format() {
    for (command, usage, name) in authority_leaf_commands() {
        let options = deploy_authority::DeployAuthorityOptions::parse(
            [
                OsString::from("--format"),
                OsString::from("text"),
                OsString::from("demo"),
            ],
            command,
            usage,
        )
        .unwrap_or_else(|_| panic!("parse deploy authority {name} text"));

        assert_eq!(options.truth.deployment, "demo");
        assert_eq!(options.format, output_format::AuthorityOutputFormat::Text);
    }
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
    assert_authority_dispatches_leaf("check");
}

#[test]
fn deploy_authority_command_dispatches_evidence() {
    assert_authority_dispatches_leaf("evidence");
}

#[test]
fn deploy_authority_command_dispatches_report() {
    assert_authority_dispatches_leaf("report");
}

#[test]
fn deploy_authority_command_dispatches_receipt() {
    assert_authority_dispatches_leaf("receipt");
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
    assert_authority_rejects_unknown_format(
        deploy_authority::check_command,
        deploy_authority::check_usage,
        "csv",
    );
}

#[test]
fn authority_evidence_rejects_unknown_format() {
    assert_authority_rejects_unknown_format(
        deploy_authority::evidence_command,
        deploy_authority::evidence_usage,
        "xml",
    );
}

#[test]
fn authority_report_rejects_unknown_format() {
    assert_authority_rejects_unknown_format(
        deploy_authority::report_command,
        deploy_authority::report_usage,
        "yaml",
    );
}

#[test]
fn authority_receipt_rejects_unknown_format() {
    assert_authority_rejects_unknown_format(
        deploy_authority::receipt_command,
        deploy_authority::receipt_usage,
        "toml",
    );
}

type AuthorityCommandFactory = fn() -> ClapCommand;
type AuthorityUsageFactory = fn() -> String;

fn authority_leaf_commands() -> [(AuthorityCommandFactory, AuthorityUsageFactory, &'static str); 4]
{
    [
        (
            deploy_authority::check_command,
            deploy_authority::check_usage,
            "check",
        ),
        (
            deploy_authority::evidence_command,
            deploy_authority::evidence_usage,
            "evidence",
        ),
        (
            deploy_authority::report_command,
            deploy_authority::report_usage,
            "report",
        ),
        (
            deploy_authority::receipt_command,
            deploy_authority::receipt_usage,
            "receipt",
        ),
    ]
}

fn assert_authority_dispatches_leaf(command: &'static str) {
    let parsed = parse_subcommand(
        deploy_command(),
        [
            OsString::from("authority"),
            OsString::from(command),
            OsString::from("demo"),
        ],
    )
    .expect("parse deploy authority")
    .expect("authority command");

    assert_eq!(parsed.0, "authority");
    assert_eq!(
        parsed.1,
        vec![OsString::from(command), OsString::from("demo")]
    );

    let nested = parse_subcommand(deploy_authority::command(), parsed.1)
        .expect("parse nested authority")
        .expect("authority leaf command");
    assert_eq!(nested.0, command);
    assert_eq!(nested.1, vec![OsString::from("demo")]);
}

fn assert_authority_rejects_unknown_format(
    command: AuthorityCommandFactory,
    usage: AuthorityUsageFactory,
    format: &'static str,
) {
    let result = deploy_authority::DeployAuthorityOptions::parse(
        [
            OsString::from("--format"),
            OsString::from(format),
            OsString::from("demo"),
        ],
        command,
        usage,
    );

    std::assert_matches!(result, Err(DeployCommandError::Usage(_)));
}
