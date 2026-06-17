use canic_testing_internal::pic::{CanicWasmBuildProfile, RootBaselineMetadata, RootBaselineSpec};
use ic_testkit::{artifacts::workspace_root_for, pic::CachedPicBaseline};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

const ROOT_TOPOLOGY_RELEASE_ROLES: &[&str] = &[
    "app",
    "scale_hub",
    "scale_replica",
    "user_hub",
    "user_shard",
];
const ROOT_CAPABILITY_RELEASE_ROLES: &[&str] = &["app", "scale_hub", "test"];
const ROOT_SCALING_RELEASE_ROLES: &[&str] = &["scale_hub", "scale_replica"];
const ROOT_SHARDING_RELEASE_ROLES: &[&str] = &["test", "user_hub", "user_shard"];
const ROOT_RECONCILE_RELEASE_ROLES: &[&str] = &[
    "app",
    "scale_hub",
    "scale_replica",
    "user_hub",
    "user_shard",
];
const TEST_SMALL_STORE_RUSTFLAGS: &str = "--cfg canic_test_small_wasm_store";
const BOOTSTRAP_TICK_LIMIT: usize = 120;
const ROOT_SETUP_MAX_ATTEMPTS: usize = 2;
const ROOT_WASM_WATCH_PATHS: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "canisters",
    "fleets/test",
    "icp.yaml",
    "crates",
];

static ROOT_TOPOLOGY_BASELINE: Mutex<Option<CachedPicBaseline<RootBaselineMetadata>>> =
    Mutex::new(None);
static ROOT_CAPABILITY_BASELINE: Mutex<Option<CachedPicBaseline<RootBaselineMetadata>>> =
    Mutex::new(None);
static ROOT_SCALING_BASELINE: Mutex<Option<CachedPicBaseline<RootBaselineMetadata>>> =
    Mutex::new(None);
static ROOT_SHARDING_BASELINE: Mutex<Option<CachedPicBaseline<RootBaselineMetadata>>> =
    Mutex::new(None);
static ROOT_RECONCILE_SMALL_STORE_BASELINE: Mutex<Option<CachedPicBaseline<RootBaselineMetadata>>> =
    Mutex::new(None);

#[derive(Clone, Copy)]
pub enum RootSetupProfile {
    Topology,
    Capability,
    Scaling,
    Sharding,
    ReconcileSmallStore,
}

impl RootSetupProfile {
    pub(crate) const fn cache_label(self) -> &'static str {
        match self {
            Self::Topology => "cached root topology baseline",
            Self::Capability => "cached root capability baseline",
            Self::Scaling => "cached root scaling baseline",
            Self::Sharding => "cached root sharding baseline",
            Self::ReconcileSmallStore => "cached root reconcile small-store baseline",
        }
    }

    const fn release_roles(self) -> &'static [&'static str] {
        match self {
            Self::Topology => ROOT_TOPOLOGY_RELEASE_ROLES,
            Self::Capability => ROOT_CAPABILITY_RELEASE_ROLES,
            Self::Scaling => ROOT_SCALING_RELEASE_ROLES,
            Self::Sharding => ROOT_SHARDING_RELEASE_ROLES,
            Self::ReconcileSmallStore => ROOT_RECONCILE_RELEASE_ROLES,
        }
    }

    const fn build_profile(self) -> CanicWasmBuildProfile {
        match self {
            Self::ReconcileSmallStore => CanicWasmBuildProfile::Debug,
            Self::Topology | Self::Capability | Self::Scaling | Self::Sharding => {
                CanicWasmBuildProfile::Fast
            }
        }
    }

    pub(crate) fn cache_slot(
        self,
    ) -> &'static Mutex<Option<CachedPicBaseline<RootBaselineMetadata>>> {
        match self {
            Self::Topology => &ROOT_TOPOLOGY_BASELINE,
            Self::Capability => &ROOT_CAPABILITY_BASELINE,
            Self::Scaling => &ROOT_SCALING_BASELINE,
            Self::Sharding => &ROOT_SHARDING_BASELINE,
            Self::ReconcileSmallStore => &ROOT_RECONCILE_SMALL_STORE_BASELINE,
        }
    }

    pub(crate) fn baseline_spec(self) -> RootBaselineSpec<'static> {
        baseline_spec_for_profile(self)
    }
}

// Return the shared repo root for root-harness artifact and config discovery.
fn workspace_root() -> PathBuf {
    workspace_root_for(env!("CARGO_MANIFEST_DIR"))
}

