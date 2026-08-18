use canic_core::ids::BuildNetwork;
use ic_testkit::artifacts::{
    ArtifactCacheMaintenance, ArtifactCachePrunePolicy, SharedIncrementalTargetMaintenanceConfig,
    SharedIncrementalTargetMaintenanceFailureMode, SharedIncrementalTargetPrunePolicy,
    WasmBuildBatchConfig, WasmBuildBatchProgressEvent, WasmBuildProgressConfig,
    WasmBuildProgressEvent, WasmBuildProgressPhase, WasmBuildSpec,
    build_wasm_canisters_cached_batch_with_config_and_progress,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::OnceLock,
    time::Duration,
};

const INTERNAL_TEST_WASM_CACHE_MAX_AGE: Duration = Duration::from_hours(168);
const INTERNAL_TEST_WASM_CACHE_MAX_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const INTERNAL_TEST_WASM_CACHE_MAINTENANCE_INTERVAL: Duration = Duration::from_hours(1);
const INTERNAL_TEST_WASM_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
const INTERNAL_TEST_SHARED_WASM_TARGET_MAX_AGE: Duration = Duration::from_hours(168);
const INTERNAL_TEST_SHARED_WASM_TARGET_MAX_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const INTERNAL_TEST_SHARED_WASM_TARGET_MAINTENANCE_INTERVAL: Duration = Duration::from_hours(1);

pub(super) const INTERNAL_TEST_RELEASE_BUILD_ID: (&str, &str) = (
    canic_core::ids::RELEASE_BUILD_ID_ENV,
    "1111111111111111111111111111111111111111111111111111111111111111",
);

pub(super) fn build_canonical_fleet_coordinator_wasm(workspace_root: &Path) -> Vec<u8> {
    static WASM: OnceLock<Vec<u8>> = OnceLock::new();
    WASM.get_or_init(|| {
        let config_path = workspace_root.join("apps/test/canic.toml");
        let target_dir = workspace_root.join("target/test-artifacts/fleet-coordinator-artifact");
        let artifact_path = workspace_root
            .join(".canic/release-builds")
            .join(INTERNAL_TEST_RELEASE_BUILD_ID.1)
            .join("artifacts/fleet_coordinator/fleet_coordinator.wasm");
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let output = Command::new(cargo)
            .current_dir(workspace_root)
            .env("CARGO_INCREMENTAL", "0")
            .env("CARGO_TARGET_DIR", target_dir)
            .env("ICP_ENVIRONMENT", "local")
            .env(
                INTERNAL_TEST_RELEASE_BUILD_ID.0,
                INTERNAL_TEST_RELEASE_BUILD_ID.1,
            )
            .args([
                "run",
                "-q",
                "--profile",
                "fast",
                "-p",
                "canic-host",
                "--example",
                "build_artifact",
                "--locked",
                "--",
                "fleet_coordinator",
                "fast",
                workspace_root.to_str().expect("workspace root UTF-8"),
                workspace_root.to_str().expect("ICP root UTF-8"),
                config_path.to_str().expect("config path UTF-8"),
                "--release-build-id",
                INTERNAL_TEST_RELEASE_BUILD_ID.1,
            ])
            .output()
            .expect("run canonical Fleet Coordinator artifact builder");
        assert!(
            output.status.success(),
            "canonical Fleet Coordinator artifact build failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        fs::read(&artifact_path).unwrap_or_else(|error| {
            panic!(
                "read canonical Fleet Coordinator artifact {}: {error}",
                artifact_path.display()
            )
        })
    })
    .clone()
}

///
/// CanicWasmBuildProfile
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanicWasmBuildProfile {
    Debug,
    Fast,
}

impl CanicWasmBuildProfile {
    #[must_use]
    pub(super) const fn cargo_profile_args(self) -> &'static [&'static str] {
        match self {
            Self::Debug => &[],
            Self::Fast => &["--profile", "fast"],
        }
    }

    #[must_use]
    pub const fn target_dir_name(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Fast => "fast",
        }
    }

    #[must_use]
    pub(super) const fn canic_wasm_profile_value(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Fast => "fast",
        }
    }
}

pub fn build_internal_test_wasm_canisters(
    workspace_root: &Path,
    target_dir: &Path,
    packages: &[&str],
    profile: CanicWasmBuildProfile,
) {
    build_internal_test_wasm_canisters_with_env(workspace_root, target_dir, packages, profile, &[]);
}

