//! Admission argument parsing and help.
use super::*;

#[test]
fn plan_parser_requires_one_action_and_one_selector() {
    let principal = Principal::self_authenticating([7; 32]).to_text();
    let parsed = AdmissionPlanOptions::parse(
        [
            "demo",
            "--add",
            principal.as_str(),
            "--fleet",
            "--out",
            "admission.json",
        ]
        .into_iter()
        .map(OsString::from)
        .collect(),
    )
    .expect("parse exact Fleet addition");
    assert_eq!(parsed.action, FleetAdmissionMutationAction::Add);
    assert_eq!(parsed.selector, FleetAdmissionSelector::Fleet);
    assert_eq!(parsed.out, PathBuf::from("admission.json"));

    assert!(
        AdmissionPlanOptions::parse(
            ["demo", "--add", principal.as_str(), "--out", "plan.json"]
                .into_iter()
                .map(OsString::from)
                .collect(),
        )
        .is_err()
    );
}

#[test]
fn admission_subcommands_and_help_are_ascii_ordered() {
    let command = admission_command();
    let names = command
        .get_subcommands()
        .map(clap::Command::get_name)
        .collect::<Vec<_>>();
    assert_eq!(names, ["apply", "plan", "status"]);
    assert!(admission_usage().contains("canic admission plan"));
    assert!(plan_usage().contains("--component-instance"));
}