#[must_use]
pub fn built_root_wasm_path() -> PathBuf {
    root_wasm_path(&workspace_root())
}

// Map one test profile to its embedded config override without leaking relative
// crate-local paths into the shared build environment.
fn profile_build_extra_env(
    profile: RootSetupProfile,
    workspace_root: &std::path::Path,
) -> Vec<(String, String)> {
    let test_fleet_root = workspace_root.join("fleets").join("test");
    let test_fleet_root_env = || {
        (
            "CANIC_CANISTERS_ROOT".to_string(),
            test_fleet_root.display().to_string(),
        )
    };

    match profile {
        RootSetupProfile::Topology => vec![test_fleet_root_env()],
        RootSetupProfile::ReconcileSmallStore => vec![
            (
                "RUSTFLAGS".to_string(),
                TEST_SMALL_STORE_RUSTFLAGS.to_string(),
            ),
            test_fleet_root_env(),
        ],
        RootSetupProfile::Capability => vec![
            (
                "CANIC_CONFIG_PATH".to_string(),
                test_fleet_root
                    .join("test-configs")
                    .join("root-capability.toml")
                    .display()
                    .to_string(),
            ),
            test_fleet_root_env(),
        ],
        RootSetupProfile::Scaling => vec![
            (
                "CANIC_CONFIG_PATH".to_string(),
                test_fleet_root
                    .join("test-configs")
                    .join("root-scaling.toml")
                    .display()
                    .to_string(),
            ),
            test_fleet_root_env(),
        ],
        RootSetupProfile::Sharding => vec![
            (
                "CANIC_CONFIG_PATH".to_string(),
                test_fleet_root
                    .join("test-configs")
                    .join("root-sharding.toml")
                    .display()
                    .to_string(),
            ),
            (
                "CANIC_TEST_DELEGATION_MATERIAL".to_string(),
                "1".to_string(),
            ),
            test_fleet_root_env(),
        ],
    }
}

// Build one reusable baseline spec for a named root harness profile.
fn baseline_spec_for_profile(profile: RootSetupProfile) -> RootBaselineSpec<'static> {
    let workspace_root = workspace_root();
    let build_extra_env = profile_build_extra_env(profile, &workspace_root);
    baseline_spec_for_roles_owned_env(
        workspace_root,
        profile.release_roles(),
        profile.build_profile(),
        build_extra_env,
    )
}

// Build one reusable baseline spec from static release roles plus owned env overrides.
fn baseline_spec_for_roles_owned_env(
    workspace_root: PathBuf,
    release_roles: &'static [&'static str],
    build_profile: CanicWasmBuildProfile,
    mut build_extra_env: Vec<(String, String)>,
) -> RootBaselineSpec<'static> {
    if build_extra_env
        .iter()
        .all(|(key, _)| key != "CANIC_REFERENCE_CANISTERS")
    {
        let mut build_canisters = release_roles
            .iter()
            .map(|role| (*role).to_string())
            .collect::<Vec<_>>();
        build_canisters.push("root".to_string());
        build_extra_env.push((
            "CANIC_REFERENCE_CANISTERS".to_string(),
            build_canisters.join(" "),
        ));
    }

    RootBaselineSpec {
        progress_prefix: "root_setup",
        root_wasm_path: root_wasm_path(&workspace_root),
        root_wasm_artifact_path: root_wasm_path(&workspace_root),
        root_release_artifacts_dir: root_release_artifacts_dir(&workspace_root),
        artifact_watch_paths: ROOT_WASM_WATCH_PATHS,
        release_roles,
        icp_build_lock_path: icp_build_lock_path(&workspace_root),
        workspace_root,
        build_network: "local",
        build_profile,
        build_extra_env,
        bootstrap_tick_limit: BOOTSTRAP_TICK_LIMIT,
        root_setup_max_attempts: ROOT_SETUP_MAX_ATTEMPTS,
        pocket_ic_wasm_chunk_store_limit_bytes: 100 * 1024 * 1024,
        root_release_chunk_bytes: canic::CANIC_WASM_CHUNK_BYTES,
        package_version: env!("CARGO_PKG_VERSION"),
    }
}

fn root_wasm_path(workspace_root: &Path) -> PathBuf {
    root_release_artifacts_dir(workspace_root)
        .join("root")
        .join("root.wasm.gz")
}

fn root_release_artifacts_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".icp").join("local").join("canisters")
}

fn icp_build_lock_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".icp").join("canic-tests-build.lock")
}
