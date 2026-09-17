//! Module: canister_build::process
//!
//! Responsibility: inspect process ancestry and report bounded waits for build children.
//! Does not own: Cargo arguments, build identity, compiler output or retries.
//! Boundary: preserve command results and streams while exposing phase liveness.

use std::{
    fs, io,
    process::{Command, Output},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

/// Capture the original child result while reporting a bounded phase heartbeat.
pub(super) fn output_with_progress(
    command: &mut Command,
    interval: Duration,
    progress: impl Fn(Duration) + Send,
) -> io::Result<Output> {
    thread::scope(|scope| {
        // Dropping the sender wakes the observer on return, launch error or unwind.
        let (_complete, done) = mpsc::channel::<()>();
        let started = Instant::now();
        scope.spawn(move || {
            while done.recv_timeout(interval) == Err(mpsc::RecvTimeoutError::Timeout) {
                progress(started.elapsed());
            }
        });
        command.output()
    })
}

pub(super) fn parent_process_id() -> Option<u32> {
    let stat = fs::read_to_string("/proc/self/stat").ok()?;
    parse_parent_process_id(&stat)
}

// Walk ancestor processes until the wrapping `icp` process is found.
pub(super) fn icp_ancestor_process_id() -> Option<u32> {
    let mut pid = parent_process_id()?;
    loop {
        if process_comm(pid).as_deref() == Some("icp") {
            return Some(pid);
        }

        let parent = process_parent_id(pid)?;
        if parent == 0 || parent == pid {
            return None;
        }
        pid = parent;
    }
}

// Read one ancestor's parent process id from procfs.
fn process_parent_id(pid: u32) -> Option<u32> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    parse_parent_process_id(&stat)
}

// Read one process command name from procfs.
fn process_comm(pid: u32) -> Option<String> {
    fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|comm| comm.trim().to_string())
}

// Parse Linux `/proc/<pid>/stat` enough to extract the parent process id.
pub(super) fn parse_parent_process_id(stat: &str) -> Option<u32> {
    let (_, suffix) = stat.rsplit_once(") ")?;
    let mut parts = suffix.split_whitespace();
    let _state = parts.next()?;
    parts.next()?.parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    #[test]
    #[cfg(unix)]
    fn cargo_progress_unblocks_child_without_exposing_or_losing_its_output() {
        let root = temp_dir("cargo-heartbeat");
        fs::create_dir_all(&root).unwrap();
        let ready = root.join("ready");
        let mut command = Command::new("sh");
        command.args(["-c", "for i in $(seq 1 500); do if [ -f \"$1\" ]; then printf 'captured-out'; printf 'captured-err' >&2; exit 7; fi; sleep 0.01; done; exit 99", "heartbeat"]).arg(&ready);
        let output = output_with_progress(&mut command, Duration::from_millis(1), |_| {
            fs::write(&ready, b"ready").unwrap();
        })
        .unwrap();
        assert_eq!(output.status.code(), Some(7));
        assert_eq!(output.stdout, b"captured-out");
        assert_eq!(output.stderr, b"captured-err");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cargo_launch_failure_does_not_wait_for_the_progress_interval() {
        let root = temp_dir("cargo-heartbeat-missing");
        fs::create_dir_all(&root).unwrap();
        let error = output_with_progress(
            &mut Command::new(root.join("missing-command")),
            Duration::from_secs(2),
            |_| panic!("a launch failure must stop the observer"),
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        fs::remove_dir_all(root).unwrap();
    }
}
