use canic_core::ids::BuildNetwork;
use canic_host::role_contract::{
    CargoFeatureSelection, PackageValidationMode, RolePackageValidation,
};
use ic_testkit::artifacts::{
    ArtifactCacheMaintenance, ArtifactCachePrunePolicy, ArtifactCacheSpec, LabeledWasmBuildSpec,
    SharedIncrementalTargetMaintenanceConfig, SharedIncrementalTargetMaintenanceFailureMode,
    SharedIncrementalTargetPrunePolicy, WasmBuildBatchConfig, WasmBuildBatchProgressEvent,
    WasmBuildBatchReport, WasmBuildProgressConfig, WasmBuildProgressEvent, WasmBuildProgressPhase,
    WasmBuildRecord, WasmBuildSpec, build_wasm_canisters_cached_batch_with_config_and_progress,
    resolve_cargo_build_inputs,
};
#[cfg(all(
    feature = "pocketic-fixtures",
    any(not(test), feature = "governed-pocketic-tests")
))]
use ic_testkit::artifacts::{
    ArtifactCacheOutcome, ArtifactCachePreparation, prepare_artifact_cache,
};
use std::fs;
#[cfg(all(
    feature = "pocketic-fixtures",
    any(not(test), feature = "governed-pocketic-tests")
))]
use std::sync::OnceLock;
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
    time::Duration,
};

use super::progress::{self, ProgressStatus};

const INTERNAL_TEST_WASM_CACHE_MAX_AGE: Duration = Duration::from_hours(168);
const INTERNAL_TEST_WASM_CACHE_MAX_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const INTERNAL_TEST_WASM_CACHE_MAINTENANCE_INTERVAL: Duration = Duration::from_hours(1);
const INTERNAL_TEST_WASM_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
const INTERNAL_TEST_SHARED_WASM_TARGET_MAX_AGE: Duration = Duration::from_hours(168);
const INTERNAL_TEST_SHARED_WASM_TARGET_MAX_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const INTERNAL_TEST_SHARED_WASM_TARGET_MAINTENANCE_INTERVAL: Duration = Duration::from_hours(1);

/// Bind external artifact reuse to the actual configured canonical Root Cargo graph.
pub(super) fn with_canonical_root_cargo_inputs(
    cache: ArtifactCacheSpec,
    config_path: &Path,
    target_dir: &Path,
    profile: CanicWasmBuildProfile,
    environment: &[(&str, &str)],
) -> ArtifactCacheSpec {
    let config = canic_host::release_set::AppConfigSnapshot::load(config_path)
        .expect("canonical Root cache configuration");
    let validation = canic_host::role_contract::validate_declared_role_package(
        config_path,
        config.model(),
        &canic_core::ids::CanisterRole::ROOT,
        PackageValidationMode::Build,
        &CargoFeatureSelection::default(),
    );
    let RolePackageValidation::Supported(evidence) = validation else {
        panic!("canonical Root cache package must resolve: {validation:?}");
    };
    let build = WasmBuildSpec::new(
        &evidence.cargo_workspace_root,
        target_dir,
        &[evidence.role_package_name.as_str()],
        profile.target_dir_name(),
    )
    .with_cargo_profile_args(
        profile
            .cargo_profile_args()
            .iter()
            .copied()
            .chain(["--locked"]),
    )
    .with_extra_env(environment.iter().copied());
    let inputs = resolve_cargo_build_inputs(&build).expect("canonical Root cache Cargo inputs");
    cache.with_cargo_build_inputs("canonical-root-cargo", &build, &inputs)
}

pub(super) const INTERNAL_TEST_RELEASE_BUILD_ID: (&str, &str) = (
    canic_core::ids::RELEASE_BUILD_ID_ENV,
    "a4c128728412f11837b79ce8562e3115451db387e17361b79b4f15d02cbb36ae",
);
#[cfg(all(test, feature = "governed-pocketic-tests"))]
pub(super) const INTERNAL_TEST_RELEASE_BUILD_NONCE: [u8; 32] = [0x11; 32];
pub(super) const INTERNAL_TEST_PROTOCOL_PROFILE_DIGEST: (&str, &str) = (
    canic_core::role_contract::PROTOCOL_PROFILE_DIGEST_ENV,
    "0404040404040404040404040404040404040404040404040404040404040404",
);

