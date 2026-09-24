//! Module: fleet::progress
//!
//! Responsibility: present host events as live progress, milestones or JSON.
//! Does not own: polling, reconciliation, effects, errors or review decisions.
//! Boundary: the animation clock only redraws retained informational observations.

pub(super) mod receipt;
pub(super) mod render;
mod terminal;
#[cfg(test)]
mod tests;

use crate::fleet::{render_observation_timing, render_progress};
use canic_host::fleet_ensure::dto::{
    FleetEnsureProgress, FleetEnsureProgressState, FleetObservationStage, FleetObservationTiming,
};
use std::{
    io::{self, IsTerminal, Write},
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const HEARTBEAT: Duration = Duration::from_secs(30);

/// Invocation-local milestone identity and last emitted heartbeat.
#[derive(Default)]
struct ProgressOutput {
    last: Option<(FleetEnsureProgress, Instant)>,
}

impl ProgressOutput {
    fn should_emit(&mut self, progress: &FleetEnsureProgress, now: Instant) -> bool {
        if actionable(progress) {
            self.last = None;
            return true;
        }
        let mut identity = without_elapsed(progress);
        // Counts and per-effect targets refresh the panel, not the milestone log.
        if matches!(identity.state, FleetEnsureProgressState::Advancing) {
            identity.applied_effects = 0;
            identity.next_action = None;
        }
        if self.last.as_ref().is_some_and(|(previous, emitted)| {
            previous == &identity && now.saturating_duration_since(*emitted) < HEARTBEAT
        }) {
            return false;
        }
        self.last = Some((identity, now));
        true
    }
}

fn transition_identity(progress: &FleetEnsureProgress) -> FleetEnsureProgress {
    let mut identity = without_elapsed(progress);
    if let FleetEnsureProgressState::AwaitingProgress {
        provisioning: Some(detail),
        ..
    } = &mut identity.state
    {
        detail.pending_root_failure = None;
    }
    identity
}

fn without_elapsed(progress: &FleetEnsureProgress) -> FleetEnsureProgress {
    let mut identity = progress.clone();
    if let FleetEnsureProgressState::AwaitingProgress {
        elapsed_seconds, ..
    } = &mut identity.state
    {
        *elapsed_seconds = 0;
    }
    identity
}

const fn actionable(progress: &FleetEnsureProgress) -> bool {
    !matches!(
        progress.state,
        FleetEnsureProgressState::Advancing | FleetEnsureProgressState::AwaitingProgress { .. }
    )
}

/// Rendering transport selected from the existing JSON flag and terminal capabilities.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Transport {
    Json,
    Live,
    Plain,
}

/// Retained display state shared by callbacks and the local repaint clock.
struct Display {
    receipt: Option<receipt::Receipt>,
    transport: Transport,
    disabled: bool,
    finished: bool,
    context: Option<String>,
    receipt_path: Option<std::path::PathBuf>,
    notice: Option<String>,
    catalog: Option<Vec<String>>,
    planning_reported: bool,
    milestones: ProgressOutput,
    painter: terminal::Painter,
    latest: Option<FleetEnsureProgress>,
    observed: Instant,
    changed: Instant,
}

impl Display {
    fn progress(&mut self, progress: FleetEnsureProgress, now: Instant) -> io::Result<()> {
        if self.finished {
            return Ok(());
        }
        if let Some(receipt) = &mut self.receipt {
            receipt.progress(&progress);
        }
        if self.disabled {
            return Ok(());
        }
        if self.transport == Transport::Json {
            return writeln!(io::stderr().lock(), "{}", render_progress(&progress, true));
        }
        if self.latest.as_ref().map(transition_identity).as_ref()
            != Some(&transition_identity(&progress))
        {
            self.changed = now;
        }
        self.observed = now;
        if self.transport == Transport::Live {
            self.catalog = None;
            self.notice = None;
            self.latest = Some(progress);
            return self.repaint(now);
        }
        if actionable(&progress) {
            self.latest = None;
            self.painter
                .clear(&mut io::stderr().lock(), terminal::size())?;
            // The final report owns the single terminal outcome summary.
            if !matches!(progress.state, FleetEnsureProgressState::Complete) {
                writeln!(io::stderr().lock(), "{}", render_progress(&progress, false))?;
            }
        } else {
            self.latest = Some(progress);
            self.repaint(now)?;
        }
        Ok(())
    }

