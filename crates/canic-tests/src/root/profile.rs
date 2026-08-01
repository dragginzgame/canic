use canic::ids::BuildNetwork;
use canic_testing_internal::pic::{CanicWasmBuildProfile, RootBaselineSpec};
use ic_testkit::artifacts::workspace_root_for;
use std::path::{Path, PathBuf};

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
const BOOTSTRAP_TICK_LIMIT: usize = 120;
const ROOT_SETUP_MAX_ATTEMPTS: usize = 2;
const ROOT_WASM_WATCH_PATHS: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "canisters",
    "apps/test",
    "icp.yaml",
    "crates",
];

#[derive(Clone, Copy)]
pub enum RootSetupProfile {
    Topology,
    Capability,
    Scaling,
    Sharding,
}

impl RootSetupProfile {
    const fn release_roles(self) -> &'static [&'static str] {
        match self {
            Self::Topology => ROOT_TOPOLOGY_RELEASE_ROLES,
            Self::Capability => ROOT_CAPABILITY_RELEASE_ROLES,
            Self::Scaling => ROOT_SCALING_RELEASE_ROLES,
            Self::Sharding => ROOT_SHARDING_RELEASE_ROLES,
        }
    }

    const fn build_profile(self) -> CanicWasmBuildProfile {
        match self {
            Self::Topology | Self::Capability | Self::Scaling | Self::Sharding => {
                CanicWasmBuildProfile::Fast
            }
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

fn profile_build_config_path(profile: RootSetupProfile, workspace_root: &Path) -> PathBuf {
    let test_fleet_subnet_root = workspace_root.join("apps").join("test");
    match profile {
        RootSetupProfile::Topology => test_fleet_subnet_root.join("canic.toml"),
        RootSetupProfile::Capability => {
            test_fleet_subnet_root.join("test-configs/root-capability.toml")
        }
        RootSetupProfile::Scaling => test_fleet_subnet_root.join("test-configs/root-scaling.toml"),
        RootSetupProfile::Sharding => {
            test_fleet_subnet_root.join("test-configs/root-sharding.toml")
        }
    }
}

// Build one reusable baseline spec for a named root harness profile.
fn baseline_spec_for_profile(profile: RootSetupProfile) -> RootBaselineSpec<'static> {
    let workspace_root = workspace_root();
    let build_config_path = profile_build_config_path(profile, &workspace_root);
    baseline_spec_for_roles_owned_env(
        workspace_root,
        profile.release_roles(),
        profile.build_profile(),
        build_config_path,
        Vec::new(),
    )
}

// Build one reusable baseline spec from static release roles plus owned env overrides.
fn baseline_spec_for_roles_owned_env(
    workspace_root: PathBuf,
    release_roles: &'static [&'static str],
    build_profile: CanicWasmBuildProfile,
    build_config_path: PathBuf,
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
        build_network: BuildNetwork::Local,
        build_profile,
        build_config_path,
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
