use std::{
    cell::Cell,
    io::{self, Write},
    path::Path,
    process::{Command, Stdio},
};

use ic_host_process::{
    child::OwnedChild,
    tool::{
        ExecutionFailure, ExecutionOperation, OutputLimit, OutputLimits, SuccessfulExit, ToolError,
        communicate_child_with_observer,
    },
};

use crate::{output_with_executable_busy_retry, with_executable_busy_retry};

const FOREGROUND_DIAGNOSTIC_BYTES: usize = 64 * 1024;

use super::{
    command::{command_display, configure_inherited_fd, ensure_command_compatible},
    error::IcpCommandError,
    model::{IcpCli, IcpRawOutput},
    version::compatible_version_output,
};

/// Execute a command and capture trimmed stdout.
pub(super) fn run_output(command: &mut Command, icp: &IcpCli) -> Result<String, IcpCommandError> {
    icp.ensure_compatible_command(command)?;
    run_output_unchecked(command)
}

/// Execute a command whose successful stdout contains secret material.
///
/// The caller owns and must zero the returned allocation. This path neither
/// converts stdout into a `String` nor retains it in a diagnostic.
pub(super) fn run_secret_output(
    command: &mut Command,
    icp: &IcpCli,
) -> Result<Vec<u8>, IcpCommandError> {
    icp.ensure_compatible_command(command)?;
    let display = command_display(command);
    let mut output = output_with_executable_busy_retry(command)?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        output.stdout.fill(0);
        Err(IcpCommandError::Failed {
            command: display,
            stderr: command_stderr(&output),
        })
    }
}

fn run_output_unchecked(command: &mut Command) -> Result<String, IcpCommandError> {
    let display = command_display(command);
    let output = output_with_executable_busy_retry(command)?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(IcpCommandError::Failed {
            command: display,
            stderr: command_stderr(&output),
        })
    }
}

/// Execute a command and capture stdout plus stderr on success.
pub fn run_output_with_stderr(command: &mut Command) -> Result<String, IcpCommandError> {
    ensure_command_compatible(command)?;
    let display = command_display(command);
    let output = output_with_executable_busy_retry(command)?;
    if output.status.success() {
        let mut text = String::from_utf8_lossy(&output.stdout).to_string();
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        Ok(text.trim().to_string())
    } else {
        Err(IcpCommandError::Failed {
            command: display,
            stderr: command_stderr(&output),
        })
    }
}

/// Execute a command and parse successful stdout as JSON.
pub(super) fn run_json<T>(command: &mut Command, icp: &IcpCli) -> Result<T, IcpCommandError>
where
    T: serde::de::DeserializeOwned,
{
    icp.ensure_compatible_command(command)?;
    let display = command_display(command);
    let output = output_with_executable_busy_retry(command)?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        serde_json::from_str(&stdout).map_err(|source| IcpCommandError::Json {
            command: display,
            output: stdout,
            source,
        })
    } else {
        Err(IcpCommandError::Failed {
            command: display,
            stderr: command_stderr(&output),
        })
    }
}

/// Execute a command and require a successful status.
pub fn run_status(command: &mut Command) -> Result<(), IcpCommandError> {
    ensure_command_compatible(command)?;
    let display = command_display(command);
    let output = output_with_executable_busy_retry(command)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(IcpCommandError::Failed {
            command: display,
            stderr: command_stderr(&output),
        })
    }
}

/// Execute a command with inherited terminal I/O and require a successful status.
pub(super) fn run_status_inherit(command: &mut Command) -> Result<(), IcpCommandError> {
    ensure_command_compatible(command)?;
    run_status_inherit_unchecked(command)
}

pub(super) fn run_status_inherit_unchecked(command: &mut Command) -> Result<(), IcpCommandError> {
    run_status_inherit_to(command, &mut io::stderr().lock())
}

