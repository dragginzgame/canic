//! Module: canic_host::canister_build::cache
//!
//! Responsibility: isolate reusable Cargo state created by canister artifact builds.
//! Does not own: canonical `.icp` artifacts, build profiles, or deployment orchestration.
//! Boundary: resolves one dedicated Wasm target directory while respecting explicit Cargo input.

#[cfg(test)]
mod tests;

use ic_host_fs::durable::lock_regular_file_with_parents;
use std::{
    env,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{Duration, Instant},
};

const DEFAULT_WASM_TARGET_RELATIVE: &str = "target/canic-wasm";
const CANISTER_BUILD_LOCK_RELATIVE: &str = ".canic/locks/canister-artifact-build.lock";

pub fn configure_canister_cargo_command(command: &mut Command, workspace_root: &Path) {
    command.env("CARGO_INCREMENTAL", "0").env(
        "CARGO_TARGET_DIR",
        canister_build_target_root(workspace_root),
    );
}

///
/// CargoBuildProgress
///
/// Explicit role and batch identity with a clock shared by the whole build phase.
///
pub struct CargoBuildProgress {
    roles: String,
    batch: usize,
    batches: usize,
    phase_started: Instant,
}

impl CargoBuildProgress {
    pub fn single(role: &str) -> Self {
        Self::batch([role], 1, 1, Instant::now())
    }

    pub fn batch<'a>(
        roles: impl IntoIterator<Item = &'a str>,
        batch: usize,
        batches: usize,
        phase_started: Instant,
    ) -> Self {
        let mut roles = roles.into_iter();
        let mut names = roles
            .by_ref()
            .take(8)
            .map(|role| {
                let name: String = role
                    .chars()
                    .take(80)
                    .map(|ch| {
                        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
                            ch
                        } else {
                            '?'
                        }
                    })
                    .collect();
                if role.chars().nth(80).is_some() {
                    format!("{name}...")
                } else {
                    name
                }
            })
            .collect::<Vec<_>>();
        if roles.next().is_some() {
            names.push("...".into());
        }
        Self {
            roles: names.join(","),
            batch,
            batches,
            phase_started,
        }
    }

    fn message(&self, phase: &str, child_elapsed: Duration, phase_elapsed: Duration) -> String {
        format!(
            "Build phase {phase} Cargo/link: batch {}/{} roles [{}]; child {:.0}s, phase {:.0}s (Cargo may be compiling, linking or waiting for its own lock)",
            self.batch,
            self.batches,
            self.roles,
            child_elapsed.as_secs_f64(),
            phase_elapsed.as_secs_f64()
        )
    }
}

/// Run Cargo once with its selected compiler configuration and retain its result.
pub fn output_canister_cargo_command(
    command: &mut Command,
    progress: CargoBuildProgress,
) -> io::Result<Output> {
    crate::build_environment::apply(command);
    let declaration = command.get_envs().any(|(key, value)| {
        key == canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV
            && value == Some(OsStr::new("1"))
    });
    let phase = if declaration {
        "declaration"
    } else {
        "runtime"
    };
    crate::canister_build::process::output_with_progress(
        command,
        std::time::Duration::from_secs(30),
        move |elapsed| {
            eprintln!(
                "{}",
                progress.message(phase, elapsed, progress.phase_started.elapsed())
            );
        },
    )
}

/// Declaration passes retain runtime cfg/profile semantics without paying for LTO.
pub fn configure_declaration_command(
    command: &mut Command,
    context: &crate::canister_build::WorkspaceBuildContext,
) {
    let profile = match context.profile {
        crate::canister_build::CanisterBuildProfile::Debug => "DEV",
        crate::canister_build::CanisterBuildProfile::Fast => "FAST",
        crate::canister_build::CanisterBuildProfile::Release => "RELEASE",
    };
    command
        .env(canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV, "1")
        .env_remove(canic_contracts::ids::RELEASE_BUILD_ID_ENV)
        .env_remove(canic_core::role_contract::PROTOCOL_PROFILE_DIGEST_ENV)
        .env_remove(canic_core::role_contract::build_context::PROTOCOL_BUILD_CONTEXT_ENV)
        .env(format!("CARGO_PROFILE_{profile}_LTO"), "off")
        .env(format!("CARGO_PROFILE_{profile}_OPT_LEVEL"), "0")
        .env(format!("CARGO_PROFILE_{profile}_CODEGEN_UNITS"), "16")
        .env(
            "CARGO_TARGET_DIR",
            declaration_target_root(&context.workspace_root),
        );
}

pub fn declaration_target_root(workspace_root: &Path) -> PathBuf {
    canister_build_target_root(workspace_root).join("declarations")
}

/// Resolve the Cargo artifact target used by host builds, including an explicit override.
#[must_use]
pub fn canister_build_target_root(workspace_root: &Path) -> PathBuf {
    resolve_canister_build_target_root(
        workspace_root,
        env::var_os("CARGO_TARGET_DIR").map(PathBuf::from),
    )
}

/// Lock the complete shared Cargo-target build and artifact-materialization boundary.
pub fn lock_canister_build_target(workspace_root: &Path) -> io::Result<fs::File> {
    let path = workspace_root.join(CANISTER_BUILD_LOCK_RELATIVE);
    lock_regular_file_with_parents(&path).map_err(io::Error::from)
}

fn resolve_canister_build_target_root(
    workspace_root: &Path,
    configured_target: Option<PathBuf>,
) -> PathBuf {
    configured_target.map_or_else(
        || workspace_root.join(DEFAULT_WASM_TARGET_RELATIVE),
        |path| {
            if path.is_absolute() {
                path
            } else {
                workspace_root.join(path)
            }
        },
    )
}
