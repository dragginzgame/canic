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

// Ensure the medic report is stable, readable block text instead of a wide table.
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

    assert!(report.starts_with("network [ok]"));
    assert!(report.contains("\n  detail: local\n"));
    assert!(report.contains("deployment state [warn]"));
    assert!(report.contains("  next: run canic install"));
    assert!(!report.contains("CHECK"));
}

// Ensure long medic details and next actions wrap to terminal-readable lines.
#[test]
fn wraps_long_medic_report_fields() {
    let report = render_medic_report(&[MedicCheck::warn(
        "deployment state",
        "this is a deliberately long diagnostic message that should wrap across multiple indented lines instead of widening a terminal table",
        "run canic install <fleet-template> or canic deploy register <deployment> --fleet-template <fleet-template> --root <principal> --allow-unverified",
    )]);

    assert!(report.contains("deployment state [warn]"));
    assert!(
        report
            .lines()
            .all(|line| line.chars().count() <= MEDIC_REPORT_WIDTH)
    );
    assert!(
        report
            .lines()
            .any(|line| line.starts_with("          ") && !line.trim().is_empty())
    );
}

// Ensure ICP identity session guidance stays informational and versionless.
#[test]
fn icp_identity_session_cache_hint_is_informational() {
    let check = check_icp_identity_session_cache_hint();

    assert_eq!(check.status, MedicStatus::Ok);
    assert_eq!(check.name, "icp identity session");
    assert!(check.detail.contains("PEM identities"));
    assert!(check.next.contains("icp settings session-length"));
    assert!(check.next.contains("icp identity reauth"));
    assert!(!check.next.contains("1.0.0"));
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