fn run_status_inherit_to(
    command: &mut Command,
    terminal: &mut impl Write,
) -> Result<(), IcpCommandError> {
    let display = command_display(command);
    command.stdout(Stdio::inherit()).stderr(Stdio::piped());
    let mut child = with_executable_busy_retry(|| OwnedChild::spawn_direct(command))?;
    let forwarding_failed = Cell::new(false);
    let mut forwarding_error = None;
    let result = communicate_child_with_observer(
        &mut child,
        None,
        OutputLimits {
            stdout: OutputLimit::Terminate(0),
            // Forward every byte without terminating a verbose foreground command.
            stderr: OutputLimit::Truncate(FOREGROUND_DIAGNOSTIC_BYTES),
            timeout: None,
        },
        SuccessfulExit::Cleanup,
        || forwarding_failed.get(),
        |_, bytes| {
            if let Err(source) = terminal.write_all(bytes) {
                forwarding_error = Some(source);
                forwarding_failed.set(true);
            }
        },
    );
    if let Some(source) = forwarding_error {
        let kind = source.kind();
        if let Err(ToolError::Execution(mut error)) = result {
            error.failure = ExecutionFailure::Io {
                operation: ExecutionOperation::ReadOutput,
                source,
            };
            return Err(io::Error::new(kind, ToolError::Execution(error)).into());
        }
        return Err(source.into());
    }
    terminal.flush()?;
    result
        .map(|_| ())
        .map_err(|error| foreground_command_error(display, error))
}

fn foreground_command_error(command: String, error: ToolError) -> IcpCommandError {
    if let Some(execution) = error.execution_error()
        && matches!(execution.failure, ExecutionFailure::ExitStatus)
        && execution.cleanup.is_none()
    {
        let mut stderr = if execution.evidence.stderr.is_empty() {
            execution.evidence.status.map_or_else(
                || "command exited unsuccessfully".to_string(),
                |status| format!("command exited with status {}", exit_status_label(status)),
            )
        } else {
            String::from_utf8_lossy(&execution.evidence.stderr).to_string()
        };
        if execution.evidence.stderr_truncated {
            stderr.push_str("\n[retained stderr truncated; full output was forwarded]");
        }
        return IcpCommandError::Failed { command, stderr };
    }
    let kind = match error.execution_error().map(|execution| &execution.failure) {
        Some(ExecutionFailure::Io { source, .. }) => source.kind(),
        _ => io::ErrorKind::Other,
    };
    IcpCommandError::Io(io::Error::new(kind, error))
}

/// Execute a command and return whether it exits successfully.
pub fn run_success(command: &mut Command) -> Result<bool, IcpCommandError> {
    ensure_command_compatible(command)?;
    Ok(output_with_executable_busy_retry(command)?.status.success())
}

/// Execute a rendered ICP CLI command and return raw process output.
pub fn run_raw_output(
    program: &str,
    args: &[String],
    inherited_fd: Option<i32>,
) -> Result<IcpRawOutput, std::io::Error> {
    if is_icp_program(program) {
        compatible_version_output(program, None)
            .map_err(|err| io::Error::other(err.to_string()))?;
    }
    let mut command = Command::new(program);
    command.args(args);
    configure_inherited_fd(&mut command, inherited_fd);
    let output = output_with_executable_busy_retry(&mut command)?;
    Ok(IcpRawOutput {
        success: output.status.success(),
        status: exit_status_label(output.status),
        stdout: output.stdout,
        stderr: output.stderr,
    })
}

fn is_icp_program(program: &str) -> bool {
    Path::new(program)
        .file_name()
        .is_some_and(|file_name| file_name == "icp")
}

// Prefer stderr, but keep stdout diagnostics for CLI commands that report there.
pub(super) fn command_stderr(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.trim().is_empty() {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        stderr.to_string()
    }
}