    fn repaint(&mut self, now: Instant) -> io::Result<()> {
        if self.disabled || self.finished {
            return Ok(());
        }
        if self.transport == Transport::Live {
            let mut lines = vec![
                self.context
                    .clone()
                    .unwrap_or_else(|| "Fleet deployment".into()),
                String::new(),
            ];
            if let Some(progress) = &self.latest {
                lines.extend(render::panel(
                    progress,
                    now.saturating_duration_since(self.observed),
                    now.saturating_duration_since(self.changed),
                ));
            } else if let Some(catalog) = &self.catalog {
                lines.extend(catalog.iter().cloned());
            } else {
                lines.push("Preparing deployment plan...".into());
            }
            if let Some(notice) = &self.notice {
                lines.push(String::new());
                lines.push(notice.clone());
            }
            if let Some(path) = &self.receipt_path {
                lines.push(String::new());
                if self
                    .receipt
                    .as_ref()
                    .is_some_and(receipt::Receipt::has_failed)
                {
                    lines.push("Timing receipt is incomplete: writing failed".into());
                }
                lines.push(format!("Timing receipt: {}", path.display()));
            }
            return self
                .painter
                .paint(&mut io::stderr().lock(), &lines, terminal::size());
        }
        if let Some(progress) = &self.latest
            && self.milestones.should_emit(progress, now)
        {
            writeln!(
                io::stderr().lock(),
                "{}",
                render::milestone(
                    progress,
                    now.saturating_duration_since(self.observed),
                    now.saturating_duration_since(self.changed)
                )
            )?;
        }
        Ok(())
    }

    fn observation(&mut self, timing: FleetObservationTiming) -> io::Result<()> {
        if self.finished {
            return Ok(());
        }
        if let Some(receipt) = &mut self.receipt {
            receipt.observation(&timing);
        }
        if self.disabled {
            return Ok(());
        }
        if self.transport == Transport::Json {
            return writeln!(
                io::stderr().lock(),
                "{}",
                render_observation_timing(&timing, true)
            );
        }
        if timing.succeeded.is_none() {
            return Ok(());
        }
        if self.transport == Transport::Live {
            if timing.succeeded == Some(false) {
                self.notice = Some(render_observation_timing(&timing, false));
            }
            return self.repaint(Instant::now());
        }
        let summary = timing.stage == FleetObservationStage::Planning
            && timing.parent_stage.is_none()
            && !self.planning_reported;
        if timing.succeeded == Some(false) || summary {
            self.painter
                .clear(&mut io::stderr().lock(), terminal::size())?;
            if timing.succeeded == Some(true) {
                self.planning_reported = true;
                writeln!(
                    io::stderr().lock(),
                    "Planning: {}ms, {} remote call attempts (inclusive)",
                    timing.elapsed_millis,
                    timing.remote_call_attempts
                )?;
            } else {
                self.latest = None;
                writeln!(
                    io::stderr().lock(),
                    "{}",
                    render_observation_timing(&timing, false)
                )?;
            }
        }
        Ok(())
    }
}

/// Cloneable callback sink; display failure never changes paid-work decisions.
#[derive(Clone)]
pub(super) struct ProgressSink(Arc<Mutex<Display>>);

impl ProgressSink {
    pub(super) fn progress(&self, progress: FleetEnsureProgress) {
        self.update(|display| display.progress(progress, Instant::now()));
    }

    pub(super) fn observation(&self, timing: FleetObservationTiming) {
        self.update(|display| display.observation(timing));
    }

    pub(super) fn request(&self, timing: canic_host::icp::IcpRequestTiming) {
        self.update(|display| {
            if let Some(receipt) = &mut display.receipt {
                receipt.request(&timing);
            }
            Ok(())
        });
    }

    pub(super) fn catalog(
        &self,
        progress: &canic_host::subnet_catalog::view::CatalogAcquisitionProgress,
    ) {
        self.update(|display| {
            if let Some(receipt) = &mut display.receipt {
                receipt.record("subnet_catalog_progress", progress);
            }
            if display.finished || display.disabled {
                return Ok(());
            }
            if display.transport == Transport::Live {
                display.catalog = Some(
                    crate::fleet::subnet_catalog::render_progress(progress)
                        .lines()
                        .map(str::to_owned)
                        .collect(),
                );
                display.repaint(Instant::now())
            } else {
                crate::fleet::subnet_catalog::print_progress(progress);
                Ok(())
            }
        });
    }

    fn update(&self, update: impl FnOnce(&mut Display) -> io::Result<()>) {
        if let Ok(mut display) = self.0.lock()
            && update(&mut display).is_err()
        {
            display.disabled = true;
            display.latest = None;
            let _ = display
                .painter
                .clear(&mut io::stderr().lock(), terminal::size());
        }
    }
}

/// Joins the repaint worker and clears its frame before final output or errors.
pub(super) struct ProgressSession {
    sink: ProgressSink,
    stop: mpsc::Sender<()>,
    worker: Option<JoinHandle<()>>,
}

