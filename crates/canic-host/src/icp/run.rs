use std::{
    io::{self, Read, Write},
    path::Path,
    process::{Command, Stdio},
    thread,
};

use super::{
    command::{command_display, ensure_command_compatible},
    error::IcpCommandError,
    model::IcpRawOutput,
    version::compatible_version_output,
};

/// Execute a command and capture trimmed stdout.
pub fn run_output(command: &mut Command) -> Result<String, IcpCommandError> {
    ensure_command_compatible(command)?;
    run_output_unchecked(command)
}

pub(super) fn run_output_unchecked(command: &mut Command) -> Result<String, IcpCommandError> {
    let display = command_display(command);
    let output = command.output()?;
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
    let output = command.output()?;
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
pub fn run_json<T>(command: &mut Command) -> Result<T, IcpCommandError>
where
    T: serde::de::DeserializeOwned,
{
    ensure_command_compatible(command)?;
    let display = command_display(command);
    let output = command.output()?;
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
    let output = command.output()?;
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
pub fn run_status_inherit(command: &mut Command) -> Result<(), IcpCommandError> {
    ensure_command_compatible(command)?;
    let display = command_display(command);
    let mut child = command
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .spawn()?;
    let stderr_handle = child
        .stderr
        .take()
        .map(|stderr| thread::spawn(move || stream_and_capture_stderr(stderr)));
    let status = child.wait()?;
    let stderr = match stderr_handle {
        Some(handle) => match handle.join() {
            Ok(result) => result?,
            Err(_) => Vec::new(),
        },
        None => Vec::new(),
    };
    if status.success() {
        Ok(())
    } else {
        let stderr = if stderr.is_empty() {
            format!("command exited with status {}", exit_status_label(status))
        } else {
            String::from_utf8_lossy(&stderr).to_string()
        };
        Err(IcpCommandError::Failed {
            command: display,
            stderr,
        })
    }
}

fn stream_and_capture_stderr(mut stderr: impl Read) -> io::Result<Vec<u8>> {
    let mut captured = Vec::new();
    let mut buffer = [0_u8; 8192];
    let mut terminal = io::stderr().lock();
    loop {
        let read = stderr.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        terminal.write_all(&buffer[..read])?;
        captured.extend_from_slice(&buffer[..read]);
    }
    terminal.flush()?;
    Ok(captured)
}

/// Execute a command and return whether it exits successfully.
pub fn run_success(command: &mut Command) -> Result<bool, IcpCommandError> {
    ensure_command_compatible(command)?;
    Ok(command.output()?.status.success())
}

/// Execute a rendered ICP CLI command and return raw process output.
pub fn run_raw_output(program: &str, args: &[String]) -> Result<IcpRawOutput, std::io::Error> {
    if is_icp_program(program) {
        compatible_version_output(program, None)
            .map_err(|err| io::Error::other(err.to_string()))?;
    }
    let output = Command::new(program).args(args).output()?;
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
