//! Canic fixture workspace discovery and caller-owned Cargo observation budgets.
//!
//! Cargo owns workspace membership; this module selects the framework fixture
//! manifest and bounded diagnostics without adding an elapsed-time deadline.

use ic_host_process::tool::{OutputLimit, OutputLimits};
use std::{io, path::PathBuf, process::Command};

/// Discover the workspace containing Canic's internal fixture package.
///
/// # Errors
/// Returns Cargo discovery, malformed response or capture-limit failures.
pub fn discover() -> io::Result<PathBuf> {
    ic_build::cargo::observation::workspace_root_for(
        env!("CARGO_MANIFEST_DIR"),
        OutputLimits {
            stdout: OutputLimit::Terminate(64 * 1024),
            stderr: OutputLimit::Terminate(256 * 1024),
            timeout: None,
        },
        Command::new("cargo"),
    )
}

/// Capture build inputs while leaving output placement to `WasmBuildSpec`.
pub(crate) fn wasm_build_environment()
-> impl Iterator<Item = (std::ffi::OsString, std::ffi::OsString)> {
    std::env::vars_os().filter(|(key, _)| key != "CARGO_TARGET_DIR")
}
