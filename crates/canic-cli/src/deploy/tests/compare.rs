use super::super::compare as deploy_compare;
use super::super::inspect as deploy_inspect;
use super::super::output_format::JsonTextOutputFormat;
use super::fixtures::*;
use super::*;

fn compare_required_args() -> Vec<OsString> {
    vec![
        OsString::from("--left"),
        OsString::from("staging-check.json"),
        OsString::from("--right"),
        OsString::from("prod-check.json"),
    ]
}

#[test]
fn deploy_compare_parses_artifact_paths_and_text_flag() {
    let mut args = compare_required_args();
    args.extend([
        OsString::from("--left-label"),
        OsString::from("staging"),
        OsString::from("--right-label"),
        OsString::from("prod"),
        OsString::from("--text"),
    ]);
    let options = deploy_compare::DeployCompareOptions::parse(args).expect("parse deploy compare");

    assert_eq!(options.left, PathBuf::from("staging-check.json"));
    assert_eq!(options.right, PathBuf::from("prod-check.json"));
    assert_eq!(options.left_label.as_deref(), Some("staging"));
    assert_eq!(options.right_label.as_deref(), Some("prod"));
    assert_eq!(options.format, JsonTextOutputFormat::Text);
}

#[test]
fn deploy_compare_builder_uses_existing_check_artifacts() {
    let left = sample_authority_check();
    let mut right = sample_authority_check();
    right.plan.deployment_identity.deployment_name = "prod".to_string();

    let report = deploy_compare::build_report_from_checks(&left, &right, Some("stage"), None)
        .expect("comparison report should build");

    assert_eq!(report.report_id, "local:stage:prod:deployment-comparison");
    assert_eq!(report.left.label, "stage");
    assert_eq!(report.right.label, "prod");
    assert!(!report.identity_diff.is_empty());
    assert_eq!(report.report_digest.len(), 64);
}

#[test]
fn deploy_compare_help_documents_passive_artifact_scope() {
    let help = deploy_compare::usage();

    assert!(help.contains("Compare two deployment truth check artifacts"));
    assert!(help.contains("DeploymentCheckV1 JSON artifacts"));
    assert!(help.contains("does not query live"));
    assert!(help.contains("install code"));
    assert!(help.contains("mutate deployments"));
    assert!(help.contains("embedded"));
    assert!(help.contains("revalidated"));
}

#[test]
fn deploy_inspect_compare_command_dispatches_compare() {
    let mut args = vec![OsString::from("inspect"), OsString::from("compare")];
    args.extend(compare_required_args());
    let parsed = parse_subcommand(deploy_command(), args)
        .expect("parse deploy inspect")
        .expect("inspect command");

    assert_eq!(parsed.0, "inspect");

    let nested = parse_subcommand(deploy_inspect::command(), parsed.1)
        .expect("parse deploy inspect compare")
        .expect("compare command");

    assert_eq!(nested.0, "compare");
    assert_eq!(nested.1, compare_required_args());

    let options =
        deploy_compare::DeployCompareOptions::parse(nested.1).expect("parse compare options");
    assert_eq!(options.left, PathBuf::from("staging-check.json"));
    assert_eq!(options.right, PathBuf::from("prod-check.json"));
}
