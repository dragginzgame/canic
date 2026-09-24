//! Module: support::build_lock
//!
//! Responsibility: render bounded lock wait progress and read-only recovery guidance.
//! Does not own: lock exclusion, cache admission, or owner termination.
//! Boundary: live output is stderr-TTY-only; process snapshots never assert build progress.

mod cancel;
#[cfg(test)]
mod tests;

use crate::support::path_stamp::utc_timestamp_ns;
use canic_host::canister_build::{
    BuildLockInspection, BuildLockOwner, BuildLockWait, BuildProcessVisibility,
};
use std::{
    io::{self, IsTerminal, Write},
    path::Path,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// CLI wait lifetime; clears live output before subsequent build messages or errors.
pub struct LockWaitDisplay {
    cancel: Option<cancel::Cancellation>,
    painter: WaitPainter,
    started: Instant,
    finished: bool,
}

impl LockWaitDisplay {
    pub(crate) fn start() -> io::Result<Self> {
        let interactive = io::stderr().is_terminal()
            && std::env::var_os("NO_COLOR").is_none()
            && std::env::var("TERM").is_ok_and(|term| !term.is_empty() && term != "dumb");
        Ok(Self {
            cancel: Some(cancel::Cancellation::start()?),
            painter: WaitPainter::new(interactive),
            started: Instant::now(),
            finished: false,
        })
    }

    pub(crate) fn waiting(&mut self, wait: &BuildLockWait) -> io::Result<()> {
        self.check_cancelled()?;
        // An output failure is diagnostic only; it cannot authorize bypassing the lock.
        let _ = self
            .painter
            .update(&mut io::stderr(), wait, terminal_width());
        Ok(())
    }

    pub(crate) fn check_cancelled(&self) -> io::Result<()> {
        self.cancel
            .as_ref()
            .map_or(Ok(()), cancel::Cancellation::check)
    }

    pub(crate) fn finish(&mut self, outcome: &str) {
        let _ = self.painter.finish(
            &mut io::stderr(),
            outcome,
            self.started.elapsed(),
            terminal_width(),
        );
        self.finished = true;
        self.cancel.take();
    }
}

impl Drop for LockWaitDisplay {
    fn drop(&mut self) {
        if !self.finished {
            self.finish("interrupted");
        }
    }
}

fn terminal_width() -> usize {
    rustix::termios::tcgetwinsize(io::stderr()).map_or(80, |size| usize::from(size.ws_col).max(1))
}

/// Stateful presentation only; changes in CPU or scheduler state do not reset progress age.
struct WaitPainter {
    interactive: bool,
    painted_width: usize,
    last_owner: Option<BuildLockOwner>,
    reported: bool,
    next_log: Duration,
}

impl WaitPainter {
    const fn new(interactive: bool) -> Self {
        Self {
            interactive,
            painted_width: 0,
            last_owner: None,
            reported: false,
            next_log: Duration::ZERO,
        }
    }

    fn clear(&mut self, writer: &mut impl Write, width: usize) -> io::Result<()> {
        if self.painted_width > 0 {
            let rows = self.painted_width.div_ceil(width.max(1));
            write!(writer, "\r\x1b[2K")?;
            for _ in 1..rows {
                write!(writer, "\x1b[1A\r\x1b[2K")?;
            }
            self.painted_width = 0;
        }
        Ok(())
    }

    fn update(
        &mut self,
        writer: &mut impl Write,
        wait: &BuildLockWait,
        width: usize,
    ) -> io::Result<()> {
        let changed = !self.reported || self.last_owner != wait.inspection.recorded_owner;
        if changed {
            self.clear(writer, width)?;
            writeln!(writer, "{}", render_inspection(&wait.inspection))?;
            self.last_owner.clone_from(&wait.inspection.recorded_owner);
            self.reported = true;
        }
        let line = compact_wait(wait);
        if self.interactive {
            self.clear(writer, width)?;
            let line = line
                .chars()
                .filter(char::is_ascii)
                .take(width.saturating_sub(1))
                .collect::<String>();
            write!(writer, "{line}")?;
            self.painted_width = line.len();
        } else if changed || wait.elapsed >= self.next_log {
            writeln!(writer, "{line}")?;
            self.next_log = wait.elapsed + Duration::from_secs(30);
        }
        writer.flush()
    }

    fn finish(
        &mut self,
        writer: &mut impl Write,
        outcome: &str,
        elapsed: Duration,
        width: usize,
    ) -> io::Result<()> {
        self.clear(writer, width)?;
        writeln!(
            writer,
            "Build reuse lock {outcome} after {:.2}s",
            elapsed.as_secs_f64()
        )?;
        writer.flush()
    }
}

fn compact_wait(wait: &BuildLockWait) -> String {
    let report = &wait.inspection;
    let owner = report.recorded_owner.as_ref().map_or_else(
        || "owner unknown".into(),
        |owner| {
            if report.owner_visibility == BuildProcessVisibility::Matched {
                format!(
                    "pid={} {:?}; phase age {}",
                    owner.pid,
                    owner.phase,
                    age(owner.phase_started_at_unix_seconds)
                )
            } else {
                format!("advisory pid={}; process unverified", owner.pid)
            }
        },
    );
    let children = if report.process_snapshot_complete {
        report.processes.len().saturating_sub(1).to_string()
    } else {
        "unknown".into()
    };
    format!(
        "Waiting for build lock: {}s; {owner}; children={children}",
        wait.elapsed.as_secs()
    )
}

/// Human snapshot and safe read-only next action, with untrusted paths escaped.
#[expect(
    clippy::unnecessary_debug_formatting,
    reason = "escape diagnostic paths against terminal injection"
)]
pub fn render_inspection(report: &BuildLockInspection) -> String {
    let mut lines = vec![
        format!("Build lock: {:?}", report.lock_path),
        format!(
            "Kernel holder (this process view): {:?}; metadata binding: {:?}",
            report.kernel, report.owner_visibility
        ),
    ];
    if let Some(owner) = &report.recorded_owner {
        lines.push(format!(
            "Recorded owner (advisory): pid={} workspace={:?} profile={}",
            owner.pid, owner.workspace, owner.profile
        ));
        lines.push(format!(
            "Acquired: {} (age {}); last phase: {:?} at {} (age {})",
            timestamp(owner.started_at_unix_seconds),
            age(owner.started_at_unix_seconds),
            owner.phase,
            timestamp(owner.phase_started_at_unix_seconds),
            age(owner.phase_started_at_unix_seconds)
        ));
    } else {
        lines.push(
            "Recorded owner unavailable (empty, malformed, stale-format or unreadable metadata)."
                .into(),
        );
    }
    for process in &report.processes {
        lines.push(format!(
            "Observed process: pid={} kind={:?} state={:?} cpu_ticks={} start_ticks={}",
            process.pid, process.kind, process.activity, process.cpu_ticks, process.start_ticks
        ));
    }
    if !report.process_snapshot_complete {
        lines.push("Process snapshot incomplete or unavailable in this namespace.".into());
    }
    lines.push("Phase timestamps mark actual phase entry; CPU/scheduler observations do not prove forward progress or a stall.".into());
    if report.owner_visibility == BuildProcessVisibility::Matched {
        lines.push("If the owner is progressing, wait. For a quiet owner, inspect its observed child on the host. Cancel only this invocation if it is a redundant waiter; restarting it will not fix the owner.".into());
    } else {
        lines.push("Verify the holder's PID, birth identity and namespace on the host. A hidden/missing PID does not prove an exited owner. After a confirmed exit the kernel releases its lock; a surviving waiter acquires normally and verifies artifacts.".into());
    }
    if let Some(command) = inspection_command(&report.lock_path) {
        lines.push(format!("Read-only inspection: {command}"));
    } else {
        lines.push("Inspection command unavailable for a path containing control or non-UTF-8 bytes; pass the exact path as --lock using your shell's path handling.".into());
    }
    lines.push(
        "Never unlink or replace this lock file, or terminate an owner based only on wait age."
            .into(),
    );
    lines.join("\n")
}

fn inspection_command(path: &Path) -> Option<String> {
    let path = path.to_str()?;
    if path.chars().any(char::is_control) {
        return None;
    }
    Some(format!(
        "canic diagnostic build-lock --lock '{}'",
        path.replace('\'', "'\\''")
    ))
}

fn timestamp(seconds: u64) -> String {
    seconds
        .checked_mul(1_000_000_000)
        .map_or_else(|| "unavailable".into(), utc_timestamp_ns)
}

fn age(seconds: u64) -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|now| now.as_secs().checked_sub(seconds))
        .map_or_else(|| "unavailable".into(), |age| format!("{age}s"))
}
