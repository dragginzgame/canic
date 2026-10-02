//! Verify the maintained recovery leaf and structured effect-free usage failures.

use super::*;

#[test]
fn recovery_requires_exact_digest_and_exposes_explicit_local_approval() {
    let usage = render_usage(command);
    assert!(usage.contains("--apply <REVIEW_SHA256>"));
    assert!(
        command()
            .try_get_matches_from(["recover-attempts", "staging", "--apply", "bad"])
            .is_err()
    );
    let digest = "ab".repeat(32);
    let matches = command()
        .try_get_matches_from(["recover-attempts", "staging", "--apply", &digest])
        .unwrap();
    assert_eq!(string_option(&matches, "apply"), Some(digest));
}

#[test]
fn json_usage_failure_retains_its_typed_source() {
    let error = run(vec!["staging".into(), "--json".into()]).unwrap_err();
    let FleetCommandError::JsonReported { report, source } = error else {
        panic!("structured recovery failure")
    };
    assert!(matches!(*source, FleetCommandError::Usage(_)));
    let report: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["event"], "fleet_attempt_recovery_error");
}

#[test]
fn recovery_accepts_forwarded_globals_without_resolving_the_executable() {
    let mut args = vec!["staging".into()];
    crate::cli::globals::apply_global_icp("fleet", &mut args, Some("/nonexistent/icp".into()));
    crate::cli::globals::apply_global_environment("fleet", &mut args, Some("local".into()));
    let matches = parse_matches(command(), args).unwrap();
    assert_eq!(
        string_option(&matches, "environment").as_deref(),
        Some("local")
    );
    assert_eq!(
        string_option(&matches, "icp").as_deref(),
        Some("/nonexistent/icp")
    );
}
