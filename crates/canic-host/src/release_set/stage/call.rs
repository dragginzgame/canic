use crate::icp;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

// Run one `icp canister call` and return stdout, preserving stderr on failure.
pub(super) fn icp_call_on_network(
    network: &str,
    canister: &str,
    method: &str,
    argument: Option<&str>,
    output: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    icp_call_on_network_with_mode(network, canister, method, argument, output, false)
}

// Run one query-only `icp canister call` and return stdout, preserving stderr on failure.
pub(super) fn icp_query_on_network(
    network: &str,
    canister: &str,
    method: &str,
    argument: Option<&str>,
    output: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    icp_call_on_network_with_mode(network, canister, method, argument, output, true)
}

fn icp_call_on_network_with_mode(
    network: &str,
    canister: &str,
    method: &str,
    argument: Option<&str>,
    output: Option<&str>,
    query: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let icp_root = super::super::icp_root()?;
    let mut command = icp::default_command_in(&icp_root);
    command.env("ICP_ENVIRONMENT", network).arg("canister");
    command.args(["call", canister, method]);

    if let Some(output) = output {
        icp::add_output_arg(&mut command, output);
    }

    let temp_argument_path = argument.map(write_argument_file).transpose()?;
    if let Some(path) = temp_argument_path.as_ref() {
        command.arg("--args-file").arg(path);
    } else {
        command.arg("()");
    }
    if query {
        command.arg("--query");
    }
    icp::add_target_args(&mut command, Some(network), None);

    icp::ensure_command_compatible(&command)?;
    let result = command.output()?;

    if let Some(path) = temp_argument_path {
        let _ = fs::remove_file(path);
    }

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

// Persist one temporary Candid argument file for `icp --args-file`.
fn write_argument_file(argument: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = std::env::temp_dir().join(format!(
        "canic-release-set-stage-{}-{unique}.did",
        std::process::id()
    ));
    fs::write(&path, argument)?;
    Ok(path)
}