#[cfg(all(
    feature = "pocketic-fixtures",
    any(not(test), feature = "governed-pocketic-tests")
))]
pub(super) fn build_canonical_fleet_coordinator_wasm(workspace_root: &Path) -> Vec<u8> {
    static WASM: OnceLock<Vec<u8>> = OnceLock::new();
    WASM.get_or_init(|| {
        let config_path = workspace_root.join("apps/test/canic.toml");
        let target_dir = canic_host::canister_build::canister_build_target_root(workspace_root);
        let artifact_path = workspace_root
            .join(".canic/release-builds")
            .join(INTERNAL_TEST_RELEASE_BUILD_ID.1)
            .join("artifacts/fleet_coordinator/fleet_coordinator.wasm");
        let cache = canonical_fleet_coordinator_cache_spec(
            workspace_root,
            &target_dir,
            &config_path,
            &artifact_path,
        );
        let outcome = match prepare_artifact_cache(&cache)
            .expect("prepare canonical Fleet Coordinator artifact cache")
        {
            ArtifactCachePreparation::Reused(record) => ArtifactCacheOutcome::Reused(record),
            ArtifactCachePreparation::Build(transaction) => {
                build_generated_fleet_wasm(
                    workspace_root,
                    &config_path,
                    "fleet_coordinator",
                    CanicWasmBuildProfile::Fast,
                );
                transaction
                    .import_output("fleet_coordinator", &artifact_path)
                    .expect("import canonical Fleet Coordinator artifact");
                transaction
                    .commit()
                    .expect("commit canonical Fleet Coordinator artifact cache")
            }
        };
        progress::timed(
            "WASM",
            if outcome.is_reused() {
                ProgressStatus::Cache
            } else {
                ProgressStatus::Done
            },
            "Fleet Coordinator artifact",
            outcome.record().timings().total(),
        );
        progress::detail("WASM", &format!("Fleet Coordinator cache: {outcome}"));
        report_artifact_cache_maintenance(
            "canonical-fleet-coordinator",
            outcome.record().maintenance(),
        );
        let artifact_path = retained_artifact_path(outcome.record(), "fleet_coordinator");
        fs::read(artifact_path).unwrap_or_else(|error| {
            panic!(
                "read canonical Fleet Coordinator artifact {}: {error}",
                artifact_path.display()
            )
        })
    })
    .clone()
}

/// Borrow one named immutable cache artifact while its record owns retention.
pub(super) fn retained_artifact_path<'a>(
    record: &'a ic_testkit::artifacts::ArtifactCacheRecord,
    name: &str,
) -> &'a Path {
    record
        .artifacts()
        .iter()
        .find(|artifact| artifact.name() == name)
        .expect("required retained artifact")
        .path()
}

/// Build a generated Fleet artifact through the already-linked production host owner.
/// Fixture acquisition must not compile a second native host executable.
pub(super) fn build_generated_fleet_wasm(
    workspace: &Path,
    config: &Path,
    role: &str,
    profile: CanicWasmBuildProfile,
) -> Vec<u8> {
    let workspace = workspace
        .canonicalize()
        .expect("canonical fixture workspace");
    let config = config.canonicalize().expect("canonical fixture config");
    let context = canic_host::canister_build::WorkspaceBuildContext {
        role: role.to_string(),
        profile: profile.target_dir_name().parse().expect("build profile"),
        environment: "local".to_string(),
        build_network: BuildNetwork::Local,
        workspace_root: workspace.clone(),
        icp_root: workspace,
        config_path: config,
        local_replica: None,
        refresh_canonical_infrastructure_did: false,
        release_build_id: Some(
            INTERNAL_TEST_RELEASE_BUILD_ID
                .1
                .parse()
                .expect("fixture release ID"),
        ),
    };
    let output = canic_host::canister_build::build_workspace_canister_artifact(&context)
        .expect("build generated Fleet artifact");
    fs::read(output.wasm_path).expect("read generated Fleet Wasm")
}

