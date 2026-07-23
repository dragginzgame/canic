use candid::Principal;
use ic_testkit::pic::CachedPicBaselineGuard;
use std::io::Write;

use super::baseline::{self, AttestationBaselineMetadata};

///
/// CachedInstalledRoot
///

pub struct CachedInstalledRoot {
    pub pic: BaselinePicGuard<'static>,
    pub root_id: Principal,
    pub issuer_id: Principal,
}

pub type BaselinePicGuard<'a> = CachedPicBaselineGuard<'a, AttestationBaselineMetadata>;

// Emit one short progress marker for long grouped PocketIC scenario tests.
pub(super) fn progress(phase: &str) {
    eprintln!("[pic_role_attestation] fixture: {phase}");
    let _ = std::io::stderr().flush();
}

/// Restore or create the cached `root + issuer` baseline.
#[must_use]
pub fn install_test_root_cached() -> CachedInstalledRoot {
    baseline::install_cached_root_fixture()
}
