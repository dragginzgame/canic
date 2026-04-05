// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

use canic::{cdk::types::Principal, ids::CanisterRole};
use canic_testing_internal::pic::{
    RootBaselineMetadata, RootBaselineSpec, build_root_cached_baseline,
    ensure_root_release_artifacts_built, load_root_wasm, restore_root_cached_baseline,
    setup_root_topology as bootstrap_root_topology,
};
use canic_testkit::{
    artifacts::{WasmBuildProfile, workspace_root_for},
    pic::{
        CachedPicBaseline, CachedPicBaselineGuard, Pic, PicSerialGuard,
        acquire_cached_pic_baseline, acquire_pic_serial_guard,
    },
};
use std::{
    collections::HashMap,
    io::Write,
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};

/// Default location of the root wasm relative to the workspace root.
const ROOT_WASM_RELATIVE: &str = ".dfx/local/canisters/root/root.wasm.gz";
const ROOT_WASM_ARTIFACT_RELATIVE: &str = ".dfx/local/canisters/root/root.wasm.gz";
const ROOT_RELEASE_ARTIFACTS_RELATIVE: &str = ".dfx/local/canisters";
const ROOT_TOPOLOGY_RELEASE_ROLES: &[&str] = &["app", "scale_hub", "test", "user_hub"];
const ROOT_CAPABILITY_RELEASE_ROLES: &[&str] = &["app", "scale_hub", "test"];
const ROOT_SCALING_RELEASE_ROLES: &[&str] = &["scale", "scale_hub"];
const ROOT_SHARDING_RELEASE_ROLES: &[&str] = &["test", "user_hub", "user_shard"];
const ROOT_RECONCILE_RELEASE_ROLES: &[&str] =
    &["app", "minimal", "scale", "scale_hub", "test", "user_hub"];
const TEST_SMALL_STORE_RUSTFLAGS: &str = "--cfg canic_test_small_wasm_store";
const DFX_BUILD_LOCK_RELATIVE: &str = ".dfx/canic-tests-build.lock";
const BOOTSTRAP_TICK_LIMIT: usize = 120;
const ROOT_SETUP_MAX_ATTEMPTS: usize = 2;
const ROOT_WASM_WATCH_PATHS: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "canisters",
    "dfx.json",
    "crates",
    "scripts/app/build.sh",
];
static ROOT_SETUP_SERIAL: Mutex<()> = Mutex::new(());
static ROOT_TOPOLOGY_BASELINE: Mutex<Option<CachedPicBaseline<RootBaselineMetadata>>> =
    Mutex::new(None);
static ROOT_CAPABILITY_BASELINE: Mutex<Option<CachedPicBaseline<RootBaselineMetadata>>> =
    Mutex::new(None);
static ROOT_SCALING_BASELINE: Mutex<Option<CachedPicBaseline<RootBaselineMetadata>>> =
    Mutex::new(None);
static ROOT_SHARDING_BASELINE: Mutex<Option<CachedPicBaseline<RootBaselineMetadata>>> =
    Mutex::new(None);
static ROOT_RECONCILE_BASELINE: Mutex<Option<CachedPicBaseline<RootBaselineMetadata>>> =
    Mutex::new(None);

fn test_progress(phase: &str) {
    eprintln!("[root_harness] {phase}");
    let _ = std::io::stderr().flush();
}

///
/// RootSetup
/// Result of setting up a fresh root canister for tests.
///

pub struct RootSetup {
    pub pic: RootPicHandle,
    pub root_id: Principal,
    pub subnet_directory: HashMap<CanisterRole, Principal>,
    _serial_guard: MutexGuard<'static, ()>,
    _pic_serial_guard: PicSerialGuard,
}

///
/// RootPicHandle
///

pub enum RootPicHandle {
    Fresh(Box<Pic>),
    Cached(CachedPicBaselineGuard<'static, RootBaselineMetadata>),
}

#[derive(Clone, Copy)]
enum RootSetupProfile {
    Topology,
    Capability,
    Scaling,
    Sharding,
    ReconcileSmallStore,
}

impl RootSetupProfile {
    const fn cache_label(self) -> &'static str {
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

    fn cache_slot(self) -> &'static Mutex<Option<CachedPicBaseline<RootBaselineMetadata>>> {
        match self {
            Self::Topology => &ROOT_TOPOLOGY_BASELINE,
            Self::Capability => &ROOT_CAPABILITY_BASELINE,
            Self::Scaling => &ROOT_SCALING_BASELINE,
            Self::Sharding => &ROOT_SHARDING_BASELINE,
            Self::ReconcileSmallStore => &ROOT_RECONCILE_BASELINE,
        }
    }

    const fn build_profile(self) -> WasmBuildProfile {
        match self {
            Self::ReconcileSmallStore => WasmBuildProfile::Debug,
            Self::Topology | Self::Capability | Self::Scaling | Self::Sharding => {
                WasmBuildProfile::Fast
            }
        }
    }

    fn baseline_spec(self) -> RootBaselineSpec<'static> {
        baseline_spec_for_profile(self)
    }
}

impl Deref for RootPicHandle {
    type Target = Pic;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Fresh(pic) => pic,
            Self::Cached(baseline) => &baseline.pic,
        }
    }
}

impl DerefMut for RootPicHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Fresh(pic) => pic,
            Self::Cached(baseline) => &mut baseline.pic,
        }
    }
}

