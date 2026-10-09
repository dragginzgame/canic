//! Bounded query process ownership; no tool diagnostics enter published views.

use crate::observatory::view::ObservationFailure;
use ic_host_process::tool::{
    ExecutionFailure, InvalidInvocation, OutputLimits, ToolError, capture_command,
};
use std::{process::Command, time::Duration};

pub fn query_output(
    command: &mut Command,
    maximum: usize,
    timeout: Duration,
) -> Result<String, ObservationFailure> {
    let output = capture_command(
        command,
        OutputLimits {
            stdout_bytes: maximum,
            stderr_bytes: maximum,
            timeout,
        },
    )
    .map_err(|error| observation_failure(&error))?;
    String::from_utf8(output.stdout).map_err(|_| ObservationFailure::InvalidResponse)
}

fn observation_failure(error: &ToolError) -> ObservationFailure {
    if matches!(
        error,
        ToolError::InvalidInvocation(InvalidInvocation::Deadline)
    ) {
        return ObservationFailure::TimedOut;
    }
    match error.execution_error().map(|error| &error.failure) {
        Some(ExecutionFailure::TimedOut) => ObservationFailure::TimedOut,
        Some(ExecutionFailure::OutputLimit { .. }) => ObservationFailure::BudgetExceeded,
        _ => ObservationFailure::TransportUnavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_host_process::tool::{ExecutionError, ExecutionEvidence, OutputStream};

    #[test]
    fn shared_capture_failures_project_to_redacted_observation_failures() {
        for (failure, expected) in [
            (ExecutionFailure::TimedOut, ObservationFailure::TimedOut),
            (
                ExecutionFailure::OutputLimit {
                    stream: OutputStream::Stdout,
                },
                ObservationFailure::BudgetExceeded,
            ),
            (
                ExecutionFailure::OutputLimit {
                    stream: OutputStream::Stderr,
                },
                ObservationFailure::BudgetExceeded,
            ),
            (
                ExecutionFailure::ExitStatus,
                ObservationFailure::TransportUnavailable,
            ),
        ] {
            let error = ToolError::Execution(Box::new(ExecutionError {
                failure,
                evidence: ExecutionEvidence {
                    stderr: b"private diagnostics".to_vec(),
                    ..ExecutionEvidence::default()
                },
                term_error: None,
                group_error: None,
                kill_error: None,
                wait_error: None,
            }));
            assert_eq!(observation_failure(&error), expected);
        }
        assert_eq!(
            observation_failure(&ToolError::InvalidInvocation(InvalidInvocation::Deadline)),
            ObservationFailure::TimedOut
        );
    }

    #[test]
    fn shared_capture_preserves_text_and_refuses_invalid_utf8() {
        for (script, expected) in [
            ("printf ok", Ok("ok".into())),
            ("printf '\\377'", Err(ObservationFailure::InvalidResponse)),
            (
                "printf private >&2; exit 7",
                Err(ObservationFailure::TransportUnavailable),
            ),
        ] {
            let mut command = Command::new("sh");
            command.args(["-c", script]);
            assert_eq!(
                query_output(&mut command, 32, Duration::from_secs(1)),
                expected
            );
        }
    }
}
