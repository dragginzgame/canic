use super::*;

// Ensure medic options parse the deployment target, network, and ICP CLI selectors.
#[test]
fn parses_medic_options() {
    let options = MedicOptions::parse_info([
        OsString::from("demo"),
        OsString::from(crate::cli::globals::INTERNAL_NETWORK_OPTION),
        OsString::from("local"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/tmp/icp"),
    ])
    .expect("parse medic options");

    assert_eq!(options.deployment, "demo");
    assert_eq!(options.network, "local");
    assert_eq!(options.icp, "/tmp/icp");
}

// Ensure medic help explains the diagnostic command rather than printing a one-liner.
#[test]
fn medic_usage_includes_examples() {
    let text = info_usage();

    assert!(text.contains("Diagnose local Canic deployment target setup"));
    assert!(text.contains("Usage: canic info medic <deployment>"));
    assert!(text.contains("<deployment>"));
    assert!(!text.contains("--fleet <name>"));
    assert!(!text.contains("--network"));
    assert!(!text.contains("--icp"));
    assert!(text.contains("Examples:"));
}

// Ensure the medic report is a stable whitespace table.
#[test]
fn renders_medic_report() {
    let report = render_medic_report(&[
        MedicCheck::ok("network", "local", "-"),
        MedicCheck::warn(
            "deployment state",
            "no installed deployment found",
            "run canic install",
        ),
    ]);

    assert!(report.starts_with("CHECK"));
    assert!(report.contains("network"));
    assert!(report.contains("deployment state"));
    assert!(report.contains("warn"));
}

// Ensure host installed-deployment missing-state errors remain warnings, not failures.
#[test]
fn missing_installed_deployment_error_is_warnable() {
    assert!(is_missing_installed_deployment(
        "deployment target demo is not installed on network local"
    ));
    assert!(!is_missing_installed_deployment(
        "failed to read canic deployment state: bad json"
    ));
}

// Ensure common command-line JSON shapes are accepted for readiness.
#[test]
fn parses_ready_json_shapes() {
    assert!(replica_query::parse_ready_json_value(&serde_json::json!(
        true
    )));
    assert!(replica_query::parse_ready_json_value(
        &serde_json::json!([{"Ok": true}])
    ));
    assert!(!replica_query::parse_ready_json_value(
        &serde_json::json!([{"Ok": false}])
    ));
}
