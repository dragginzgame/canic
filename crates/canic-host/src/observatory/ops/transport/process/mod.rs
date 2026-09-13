//! Bounded query process ownership; no tool diagnostics enter published views.

use crate::observatory::view::ObservationFailure;
use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};
use std::{
    io::{self, Read},
    os::fd::AsFd,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        // Only the exact child we spawned is reaped. No PID files or process sweeps.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub fn query_output(
    command: &mut Command,
    maximum: usize,
    timeout: Duration,
) -> Result<String, ObservationFailure> {
    let mut child = OwnedChild(
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| ObservationFailure::TransportUnavailable)?,
    );
    let mut stdout = child
        .0
        .stdout
        .take()
        .ok_or(ObservationFailure::TransportUnavailable)?;
    let mut stderr = child
        .0
        .stderr
        .take()
        .ok_or(ObservationFailure::TransportUnavailable)?;
    nonblocking(&stdout)?;
    nonblocking(&stderr)?;
    let start = Instant::now();
    let mut bytes = Vec::new();
    let mut diagnostics = Vec::new();
    loop {
        if start.elapsed() >= timeout {
            return Err(ObservationFailure::TimedOut);
        }
        let output_end = read_available(&mut stdout, &mut bytes, maximum)?;
        let error_end = read_available(&mut stderr, &mut diagnostics, maximum)?;
        let status = child
            .0
            .try_wait()
            .map_err(|_| ObservationFailure::TransportUnavailable)?;
        if let Some(status) = status
            && output_end
            && error_end
        {
            if !status.success() {
                return Err(ObservationFailure::TransportUnavailable);
            }
            return String::from_utf8(bytes).map_err(|_| ObservationFailure::InvalidResponse);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn nonblocking(pipe: &impl AsFd) -> Result<(), ObservationFailure> {
    let flags = fcntl_getfl(pipe).map_err(|_| ObservationFailure::TransportUnavailable)?;
    fcntl_setfl(pipe, flags | OFlags::NONBLOCK)
        .map_err(|_| ObservationFailure::TransportUnavailable)
}

fn read_available(
    pipe: &mut impl Read,
    bytes: &mut Vec<u8>,
    maximum: usize,
) -> Result<bool, ObservationFailure> {
    let mut buffer = [0; 8192];
    // One read per loop keeps a noisy stream from starving the deadline or the other stream.
    match pipe.read(&mut buffer) {
        Ok(0) => Ok(true),
        Ok(length) => {
            if length > maximum.saturating_sub(bytes.len()) {
                return Err(ObservationFailure::BudgetExceeded);
            }
            bytes.extend_from_slice(&buffer[..length]);
            Ok(false)
        }
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
            ) =>
        {
            Ok(false)
        }
        Err(_) => Err(ObservationFailure::TransportUnavailable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_bounds_both_streams_and_reaps_a_hung_child() {
        let mut success = Command::new("sh");
        success.args(["-c", "printf ok"]);
        assert_eq!(
            query_output(&mut success, 32, Duration::from_secs(1)).unwrap(),
            "ok"
        );
        for script in ["printf overflow", "printf overflow >&2"] {
            let mut noisy = Command::new("sh");
            noisy.args(["-c", script]);
            assert_eq!(
                query_output(&mut noisy, 2, Duration::from_secs(1)),
                Err(ObservationFailure::BudgetExceeded)
            );
        }
        let started = Instant::now();
        let mut hung = Command::new("sh");
        hung.args(["-c", "exec sleep 30"]);
        assert_eq!(
            query_output(&mut hung, 32, Duration::from_millis(50)),
            Err(ObservationFailure::TimedOut)
        );
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
