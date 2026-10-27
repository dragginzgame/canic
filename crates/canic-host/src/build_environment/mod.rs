//! Module: build_environment
//!
//! Responsibility: establish the same deterministic environment for build tools and cache inputs.
//! Does not own: operator transport, release identity or a build sandbox.
//! Boundary: withhold credentials and run plumbing from both children and fingerprints.

use crate::icp::CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV;
use std::{
    env,
    ffi::{OsStr, OsString},
    process::Command,
    sync::OnceLock,
};

// Build scripts may read arbitrary variables. Remove only owned execution
// plumbing, from the child as well as its fingerprint; unknown inputs stay bound.
const WITHHELD_INPUTS: &[&str] = &[
    CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV,
    "CODEX_SESSION_ID",
    "CODEX_THREAD_ID",
    "CANIC_TEST_SCRATCH",
    "CANIC_TEST_COLOR",
    "CANIC_TEST_PLAN_ONLY",
    "CANIC_TARGET_GOVERNED_CASE",
    "CANIC_POCKETIC_WORKER",
    "CANIC_GOVERNED_CASE_FILE",
    "CANIC_GOVERNED_REPORT_FILE",
    "IC_TESTKIT_POCKET_IC_URL",
    "POCKET_IC_BIN",
    "TMPDIR",
    "TMP",
    "TEMP",
    "PWD",
    "OLDPWD",
    "_",
    "MAKEFLAGS",
    "MFLAGS",
    "MAKELEVEL",
    "MAKEOVERRIDES",
    "CARGO_MAKEFLAGS",
    "VALIDATION_LOG_DIR",
    "VALIDATION_FAILURE_LOG_DIR",
    "VALIDATION_RUNNER_DEPTH",
    "VALIDATION_INPUT_ROOT",
    "VALIDATION_INPUT_BASELINE",
    "GITHUB_STEP_SUMMARY",
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
#[must_use]
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
