use super::*;
use crate::fleet::{FleetCommandError, json_error, tests::cycle_quantity_report};
use canic_host::fleet_ensure::{
    model::FleetEnsureSuccessorReviewReason, workflow::EnsureWorkflowError,
};
use std::{ffi::OsString, path::PathBuf};

fn options() -> EnsureOptions {
    let mut options =
        EnsureOptions::parse(["ensure", "demo", "--json"].map(OsString::from)).unwrap();
    options.desired = PathBuf::from("fleets/operator's desired.toml");
    options.source = PathBuf::from(".tools/operator policy.toml");
    options.seed = PathBuf::from(".tools/operator estate.toml");
    options.identity = Some("operator with spaces".into());
    options
}

#[test]
fn next_actions_preserve_exact_arguments_and_distinguish_new_approval_from_resume() {
    let mut report = cycle_quantity_report("rrkah-fqaaa-aaaaa-aaaaq-cai");
    report.plan.plan_sha256 = "ab".repeat(32);
    report.terminal = false;
    let mut options = options();
    let result = ensure(&report, &options, false);
    let next = result.next_action.unwrap();
    assert_eq!(next.kind, ActionKind::Apply);
    assert!(next.requires_approval);
    assert!(
        next.arguments
            .contains(&"fleets/operator's desired.toml".into())
    );
    assert!(next.arguments.contains(&"operator with spaces".into()));
    assert_eq!(next.arguments.last(), Some(&report.plan.plan_sha256));
    assert_valid_ensure_arguments(&next.arguments);
    options.apply = Some(report.plan.plan_sha256.clone());
    let next = ensure(&report, &options, false).next_action.unwrap();
    assert_eq!(next.kind, ActionKind::Resume);
    assert!(!next.requires_approval);
    assert_valid_ensure_arguments(&next.arguments);
}

fn assert_valid_ensure_arguments(arguments: &[String]) -> EnsureOptions {
    use crate::cli::globals::{DISPATCH_ARGS, apply_global_environment, apply_global_icp};
    let matches = crate::cli::clap::parse_matches(
        crate::cli::top_level_command(),
        arguments.iter().map(OsString::from),
    )
    .unwrap();
    let (command, subcommand) = matches.subcommand().unwrap();
    let mut tail = subcommand
        .get_many::<OsString>(DISPATCH_ARGS)
        .unwrap()
        .cloned()
        .collect::<Vec<_>>();
    apply_global_icp(
        command,
        &mut tail,
        matches.get_one::<String>("icp").cloned(),
    );
    apply_global_environment(
        command,
        &mut tail,
        matches.get_one::<String>("environment").cloned(),
    );
    let parsed = EnsureOptions::parse(tail).unwrap();
    assert!(parsed.json);
    assert_eq!(
        parsed.desired,
        PathBuf::from("fleets/operator's desired.toml")
    );
    assert_eq!(parsed.identity.as_deref(), Some("operator with spaces"));
    assert_eq!(parsed.source, options().source);
    assert_eq!(parsed.seed, options().seed);
    parsed
}

#[test]
fn intermediate_completion_requests_review_and_only_full_terminal_completes_fleet() {
    let mut report = cycle_quantity_report("rrkah-fqaaa-aaaaa-aaaaq-cai");
    report.terminal = true;
    let options = options();
    for scope in [
        FleetEnsurePlanScope::InfrastructureBootstrap,
        FleetEnsurePlanScope::RootReinstallPrerequisite,
        FleetEnsurePlanScope::RootStartPrerequisite,
    ] {
        report.plan.scope = scope;
        let result = ensure(&report, &options, true);
        assert!(result.phase_completed);
        assert!(!result.fleet_completed);
        let next = result.next_action.unwrap();
        assert_eq!(next.kind, ActionKind::Review);
        assert!(!next.requires_approval);
        assert!(next.arguments.contains(&"--reinstall".into()));
        assert_valid_ensure_arguments(&next.arguments);
    }
    report.plan.scope = FleetEnsurePlanScope::Full;
    let result = ensure(&report, &options, true);
    assert!(result.fleet_completed);
    assert!(result.next_action.is_none());
}

