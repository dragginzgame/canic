//! Module: fleet::progress::receipt
//!
//! Responsibility: retain bounded invocation evidence from the existing progress owner.
//! Boundary: diagnostics never authorize effects, replace a journal or change command results.

mod summary;
#[cfg(test)]
mod tests;

use canic_host::fleet_ensure::{
    dto::{FleetEnsureProgress, FleetEnsureProgressState, FleetObservationTiming},
    model::{FleetEnsurePlanScope, FleetEnsureReport},
};
use serde::Serialize;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_EVENT_BYTES: usize = 64 * 1024;
static NEXT_FILE: AtomicU64 = AtomicU64::new(1);

/// The command whose existing diagnostics are retained by this invocation.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::fleet) enum CommandKind {
    Ensure,
    Generate,
}

/// Exact local input binding; this diagnostic is never execution authority.
#[derive(Serialize)]
pub(in crate::fleet) struct Invocation<'a> {
    pub command: CommandKind,
    pub fleet: &'a str,
    pub environment: &'a str,
    pub desired_sha256: Option<&'a str>,
    pub applied_plan_sha256: Option<&'a str>,
    pub reinstall: bool,
    pub next_review_command: &'a str,
}

/// Append-only evidence with bounded output and an explicit terminal record when available.
pub(super) struct Receipt {
    file: File,
    started: Instant,
    bytes: usize,
    omitted_events: u64,
    finished: bool,
    failed: bool,
    emit_errors: bool,
    active_stages: Vec<u64>,
    last_progress: Option<FleetEnsureProgress>,
    summary: summary::Summary,
    next_review_command: String,
}

#[derive(Serialize)]
struct Event<'a, T> {
    schema_version: u8,
    event: &'a str,
    /// UTC Unix epoch milliseconds, independent of the monotonic duration clock.
    utc_unix_millis: u128,
    elapsed_micros: u128,
    data: &'a T,
}

#[derive(Serialize)]
struct Outcome<'a> {
    state: &'static str,
    operation_id: Option<&'a str>,
    plan_sha256: Option<&'a str>,
    terminal: Option<bool>,
    plan_scope: Option<canic_host::fleet_ensure::model::FleetEnsurePlanScope>,
    effects_applied: Option<u32>,
    omitted_events: u64,
}

