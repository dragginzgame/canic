// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

use canic::{cdk::types::Principal, ids::CanisterRole};
use canic_testing_internal::pic::{
    RootBaselineMetadata, RootBaselineSpec, build_root_cached_baseline,
    ensure_root_release_artifacts_built, load_root_wasm, restore_root_cached_baseline,
    setup_root_topology as bootstrap_root_topology,
};
use canic_testkit::pic::{
    CachedPicBaseline, CachedPicBaselineGuard, Pic, PicSerialGuard, acquire_pic_serial_guard,
    restore_or_rebuild_cached_pic_baseline,
};
use std::{
    collections::HashMap,
    io::Write,
    ops::{Deref, DerefMut},
    sync::{Mutex, MutexGuard},
};

use super::profile::RootSetupProfile;

static ROOT_SETUP_SERIAL: Mutex<()> = Mutex::new(());

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

impl Deref for RootPicHandle {
    type Target = Pic;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Fresh(pic) => pic,
            Self::Cached(baseline) => baseline.pic(),
        }
    }
}

impl DerefMut for RootPicHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Fresh(pic) => pic,
            Self::Cached(baseline) => baseline.pic_mut(),
        }
    }
}

/// Acquire an isolated fresh root setup for one named root test profile.
pub fn setup_root(profile: RootSetupProfile) -> RootSetup {
    setup_root_fresh(profile)
}

/// Acquire a cached root setup for one named root test profile.
pub fn setup_cached_root(profile: RootSetupProfile) -> RootSetup {
    setup_root_cached(profile)
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
        subnet_directory: baseline.metadata().subnet_directory.clone(),
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