#[test]
fn completed_import_advances_to_review_without_reusing_its_approval() {
    let options = options();
    let digest = "ab".repeat(32);
    let result = import(
        digest.clone(),
        true,
        true,
        reinstall_review_command(&options, "local"),
        true,
    );
    assert_eq!(result.review_sha256, Some(digest));
    assert!(result.phase_completed);
    assert!(!result.fleet_completed);
    let next = result.next_action.unwrap();
    assert_eq!(next.kind, ActionKind::Review);
    assert_valid_ensure_arguments(&next.arguments);
    assert!(!next.arguments.contains(&"--apply".into()));
}

#[test]
fn typed_successor_pause_reports_review_without_reading_private_journals() {
    let error =
        FleetCommandError::Workflow(Box::new(EnsureWorkflowError::SuccessorReviewRequired {
            reason: FleetEnsureSuccessorReviewReason::BudgetExceeded,
            review: None,
        }));
    let mut options = options();
    options.reinstall = true;
    options.apply = Some("ab".repeat(32));
    let FleetCommandError::JsonReported { report, .. } = json_error(error, Some(&options)) else {
        panic!("JSON error")
    };
    let value: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(value["code"], "successor_review_required");
    assert_eq!(value["next_action"]["kind"], "review");
    assert_eq!(value["next_action"]["requires_approval"], false);
    let arguments = value["next_action"]["arguments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert!(assert_valid_ensure_arguments(&arguments).reinstall);
    assert!(
        value["next_action"]["arguments"]
            .as_array()
            .unwrap()
            .iter()
            .all(|arg| arg != "--apply")
    );
}

#[test]
fn import_apply_and_resume_keep_nondefault_selection() {
    let options = options();
    for applying in [false, true] {
        let result = import(
            "ab".repeat(32),
            false,
            applying,
            ensure_command(&options, "local"),
            true,
        );
        let next = result.next_action.unwrap();
        assert_eq!(
            next.kind,
            if applying {
                ActionKind::Resume
            } else {
                ActionKind::Apply
            }
        );
        assert_valid_ensure_arguments(&next.arguments);
    }
}

#[test]
fn reinstall_roundtrip_keeps_inputs_through_infrastructure_import_and_successor() {
    let mut report = cycle_quantity_report("rrkah-fqaaa-aaaaa-aaaaq-cai");
    report.plan.scope = FleetEnsurePlanScope::InfrastructureBootstrap;
    report.plan.plan_sha256 = "ab".repeat(32);
    report.terminal = false;
    let mut selected = options();
    selected.reinstall = true;
    let next = ensure(&report, &selected, true).next_action.unwrap();
    let applying = assert_valid_ensure_arguments(&next.arguments);
    assert!(!applying.reinstall);
    assert_eq!(applying.apply.as_ref(), Some(&report.plan.plan_sha256));
    let retry = ensure(&report, &applying, true).next_action.unwrap();
    assert_eq!(retry.kind, ActionKind::Resume);
    let resumed = assert_valid_ensure_arguments(&retry.arguments);
    report.terminal = true;
    let next = ensure(&report, &resumed, true).next_action.unwrap();
    let reviewing = assert_valid_ensure_arguments(&next.arguments);
    assert!(reviewing.reinstall);
    assert!(reviewing.apply.is_none());
    let next = import(
        "cd".repeat(32),
        false,
        false,
        ensure_command(&reviewing, "local"),
        true,
    )
    .next_action
    .unwrap();
    let importing = assert_valid_ensure_arguments(&next.arguments);
    let next = import(
        "cd".repeat(32),
        true,
        true,
        reinstall_review_command(&importing, "local"),
        true,
    )
    .next_action
    .unwrap();
    let successor = assert_valid_ensure_arguments(&next.arguments);
    assert!(successor.reinstall);
    report.plan.scope = FleetEnsurePlanScope::Full;
    assert!(ensure(&report, &successor, true).next_action.is_none());
}
