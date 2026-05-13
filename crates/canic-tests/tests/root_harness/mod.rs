// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

use canic::{cdk::types::Principal, ids::CanisterRole};
use canic_testing_internal::pic::{
    RootBaselineSpec, ensure_root_release_artifacts_built, load_root_wasm,
    setup_root_topology as bootstrap_root_topology,
};
use canic_testkit::{
    artifacts::{WasmBuildProfile, workspace_root_for},
    pic::{Pic, PicSerialGuard, acquire_pic_serial_guard},
};
use std::{
    collections::HashMap,
    io::Write,
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};

const ROOT_WASM_RELATIVE: &str = ".icp/local/canisters/root/root.wasm.gz";
const ROOT_WASM_ARTIFACT_RELATIVE: &str = ".icp/local/canisters/root/root.wasm.gz";
const ROOT_RELEASE_ARTIFACTS_RELATIVE: &str = ".icp/local/canisters";
const ROOT_TOPOLOGY_RELEASE_ROLES: &[&str] = &[
    "app",
    "scale_hub",
    "scale_replica",
    "user_hub",
    "user_shard",
];
const ROOT_CAPABILITY_RELEASE_ROLES: &[&str] = &["app", "scale_hub", "test"];
const ROOT_SHARDING_RELEASE_ROLES: &[&str] = &["test", "user_hub", "user_shard"];
const ICP_BUILD_LOCK_RELATIVE: &str = ".icp/canic-tests-build.lock";
const BOOTSTRAP_TICK_LIMIT: usize = 120;
const ROOT_SETUP_MAX_ATTEMPTS: usize = 2;
const ROOT_WASM_WATCH_PATHS: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "canisters",
    "icp.yaml",
    "crates",
];

static ROOT_SETUP_SERIAL: Mutex<()> = Mutex::new(());

///
/// RootSetupProfile
///

#[derive(Clone, Copy)]
pub enum RootSetupProfile {
    Topology,
    Capability,
    Sharding,
}

impl RootSetupProfile {
    const fn release_roles(self) -> &'static [&'static str] {
        match self {
            Self::Topology => ROOT_TOPOLOGY_RELEASE_ROLES,
            Self::Capability => ROOT_CAPABILITY_RELEASE_ROLES,
            Self::Sharding => ROOT_SHARDING_RELEASE_ROLES,
        }
    }

    fn baseline_spec(self) -> RootBaselineSpec<'static> {
        baseline_spec_for_profile(self)
    }
}

///
/// RootSetup
///

pub struct RootSetup {
    pub pic: Pic,
    pub root_id: Principal,
    pub subnet_index: HashMap<CanisterRole, Principal>,
    _serial_guard: MutexGuard<'static, ()>,
    _pic_serial_guard: PicSerialGuard,
}

/// Acquire an isolated fresh root setup for one named root test profile.
pub fn setup_root(profile: RootSetupProfile) -> RootSetup {
    let spec = profile.baseline_spec();

    test_progress("request fresh root setup");

    let serial_guard = acquire_root_setup_serial_guard();
    let pic_serial_guard = acquire_pic_serial_guard();

    ensure_root_release_artifacts_built(&spec);
    let root_wasm = load_root_wasm(&spec).expect("load root wasm");
    let state = bootstrap_root_topology(&spec, root_wasm);
    test_progress("fresh root setup ready");

    RootSetup {
        pic: state.pic,
        root_id: state.metadata.root_id,
        subnet_index: state.metadata.subnet_index,
        _serial_guard: serial_guard,
        _pic_serial_guard: pic_serial_guard,
    }
}

fn test_progress(phase: &str) {
    eprintln!("[root_harness] {phase}");
    let _ = std::io::stderr().flush();
}

// Serialize full root PocketIC usage to avoid concurrent runtime contention.
fn acquire_root_setup_serial_guard() -> MutexGuard<'static, ()> {
    ROOT_SETUP_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

// Return the shared repo root for root-harness artifact and config discovery.
fn workspace_root() -> PathBuf {
    workspace_root_for(env!("CARGO_MANIFEST_DIR"))
}

// Map one test profile to its embedded config override without leaking relative
// crate-local paths into the shared build environment.
fn profile_build_extra_env(
    profile: RootSetupProfile,
    workspace_root: &std::path::Path,
) -> Vec<(String, String)> {
    match profile {
        RootSetupProfile::Topology => Vec::new(),
        RootSetupProfile::Capability => vec![(
            "CANIC_CONFIG_PATH".to_string(),
            workspace_root
                .join("fleets/test/test-configs/root-capability.toml")
                .display()
                .to_string(),
        )],
        RootSetupProfile::Sharding => vec![(
            "CANIC_CONFIG_PATH".to_string(),
            workspace_root
                .join("fleets/test/test-configs/root-sharding.toml")
                .display()
                .to_string(),
        )],
    }
}

// Build one reusable baseline spec for a named root harness profile.
fn baseline_spec_for_profile(profile: RootSetupProfile) -> RootBaselineSpec<'static> {
    let workspace_root = workspace_root();
    let build_extra_env = profile_build_extra_env(profile, &workspace_root);
    baseline_spec_for_roles_owned_env(
        workspace_root,
        profile.release_roles(),
        WasmBuildProfile::Fast,
        build_extra_env,
    )
}

// Build one reusable baseline spec from static release roles plus owned env overrides.
fn baseline_spec_for_roles_owned_env(
    workspace_root: PathBuf,
    release_roles: &'static [&'static str],
    build_profile: WasmBuildProfile,
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
        progress_prefix: "root_harness",
        workspace_root,
        root_wasm_relative: ROOT_WASM_RELATIVE,
        root_wasm_artifact_relative: ROOT_WASM_ARTIFACT_RELATIVE,
        root_release_artifacts_relative: ROOT_RELEASE_ARTIFACTS_RELATIVE,
        artifact_watch_paths: ROOT_WASM_WATCH_PATHS,
        release_roles,
        icp_build_lock_relative: ICP_BUILD_LOCK_RELATIVE,
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
