use super::super::check as deploy_check;
use super::super::output_format::CheckOutputFormat;
use super::super::{DeployCommandError, DeployTruthOptions};
use super::fixtures::*;
use canic_host::{
    canister_build::CanisterBuildProfile,
    deployment_truth::{SafetyFindingV1, SafetyReportV1, SafetySeverityV1, SafetyStatusV1},
};
use std::{ffi::OsString, fs, path::PathBuf};

fn check_deployment_arg() -> OsString {
    OsString::from("demo")
}

fn envelope_format_args() -> Vec<OsString> {
    vec![
        check_deployment_arg(),
        OsString::from("--format"),
        OsString::from("envelope-json"),
    ]
}

fn build_provenance_args() -> Vec<OsString> {
    vec![
        OsString::from("--build-provenance"),
        OsString::from("build-provenance.json"),
    ]
}

#[test]
fn deploy_check_parses_required_deployment() {
    let options = DeployTruthOptions::parse(
        [check_deployment_arg()],
        deploy_check::command,
        deploy_check::usage,
    )
    .expect("parse deploy check");

    assert_eq!(options.deployment, "demo");
    assert_eq!(options.network, "local");
    assert_eq!(options.profile, None);
}

#[test]
fn deploy_check_accepts_internal_network_and_profile() {
    let options = DeployTruthOptions::parse(
        [
            OsString::from("--profile"),
            OsString::from("fast"),
            OsString::from("demo"),
            OsString::from("--__canic-network"),
            OsString::from("ic"),
        ],
        deploy_check::command,
        deploy_check::usage,
    )
    .expect("parse deploy check");

    assert_eq!(options.network, "ic");
    assert_eq!(options.profile, Some(CanisterBuildProfile::Fast));
}

#[test]
fn deploy_check_rejects_invalid_profile() {
    std::assert_matches!(
        DeployTruthOptions::parse(
            [
                OsString::from("--profile"),
                OsString::from("turbo"),
                OsString::from("demo"),
            ],
            deploy_check::command,
            deploy_check::usage,
        ),
        Err(DeployCommandError::Usage(_))
    );
}

#[test]
fn deploy_check_status_rejects_blocked_report() {
    let report = SafetyReportV1 {
        schema_version: 1,
        report_id: "report-1".to_string(),
        diff_id: None,
        status: SafetyStatusV1::Blocked,
        summary: "deployment inventory has 1 blocking issue(s) and 0 warning(s)".to_string(),
        hard_failures: Vec::new(),
        warnings: Vec::new(),
        next_actions: Vec::new(),
    };

    std::assert_matches!(
        deploy_check::enforce_deployment_check_status(&report),
        Err(DeployCommandError::Blocked(message))
            if message == "deployment inventory has 1 blocking issue(s) and 0 warning(s)"
    );
}

#[test]
fn deploy_check_status_allows_warning_report() {
    let report = SafetyReportV1 {
        schema_version: 1,
        report_id: "report-1".to_string(),
        diff_id: None,
        status: SafetyStatusV1::Warning,
        summary: "deployment inventory has 1 warning(s)".to_string(),
        hard_failures: Vec::new(),
        warnings: Vec::new(),
        next_actions: Vec::new(),
    };

    deploy_check::enforce_deployment_check_status(&report)
        .expect("warning report should not fail check");
}

#[test]
fn deploy_check_parses_envelope_json_format() {
    let options = deploy_check::DeployCheckOptions::parse(envelope_format_args())
        .expect("parse deploy check");

    assert_eq!(options.truth.deployment, "demo");
    assert_eq!(options.format, CheckOutputFormat::EnvelopeJson);
    assert_eq!(options.build_provenance, None);
}

#[test]
fn deploy_check_parses_text_format() {
    let options = deploy_check::DeployCheckOptions::parse([
        check_deployment_arg(),
        OsString::from("--format"),
        OsString::from("text"),
    ])
    .expect("parse deploy check");

    assert_eq!(options.truth.deployment, "demo");
    assert_eq!(options.format, CheckOutputFormat::Text);
    assert_eq!(options.build_provenance, None);
}

#[test]
fn deploy_check_parses_build_provenance_envelope_input() {
    let mut args = envelope_format_args();
    args.extend(build_provenance_args());
    let options = deploy_check::DeployCheckOptions::parse(args).expect("parse deploy check");

    assert_eq!(
        options.build_provenance,
        Some(PathBuf::from("build-provenance.json"))
    );
}

#[test]
fn deploy_check_rejects_build_provenance_without_envelope_output() {
    let mut args = vec![check_deployment_arg()];
    args.extend(build_provenance_args());
    let err = deploy_check::DeployCheckOptions::parse(args)
        .expect_err("build provenance requires envelope output");

    std::assert_matches!(
        err,
        DeployCommandError::Usage(message)
            if message.contains("--build-provenance requires --format envelope-json")
    );
}

#[test]
fn deploy_check_usage_lists_build_provenance_input() {
    let text = deploy_check::usage();

    assert!(text.contains("--format <json|envelope-json|text>"));
    assert!(text.contains("--build-provenance <path>"));
}

