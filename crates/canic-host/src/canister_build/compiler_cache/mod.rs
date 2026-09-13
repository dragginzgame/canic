//! Module: canister_build::compiler_cache
//!
//! Responsibility: diagnose compiler startup through an implicitly selected cache.
//! Does not own: wrapper selection, Cargo compilation or automatic build retries.
//! Boundary: probes inherit the pending Cargo command's working directory and environment.

use std::{
    env,
    ffi::{OsStr, OsString},
    io,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use thiserror::Error;

/// Failures before Cargo compilation, preserving the original process evidence.
#[derive(Debug, Error)]
pub enum CompilerCacheError {
    #[error("failed to launch Cargo: {0}")]
    CargoLaunch(#[source] io::Error),

    #[error("compiler startup probe failed for {compiler:?}: {source}")]
    Compiler {
        compiler: OsString,
        #[source]
        source: ProbeError,
    },

    #[error(
        "automatically selected compiler cache {wrapper:?} could not run the compiler startup probe: {source}\nSet RUSTC_WRAPPER= to disable automatic compiler caching, or set RUSTC_WRAPPER to your chosen wrapper. Cargo compilation was not started."
    )]
    ImplicitCache {
        wrapper: PathBuf,
        #[source]
        source: ProbeError,
    },
}

/// Original compiler/cache launch or exit failure; text is diagnostic only.
#[derive(Debug, Error)]
pub enum ProbeError {
    #[error("{status}; stdout: {stdout}; stderr: {stderr}")]
    Exit {
        status: std::process::ExitStatus,
        stdout: String,
        stderr: String,
    },

    #[error(transparent)]
    Launch(#[from] io::Error),
}

pub(super) fn check_implicit_cache(
    cargo: &Command,
    wrapper: &Path,
) -> Result<(), CompilerCacheError> {
    let compiler = command_env(cargo, "RUSTC")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "rustc".into());
    let workspace_wrapper =
        command_env(cargo, "RUSTC_WORKSPACE_WRAPPER").filter(|value| !value.is_empty());
    let mut args = Vec::new();
    if let Some(workspace_wrapper) = workspace_wrapper {
        args.push(workspace_wrapper);
    }
    args.push(compiler.clone());
    args.push("-vV".into());

    // Establish that the compiler chain itself works before attributing failure
    // to the automatically added outer wrapper. Neither probe builds a package.
    let mut direct = probe_command(cargo, &args[0]);
    direct.args(&args[1..]);
    run_probe(&mut direct).map_err(|source| CompilerCacheError::Compiler { compiler, source })?;
    let mut cached = probe_command(cargo, wrapper.as_os_str());
    cached.args(&args);
    run_probe(&mut cached).map_err(|source| CompilerCacheError::ImplicitCache {
        wrapper: wrapper.to_path_buf(),
        source,
    })
}

fn command_env(command: &Command, name: &str) -> Option<OsString> {
    command
        .get_envs()
        .find(|(key, _)| *key == name)
        .map_or_else(
            || env::var_os(name),
            |(_, value)| value.map(OsStr::to_os_string),
        )
}

fn probe_command(cargo: &Command, program: &OsStr) -> Command {
    let mut probe = Command::new(program);
    if let Some(directory) = cargo.get_current_dir() {
        probe.current_dir(directory);
    }
    for (name, value) in cargo.get_envs() {
        if let Some(value) = value {
            probe.env(name, value);
        } else {
            probe.env_remove(name);
        }
    }
    probe
}

fn run_probe(command: &mut Command) -> Result<(), ProbeError> {
    let Output {
        status,
        stdout,
        stderr,
    } = command.output()?;
    if status.success() {
        Ok(())
    } else {
        Err(ProbeError::Exit {
            status,
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        })
    }
}
