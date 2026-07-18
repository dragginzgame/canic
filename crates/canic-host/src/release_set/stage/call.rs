//! Module: release_set::stage::call
//!
//! Responsibility: execute one explicitly targeted ICP canister call.
//! Does not own: release-set sequencing, request construction, or target resolution.
//! Boundary: streams Candid arguments to the ICP child without disk persistence.

use crate::icp::{self, LocalReplicaTarget};
use std::{
    io,
    process::{Command, Output},
};

const ICP_ARGUMENT_STDIN_PATH: &str = "/dev/stdin";

#[derive(Clone, Copy)]
enum CallArgument<'a> {
    None,
    Text(&'a str),
    Binary(&'a [u8]),
}

// Run one binary-Candid `icp canister call` and return stdout, preserving stderr on failure.
pub(super) fn icp_call_on_network(
    icp_root: &std::path::Path,
    network: &str,
    local_replica: Option<&LocalReplicaTarget>,
    canister: &str,
    method: &str,
    argument: Option<&[u8]>,
    output: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    icp_call_on_network_with_mode(
        icp_root,
        network,
        local_replica,
        canister,
        method,
        argument.map_or(CallArgument::None, CallArgument::Binary),
        output,
        false,
    )
}

// Run one query-only `icp canister call` and return stdout, preserving stderr on failure.
pub(super) fn icp_query_on_network(
    icp_root: &std::path::Path,
    network: &str,
    local_replica: Option<&LocalReplicaTarget>,
    canister: &str,
    method: &str,
    argument: Option<&str>,
    output: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    icp_call_on_network_with_mode(
        icp_root,
        network,
        local_replica,
        canister,
        method,
        argument.map_or(CallArgument::None, CallArgument::Text),
        output,
        true,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "the ICP target and call request stay explicit at this command boundary"
)]
fn icp_call_on_network_with_mode(
    icp_root: &std::path::Path,
    network: &str,
    local_replica: Option<&LocalReplicaTarget>,
    canister: &str,
    method: &str,
    argument: CallArgument<'_>,
    output: Option<&str>,
    query: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut command = icp::default_command_in(icp_root);
    command.arg("canister");
    command.args(["call", canister, method]);

    if let Some(output) = output {
        icp::add_output_arg(&mut command, output);
    }

    let stdin = add_call_argument(&mut command, argument);
    if query {
        command.arg("--query");
    }
    icp::add_target_args(&mut command, Some(network), local_replica);

    icp::ensure_command_compatible(&command)?;
    let result = command_output(&mut command, stdin)?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let stdout = String::from_utf8_lossy(&result.stdout);
        return Err(format!(
            "icp canister call {} {} failed: {}\n{}",
            canister,
            method,
            result.status,
            if stderr.trim().is_empty() {
                stdout.trim()
            } else {
                stderr.trim()
            }
        )
        .into());
    }

    let stdout = String::from_utf8(result.stdout)?;
    Ok(stdout)
}

fn add_call_argument<'a>(command: &mut Command, argument: CallArgument<'a>) -> Option<&'a [u8]> {
    match argument {
        CallArgument::None => {
            command.arg("()");
            None
        }
        CallArgument::Text(argument) => {
            command.arg("--args-file").arg(ICP_ARGUMENT_STDIN_PATH);
            Some(argument.as_bytes())
        }
        CallArgument::Binary(argument) => {
            command
                .arg("--args-file")
                .arg(ICP_ARGUMENT_STDIN_PATH)
                .args(["--args-format", "bin"]);
            Some(argument)
        }
    }
}

fn command_output(
    command: &mut Command,
    argument: Option<&[u8]>,
) -> Result<Output, Box<dyn std::error::Error>> {
    let Some(argument) = argument else {
        return Ok(command.output()?);
    };

    #[cfg(not(unix))]
    {
        let _ = argument;
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "ICP call argument streaming requires a Unix /dev/stdin endpoint",
        )
        .into());
    }

    #[cfg(unix)]
    {
        command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let mut child = command.spawn()?;
        let Some(mut stdin) = child.stdin.take() else {
            terminate_child(&mut child);
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "ICP child did not expose piped stdin",
            )
            .into());
        };
        let write_result = std::io::Write::write_all(&mut stdin, argument);
        drop(stdin);

        if let Err(error) = write_result {
            terminate_child(&mut child);
            return Err(error.into());
        }

        Ok(child.wait_with_output()?)
    }
}

#[cfg(unix)]
fn terminate_child(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn command_output_streams_binary_argument_over_stdin() {
        let mut command = Command::new("cat");
        let argument = b"\0DIDL\x01\xff";

        let output = command_output(&mut command, Some(argument)).expect("stdin transport works");

        assert!(output.status.success());
        assert_eq!(output.stdout, argument);
    }

    #[test]
    fn binary_argument_selects_icp_binary_format() {
        let mut command = Command::new("icp");
        let argument = b"DIDL";

        let stdin = add_call_argument(&mut command, CallArgument::Binary(argument));
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            args,
            [
                "--args-file",
                ICP_ARGUMENT_STDIN_PATH,
                "--args-format",
                "bin"
            ]
        );
        assert_eq!(stdin, Some(argument.as_slice()));
    }
}
