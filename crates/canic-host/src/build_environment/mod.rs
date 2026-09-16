//! Module: build_environment
//!
//! Responsibility: keep deployment-only credentials out of build subprocesses and cache inputs.
//! Does not own: operator transport, release identity or a build sandbox.
//! Boundary: every other inherited environment entry remains a build input.

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

/// Remove the deployment credential even from an explicitly configured command.
pub fn apply(command: &mut Command) {
    command.env_remove(CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV);
}

/// Retain every inherited build input except the credential withheld from children.
pub fn inputs() -> Vec<(OsString, OsString)> {
    env::vars_os()
        .filter(|(key, _)| key != CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV)
        .collect()
}
