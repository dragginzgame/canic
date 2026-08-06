use canic_core::ids::BuildNetwork;
use ic_testkit::artifacts::{
    ArtifactCacheMaintenance, ArtifactCachePrunePolicy, WasmBuildSpec, build_wasm_canisters_cached,
};
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
    time::Duration,
};

const INTERNAL_TEST_WASM_CACHE_MAX_AGE: Duration = Duration::from_hours(168);
const INTERNAL_TEST_WASM_CACHE_MAX_BYTES: u64 = 2 * 1024 * 1024 * 1024;

pub(super) const INTERNAL_TEST_ENDPOINTS_ENV: (&str, &str) = ("CANIC_INTERNAL_TEST_ENDPOINTS", "1");
pub(super) const INTERNAL_TEST_RELEASE_BUILD_ID: (&str, &str) = (
    canic_core::ids::RELEASE_BUILD_ID_ENV,
    "1111111111111111111111111111111111111111111111111111111111111111",
);

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
        INTERNAL_TEST_ENDPOINTS_ENV,
        INTERNAL_TEST_RELEASE_BUILD_ID,
    ];
    build_env.extend_from_slice(extra_env);
    let build = WasmBuildSpec::new(
        workspace_root,
        target_dir,
        packages,
        profile.target_dir_name(),
    )
    .with_cargo_profile_args(&cargo_args)
    .with_extra_env(&build_env)
    .with_prune_policy(internal_test_artifact_prune_policy());
    let outcome = build_wasm_canisters_cached(&build)
        .unwrap_or_else(|err| panic!("internal test Wasm build failed: {err}"));
    let timings = outcome.record().timings();
    eprintln!(
        "[canic-test-wasm] {} {} package(s) in {:?} (lock {:?}, inputs {:?}, cargo {:?})",
        if outcome.is_reused() {
            "reused"
        } else {
            "built"
        },
        packages.len(),
        timings.total(),
        timings.lock_wait(),
        timings.input_resolution(),
        timings.cargo_build(),
    );
    report_artifact_cache_maintenance("canic-test-wasm", outcome.record().maintenance());
}

pub(super) const fn internal_test_artifact_prune_policy() -> ArtifactCachePrunePolicy {
    ArtifactCachePrunePolicy::new()
        .with_max_age(INTERNAL_TEST_WASM_CACHE_MAX_AGE)
        .with_max_size_bytes(INTERNAL_TEST_WASM_CACHE_MAX_BYTES)
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
