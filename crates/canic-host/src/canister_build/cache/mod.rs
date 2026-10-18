//! Module: canic_host::canister_build::cache
//!
//! Responsibility: isolate reusable Cargo state created by canister artifact builds.
//! Does not own: canonical `.icp` artifacts, build profiles, or deployment orchestration.
//! Boundary: resolves one dedicated Wasm target directory while respecting explicit Cargo input.

#[cfg(test)]
mod tests;

use ic_build::artifacts::CargoBuildFacts;
use ic_build::cargo::{execution::communicate_cargo, observation::schema::Message};
use ic_host_fs::durable::{lock_exclusive, open_regular_lock_file_with_parents};
use ic_host_process::{
    child::OwnedChild,
    tool::{OutputLimit, OutputLimits},
};
use std::{
    collections::BTreeMap,
    env,
    ffi::{OsStr, OsString},
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
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
) -> io::Result<(
    ic_host_process::tool::CompletedExecution,
    Vec<ic_build::cargo::observation::schema::Message>,
)> {
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
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = OwnedChild::spawn_direct(command)?;
    let started = Instant::now();
    let mut next_heartbeat = Duration::from_secs(30);
    let mut messages = Vec::new();
    let observed = communicate_cargo(
        &mut child,
        OutputLimits {
            // Canonical typed records replace the former raw stdout buffer.
            stdout: OutputLimit::Truncate(0),
            stderr: OutputLimit::Truncate(usize::MAX),
            timeout: None,
        },
        usize::MAX,
        || {
            let elapsed = started.elapsed();
            if elapsed >= next_heartbeat {
                eprintln!(
                    "{}",
                    progress.message(phase, elapsed, progress.phase_started.elapsed())
                );
                next_heartbeat = elapsed.saturating_add(Duration::from_secs(30));
            }
            false
        },
        |_, _| Ok(()),
        |message| {
            if matches!(
                message,
                Message::CompilerArtifact(_) | Message::BuildScriptExecuted(_)
            ) {
                messages.push(message);
            }
            Ok(())
        },
    );
    let execution = observed.execution.map_err(io::Error::other)?;
    if execution.status.success() {
        observed.messages?;
    }
    Ok((execution, messages))
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
    let lock = open_regular_lock_file_with_parents(&path).map_err(io::Error::from)?;
    lock_exclusive(&lock)?;
    Ok(lock)
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

/// Capture the actual custom Cargo command's recipe and tool identities.
/// Canic owns these generated/runtime recipes; Build owns the fact representation.
pub fn output_runtime_cargo_command(
    command: &mut Command,
    progress: CargoBuildProgress,
    selection: ic_build::cargo::selection::CargoSelection,
) -> io::Result<(
    ic_host_process::tool::CompletedExecution,
    Vec<Message>,
    CargoBuildFacts,
)> {
    crate::build_environment::apply(command);
    let mut environment = crate::build_environment::inputs()
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    for (key, value) in command.get_envs() {
        if let Some(value) = value {
            environment.insert(key.to_owned(), value.to_owned());
        } else {
            environment.remove(key);
        }
    }
    command.env_clear().envs(&environment);
    let facts = runtime_cargo_facts(command, selection, environment)?;
    let (execution, messages) = output_canister_cargo_command(command, progress)?;
    if execution.status.success()
        && runtime_cargo_facts(command, facts.selection.clone(), facts.environment.clone())?
            != facts
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Cargo/compiler identity changed during compilation",
        ));
    }
    Ok((execution, messages, facts))
}

fn runtime_cargo_facts(
    command: &Command,
    selection: ic_build::cargo::selection::CargoSelection,
    environment: BTreeMap<OsString, OsString>,
) -> io::Result<CargoBuildFacts> {
    let current_dir = command
        .get_current_dir()
        .map(Path::to_path_buf)
        .map_or_else(std::env::current_dir, Ok)?;
    let probe = |program: &OsStr, arguments: &[&str]| -> io::Result<Vec<u8>> {
        let mut probe = Command::new(program);
        probe
            .current_dir(&current_dir)
            .env_clear()
            .envs(&environment)
            .args(arguments);
        let output = ic_host_process::tool::capture_command(
            &mut probe,
            ic_host_process::tool::OutputLimits {
                stdout: OutputLimit::Terminate(64 * 1024),
                stderr: OutputLimit::Terminate(64 * 1024),
                timeout: None,
            },
        )
        .and_then(ic_host_process::tool::CompletedExecution::require_success)
        .map_err(io::Error::other)?;
        let mut identity = output.stdout;
        identity.extend_from_slice(&output.stderr);
        Ok(identity)
    };
    let cargo_identity = probe(command.get_program(), &["--version", "--verbose"])?;
    let rustc = environment
        .get(OsStr::new("RUSTC"))
        .map_or_else(|| OsStr::new("rustc"), OsString::as_os_str);
    let rustc_identity = probe(rustc, &["-vV"])?;
    Ok(CargoBuildFacts {
        selection,
        environment,
        target: "wasm32-unknown-unknown".into(),
        cargo_identity,
        rustc_identity,
    })
}

pub fn runtime_cargo_selection(
    profile: crate::canister_build::CanisterBuildProfile,
    options: &crate::canister_build::CanisterArtifactBuildOptions,
) -> ic_build::cargo::selection::CargoSelection {
    use ic_build::cargo::selection::{CargoFeatureSelection, CargoSelection};
    CargoSelection {
        profile: match profile {
            crate::canister_build::CanisterBuildProfile::Debug => None,
            crate::canister_build::CanisterBuildProfile::Fast => Some("fast".into()),
            crate::canister_build::CanisterBuildProfile::Release => Some("release".into()),
        },
        features: CargoFeatureSelection {
            features: options.cargo_features.clone(),
            default_features: options.default_features,
            all_features: false,
        },
        locked: true,
        offline: false,
    }
}