pub(super) fn build_internal_test_wasm_canisters_with_env(
    workspace_root: &Path,
    target_dir: &Path,
    packages: &[&str],
    profile: CanicWasmBuildProfile,
    extra_env: &[(&str, &str)],
) {
    assert!(
        !packages.is_empty(),
        "internal PocketIC Wasm build requires at least one package"
    );
    canic_host::role_contract::validate_internal_test_wasm_packages(workspace_root, packages)
        .unwrap_or_else(|finding| {
            panic!(
                "internal PocketIC wasm role validation failed ({}): {}",
                finding.code(),
                canic_host::role_contract::finding_detail(&finding)
            )
        });

    let mut cargo_args = profile.cargo_profile_args().to_vec();
    cargo_args.push("--locked");

    let mut build_env = vec![
        ("CARGO_INCREMENTAL", "0"),
        ("ICP_ENVIRONMENT", "local"),
        (
            canic_core::role_contract::CANONICAL_BUILD_MARKER_ENV,
            canic_core::role_contract::CANONICAL_BUILD_MARKER_VALUE,
        ),
        INTERNAL_TEST_RELEASE_BUILD_ID,
    ];
    build_env.extend_from_slice(extra_env);
    let additional_inputs = canonical_build_config_inputs(workspace_root, &build_env);
    let builds = packages
        .iter()
        .map(|package| {
            WasmBuildSpec::new(
                workspace_root,
                target_dir,
                &[*package],
                profile.target_dir_name(),
            )
            .with_cargo_profile_args(cargo_args.iter().copied())
            .with_extra_env(build_env.iter().copied())
            .with_additional_inputs(additional_inputs.iter().cloned())
            .with_shared_incremental_target(internal_test_shared_wasm_target(
                workspace_root,
                &build_env,
            ))
            .with_prune_policy_at_most_every(
                internal_test_artifact_prune_policy(),
                internal_test_artifact_maintenance_interval(),
            )
        })
        .collect::<Vec<_>>();
    let batch_config = internal_test_wasm_batch_config();
    let progress = WasmBuildProgressConfig::new()
        .with_heartbeat_interval(INTERNAL_TEST_WASM_HEARTBEAT_INTERVAL)
        .with_cargo_output(false);
    let batch = build_wasm_canisters_cached_batch_with_config_and_progress(
        &builds,
        batch_config,
        progress,
        |event| {
            report_wasm_build_progress(packages, event);
        },
    );
    for (index, outcome) in batch.outcomes() {
        let package = packages.get(index).copied().unwrap_or("unknown");
        eprintln!(
            "[canic-test-wasm:{package}] {outcome}; exact-cache={}",
            outcome.record().exact_cache_path().display()
        );
        report_artifact_cache_maintenance("canic-test-wasm", outcome.record().maintenance());
    }
    eprintln!("[canic-test-wasm] {batch}");
    let failures = batch
        .failures()
        .map(|(index, error)| {
            let package = packages.get(index).copied().unwrap_or("unknown");
            format!("`{package}`: {error}")
        })
        .collect::<Vec<_>>();
    assert!(
        failures.is_empty(),
        "internal test Wasm builds failed:\n{}",
        failures.join("\n")
    );
}

fn canonical_build_config_inputs(
    workspace_root: &Path,
    build_env: &[(&str, &str)],
) -> Vec<PathBuf> {
    build_env
        .iter()
        .filter(|(key, _)| *key == canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV)
        .map(|(_, value)| {
            let path = PathBuf::from(value);
            if path.is_absolute() {
                path
            } else {
                workspace_root.join(path)
            }
        })
        .collect()
}

fn internal_test_shared_wasm_target(workspace_root: &Path, build_env: &[(&str, &str)]) -> PathBuf {
    let build_network = build_env
        .iter()
        .rev()
        .find_map(|(key, value)| (*key == "ICP_ENVIRONMENT").then_some(*value))
        .expect("internal test Wasm build network");
    assert!(
        matches!(build_network, "ic" | "local"),
        "internal test Wasm build network must be `ic` or `local`"
    );
    workspace_root
        .join("target/test-artifacts/cargo-wasm-incremental")
        .join(build_network)
}

pub(super) const fn internal_test_artifact_prune_policy() -> ArtifactCachePrunePolicy {
    ArtifactCachePrunePolicy::new()
        .with_max_age(INTERNAL_TEST_WASM_CACHE_MAX_AGE)
        .with_max_size_bytes(INTERNAL_TEST_WASM_CACHE_MAX_BYTES)
}

pub(super) const fn internal_test_artifact_maintenance_interval() -> Duration {
    INTERNAL_TEST_WASM_CACHE_MAINTENANCE_INTERVAL
}

const fn internal_test_shared_wasm_target_prune_policy() -> SharedIncrementalTargetPrunePolicy {
    SharedIncrementalTargetPrunePolicy::new()
        .with_max_age(INTERNAL_TEST_SHARED_WASM_TARGET_MAX_AGE)
        .with_max_size_bytes(INTERNAL_TEST_SHARED_WASM_TARGET_MAX_BYTES)
}

const fn internal_test_wasm_batch_config() -> WasmBuildBatchConfig {
    let maintenance = SharedIncrementalTargetMaintenanceConfig::new(
        internal_test_shared_wasm_target_prune_policy(),
        INTERNAL_TEST_SHARED_WASM_TARGET_MAINTENANCE_INTERVAL,
    )
    .with_failure_mode(SharedIncrementalTargetMaintenanceFailureMode::BestEffort);
    WasmBuildBatchConfig::new().with_shared_incremental_target_maintenance(maintenance)
}

