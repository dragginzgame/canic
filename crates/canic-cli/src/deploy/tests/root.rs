use super::super::root as deploy_root;
use super::fixtures::*;
use super::*;
use canic_host::deployment_truth::{
    DeploymentRootVerificationEvidenceStatusV1, DeploymentRootVerificationStateTransitionV1,
};

#[test]
fn deploy_root_inspect_parses_request_and_text_flag() {
    let options = deploy_root::DeployRootInspectOptions::parse([
        OsString::from("--request"),
        OsString::from("root-verification.json"),
        OsString::from("--text"),
    ])
    .expect("parse deploy inspect root");

    assert_eq!(options.request, PathBuf::from("root-verification.json"));
    assert_eq!(options.format, output_format::JsonTextOutputFormat::Text);
}

#[test]
fn deploy_root_inspect_defaults_to_json() {
    let options = deploy_root::DeployRootInspectOptions::parse([
        OsString::from("--request"),
        OsString::from("root-verification.json"),
    ])
    .expect("parse deploy inspect root");

    assert_eq!(options.request, PathBuf::from("root-verification.json"));
    assert_eq!(options.format, output_format::JsonTextOutputFormat::Json);
}

#[test]
fn deploy_root_verify_parses_deployment_check_and_text_flag() {
    let options = deploy_root::DeployRootVerifyOptions::parse([
        OsString::from("demo-local"),
        OsString::from("--from-check"),
        OsString::from("deployment-check.json"),
        OsString::from("--text"),
        OsString::from("--__canic-network"),
        OsString::from("ic"),
    ])
    .expect("parse deploy root verify");

    assert_eq!(options.deployment, "demo-local");
    assert_eq!(options.from_check, PathBuf::from("deployment-check.json"));
    assert_eq!(options.network, "ic");
    assert_eq!(options.format, output_format::JsonTextOutputFormat::Text);
}

#[test]
fn deploy_root_help_documents_passive_boundary() {
    let help = deploy_root::usage();
    let inspect_help = deploy_root::inspect_usage();
    let verify_help = deploy_root::verify_usage();

    assert!(help.contains("Verify deployment-root state"));
    assert!(help.contains("canic deploy inspect root"));
    assert!(inspect_help.contains("Usage: canic deploy inspect root --request <file>"));
    assert!(inspect_help.contains("DeploymentRootVerificationRequestV1-shaped JSON"));
    assert!(inspect_help.contains("does not persist verified root state"));
    assert!(inspect_help.contains("EvidenceSatisfied means"));
    assert!(verify_help.contains("Verifies a registered deployment root"));
    assert!(verify_help.contains("not full deployment verification"));
    assert!(verify_help.contains("does not install"));
}

#[test]
fn deploy_root_command_dispatches_verify() {
    assert_root_dispatches_leaf(
        "verify",
        [
            OsString::from("demo-local"),
            OsString::from("--from-check"),
            OsString::from("deployment-check.json"),
        ],
    );
}

#[test]
fn root_verification_report_builder_delegates_to_host_report() {
    let report = deploy_root::build_verification_report(sample_root_verification_request())
        .expect("build root verification report");

    assert_eq!(
        report.evidence_status,
        DeploymentRootVerificationEvidenceStatusV1::EvidenceSatisfied
    );
    assert_eq!(
        report.state_transition,
        DeploymentRootVerificationStateTransitionV1::WouldPromoteNotVerifiedToVerified
    );
    assert_eq!(report.deployment_name, "demo");
    assert_eq!(report.source_check_id, "check-1");
    assert_eq!(report.source_inventory_id, "inventory-1");
    assert_eq!(report.report_digest.len(), 64);
}

fn assert_root_dispatches_leaf<const N: usize>(command: &'static str, args: [OsString; N]) {
    let expected_args = args.to_vec();
    let parsed = parse_subcommand(
        deploy_command(),
        std::iter::once(OsString::from("root"))
            .chain(std::iter::once(OsString::from(command)))
            .chain(args),
    )
    .expect("parse deploy root")
    .expect("root command");

    assert_eq!(parsed.0, "root");

    let root = parse_subcommand(deploy_root::command(), parsed.1)
        .expect("parse nested root")
        .expect("root leaf command");
    assert_eq!(root.0, command);
    assert_eq!(root.1, expected_args);
}
