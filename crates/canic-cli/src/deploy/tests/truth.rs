use super::super::resume_report as deploy_resume_report;
use super::super::truth as deploy_truth;
use super::*;

#[test]
fn deploy_leaf_commands_parse_like_check() {
    let plan = DeployTruthOptions::parse(
        [OsString::from("demo")],
        deploy_truth::plan_command,
        deploy_truth::plan_usage,
    )
    .expect("parse deploy plan");
    let inventory = DeployTruthOptions::parse(
        [OsString::from("demo")],
        deploy_truth::inventory_command,
        deploy_truth::inventory_usage,
    )
    .expect("parse deploy inventory");
    let diff = DeployTruthOptions::parse(
        [OsString::from("demo")],
        deploy_truth::diff_command,
        deploy_truth::diff_usage,
    )
    .expect("parse deploy diff");
    let report = DeployTruthOptions::parse(
        [OsString::from("demo")],
        deploy_truth::report_command,
        deploy_truth::report_usage,
    )
    .expect("parse deploy report");
    let resume_report = deploy_resume_report::DeployResumeReportOptions::parse([
        OsString::from("--receipt"),
        OsString::from("receipt.json"),
        OsString::from("demo"),
    ])
    .expect("parse deploy resume-report");

    assert_eq!(plan.deployment, "demo");
    assert_eq!(inventory.deployment, "demo");
    assert_eq!(diff.deployment, "demo");
    assert_eq!(report.deployment, "demo");
    assert_eq!(resume_report.truth.deployment, "demo");
    assert_eq!(resume_report.receipt, Some(PathBuf::from("receipt.json")));
}

#[test]
fn deploy_resume_report_allows_latest_local_receipt_lookup() {
    let resume_report =
        deploy_resume_report::DeployResumeReportOptions::parse([OsString::from("demo")])
            .expect("parse deploy resume-report");

    assert_eq!(resume_report.truth.deployment, "demo");
    assert_eq!(resume_report.receipt, None);
}