// Render process exit status without relying on platform-specific internals.
fn exit_status_label(status: std::process::ExitStatus) -> String {
    status
        .code()
        .map_or_else(|| "signal".to_string(), |code| code.to_string())
}
// Operation-local successful qualification is informational, never controller authority.
impl IcpCli {
    pub(super) fn ensure_compatible_command(
        &self,
        command: &Command,
    ) -> Result<(), IcpCommandError> {
        if command.get_program() == std::ffi::OsStr::new(&self.executable)
            && command.get_current_dir() == self.cwd.as_deref()
        {
            self.compatible_version().map(|_| ())
        } else {
            ensure_command_compatible(command)
        }
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{fs, os::unix::process::CommandExt as _};

    #[test]
    fn foreground_runner_forwards_bytes_and_retains_failed_command_diagnostics() {
        for (script, expected) in [
            ("printf 'progress' >&2", None),
            ("printf 'failed' >&2; exit 7", Some("failed")),
            ("exit 7", Some("command exited with status 7")),
        ] {
            let mut terminal = Vec::new();
            let result =
                run_status_inherit_to(Command::new("sh").args(["-c", script]), &mut terminal);
            match expected {
                None => {
                    result.unwrap();
                    assert_eq!(terminal, b"progress");
                }
                Some(expected) => {
                    let IcpCommandError::Failed { stderr, .. } = result.unwrap_err() else {
                        panic!("expected failed command diagnostics");
                    };
                    assert_eq!(stderr, expected);
                    if expected == "failed" {
                        assert_eq!(terminal, b"failed");
                    } else {
                        assert!(terminal.is_empty());
                    }
                }
            }
        }
    }

    #[test]
    fn foreground_runner_forwards_complete_output_with_bounded_failure_diagnostics() {
        let mut command = Command::new("sh");
        command.args(["-c", "perl -e 'print \"x\" x 131072' >&2; exit 7"]);
        let mut terminal = Vec::new();
        let IcpCommandError::Failed { stderr, .. } =
            run_status_inherit_to(&mut command, &mut terminal).unwrap_err()
        else {
            panic!("expected failed command diagnostics");
        };
        assert_eq!(terminal, vec![b'x'; 131_072]);
        assert_eq!(
            stderr,
            format!(
                "{}\n[retained stderr truncated; full output was forwarded]",
                "x".repeat(FOREGROUND_DIAGNOSTIC_BYTES)
            )
        );
    }

    #[test]
    fn foreground_runner_preserves_inherited_and_configured_process_groups() {
        for configured_group in [false, true] {
            let mut command = Command::new("sh");
            command.args(["-c", "printf '%s ' \"$$\" >&2; ps -o pgid= -p $$ >&2"]);
            if configured_group {
                command.process_group(0);
            }
            let mut terminal = Vec::new();
            run_status_inherit_to(&mut command, &mut terminal).unwrap();
            let ids = String::from_utf8(terminal)
                .unwrap()
                .split_whitespace()
                .map(|id| id.parse::<u32>().unwrap())
                .collect::<Vec<_>>();
            let expected_group = if configured_group {
                ids[0]
            } else {
                rustix::process::getpgrp()
                    .as_raw_nonzero()
                    .get()
                    .cast_unsigned()
            };
            assert_eq!(ids[1], expected_group);
        }
    }

    #[test]
    fn foreground_terminal_failure_reaps_the_direct_child_and_retains_io_cause() {
        struct BrokenTerminal;
        impl Write for BrokenTerminal {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let root = crate::test_support::temp_dir("foreground-terminal-error");
        fs::create_dir_all(&root).unwrap();
        let pid_file = root.join("pid");
        let mut command = Command::new("sh");
        command.env("FOREGROUND_PID_FILE", &pid_file).args([
            "-c",
            "printf '%s' \"$$\" > \"$FOREGROUND_PID_FILE\"; printf ready >&2; exec sleep 30",
        ]);
        let IcpCommandError::Io(source) =
            run_status_inherit_to(&mut command, &mut BrokenTerminal).expect_err("terminal failure")
        else {
            panic!("expected terminal IO failure");
        };
        assert_eq!(source.kind(), io::ErrorKind::BrokenPipe);
        let execution = source
            .get_ref()
            .and_then(|source| source.downcast_ref::<ToolError>())
            .and_then(ToolError::execution_error)
            .expect("shared cleanup evidence");
        assert!(matches!(
            &execution.failure,
            ExecutionFailure::Io { source, .. } if source.kind() == io::ErrorKind::BrokenPipe
        ));
        assert!(execution.cleanup.is_none());
        let pid = fs::read_to_string(pid_file)
            .unwrap()
            .parse::<i32>()
            .unwrap();
        assert_eq!(
            rustix::process::test_kill_process(rustix::process::Pid::from_raw(pid).unwrap()),
            Err(rustix::io::Errno::SRCH)
        );
        fs::remove_dir_all(root).unwrap();
    }
}