#[test]
fn deployment_check_text_renders_operator_summary() {
    let mut check = sample_authority_check();
    check.report.status = SafetyStatusV1::Warning;
    check.report.warnings.push(SafetyFindingV1 {
        code: "test_warning".to_string(),
        message: "operator should review warning".to_string(),
        severity: SafetySeverityV1::Warning,
        subject: Some("app".to_string()),
    });
    check
        .report
        .next_actions
        .push("review deployment warnings before continuing".to_string());

    let text = deploy_check::deployment_check_text(&check);

    assert!(text.contains("Deployment check"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("status: Warning"));
    assert!(text.contains("deployment: demo"));
    assert!(text.contains("counts:"));
    assert!(text.contains("warnings:"));
    assert!(text.contains("code=test_warning"));
    assert!(text.contains("next_actions:"));
}

#[test]
fn deployment_check_envelope_wraps_raw_payload() {
    let config_path = temp_json_path("deploy-check-envelope-canic.toml");
    let build_provenance_path = temp_json_path("deploy-check-build-provenance.json");
    fs::write(&config_path, "[fleet]\nname = \"demo\"\n").expect("write config");
    fs::write(
        &build_provenance_path,
        br#"{"payload_schema":{"id":"canic.build_provenance.v1"}}"#,
    )
    .expect("write build provenance");
    let mut check = sample_authority_check();
    check.inventory.local_config.config_path = Some(config_path.to_string_lossy().to_string());
    check.inventory.local_config.raw_config_sha256 = Some(sample_sha256("a"));
    check.report.warnings.push(SafetyFindingV1 {
        code: "test_warning".to_string(),
        message: "operator should review warning".to_string(),
        severity: SafetySeverityV1::Warning,
        subject: Some("test".to_string()),
    });
    check.report.status = SafetyStatusV1::Warning;
    let options = deploy_check::DeployCheckOptions {
        truth: DeployTruthOptions {
            deployment: "demo".to_string(),
            network: "local".to_string(),
            profile: Some(CanisterBuildProfile::Fast),
        },
        format: CheckOutputFormat::EnvelopeJson,
        build_provenance: Some(build_provenance_path.clone()),
    };

    let envelope = deploy_check::build_deployment_check_envelope(&options, &check)
        .expect("deployment check envelope");
    let value = serde_json::to_value(&envelope).expect("serialize envelope");

    fs::remove_file(config_path).expect("clean config");
    fs::remove_file(build_provenance_path).expect("clean build provenance");
    assert_eq!(value["envelope_schema"]["id"], "canic.evidence_envelope.v1");
    assert_eq!(value["payload_schema"]["id"], "canic.deployment_check.v1");
    assert_eq!(value["payload_schema"]["stability"], "internal");
    assert_eq!(value["target"]["kind"], "deployment");
    assert_eq!(value["target"]["deployment"], "demo");
    assert_eq!(value["target"]["fleet"], "demo");
    assert_eq!(value["target"]["profile"], "fast");
    assert_eq!(value["command"]["name"], "canic deploy check");
    assert_eq!(value["command"]["format"], "envelope-json");
    assert_eq!(value["payload"]["check_id"], "check-1");
    assert_eq!(value["exit_class"], "success_with_warnings");
    assert!(
        value["payload_sha256"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64)
    );
    assert_eq!(value["source_config"]["kind"], "canic_config");
    assert_eq!(value["source_config"]["path_display"], "relative");
    assert!(
        value["source_config"]["path"]
            .as_str()
            .is_some_and(|path| path.contains("deploy-check-envelope-canic.toml"))
    );
    assert!(
        value["summary"]["warnings"]
            .as_array()
            .expect("warnings")
            .iter()
            .any(|warning| warning["code"] == "deploy.warning.test_warning")
    );
    assert!(
        value["inputs"]
            .as_array()
            .expect("inputs")
            .iter()
            .any(|input| input["kind"] == "build_provenance"
                && input["schema"]["id"] == "canic.build_provenance.v1")
    );
    assert!(
        value["command"]["argv_normalized"]
            .as_array()
            .expect("argv")
            .iter()
            .any(|arg| arg == "--build-provenance")
    );
}

#[test]
fn deployment_check_envelope_prefers_evidence_conflict_exit_class() {
    let mut check = sample_authority_check();
    check.report.status = SafetyStatusV1::Blocked;
    check.report.hard_failures.push(SafetyFindingV1 {
        code: "artifact_conflict".to_string(),
        message: "artifact evidence disagrees".to_string(),
        severity: SafetySeverityV1::HardFailure,
        subject: Some("store".to_string()),
    });
    let options = deploy_check::DeployCheckOptions {
        truth: DeployTruthOptions {
            deployment: "demo".to_string(),
            network: "local".to_string(),
            profile: None,
        },
        format: CheckOutputFormat::EnvelopeJson,
        build_provenance: None,
    };

    let envelope = deploy_check::build_deployment_check_envelope(&options, &check)
        .expect("deployment check envelope");
    let value = serde_json::to_value(&envelope).expect("serialize envelope");

    assert_eq!(value["exit_class"], "evidence_conflict");
    assert!(
        value["summary"]["evidence_conflicts"]
            .as_array()
            .expect("evidence conflicts")
            .iter()
            .any(|conflict| conflict["code"] == "deploy.evidence_conflict.artifact_conflict")
    );
}

#[test]
fn deploy_check_builds_current_install_options() {
    let options = DeployTruthOptions {
        deployment: "demo".to_string(),
        network: "local".to_string(),
        profile: Some(CanisterBuildProfile::Fast),
    }
    .into_install_root_options_with_icp_root(Some(std::path::PathBuf::from("/tmp/icp")));

    assert_eq!(options.root_canister, "root");
    assert_eq!(options.root_build_target, "root");
    assert_eq!(options.network, "local");
    assert_eq!(options.build_profile, Some(CanisterBuildProfile::Fast));
    assert_eq!(options.deployment_name.as_deref(), Some("demo"));
    assert_eq!(options.config_path, None);
    assert_eq!(options.expected_fleet, None);
}