/// Acquire an isolated fresh root setup for the reference topology profile.
pub fn setup_root_topology() -> RootSetup {
    setup_root_fresh(RootSetupProfile::Topology)
}

/// Acquire an isolated fresh root setup that includes the `scale` template.
pub fn setup_root_scaling() -> RootSetup {
    setup_root_fresh(RootSetupProfile::Scaling)
}

/// Acquire an isolated fresh root setup for root capability and replay paths.
pub fn setup_root_capability() -> RootSetup {
    setup_root_fresh(RootSetupProfile::Capability)
}

/// Acquire an isolated fresh root setup that includes the `user_shard` template.
pub fn setup_root_sharding() -> RootSetup {
    setup_root_fresh(RootSetupProfile::Sharding)
}

/// Acquire an isolated topology-only cached root setup.
///
/// This stages only the ordinary releases needed by hierarchy assertions.
pub fn setup_root_cached_topology() -> RootSetup {
    setup_root_cached(RootSetupProfile::Topology)
}

/// Acquire a cached root setup for root capability and replay paths.
pub fn setup_root_cached_capability() -> RootSetup {
    setup_root_cached(RootSetupProfile::Capability)
}

/// Acquire a cached root setup that includes the `scale` template for replay/scaling paths.
pub fn setup_root_cached_scaling() -> RootSetup {
    setup_root_cached(RootSetupProfile::Scaling)
}

/// Acquire a cached root setup that includes the `user_shard` template for auth/sharding paths.
pub fn setup_root_cached_sharding() -> RootSetup {
    setup_root_cached(RootSetupProfile::Sharding)
}

/// Acquire a cached root setup for small-store reconcile and wasm_store lifecycle tests.
pub fn setup_root_cached_reconcile_small_store() -> RootSetup {
    setup_root_cached(RootSetupProfile::ReconcileSmallStore)
}

fn setup_root_fresh(profile: RootSetupProfile) -> RootSetup {
    setup_root_fresh_spec(profile.baseline_spec())
}

fn setup_root_fresh_spec(spec: RootBaselineSpec<'static>) -> RootSetup {
    test_progress("request fresh root setup");

    // Each setup spins up a full PocketIC topology; serialize to avoid
    // exhausting local temp storage under parallel test execution.
    let serial_guard = acquire_root_setup_serial_guard();
    let pic_serial_guard = acquire_pic_serial_guard();

    ensure_root_release_artifacts_built(&spec);
    let root_wasm = load_root_wasm(&spec).expect("load root wasm");
    let state = bootstrap_root_topology(&spec, root_wasm);
    test_progress("fresh root setup ready");

    RootSetup {
        pic: RootPicHandle::Fresh(Box::new(state.pic)),
        root_id: state.metadata.root_id,
        subnet_directory: state.metadata.subnet_directory,
        _serial_guard: serial_guard,
        _pic_serial_guard: pic_serial_guard,
    }
}

fn setup_root_cached(profile: RootSetupProfile) -> RootSetup {
    setup_root_cached_spec(
        profile.cache_label(),
        profile.cache_slot(),
        profile.baseline_spec(),
    )
}

fn setup_root_cached_spec(
    cache_label: &str,
    cache_slot: &'static Mutex<Option<CachedPicBaseline<RootBaselineMetadata>>>,
    spec: RootBaselineSpec<'static>,
) -> RootSetup {
    test_progress(&format!("request {cache_label}"));

    let serial_guard = acquire_root_setup_serial_guard();
    let pic_serial_guard = acquire_pic_serial_guard();
    ensure_root_release_artifacts_built(&spec);
    let root_wasm = load_root_wasm(&spec).expect("load root wasm");

    let (baseline, cache_hit) = acquire_cached_pic_baseline(cache_slot, || {
        test_progress("cache miss, building fresh root baseline");
        build_root_cached_baseline(&spec, root_wasm)
    });

    if cache_hit {
        test_progress("cache hit, restoring cached root baseline");
        restore_root_cached_baseline(&spec, &baseline);
        test_progress("cached root baseline restore complete");
    } else {
        test_progress("fresh root baseline ready");
    }

    RootSetup {
        root_id: baseline.metadata.root_id,
        subnet_directory: baseline.metadata.subnet_directory.clone(),
        pic: RootPicHandle::Cached(baseline),
        _serial_guard: serial_guard,
        _pic_serial_guard: pic_serial_guard,
    }
}

// Serialize full root PocketIC usage to avoid concurrent runtime contention.
fn acquire_root_setup_serial_guard() -> MutexGuard<'static, ()> {
    ROOT_SETUP_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

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
                .join("canisters/test-configs/root-capability.toml")
                .display()
                .to_string(),
        )],
        RootSetupProfile::Scaling => vec![(
            "CANIC_CONFIG_PATH".to_string(),
            workspace_root
                .join("canisters/test-configs/root-scaling.toml")
                .display()
                .to_string(),
        )],
        RootSetupProfile::Sharding => vec![(
            "CANIC_CONFIG_PATH".to_string(),
            workspace_root
                .join("canisters/test-configs/root-sharding.toml")
                .display()
                .to_string(),
        )],
        RootSetupProfile::ReconcileSmallStore => vec![(
            "RUSTFLAGS".to_string(),
            TEST_SMALL_STORE_RUSTFLAGS.to_string(),
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
        profile.build_profile(),
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
        dfx_build_lock_relative: DFX_BUILD_LOCK_RELATIVE,
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
