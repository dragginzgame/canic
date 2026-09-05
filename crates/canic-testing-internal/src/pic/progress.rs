//! Module: pic::progress
//!
//! Responsibility: render compact, terminal-aware progress for long internal test journeys.
//! Does not own: test selection, failure policy, or outer validation-runner decoration.
//! Boundary: failed events retain one stable machine identifier across human rendering.

use std::{
    env,
    io::{self, IsTerminal, Write},
    time::Duration,
};

const RESET: &str = "\u{1b}[0m";
const BOLD_CYAN: &str = "\u{1b}[1;36m";
const DIM: &str = "\u{1b}[2m";
const GREEN: &str = "\u{1b}[32m";
const RED: &str = "\u{1b}[31m";
const YELLOW: &str = "\u{1b}[33m";
const FAILURE_EVENT_CODE: &str = "CANIC-TEST:E001";
const DESCRIPTION_WIDTH: usize = 50;
const SCOPE_WIDTH: usize = 12;
const STATUS_WIDTH: usize = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ProgressStatus {
    Cache,
    Done,
    Fail,
    Info,
    #[cfg(test)]
    Pass,
    #[cfg(feature = "pocketic-fixtures")]
    Ready,
    Run,
    #[cfg(test)]
    Slow,
    Wait,
    Warn,
}

impl ProgressStatus {
    const fn label(self) -> &'static str {
        match self {
            Self::Cache => "CACHE",
            Self::Done => "DONE",
            Self::Fail => "FAIL",
            Self::Info => "INFO",
            #[cfg(test)]
            Self::Pass => "PASS",
            #[cfg(feature = "pocketic-fixtures")]
            Self::Ready => "READY",
            Self::Run => "RUN",
            #[cfg(test)]
            Self::Slow => "SLOW",
            Self::Wait => "WAIT",
            Self::Warn => "WARN",
        }
    }

    const fn color(self) -> &'static str {
        match self {
            Self::Cache | Self::Done => GREEN,
            #[cfg(feature = "pocketic-fixtures")]
            Self::Ready => GREEN,
            #[cfg(test)]
            Self::Pass => GREEN,
            Self::Fail => RED,
            Self::Run => BOLD_CYAN,
            Self::Wait | Self::Warn => YELLOW,
            #[cfg(test)]
            Self::Slow => YELLOW,
            Self::Info => DIM,
        }
    }

    const fn event_code(self) -> Option<&'static str> {
        match self {
            Self::Fail => Some(FAILURE_EVENT_CODE),
            _ => None,
        }
    }
}

pub(super) fn event(scope: &str, status: ProgressStatus, description: &str) {
    write_line(render_line(
        scope,
        status,
        description,
        None,
        color_enabled(),
    ));
}

pub(super) fn timed(scope: &str, status: ProgressStatus, description: &str, elapsed: Duration) {
    write_line(render_line(
        scope,
        status,
        description,
        Some(elapsed),
        color_enabled(),
    ));
}

pub(super) fn detail(scope: &str, description: &str) {
    if verbose() {
        event(scope, ProgressStatus::Info, description);
    }
}

pub(super) fn verbose() -> bool {
    env::var("CANIC_TEST_OUTPUT").is_ok_and(|value| value.eq_ignore_ascii_case("verbose"))
}

fn write_line(line: String) {
    let stderr = io::stderr();
    let mut output = stderr.lock();
    let _ = writeln!(output, "{line}");
    let _ = output.flush();
}

fn render_line(
    scope: &str,
    status: ProgressStatus,
    description: &str,
    elapsed: Option<Duration>,
    color: bool,
) -> String {
    let event_code = status
        .event_code()
        .map_or_else(String::new, |code| format!("[{code}] "));
    let scope = format!("[{}]", scope.to_ascii_uppercase().replace('_', "-"));
    let scope = format!("{scope:<SCOPE_WIDTH$}");
    let status_text = format!("{:<STATUS_WIDTH$}", status.label());
    let scope = paint(BOLD_CYAN, &scope, color);
    let status = paint(status.color(), &status_text, color);
    match elapsed {
        Some(elapsed) => format!(
            "{event_code}{scope} {status} {description:<DESCRIPTION_WIDTH$} {:>8}",
            format_duration(elapsed)
        ),
        None => format!("{event_code}{scope} {status} {description}"),
    }
}

fn format_duration(elapsed: Duration) -> String {
    if elapsed >= Duration::from_mins(1) {
        let seconds = elapsed.as_secs();
        format!("{}m {:02}s", seconds / 60, seconds % 60)
    } else {
        format!("{:.2}s", elapsed.as_secs_f64())
    }
}

fn paint(code: &str, text: &str, color: bool) -> String {
    if color {
        format!("{code}{text}{RESET}")
    } else {
        text.to_owned()
    }
}

fn color_enabled() -> bool {
    if env::var_os("NO_COLOR").is_some() {
        return false;
    }
    match env::var("CANIC_TEST_COLOR").as_deref() {
        Ok("always") => true,
        Ok("never") => false,
        _ => {
            io::stderr().is_terminal()
                && env::var("TERM").map_or(true, |term| !term.eq_ignore_ascii_case("dumb"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_progress_is_aligned_and_human_timed() {
        let short = render_line(
            "wasm",
            ProgressStatus::Cache,
            "cycles_ledger_stub",
            Some(Duration::from_millis(450)),
            false,
        );
        let long = render_line(
            "fleet",
            ProgressStatus::Done,
            "literal-zero release artifacts (19)",
            Some(Duration::from_secs(219)),
            false,
        );

        assert_eq!(
            short,
            "[WASM]       CACHE  cycles_ledger_stub                                    0.45s"
        );
        assert_eq!(
            long,
            "[FLEET]      DONE   literal-zero release artifacts (19)                  3m 39s"
        );
    }

    #[test]
    fn ansi_progress_preserves_the_plain_text_fields() {
        let rendered = render_line("suite", ProgressStatus::Fail, "terminal replay", None, true);

        assert!(rendered.starts_with("[CANIC-TEST:E001] "));
        assert!(rendered.contains("\u{1b}[1;36m[SUITE]"));
        assert!(rendered.contains("\u{1b}[31mFAIL"));
        assert!(rendered.ends_with("terminal replay"));
    }
}
