// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

use candid::Principal;
use canic::ids::CanisterRole;
use canic_testing_internal::pic::{
    RootBaselineSpec, ensure_root_release_artifacts_built, load_root_wasm,
    setup_root_topology as bootstrap_root_topology,
};
use ic_testkit::pic::{Pic, PicSerialGuard, acquire_pic_serial_guard};
use std::{
    collections::HashMap,
    io::Write,
    sync::{Mutex, MutexGuard},
};

use super::profile::RootSetupProfile;

static ROOT_SETUP_SERIAL: Mutex<()> = Mutex::new(());

fn test_progress(phase: &str) {
    eprintln!("[root_setup] {phase}");
    let _ = std::io::stderr().flush();
}

///
/// RootSetup
/// Result of setting up a fresh root canister for tests.
///

pub struct RootSetup {
    pub pic: Box<Pic>,
    pub root_id: Principal,
    pub subnet_directory: HashMap<CanisterRole, Principal>,
    _serial_guard: MutexGuard<'static, ()>,
    _pic_serial_guard: PicSerialGuard,
}

/// Acquire an isolated fresh root setup for one named root test profile.
#[must_use]
pub fn setup_root(profile: RootSetupProfile) -> RootSetup {
    setup_root_fresh(profile)
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
        pic: Box::new(state.pic),
        root_id: state.metadata.root_id,
        subnet_directory: state.metadata.subnet_directory,
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
