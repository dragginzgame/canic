// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

use canic::cdk::types::Principal;
use canic_testing_internal::pic::{
    RootBaselineMetadata, RootBaselineSpec, build_root_cached_baseline,
    ensure_root_release_artifacts_built, load_root_wasm, restore_root_cached_baseline,
};
use canic_testkit::{
    artifacts::{WasmBuildProfile, workspace_root_for},
    pic::{
        CachedPicBaseline, CachedPicBaselineGuard, Pic, PicSerialGuard, acquire_pic_serial_guard,
        restore_or_rebuild_cached_pic_baseline,
    },
};
use std::{
    io::Write,
    ops::Deref,
    sync::{Mutex, MutexGuard},
};

const ROOT_WASM_RELATIVE: &str = ".icp/local/canisters/root/root.wasm.gz";
const ROOT_WASM_ARTIFACT_RELATIVE: &str = ".icp/local/canisters/root/root.wasm.gz";
const ROOT_RELEASE_ARTIFACTS_RELATIVE: &str = ".icp/local/canisters";
const ROOT_RECONCILE_RELEASE_ROLES: &[&str] = &[
    "app",
    "scale_hub",
    "scale_replica",
    "user_hub",
    "user_shard",
];
const TEST_SMALL_STORE_RUSTFLAGS: &str = "--cfg canic_test_small_wasm_store";
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
static ROOT_RECONCILE_BASELINE: Mutex<Option<CachedPicBaseline<RootBaselineMetadata>>> =
    Mutex::new(None);

///
/// RootSetup
///

pub struct RootSetup {
    pub pic: RootPicHandle,
    pub root_id: Principal,
    _serial_guard: MutexGuard<'static, ()>,
    _pic_serial_guard: PicSerialGuard,
}

///
/// RootPicHandle
///

pub struct RootPicHandle(CachedPicBaselineGuard<'static, RootBaselineMetadata>);

impl Deref for RootPicHandle {
    type Target = Pic;

    fn deref(&self) -> &Self::Target {
        self.0.pic()
    }
}

/// Acquire the shared cached reconcile root setup for wasm-store reconcile tests.
pub fn setup_cached_root() -> RootSetup {
    test_progress("request cached root reconcile small-store baseline");

    let serial_guard = acquire_root_setup_serial_guard();
    let pic_serial_guard = acquire_pic_serial_guard();
    let spec = baseline_spec();
    ensure_root_release_artifacts_built(&spec);
    let root_wasm = load_root_wasm(&spec).expect("load root wasm");

    let (baseline, cache_hit) = restore_or_rebuild_cached_pic_baseline(
        &ROOT_RECONCILE_BASELINE,
        || {
            test_progress("cache miss, building fresh root baseline");
            build_root_cached_baseline(&spec, root_wasm.clone())
        },
        |baseline| restore_root_cached_baseline(&spec, baseline),
    );

    if cache_hit {
        test_progress("cache hit, restoring cached root baseline");
        test_progress("cached root baseline restore complete");
    } else {
        test_progress("fresh root baseline ready");
    }

    RootSetup {
        root_id: baseline.metadata().root_id,
        pic: RootPicHandle(baseline),
        _serial_guard: serial_guard,
        _pic_serial_guard: pic_serial_guard,
    }
}

fn baseline_spec() -> RootBaselineSpec<'static> {
    let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
    let build_extra_env = vec![(
        "RUSTFLAGS".to_string(),
        TEST_SMALL_STORE_RUSTFLAGS.to_string(),
    )];

    let mut build_extra_env = build_extra_env;
    let mut build_canisters = ROOT_RECONCILE_RELEASE_ROLES
        .iter()
        .map(|role| (*role).to_string())
        .collect::<Vec<_>>();
    build_canisters.push("root".to_string());
    build_extra_env.push((
        "CANIC_REFERENCE_CANISTERS".to_string(),
        build_canisters.join(" "),
    ));

    RootBaselineSpec {
        progress_prefix: "root_harness",
        workspace_root,
        root_wasm_relative: ROOT_WASM_RELATIVE,
        root_wasm_artifact_relative: ROOT_WASM_ARTIFACT_RELATIVE,
        root_release_artifacts_relative: ROOT_RELEASE_ARTIFACTS_RELATIVE,
        artifact_watch_paths: ROOT_WASM_WATCH_PATHS,
        release_roles: ROOT_RECONCILE_RELEASE_ROLES,
        icp_build_lock_relative: ICP_BUILD_LOCK_RELATIVE,
        build_network: "local",
        build_profile: WasmBuildProfile::Debug,
        build_extra_env,
        bootstrap_tick_limit: BOOTSTRAP_TICK_LIMIT,
        root_setup_max_attempts: ROOT_SETUP_MAX_ATTEMPTS,
        pocket_ic_wasm_chunk_store_limit_bytes: 100 * 1024 * 1024,
        root_release_chunk_bytes: canic::CANIC_WASM_CHUNK_BYTES,
        package_version: env!("CARGO_PKG_VERSION"),
    }
}

fn test_progress(phase: &str) {
    eprintln!("[root_harness] {phase}");
    let _ = std::io::stderr().flush();
}

fn acquire_root_setup_serial_guard() -> MutexGuard<'static, ()> {
    ROOT_SETUP_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