impl Receipt {
    pub(super) fn create(root: &Path, invocation: &Invocation<'_>) -> io::Result<(Self, PathBuf)> {
        let directory = root.join(".canic/diagnostics/fleet");
        fs::create_dir_all(&directory)?;
        let path = directory.join(format!(
            "{}-{}-{}.jsonl",
            utc_millis(),
            std::process::id(),
            NEXT_FILE.fetch_add(1, Ordering::Relaxed),
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        // create_new rejects any existing destination, including a final symlink.
        let file = options.open(&path)?;
        let mut receipt = Self {
            file,
            started: Instant::now(),
            bytes: 0,
            omitted_events: 0,
            finished: false,
            failed: false,
            // The caller reports construction failures once; Drop must not
            // print a second diagnostic before output ownership is established.
            emit_errors: false,
            active_stages: Vec::new(),
            last_progress: None,
            summary: summary::Summary::default(),
            next_review_command: invocation.next_review_command.to_owned(),
        };
        receipt.append("invocation_started", invocation, true)?;
        if receipt.omitted_events > 0 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        receipt.file.sync_data()?;
        receipt.emit_errors = true;
        Ok((receipt, path))
    }

    /// Let the screen owner display failures; final summaries still mark partial evidence.
    pub(super) const fn defer_error_output(&mut self) {
        self.emit_errors = false;
    }

    pub(super) const fn has_failed(&self) -> bool {
        self.failed
    }

    pub(super) fn progress(&mut self, progress: &FleetEnsureProgress) {
        let confirmed_remote_advancement = self.last_progress.as_ref().is_some_and(|previous| {
            previous.operation_id == progress.operation_id
                && previous.plan_sha256 == progress.plan_sha256
                && (progress.applied_effects > previous.applied_effects
                    || provisioning_advanced(previous, progress))
        });
        self.record(
            "fleet_ensure_progress",
            &serde_json::json!({
                "progress": progress, "confirmed_remote_advancement": confirmed_remote_advancement,
            }),
        );
        self.last_progress = Some(progress.clone());
    }

    pub(super) fn observation(&mut self, timing: &FleetObservationTiming) {
        self.summary.observe(timing);
        self.record("fleet_ensure_observation", timing);
        if timing.succeeded.is_none() {
            self.active_stages.push(timing.span_id);
        } else {
            self.active_stages.retain(|id| *id != timing.span_id);
        }
    }

    pub(super) fn request(&mut self, timing: &canic_host::icp::IcpRequestTiming) {
        self.record(
            "icp_request_timing",
            &serde_json::json!({
                "parent_span_id": self.active_stages.last(), "request": timing,
            }),
        );
    }

    pub(super) fn record(&mut self, name: &str, value: &impl Serialize) {
        if !self.failed && !self.finished && self.append(name, value, false).is_err() {
            self.failed = true;
            if self.emit_errors {
                let _ = writeln!(
                    io::stderr().lock(),
                    "{}",
                    serde_json::json!({"event": "fleet_timing_receipt_error", "schema_version": 1, "kind": "write", "incomplete": true})
                );
            }
        }
    }

    fn append(&mut self, name: &str, value: &impl Serialize, terminal: bool) -> io::Result<()> {
        let event = Event {
            schema_version: 1,
            event: name,
            utc_unix_millis: utc_millis(),
            elapsed_micros: self.started.elapsed().as_micros(),
            data: value,
        };
        let mut bytes = serde_json::to_vec(&event)?;
        bytes.push(b'\n');
        let limit = if terminal {
            MAX_BYTES
        } else {
            MAX_BYTES - MAX_EVENT_BYTES
        };
        if bytes.len() > MAX_EVENT_BYTES || self.bytes.saturating_add(bytes.len()) > limit {
            self.omitted_events = self.omitted_events.saturating_add(1);
            return Ok(());
        }
        // Each callback writes directly; a killed process leaves complete earlier lines.
        // A partial final line or absent outcome must be treated as incomplete evidence.
        self.file.write_all(&bytes)?;
        self.bytes += bytes.len();
        Ok(())
    }

    pub(super) fn finish(&mut self, report: Option<&FleetEnsureReport>) {
        self.close(
            if report.is_some() {
                "completed"
            } else {
                "failed"
            },
            report,
        );
    }

    pub(super) fn close(&mut self, state: &'static str, report: Option<&FleetEnsureReport>) {
        if self.finished || self.failed {
            return;
        }
        let outcome = Outcome {
            state,
            operation_id: report
                .map(|report| report.plan.operation_id.as_str())
                .or_else(|| {
                    self.last_progress
                        .as_ref()
                        .map(|progress| progress.operation_id.as_str())
                }),
            plan_sha256: report
                .map(|report| report.plan.plan_sha256.as_str())
                .or_else(|| {
                    self.last_progress
                        .as_ref()
                        .map(|progress| progress.plan_sha256.as_str())
                }),
            terminal: report.map(|report| report.terminal),
            plan_scope: report.map(|report| report.plan.scope),
            effects_applied: report.map(|report| report.effects_applied),
            omitted_events: self.omitted_events,
        };
        let outcome = serde_json::to_value(outcome).expect("bounded diagnostic outcome serializes");
        if self
            .append("invocation_finished", &outcome, true)
            .and_then(|()| self.file.sync_data())
            .is_err()
        {
            self.failed = true;
            if self.emit_errors {
                let _ = writeln!(
                    io::stderr().lock(),
                    "{}",
                    serde_json::json!({"event": "fleet_timing_receipt_error", "schema_version": 1, "kind": "finalization", "incomplete": true})
                );
            }
        }
        self.finished = true;
    }

    pub(super) fn write_summary(
        &self,
        output: &mut impl Write,
        state: &str,
        report: Option<&FleetEnsureReport>,
    ) -> io::Result<()> {
        let partial = self.failed || self.omitted_events > 0 || !self.active_stages.is_empty();
        writeln!(
            output,
            "Fleet invocation {state}: {} ms; timing evidence {}.",
            self.started.elapsed().as_millis(),
            if partial { "partial" } else { "retained" },
        )?;
        self.summary.write(output)?;
        writeln!(
            output,
            "Phase costs cover completed outer observations only; they are not an end-to-end breakdown."
        )?;
        if let Some(progress) = &self.last_progress {
            writeln!(
                output,
                "Last persisted effects: {}; operation {}; plan {}.",
                progress.applied_effects, progress.operation_id, progress.plan_sha256,
            )?;
        }
        if report.is_some_and(|report| {
            report.terminal && report.plan.scope == FleetEnsurePlanScope::Full
        }) {
            writeln!(output, "Fleet convergence verified for this invocation.")?;
        } else {
            writeln!(
                output,
                "Fleet convergence is not established by this invocation."
            )?;
            writeln!(output, "Review: {}", self.next_review_command)?;
        }
        Ok(())
    }
}

impl Drop for Receipt {
    fn drop(&mut self) {
        self.close("incomplete", None);
    }
}

fn utc_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

const fn provisioning_advanced(
    previous: &FleetEnsureProgress,
    current: &FleetEnsureProgress,
) -> bool {
    let (
        FleetEnsureProgressState::AwaitingProgress {
            provisioning: Some(before),
            ..
        },
        FleetEnsureProgressState::AwaitingProgress {
            provisioning: Some(after),
            ..
        },
    ) = (&previous.state, &current.state)
    else {
        return false;
    };
    after.provisioned_root_count > before.provisioned_root_count
        || after.directory_confirmed_root_count > before.directory_confirmed_root_count
        || after.runtime_activated_root_count > before.runtime_activated_root_count
}