fn report_wasm_build_progress(packages: &[&str], event: WasmBuildBatchProgressEvent) {
    match event {
        WasmBuildBatchProgressEvent::BuildStarted { index, total } => {
            let package = packages.get(index).copied().unwrap_or("unknown");
            eprintln!(
                "[canic-test-wasm:{package}] acquiring independent build {}/{total}",
                index + 1
            );
        }
        WasmBuildBatchProgressEvent::BuildProgress {
            index,
            event:
                WasmBuildProgressEvent::Heartbeat {
                    phase: WasmBuildProgressPhase::CargoBuild,
                    elapsed,
                },
        } => {
            let package = packages.get(index).copied().unwrap_or("unknown");
            eprintln!("[canic-test-wasm:{package}] Cargo still running after {elapsed:?}");
        }
        WasmBuildBatchProgressEvent::BuildProgress {
            index,
            event: WasmBuildProgressEvent::SharedTargetMaintenanceFinished { outcome },
        } => {
            let package = packages.get(index).copied().unwrap_or("unknown");
            eprintln!("[canic-test-wasm:{package}] shared target {outcome}");
        }
        _ => {}
    }
}

pub(super) fn report_artifact_cache_maintenance(
    label: &str,
    maintenance: Option<&ArtifactCacheMaintenance>,
) {
    match maintenance {
        Some(maintenance)
            if maintenance
                .prune_report()
                .is_some_and(|report| report.entries_removed() > 0) =>
        {
            let report = maintenance.prune_report().expect("checked prune report");
            eprintln!(
                "[{label}] pruned {} cache entr{} ({} bytes); retained {} entr{} ({} bytes)",
                report.entries_removed(),
                if report.entries_removed() == 1 {
                    "y"
                } else {
                    "ies"
                },
                report.bytes_removed(),
                report.entries_retained(),
                if report.entries_retained() == 1 {
                    "y"
                } else {
                    "ies"
                },
                report.bytes_retained(),
            );
        }
        Some(maintenance) if maintenance.failure_message().is_some() => eprintln!(
            "[{label}] warning: cache maintenance failed: {}",
            maintenance
                .failure_message()
                .expect("checked failure message")
        ),
        Some(_) | None => {}
    }
}

pub(super) fn run_icp_all_with_env(
    workspace_root: &Path,
    build_network: BuildNetwork,
    profile: CanicWasmBuildProfile,
    config_path: &Path,
    extra_env: &[(&str, &str)],
) -> Output {
    let target_dir = icp_build_target_dir(workspace_root);

    let mut build = Command::new("bash");
    build
        .current_dir(workspace_root)
        .env("ICP_ENVIRONMENT", build_network.as_str())
        .env("CARGO_TARGET_DIR", &target_dir)
        .arg(build_ci_wasm_artifacts_script(workspace_root))
        .arg(profile.canic_wasm_profile_value())
        .arg(config_path);
    for (key, value) in extra_env {
        build.env(key, value);
    }

    build
        .output()
        .expect("failed to run local artifact build helper")
}

fn icp_build_target_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join("target").join("icp-build")
}

fn build_ci_wasm_artifacts_script(workspace_root: &Path) -> PathBuf {
    workspace_root
        .join("scripts")
        .join("ci")
        .join("build-ci-wasm-artifacts.sh")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_wasm_target_uses_effective_network() {
        let workspace_root = Path::new("/workspace");
        let build_env = [
            ("ICP_ENVIRONMENT", "local"),
            ("UNRELATED", "value"),
            ("ICP_ENVIRONMENT", "ic"),
        ];

        assert_eq!(
            internal_test_shared_wasm_target(workspace_root, &build_env),
            workspace_root.join("target/test-artifacts/cargo-wasm-incremental/ic")
        );
    }

    #[test]
    fn canonical_build_config_is_an_explicit_artifact_input() {
        let workspace_root = Path::new("/workspace");
        let build_env = [
            (
                canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV,
                "canisters/test/root/canic.toml",
            ),
            ("UNRELATED", "value"),
        ];

        assert_eq!(
            canonical_build_config_inputs(workspace_root, &build_env),
            vec![workspace_root.join("canisters/test/root/canic.toml")]
        );
    }

    #[test]
    fn shared_wasm_target_retention_is_bounded() {
        let maintenance = internal_test_wasm_batch_config()
            .shared_incremental_target_maintenance()
            .expect("shared target maintenance");
        let policy = maintenance.policy();

        assert_eq!(
            policy.max_age(),
            Some(INTERNAL_TEST_SHARED_WASM_TARGET_MAX_AGE)
        );
        assert_eq!(
            policy.max_size_bytes(),
            Some(INTERNAL_TEST_SHARED_WASM_TARGET_MAX_BYTES)
        );
        assert_eq!(maintenance.minimum_interval(), Duration::from_hours(1));
        assert_eq!(
            maintenance.failure_mode(),
            SharedIncrementalTargetMaintenanceFailureMode::BestEffort
        );
    }
}
