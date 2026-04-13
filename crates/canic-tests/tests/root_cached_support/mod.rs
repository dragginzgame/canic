// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

use canic::{cdk::types::Principal, ids::CanisterRole};
use canic_testing_internal::pic::{
    RootBaselineMetadata, RootBaselineSpec, build_root_cached_baseline,
    ensure_root_release_artifacts_built, load_root_wasm, restore_root_cached_baseline,
};
use canic_testkit::{
    artifacts::WasmBuildProfile,
    pic::{
        CachedPicBaseline, CachedPicBaselineGuard, Pic, PicSerialGuard, acquire_pic_serial_guard,
        restore_or_rebuild_cached_pic_baseline,
    },
};
use std::{
    collections::HashMap,
    io::Write,
    ops::Deref,
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};

const ROOT_WASM_RELATIVE: &str = ".dfx/local/canisters/root/root.wasm.gz";
const ROOT_WASM_ARTIFACT_RELATIVE: &str = ".dfx/local/canisters/root/root.wasm.gz";
const ROOT_RELEASE_ARTIFACTS_RELATIVE: &str = ".dfx/local/canisters";
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

///
/// RootSetup
///

pub struct RootSetup {
    pub pic: RootPicHandle,
    pub root_id: Principal,
    pub subnet_index: HashMap<CanisterRole, Principal>,
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

/// Acquire one cached root setup from the provided cache slot and baseline spec.
pub fn setup_cached_root(
    cache_label: &str,
    cache_slot: &'static Mutex<Option<CachedPicBaseline<RootBaselineMetadata>>>,
    spec: RootBaselineSpec<'static>,
) -> RootSetup {
    test_progress(&format!("request {cache_label}"));

    let serial_guard = acquire_root_setup_serial_guard();
    let pic_serial_guard = acquire_pic_serial_guard();
    ensure_root_release_artifacts_built(&spec);
    let root_wasm = load_root_wasm(&spec).expect("load root wasm");

    let (baseline, cache_hit) = restore_or_rebuild_cached_pic_baseline(
        cache_slot,
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
        subnet_index: baseline.metadata().subnet_index.clone(),
        pic: RootPicHandle(baseline),
        _serial_guard: serial_guard,
        _pic_serial_guard: pic_serial_guard,
    }
}

/// Build one reusable baseline spec from static release roles plus owned env overrides.
pub fn baseline_spec_for_roles_owned_env(
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
