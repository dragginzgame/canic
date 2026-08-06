// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

use candid::Principal;
use canic::ids::CanisterRole;
use canic_testing_internal::pic::{RootBaselineRecipe, RootBaselineSpec};
use ic_testkit::pic::{CachedPocketIcBaselinePool, CachedPocketIcBaselinePoolGuard, PocketIc};
use std::{
    collections::HashMap,
    io::Write,
    num::NonZeroUsize,
    ops::Deref,
    sync::{Mutex, MutexGuard, OnceLock},
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
    pub pic: RootPocketIc,
    pub root_id: Principal,
    pub component_canisters: HashMap<CanisterRole, Principal>,
    _serial_guard: MutexGuard<'static, ()>,
}

/// Exclusive lease of one restored root-test PocketIC baseline.
pub struct RootPocketIc {
    baseline: CachedPocketIcBaselinePoolGuard<'static, RootBaselineRecipe>,
}

impl Deref for RootPocketIc {
    type Target = PocketIc;

    fn deref(&self) -> &Self::Target {
        self.baseline.pocket_ic()
    }
}

/// Acquire an isolated, restored root setup for one named root test profile.
///
/// # Panics
///
/// Panics if the profile's artifact set, PocketIC topology, snapshot reset or
/// post-reset invariant validation cannot be prepared.
#[must_use]
pub fn setup_root(profile: RootSetupProfile) -> RootSetup {
    test_progress("request pooled root setup");
    let serial_guard = acquire_root_setup_serial_guard();
    let baseline_spec = profile.baseline_spec();
    let (baseline, outcome) = root_baseline_pool(profile, baseline_spec)
        .acquire()
        .expect("acquire root topology baseline");
    let metadata = baseline.metadata().clone();
    test_progress(&format!("pooled root setup ready: {outcome:?}"));

    RootSetup {
        pic: RootPocketIc { baseline },
        root_id: metadata.root_id,
        component_canisters: metadata.component_canisters,
        _serial_guard: serial_guard,
    }
}

fn root_baseline_pool(
    profile: RootSetupProfile,
    spec: RootBaselineSpec<'static>,
) -> &'static CachedPocketIcBaselinePool<RootBaselineRecipe> {
    static TOPOLOGY: OnceLock<CachedPocketIcBaselinePool<RootBaselineRecipe>> = OnceLock::new();
    static CAPABILITY: OnceLock<CachedPocketIcBaselinePool<RootBaselineRecipe>> = OnceLock::new();
    static SCALING: OnceLock<CachedPocketIcBaselinePool<RootBaselineRecipe>> = OnceLock::new();
    static SHARDING: OnceLock<CachedPocketIcBaselinePool<RootBaselineRecipe>> = OnceLock::new();

    let (pool, identity) = match profile {
        RootSetupProfile::Topology => (&TOPOLOGY, "canic/root-audit/topology/v1"),
        RootSetupProfile::Capability => (&CAPABILITY, "canic/root-audit/capability/v1"),
        RootSetupProfile::Scaling => (&SCALING, "canic/root-audit/scaling/v1"),
        RootSetupProfile::Sharding => (&SHARDING, "canic/root-audit/sharding/v1"),
    };
    pool.get_or_init(|| {
        CachedPocketIcBaselinePool::new(
            NonZeroUsize::new(1).expect("one is nonzero"),
            RootBaselineRecipe::try_new(identity, spec).expect("valid root baseline recipe"),
        )
    })
}

// Serialize full root PocketIC usage to avoid concurrent runtime contention.
fn acquire_root_setup_serial_guard() -> MutexGuard<'static, ()> {
    ROOT_SETUP_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
