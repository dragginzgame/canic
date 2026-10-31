//! Module: build_environment
//!
//! Responsibility: establish the same deterministic environment for build tools and cache inputs.
//! Does not own: operator transport, release identity or a build sandbox.
//! Boundary: withhold credentials and run plumbing from both children and fingerprints.

use crate::icp::CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV;
use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

// Build scripts may read arbitrary variables. Remove only owned execution
// plumbing, from the child as well as its fingerprint; unknown inputs stay bound.
const WITHHELD_INPUTS: &[&str] = &[
    CANIC_ICP_IDENTITY_PASSWORD_FILE_ENV,
    "CODEX_SESSION_ID",
    "CODEX_THREAD_ID",
    "CODEX_TOOL_CALL_ID",
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
    // Shared release delivery context is not the artifact's release identity.
    // CANIC_RELEASE_BUILD_ID and other explicit build inputs remain bound.
    "RELEASE_SOURCE",
    "RELEASE_PREVIOUS",
    "RELEASE_VERSION",
    "RELEASE_DATE",
    "RELEASE_KIND",
    "RELEASE_REMOTE",
    "RELEASE_BRANCH",
    "RELEASE_COMMIT",
    "RELEASE_DELIVERY",
    "RELEASE_PREPARATION_SOURCE",
    "RELEASE_PREPARED_COMMIT",
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
        let codex_home = env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")));
        env::vars_os()
            .filter(|(key, _)| {
                key != "SHLVL" && !WITHHELD_INPUTS.iter().any(|name| key == OsStr::new(name))
            })
            .map(|(key, value)| {
                let value = if key == "PATH" {
                    build_path(&value, codex_home.as_deref())
                } else {
                    value
                };
                (key, value)
            })
            .chain([(OsString::from("SHLVL"), OsString::from("0"))])
            .collect()
    })
}

// Codex creates a fresh PATH directory for editor/sandbox helpers per tool call.
// Keep real tool-search inputs, including unknown entries in that directory.
fn build_path(value: &OsStr, codex_home: Option<&Path>) -> OsString {
    let Some(home) = codex_home else {
        return value.to_owned();
    };
    env::join_paths(env::split_paths(value).filter(|path| !execution_helpers_only(path, home)))
        .unwrap_or_else(|_| value.to_owned())
}

fn execution_helpers_only(path: &Path, home: &Path) -> bool {
    let Some(parent) = path.parent() else {
        return false;
    };
    if parent != home.join("tmp/arg0") && parent != home.join("tmp/arg0-linux") {
        return false;
    }
    if !path
        .file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| name.starts_with("codex-arg0"))
    {
        return false;
    }
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return true,
        Ok(metadata) if metadata.file_type().is_dir() => {}
        _ => return false,
    }
    let Ok(mut entries) = fs::read_dir(path) else {
        return false;
    };
    entries.all(|entry| {
        entry.is_ok_and(|entry| {
            matches!(
                entry.file_name().to_str(),
                Some(".lock" | "apply_patch" | "applypatch" | "codex-linux-sandbox")
            )
        })
    })
}

#[cfg(test)]
mod tests;
