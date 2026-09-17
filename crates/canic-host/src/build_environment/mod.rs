//! Module: build_environment
//!
//! Responsibility: establish the same deterministic environment for build tools and cache inputs.
//! Does not own: operator transport, release identity or a build sandbox.
//! Boundary: withhold deployment credentials, reset shell depth, bind every other inherited input.

use crate::icp::CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV;
use std::{
    env,
    ffi::{OsStr, OsString},
    process::Command,
};

/// Construct a build/tool command with the same exclusions as its fingerprint.
pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    apply(&mut command);
    command
}

/// Withhold deployment credentials and start build tools at a fixed shell depth.
pub fn apply(command: &mut Command) {
    command
        .env_remove(CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV)
        .env("SHLVL", "0");
}

/// Fingerprint the environment tools receive, including the fixed shell-depth value.
pub fn inputs() -> Vec<(OsString, OsString)> {
    env::vars_os()
        .filter(|(key, _)| key != CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV && key != "SHLVL")
        .chain([(OsString::from("SHLVL"), OsString::from("0"))])
        .collect()
}