/// Output directory for the internal cached test-Wasm fixtures.
#[must_use]
#[cfg(any(test, feature = "pocketic-fixtures"))]
pub(super) fn internal_test_artifact_build_target(workspace_root: &Path) -> PathBuf {
    workspace_root.join("target/pic-wasm")
}

#[cfg(all(test, feature = "governed-pocketic-tests"))]
pub(super) fn preflight_governed_shared_artifacts() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let _ = build_canonical_fleet_coordinator_wasm(&workspace_root);
}

#[cfg(all(
    feature = "pocketic-fixtures",
    any(not(test), feature = "governed-pocketic-tests")
))]
fn canonical_fleet_coordinator_cache_spec(
    workspace_root: &Path,
    target_dir: &Path,
    config_path: &Path,
    artifact_path: &Path,
) -> ArtifactCacheSpec {
    let environment = [
        ("CARGO_INCREMENTAL", "0"),
        ("ICP_ENVIRONMENT", "local"),
        INTERNAL_TEST_RELEASE_BUILD_ID,
    ];
    let cargo_build = WasmBuildSpec::new(
        workspace_root,
        target_dir,
        &["canic", "canic-host"],
        CanicWasmBuildProfile::Fast.target_dir_name(),
    )
    .with_cargo_profile_args(["--profile", "fast", "--locked"])
    .with_extra_env(environment);
    let cargo_inputs = resolve_cargo_build_inputs(&cargo_build)
        .expect("resolve canonical Fleet Coordinator Cargo inputs");
    let config_relative = config_path
        .strip_prefix(workspace_root)
        .expect("Coordinator config must be workspace-confined")
        .to_str()
        .expect("Coordinator config path UTF-8");

    ArtifactCacheSpec::new(
        &workspace_root.join("target/test-artifacts/external-artifact-cache"),
        "canonical-fleet-coordinator",
        "canic/canonical-fleet-coordinator/v1",
    )
    .with_coordination_scope("canic-external-artifact-builds")
    .with_arguments([
        "canic-host::build_workspace_canister_artifact",
        "fleet_coordinator",
        "fast",
        config_relative,
    ])
    .with_environment(environment)
    .with_input("build-config", config_path)
    .with_input(
        "fixture-builder",
        &workspace_root.join("crates/canic-testing-internal/src/pic/artifacts.rs"),
    )
    .with_input("icp-config", &workspace_root.join("icp.yaml"))
    .with_input(
        "canonical-candid",
        &workspace_root.join("crates/canic/candid/fleet_coordinator.did"),
    )
    .with_cargo_build_inputs("coordinator-cargo", &cargo_build, &cargo_inputs)
    .with_output("fleet_coordinator", artifact_path)
    .with_prune_policy_at_most_every(
        internal_test_artifact_prune_policy(),
        internal_test_artifact_maintenance_interval(),
    )
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

/// Exact cached test Wasms and the leases retaining them until consumption finishes.
#[derive(Debug)]
#[must_use = "retain the build records while consuming their exact artifacts"]
pub struct InternalTestWasms {
    records: std::collections::BTreeMap<String, WasmBuildRecord>,
}

impl InternalTestWasms {
    /// Read a package's immutable artifact while its build record remains retained.
    ///
    /// # Panics
    /// Panics if the package was not built, its output is ambiguous, or reading fails.
    #[must_use]
    pub fn wasm(&self, package: &str) -> Vec<u8> {
        fs::read(self.path(package)).expect("read retained test Wasm")
    }

    /// Borrow the exact cache path for read-only extraction or post-link input.
    ///
    /// # Panics
    /// Panics if the package was not built or it did not produce exactly one Wasm.
    #[must_use]
    pub fn path(&self, package: &str) -> &Path {
        let record = self.records.get(package).expect("built test package");
        let [path] = record.artifacts() else {
            panic!("one-package build must yield exactly one Wasm");
        };
        path
    }

    pub(super) fn extend(&mut self, other: Self) {
        for (package, record) in other.records {
            assert!(
                self.records.insert(package, record).is_none(),
                "duplicate test package"
            );
        }
    }
}

/// Build and retain the exact requested internal test artifacts.
pub fn build_internal_test_wasm_canisters(
    workspace_root: &Path,
    target_dir: &Path,
    packages: &[&str],
    profile: CanicWasmBuildProfile,
) -> InternalTestWasms {
    build_internal_test_wasm_canisters_with_env(workspace_root, target_dir, packages, profile, &[])
}

pub(super) fn build_internal_test_wasm_canisters_with_env(
    workspace_root: &Path,
    target_dir: &Path,
    packages: &[&str],
    profile: CanicWasmBuildProfile,
    extra_env: &[(&str, &str)],
) -> InternalTestWasms {
    build_internal_test_wasm_canisters_with_features(
        workspace_root,
        target_dir,
        packages,
        profile,
        extra_env,
        &[],
    )
}

/// Build an audit participant with its explicitly derived role features.
pub(super) fn build_internal_test_wasm_canisters_with_features(
    workspace_root: &Path,
    target_dir: &Path,
    packages: &[&str],
    profile: CanicWasmBuildProfile,
    extra_env: &[(&str, &str)],
    features: &[String],
) -> InternalTestWasms {
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
    let features = features.join(",");
    if !features.is_empty() {
        cargo_args.extend(["--features", features.as_str()]);
    }

    let mut build_env = vec![
        ("CARGO_INCREMENTAL", "0"),
        ("ICP_ENVIRONMENT", "local"),
        (
            canic_core::role_contract::CANONICAL_BUILD_MARKER_ENV,
            canic_core::role_contract::CANONICAL_BUILD_MARKER_VALUE,
        ),
        INTERNAL_TEST_RELEASE_BUILD_ID,
        INTERNAL_TEST_PROTOCOL_PROFILE_DIGEST,
    ];
    build_env.extend_from_slice(extra_env);
    let additional_inputs = canonical_build_config_inputs(workspace_root, &build_env);
    let builds = packages
        .iter()
        .map(|package| {
            LabeledWasmBuildSpec::new(
                *package,
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
                ),
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
        report_wasm_build_progress,
    )
    .unwrap_or_else(|error| panic!("internal test Wasm batch contract failed: {error}"));
    report_wasm_batch(&batch);
    let failures = batch
        .failures()
        .map(|failure| {
            format!(
                "`{}` failed in {:?} after {:?}: {}; partial timings: {:?}",
                failure.label(),
                failure.phase(),
                failure.entry_elapsed(),
                failure.error(),
                failure.timings(),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        failures.is_empty(),
        "internal test Wasm builds failed:\n{}",
        failures.join("\n")
    );
    InternalTestWasms {
        records: batch
            .outcomes()
            .map(|entry| (entry.label().to_owned(), entry.outcome().record().clone()))
            .collect(),
    }
}

fn report_wasm_batch(batch: &WasmBuildBatchReport) {
    for entry in batch.outcomes() {
        let package = entry.label();
        let outcome = entry.outcome();
        progress::timed(
            "WASM",
            if outcome.is_reused() {
                ProgressStatus::Cache
            } else {
                ProgressStatus::Done
            },
            package,
            entry.entry_elapsed(),
        );
        progress::detail(
            "WASM",
            &format!(
                "{package}: {outcome}; exact-cache={}",
                outcome.record().exact_cache_path().display()
            ),
        );
        report_artifact_cache_maintenance("canic-test-wasm", outcome.record().maintenance());
    }
    let metrics = batch.metrics();
    if metrics.specifications() > 1 {
        progress::timed(
            "WASM",
            ProgressStatus::Done,
            &format!(
                "batch: {} built, {} cached, {} failed",
                metrics.built(),
                metrics.reused(),
                metrics.failed()
            ),
            batch.total(),
        );
    }
    progress::detail("WASM", &format!("batch detail: {batch}"));
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

fn report_wasm_build_progress(event: WasmBuildBatchProgressEvent) {
    match event {
        WasmBuildBatchProgressEvent::BuildStarted {
            index,
            label,
            total,
        } => {
            progress::event(
                "WASM",
                ProgressStatus::Run,
                &format!("{label} ({}/{total})", index + 1),
            );
        }
        WasmBuildBatchProgressEvent::BuildProgress {
            label,
            event:
                WasmBuildProgressEvent::Heartbeat {
                    phase: WasmBuildProgressPhase::CargoBuild,
                    elapsed,
                },
            ..
        } => {
            if progress::verbose() || elapsed.as_secs().is_multiple_of(60) {
                progress::timed(
                    "WASM",
                    ProgressStatus::Wait,
                    &format!("{label}: Cargo build"),
                    elapsed,
                );
            }
        }
        WasmBuildBatchProgressEvent::BuildProgress {
            label,
            event: WasmBuildProgressEvent::SharedTargetMaintenanceFinished { outcome },
            ..
        } => {
            progress::detail("WASM", &format!("{label}: shared target {outcome}"));
        }
        WasmBuildBatchProgressEvent::BuildFailed { label, .. } => {
            progress::event("WASM", ProgressStatus::Fail, &label);
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
            progress::event(
                "CACHE",
                ProgressStatus::Info,
                &format!(
                    "{label}: pruned {} entr{} ({} bytes); retained {} entr{} ({} bytes)",
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
                ),
            );
        }
        Some(maintenance) if maintenance.failure_message().is_some() => progress::event(
            "CACHE",
            ProgressStatus::Warn,
            &format!(
                "{label}: maintenance failed: {}",
                maintenance
                    .failure_message()
                    .expect("checked failure message")
            ),
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
    fn retained_artifact_handoff_survives_replacement_and_pruning() {
        use ic_testkit::artifacts::{
            ArtifactCacheOutcome, ArtifactCachePreparation, prepare_artifact_cache,
            prune_artifact_cache,
        };
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "canic-retained-artifacts-{}-{nonce}",
            std::process::id()
        ));
        let cache_root = root.join("cache");
        let output = root.join("output.wasm");
        let acquire = |identity: &str| {
            let spec = ArtifactCacheSpec::new(&cache_root, "handoff", "canic/test-handoff/v1")
                .with_arguments([identity])
                .with_output("probe", &output);
            match prepare_artifact_cache(&spec).unwrap() {
                ArtifactCachePreparation::Reused(record) => ArtifactCacheOutcome::Reused(record),
                ArtifactCachePreparation::Build(transaction) => {
                    fs::write(transaction.output_path("probe").unwrap(), identity).unwrap();
                    transaction.commit().unwrap()
                }
            }
        };
        let original = acquire("first");
        assert!(!original.is_reused());
        let retained = original.record().clone();
        drop(original);
        let replacement = acquire("second");
        assert!(!replacement.is_reused());
        fs::remove_file(&output).unwrap();
        let policy = ArtifactCachePrunePolicy::new().with_max_size_bytes(0);
        prune_artifact_cache(&cache_root, "handoff", policy).unwrap();
        let original_path = retained_artifact_path(&retained, "probe").to_path_buf();
        assert_ne!(original_path, output);
        assert_eq!(fs::read(&original_path).unwrap(), b"first");
        assert_eq!(
            fs::read(retained_artifact_path(replacement.record(), "probe")).unwrap(),
            b"second"
        );
        let replay = acquire("first");
        assert!(replay.is_reused());
        assert_eq!(
            retained_artifact_path(replay.record(), "probe"),
            original_path
        );
        drop(replay);
        drop(retained);
        prune_artifact_cache(&cache_root, "handoff", policy).unwrap();
        assert!(!original_path.exists());
        assert_eq!(
            fs::read(retained_artifact_path(replacement.record(), "probe")).unwrap(),
            b"second"
        );
        drop(replacement);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(feature = "governed-pocketic-tests")]
    #[test]
    #[ignore = "focused build qualification requires installed Wasm and artifact tools"]
    fn infrastructure_direct_builds_match_examples_and_cached_coordinator() {
        let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        for (role, config) in [
            ("fleet_coordinator", "apps/test/canic.toml"),
            (
                "wasm_store",
                "canisters/test/delegation_root_stub/canic.toml",
            ),
        ] {
            let config = workspace.join(config);
            let expected = compare_generated_artifact_with_example(&workspace, &config, role);
            if role == "fleet_coordinator" {
                assert!(
                    build_canonical_fleet_coordinator_wasm(&workspace) == expected,
                    "cached Coordinator Wasm bytes differ"
                );
                let artifact = workspace
                    .join(".canic/release-builds")
                    .join(INTERNAL_TEST_RELEASE_BUILD_ID.1)
                    .join("artifacts/fleet_coordinator/fleet_coordinator.wasm");
                let cache = canonical_fleet_coordinator_cache_spec(
                    &workspace,
                    &canic_host::canister_build::canister_build_target_root(&workspace),
                    &config,
                    &artifact,
                );
                let ArtifactCachePreparation::Reused(record) =
                    prepare_artifact_cache(&cache).unwrap()
                else {
                    panic!("expected cached Coordinator");
                };
                assert!(
                    fs::read(retained_artifact_path(&record, "fleet_coordinator")).unwrap()
                        == expected,
                    "restored Coordinator Wasm bytes differ"
                );
            }
        }
    }

    #[cfg(feature = "governed-pocketic-tests")]
    fn compare_generated_artifact_with_example(
        workspace: &Path,
        config: &Path,
        role: &str,
    ) -> Vec<u8> {
        let target = canic_host::canister_build::canister_build_target_root(workspace);
        let artifacts = workspace
            .join(".canic/release-builds")
            .join(INTERNAL_TEST_RELEASE_BUILD_ID.1)
            .join("artifacts")
            .join(role);
        let read_outputs = || {
            ["wasm", "wasm.gz", "did"]
                .map(|extension| fs::read(artifacts.join(format!("{role}.{extension}"))).unwrap())
        };
        let started = std::time::Instant::now();
        let example = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
            .current_dir(workspace)
            .env("CARGO_INCREMENTAL", "0")
            .env("CARGO_TARGET_DIR", &target)
            .env("ICP_ENVIRONMENT", "local")
            .env(
                INTERNAL_TEST_RELEASE_BUILD_ID.0,
                INTERNAL_TEST_RELEASE_BUILD_ID.1,
            )
            .args([
                "run",
                "--locked",
                "--profile",
                "fast",
                "-p",
                "canic-host",
                "--example",
                "build_artifact",
                "--message-format=json-render-diagnostics",
                "--",
                role,
                "fast",
            ])
            .arg(workspace)
            .arg(workspace)
            .arg(config)
            .args(["--release-build-id", INTERNAL_TEST_RELEASE_BUILD_ID.1])
            .output()
            .unwrap();
        assert!(
            example.status.success(),
            "{}",
            String::from_utf8_lossy(&example.stderr)
        );
        let example_elapsed = started.elapsed();
        eprintln!("{}", String::from_utf8_lossy(&example.stderr));
        let native_fresh = String::from_utf8_lossy(&example.stdout)
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .find(|message| message["target"]["name"] == "build_artifact")
            .and_then(|message| message["fresh"].as_bool())
            .expect("Cargo reports whether the native example was rebuilt");
        let expected = read_outputs();
        let started = std::time::Instant::now();
        let direct =
            build_generated_fleet_wasm(workspace, config, role, CanicWasmBuildProfile::Fast);
        let direct_elapsed = started.elapsed();
        assert!(
            read_outputs() == expected,
            "direct artifact bytes differ for {role}"
        );
        assert!(
            direct == expected[0],
            "returned Wasm bytes differ for {role}"
        );
        eprintln!(
            "Infrastructure qualification: role={role} example={example_elapsed:?} native_fresh={native_fresh} \
             direct={direct_elapsed:?} wasm_bytes={} wasm_sha256={}",
            expected[0].len(),
            canic_core::cdk::utils::hash::sha256_hex(&expected[0]),
        );
        direct
    }

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
    fn internal_test_wasms_share_the_pocketic_output_directory() {
        let workspace_root = Path::new("/workspace");

        assert_eq!(
            internal_test_artifact_build_target(workspace_root),
            workspace_root.join("target/pic-wasm")
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
