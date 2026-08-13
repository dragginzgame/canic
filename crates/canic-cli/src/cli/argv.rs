//! Module: canic_cli::cli::argv
//!
//! Responsibility: capture opt-in process argument diagnostics before parsing.
//! Does not own: argument semantics, redaction policy, or command dispatch.
//! Boundary: `run_from_env` supplies the process snapshot exactly once.

use std::{env, ffi::OsString, fmt::Write as _};

const TRACE_ARGV_ENV: &str = "CANIC_TRACE_ARGV";

/// Print the exact process argument snapshot when explicit tracing is enabled.
pub fn trace_if_enabled(argv: &[OsString]) {
    if env::var_os(TRACE_ARGV_ENV).as_deref() != Some(std::ffi::OsStr::new("1")) {
        return;
    }

    eprintln!("{}", render_trace(argv));
}

fn render_trace(argv: &[OsString]) -> String {
    let mut trace = format!(
        "canic argv trace: pid={} argc={} current_exe={:?}",
        std::process::id(),
        argv.len(),
        env::current_exe()
    );
    for (index, arg) in argv.iter().enumerate() {
        append_argument(&mut trace, index, arg);
    }
    trace
}

#[expect(
    clippy::unnecessary_debug_formatting,
    reason = "debug formatting preserves OS argument boundaries and escapes"
)]
fn append_argument(trace: &mut String, index: usize, arg: &OsString) {
    let _ = write!(trace, "\nargv[{index}]={arg:?}");
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_preserves_argument_boundaries_and_empty_values() {
        let trace = render_trace(&[
            OsString::from("canic"),
            OsString::from("build"),
            OsString::new(),
            OsString::from("two words"),
        ]);

        assert!(trace.contains("argc=4"));
        assert!(trace.contains("argv[0]=\"canic\""));
        assert!(trace.contains("argv[1]=\"build\""));
        assert!(trace.contains("argv[2]=\"\""));
        assert!(trace.contains("argv[3]=\"two words\""));
    }
}
