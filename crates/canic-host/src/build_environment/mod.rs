//! Module: build_environment
//!
//! Responsibility: establish the same deterministic environment for build tools and cache inputs.
//! Does not own: operator transport, release identity or a build sandbox.
//! Boundary: withhold credentials and session IDs, reset shell depth, bind all other inputs.

use crate::icp::CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV;
use std::{
    env,
    ffi::{OsStr, OsString},
    process::Command,
    sync::OnceLock,
};

// Exact keys only: compiler inputs, sandbox policy and unknown launcher keys stay bound.
const WITHHELD_INPUTS: [&str; 3] = [
    CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV,
    "CODEX_SESSION_ID",
    "CODEX_THREAD_ID",
];

/// Construct a build/tool command from the captured process build environment.
/// Later command-specific edits take precedence and remain the caller's inputs.
pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    command.env_clear().envs(captured().iter().cloned());
    command
}

/// Fingerprint the same base environment used to construct build commands.
/// Command-specific overrides remain bound by the owning build recipe.
pub fn inputs() -> Vec<(OsString, OsString)> {
    captured().clone()
}

// Capture once per host process, before its first build command or input observation.
// CLI invocations are separate processes; library callers select later changes on commands.
fn captured() -> &'static Vec<(OsString, OsString)> {
    static ENVIRONMENT: OnceLock<Vec<(OsString, OsString)>> = OnceLock::new();
    ENVIRONMENT.get_or_init(|| {
        env::vars_os()
            .filter(|(key, _)| {
                key != "SHLVL" && !WITHHELD_INPUTS.iter().any(|name| key == OsStr::new(name))
            })
            .chain([(OsString::from("SHLVL"), OsString::from("0"))])
            .collect()
    })
}

#[cfg(test)]
mod tests;
