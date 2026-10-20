//! Canic fixture workspace discovery and caller-owned Cargo observation budgets.
//!
//! Cargo owns workspace membership; this module selects the framework fixture
//! manifest and bounded diagnostics without adding an elapsed-time deadline.

use ic_host_process::tool::{
    CompletedExecution, ExecutionFailure, OutputLimit, OutputLimits, capture_command,
};
use std::{io, path::PathBuf, process::Command};

/// Discover the workspace containing Canic's internal fixture package.
///
/// # Errors
/// Returns Cargo discovery, malformed response or capture-limit failures.
pub fn discover() -> io::Result<PathBuf> {
    let mut command = ic_build::cargo::observation::workspace_root_command(
        env!("CARGO_MANIFEST_DIR"),
        Command::new("cargo"),
    )?;
    let output = capture_command(
        &mut command,
        OutputLimits {
            stdout: OutputLimit::Terminate(64 * 1024),
            stderr: OutputLimit::Terminate(256 * 1024),
            timeout: None,
        },
    )
    .and_then(CompletedExecution::require_success)
    .and_then(CompletedExecution::require_complete)
    .map_err(|error| {
        let kind = match error.execution_error().map(|execution| &execution.failure) {
            Some(ExecutionFailure::Io { source, .. }) => source.kind(),
            Some(ExecutionFailure::ExitStatus) => io::ErrorKind::InvalidInput,
            _ => io::ErrorKind::Other,
        };
        io::Error::new(kind, error)
    })?;
    ic_build::cargo::observation::decode_workspace_root(&output.stdout)
}

/// Capture build inputs while leaving output placement to `WasmBuildSpec`.
pub(crate) fn wasm_build_environment()
-> impl Iterator<Item = (std::ffi::OsString, std::ffi::OsString)> {
    std::env::vars_os().filter(|(key, _)| key != "CARGO_TARGET_DIR")
}
