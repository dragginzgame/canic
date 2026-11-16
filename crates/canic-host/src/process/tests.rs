use super::*;

#[test]
fn complete_bytes_and_failed_exit_keep_separate_disclosure_policy() {
    let output = capture(
        Command::new("sh").args(["-c", "printf exact; printf private >&2"]),
        16,
        16,
        Duration::from_secs(5),
    )
    .unwrap();
    assert_eq!(output, b"exact");
    let error = capture(
        Command::new("sh").args(["-c", "printf partial; printf private >&2; exit 7"]),
        16,
        16,
        Duration::from_secs(5),
    )
    .unwrap_err();
    assert_eq!(error.report.status.unwrap().code(), Some(7));
    assert!(error.report.failure.is_none() && error.report.cleanup.is_empty());
    assert_eq!(error.stdout, b"partial");
    assert_eq!(error.stderr, b"private");
    assert!(!error.to_string().contains("private"));
}

#[test]
fn output_limit_deadline_and_spawn_failure_cannot_be_success() {
    let error = capture(
        Command::new("sh").args(["-c", "printf 123456789"]),
        4,
        4,
        Duration::from_secs(5),
    )
    .unwrap_err();
    assert!(matches!(
        error.report.failure,
        Some(ic_host_process::Failure::Limit(Stream::Stdout))
    ));
    assert!(error.stdout.len() <= 4 && error.report.cleanup.is_empty());
    let error = capture(
        Command::new("sh").args(["-c", "sleep 5"]),
        4,
        4,
        Duration::from_millis(50),
    )
    .unwrap_err();
    assert!(matches!(
        error.report.failure,
        Some(ic_host_process::Failure::Deadline)
    ));
    assert!(error.report.cleanup.is_empty());
    let error = capture(
        &mut Command::new("/nonexistent/canic-signer"),
        4,
        4,
        Duration::from_secs(5),
    )
    .unwrap_err();
    assert!(matches!(
        error.report.failure,
        Some(ic_host_process::Failure::Io(_))
    ));
}
