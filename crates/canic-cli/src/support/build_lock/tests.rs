use super::*;
use canic_host::canister_build::{BuildLockPhase, KernelBuildLock};

fn wait(seconds: u64) -> BuildLockWait {
    BuildLockWait {
        elapsed: Duration::from_secs(seconds),
        inspection: BuildLockInspection {
            lock_path: "/tmp/work space/.canic/locks/complete-build-reuse.lock".into(),
            recorded_owner: None,
            kernel: KernelBuildLock::Unavailable,
            owner_visibility: BuildProcessVisibility::Unavailable,
            processes: vec![],
            process_snapshot_complete: false,
        },
    }
}

#[test]
fn redirected_output_is_sparse_escape_free_and_finishes_with_an_outcome() {
    let mut painter = WaitPainter::new(false);
    let mut bytes = vec![];
    painter.update(&mut bytes, &wait(1), 80).unwrap();
    let first = bytes.len();
    for second in 2..31 {
        painter.update(&mut bytes, &wait(second), 80).unwrap();
    }
    assert_eq!(bytes.len(), first);
    painter.update(&mut bytes, &wait(31), 80).unwrap();
    assert!(bytes.len() > first);
    painter
        .finish(
            &mut bytes,
            "cancelled (owner unaffected)",
            Duration::from_secs(32),
            80,
        )
        .unwrap();
    assert!(!bytes.contains(&b'\x1b'));
    assert!(
        String::from_utf8(bytes)
            .unwrap()
            .ends_with("Build reuse lock cancelled (owner unaffected) after 32.00s\n")
    );
}

#[test]
fn live_wait_is_one_bounded_line_and_clears_before_summary_even_after_resize() {
    let mut painter = WaitPainter::new(true);
    let mut bytes = vec![];
    painter.update(&mut bytes, &wait(1), 80).unwrap();
    bytes.clear();
    painter.update(&mut bytes, &wait(2), 30).unwrap();
    assert!(!bytes.contains(&b'\n'));
    assert!(painter.painted_width < 30);
    assert!(bytes.starts_with(b"\r\x1b[2K\x1b[1A"));
    bytes.clear();
    painter
        .finish(&mut bytes, "acquired", Duration::from_secs(3), 30)
        .unwrap();
    assert!(bytes.starts_with(b"\r\x1b[2KBuild reuse lock acquired"));
    assert_eq!(painter.painted_width, 0);
}

#[test]
fn advisory_paths_are_escaped_and_shell_inspection_preserves_quotes() {
    let mut wait = wait(1);
    wait.inspection.recorded_owner = Some(BuildLockOwner {
        pid: 42,
        profile: "fast".into(),
        workspace: "/tmp/work\nspace\x1b".into(),
        started_at_unix_seconds: 123,
        identity: None,
        phase: BuildLockPhase::BuildingArtifacts,
        phase_started_at_unix_seconds: 124,
    });
    let rendered = render_inspection(&wait.inspection);
    assert!(!rendered.contains('\x1b'));
    assert!(rendered.contains(r"work\nspace\u{1b}"));
    assert!(rendered.contains("1970-01-01T00:02:03"));
    assert_eq!(
        inspection_command(Path::new("/tmp/a'b")),
        Some("canic diagnostic build-lock --lock '/tmp/a'\\''b'".into())
    );
    assert!(inspection_command(Path::new("/tmp/a\nb")).is_none());
}

#[test]
fn actual_phase_change_is_reported_immediately_in_redirected_output() {
    let mut painter = WaitPainter::new(false);
    let mut bytes = vec![];
    let mut wait = wait(1);
    painter.update(&mut bytes, &wait, 80).unwrap();
    bytes.clear();
    wait.inspection.recorded_owner = Some(BuildLockOwner {
        pid: 42,
        profile: "fast".into(),
        workspace: "/tmp/app".into(),
        started_at_unix_seconds: 123,
        identity: None,
        phase: BuildLockPhase::BuildingArtifacts,
        phase_started_at_unix_seconds: 124,
    });
    painter.update(&mut bytes, &wait, 80).unwrap();
    assert!(!bytes.is_empty());
    assert_eq!(
        painter.last_owner.unwrap().phase,
        BuildLockPhase::BuildingArtifacts
    );
}
