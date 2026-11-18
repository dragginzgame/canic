//! Bounded native command capture; diagnostic disclosure remains with each caller.
use ic_host_process::{Input, Ownership, Policy, Report, Stderr, Stream};
use std::{io, process::Command, sync::atomic::AtomicBool, time::Duration};

#[derive(Debug)]
pub(crate) struct CaptureFailure {
    pub(crate) report: Report,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

impl std::fmt::Display for CaptureFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "command execution refused: status={:?}, failure={:?}, cleanup={}, retained stdout={} stderr={} bytes",
            self.report.status,
            self.report.failure,
            self.report.cleanup.len(),
            self.stdout.len(),
            self.stderr.len()
        )
    }
}

impl std::error::Error for CaptureFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.report.failure.as_ref().map(|error| error as _)
    }
}

pub(crate) fn capture(
    command: &mut Command,
    stdout_limit: usize,
    stderr_limit: usize,
    elapsed: Duration,
) -> Result<Vec<u8>, CaptureFailure> {
    capture_input(command, Input::Null, stdout_limit, stderr_limit, elapsed)
}

pub(crate) fn capture_input(
    command: &mut Command,
    input: Input,
    stdout_limit: usize,
    stderr_limit: usize,
    elapsed: Duration,
) -> Result<Vec<u8>, CaptureFailure> {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let report = ic_host_process::run(
        command,
        Policy {
            ownership: Ownership::Group,
            input,
            stdout_bytes: stdout_limit,
            stderr: Stderr::Capture(stderr_limit),
            elapsed,
        },
        &AtomicBool::new(false),
        |stream, bytes| {
            let buffer = match stream {
                Stream::Stdout => &mut stdout,
                Stream::Stderr => &mut stderr,
            };
            buffer.try_reserve(bytes.len()).map_err(io::Error::other)?;
            buffer.extend_from_slice(bytes);
            Ok(())
        },
    );
    if report.failure.is_some()
        || !report.cleanup.is_empty()
        || !report.status.is_some_and(|status| status.success())
    {
        return Err(CaptureFailure {
            report,
            stdout,
            stderr,
        });
    }
    Ok(stdout)
}

#[cfg(test)]
mod tests;
