//! Admission argument parsing and help.
use super::*;

#[test]
fn admission_status_escapes_phase_text_without_rewriting_json() {
    let injected = "phase\nforged\x1b]52;c;dGVzdA==\x07\u{202e}";
    let report = AdmissionStatusReport {
        fleet: injected.into(),
        environment: "local".into(),
        coordinator: Principal::anonymous(),
        registry_revision: 1,
        generation: 1,
        policy_digest: "digest".into(),
        fleet_principal_count: 1,
        narrower_rule_count: 0,
        narrower_principal_reference_count: 0,
        current_operation: None,
        current_phase: Some(injected.into()),
        last_operation: None,
        last_phase: None,
        roots: vec![canic_host::admission::view::AdmissionRootReport {
            root: Principal::anonymous(),
            active_generation: 1,
            active_policy_digest: "digest".into(),
            operation_id: None,
            phase: Some(injected.into()),
            participant_count: 1,
            pending_count: 1,
            prepared_count: 0,
            activated_count: 0,
            open_count: 1,
            first_unresolved: None,
        }],
    };
    let rendered = render_status(&report);
    assert_eq!(rendered.lines().count(), 9);
    assert!(!rendered.contains(['\x1b', '\x07', '\u{202e}']));
    assert!(rendered.contains(r"phase\nforged\u{1b}]52;c;dGVzdA==\u{7}\u{202e}"));
    assert_eq!(
        serde_json::to_value(&report).unwrap()["roots"][0]["phase"],
        injected
    );
}

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