impl ProgressSession {
    pub(super) fn new(json: bool) -> Self {
        let live = terminal::supports_live(
            io::stderr().is_terminal(),
            std::env::var("TERM").ok().as_deref(),
            std::env::var_os("NO_COLOR").is_some(),
        ) && !json
            && terminal::install_cleanup().is_ok();
        let now = Instant::now();
        let sink = ProgressSink(Arc::new(Mutex::new(Display {
            receipt: None,
            transport: if json {
                Transport::Json
            } else if live {
                Transport::Live
            } else {
                Transport::Plain
            },
            disabled: false,
            finished: false,
            context: None,
            receipt_path: None,
            notice: None,
            catalog: None,
            planning_reported: false,
            milestones: ProgressOutput::default(),
            painter: terminal::Painter::default(),
            latest: None,
            observed: now,
            changed: now,
        })));
        let (stop, receiver) = mpsc::channel();
        let worker_sink = sink.clone();
        let worker = (!json).then(|| {
            thread::spawn(move || {
                while matches!(
                    receiver.recv_timeout(Duration::from_millis(250)),
                    Err(mpsc::RecvTimeoutError::Timeout)
                ) {
                    worker_sink.update(|display| display.repaint(Instant::now()));
                }
            })
        });
        Self { sink, stop, worker }
    }

    pub(super) fn retain_receipt(
        &self,
        root: &std::path::Path,
        invocation: &receipt::Invocation<'_>,
    ) {
        match receipt::Receipt::create(root, invocation) {
            Ok((mut receipt, path)) => {
                self.sink.update(|display| {
                    if display.transport == Transport::Live { receipt.defer_error_output(); }
                    display.receipt = Some(receipt);
                    display.context = Some(format!("Fleet {} ({})", invocation.fleet, invocation.environment));
                    display.receipt_path = Some(path.clone());
                    if display.transport == Transport::Live {
                        return display.repaint(Instant::now());
                    }
                    display.painter.clear(&mut io::stderr().lock(), terminal::size())?;
                    if display.transport == Transport::Json {
                        writeln!(io::stderr().lock(), "{}", serde_json::json!({
                            "event": "fleet_ensure_timing_receipt", "schema_version": 1, "path": path,
                        }))
                    } else {
                        writeln!(io::stderr().lock(), "Fleet timing receipt: {}", path.display())
                    }
                });
            }
            Err(_) => {
                self.sink.update(|display| {
                    if display.transport == Transport::Live {
                        display.notice = Some("Timing receipt unavailable; deployment progress is not being retained".into());
                        display.repaint(Instant::now())
                    } else {
                        writeln!(io::stderr().lock(), "{}", serde_json::json!({"event": "fleet_timing_receipt_error", "schema_version": 1, "kind": "creation", "incomplete": true}))
                    }
                });
            }
        }
    }

    pub(super) fn finish(
        &self,
        report: Option<&canic_host::fleet_ensure::model::FleetEnsureReport>,
    ) {
        self.sink.update(|display| {
            display.finished = true;
            display.latest = None;
            display
                .painter
                .clear(&mut io::stderr().lock(), terminal::size())?;
            if display.transport == Transport::Live
                && let Some(path) = &display.receipt_path
            {
                writeln!(
                    io::stderr().lock(),
                    "Fleet timing receipt: {}",
                    path.display()
                )?;
            }
            if let Some(receipt) = &mut display.receipt {
                receipt.finish(report);
                if display.transport != Transport::Json {
                    display
                        .painter
                        .clear(&mut io::stderr().lock(), terminal::size())?;
                    receipt.write_summary(
                        &mut io::stderr().lock(),
                        if report.is_some() {
                            "completed"
                        } else {
                            "failed"
                        },
                        report,
                    )?;
                }
            }
            Ok(())
        });
    }

    pub(super) fn finish_generation(&self, succeeded: bool) {
        self.sink.update(|display| {
            display.finished = true;
            display.latest = None;
            display
                .painter
                .clear(&mut io::stderr().lock(), terminal::size())?;
            if display.transport == Transport::Live
                && let Some(path) = &display.receipt_path
            {
                writeln!(
                    io::stderr().lock(),
                    "Fleet timing receipt: {}",
                    path.display()
                )?;
            }
            if let Some(receipt) = &mut display.receipt {
                receipt.close(if succeeded { "completed" } else { "failed" }, None);
                if display.transport != Transport::Json {
                    display
                        .painter
                        .clear(&mut io::stderr().lock(), terminal::size())?;
                    receipt.write_summary(
                        &mut io::stderr().lock(),
                        if succeeded { "completed" } else { "failed" },
                        None,
                    )?;
                }
            }
            Ok(())
        });
    }

    pub(super) fn sink(&self) -> ProgressSink {
        self.sink.clone()
    }
}

impl Drop for ProgressSession {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let mut display = self
            .sink
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = display
            .painter
            .clear(&mut io::stderr().lock(), terminal::size());
    }
}
