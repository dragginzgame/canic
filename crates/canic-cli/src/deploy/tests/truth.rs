use super::super::resume_report as deploy_resume_report;
use super::super::truth as deploy_truth;
use super::*;

fn resume_report_receipt_args() -> Vec<OsString> {
    vec![
        OsString::from("--receipt"),
        OsString::from("receipt.json"),
        OsString::from("demo"),
    ]
}

#[test]
fn deploy_leaf_commands_parse_like_check() {
    for (command, usage, name) in truth_leaf_commands() {
        let options = DeployTruthOptions::parse([OsString::from("demo")], command, usage)
            .unwrap_or_else(|_| panic!("parse deploy {name}"));

        assert_eq!(options.deployment, "demo");
    }

    let resume_report =
        deploy_resume_report::DeployResumeReportOptions::parse(resume_report_receipt_args())
            .expect("parse deploy resume-report");

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

type TruthCommandFactory = fn() -> ClapCommand;
type TruthUsageFactory = fn() -> String;

fn truth_leaf_commands() -> [(TruthCommandFactory, TruthUsageFactory, &'static str); 4] {
    [
        (deploy_truth::plan_command, deploy_truth::plan_usage, "plan"),
        (
            deploy_truth::inventory_command,
            deploy_truth::inventory_usage,
            "inventory",
        ),
        (deploy_truth::diff_command, deploy_truth::diff_usage, "diff"),
        (
            deploy_truth::report_command,
            deploy_truth::report_usage,
            "report",
        ),
    ]
}
