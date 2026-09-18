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
};

// Exact keys only: compiler inputs, sandbox policy and unknown launcher keys stay bound.
const WITHHELD_INPUTS: [&str; 3] = [
    CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV,
    "CODEX_SESSION_ID",
    "CODEX_THREAD_ID",
];

/// Construct a build/tool command with the same exclusions as its fingerprint.
pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    apply(&mut command);
    command
}

/// Withhold credentials and session correlation IDs; start tools at a fixed shell depth.
pub fn apply(command: &mut Command) {
    for key in WITHHELD_INPUTS {
        command.env_remove(key);
    }
    command.env("SHLVL", "0");
}

/// Fingerprint the environment tools receive, including the fixed shell-depth value.
pub fn inputs() -> Vec<(OsString, OsString)> {
    env::vars_os()
        .filter(|(key, _)| {
            key != "SHLVL" && !WITHHELD_INPUTS.iter().any(|name| key == OsStr::new(name))
        })
        .chain([(OsString::from("SHLVL"), OsString::from("0"))])
        .collect()
}
