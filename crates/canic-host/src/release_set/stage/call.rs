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

const ICP_STDIN_PATH: &str = "/dev/stdin";

// Run one `icp canister call` and return stdout, preserving stderr on failure.
pub(super) fn icp_call_on_network(
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
        argument,
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
        argument,
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
    argument: Option<&str>,
    output: Option<&str>,
    query: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut command = icp::default_command_in(icp_root);
    command.env("ICP_ENVIRONMENT", network).arg("canister");
    command.args(["call", canister, method]);

    if let Some(output) = output {
        icp::add_output_arg(&mut command, output);
    }

    if argument.is_some() {
        command.arg("--args-file").arg(ICP_STDIN_PATH);
    } else {
        command.arg("()");
    }
    if query {
        command.arg("--query");
    }
    icp::add_target_args(&mut command, Some(network), None, local_replica);

    icp::ensure_command_compatible(&command)?;
    let result = command_output(&mut command, argument)?;

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

fn command_output(
    command: &mut Command,
    argument: Option<&str>,
) -> Result<Output, Box<dyn std::error::Error>> {
    let Some(argument) = argument else {
        return Ok(command.output()?);
    };

    #[cfg(not(unix))]
    {
        let _ = argument;
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "release-set argument streaming requires a Unix /dev/stdin endpoint",
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
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "ICP child did not expose piped stdin",
            )
            .into());
        };
        let write_result = std::io::Write::write_all(&mut stdin, argument.as_bytes());
        drop(stdin);

        if let Err(error) = write_result {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.into());
        }

        Ok(child.wait_with_output()?)
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn command_output_streams_argument_over_stdin() {
        let mut command = Command::new("cat");
        let argument = "(record { value = 1 : nat64 })";

        let output = command_output(&mut command, Some(argument)).expect("stdin transport works");

        assert!(output.status.success());
        assert_eq!(output.stdout, argument.as_bytes());
    }
}
