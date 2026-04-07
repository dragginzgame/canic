use candid::Principal;
use canic_testkit::pic::CachedPicBaselineGuard;
use std::{io::Write, ops::Deref};

use super::baseline::{self, AttestationBaselineMetadata};

///
/// CachedInstalledRoot
///

pub struct CachedInstalledRoot {
    pub pic: BaselinePicGuard<'static>,
    pub root_id: Principal,
    pub signer_id: Principal,
    pub verifier_id: Option<Principal>,
}

///
/// BaselinePicGuard
///

pub struct BaselinePicGuard<'a> {
    baseline: CachedPicBaselineGuard<'a, AttestationBaselineMetadata>,
}

impl<'a> BaselinePicGuard<'a> {
    // Wrap one cached attestation baseline guard in the fixture-facing Pic view.
    pub(super) const fn new(
        baseline: CachedPicBaselineGuard<'a, AttestationBaselineMetadata>,
    ) -> Self {
        Self { baseline }
    }
}

impl Deref for BaselinePicGuard<'_> {
    type Target = pocket_ic::PocketIc;

    fn deref(&self) -> &Self::Target {
        self.baseline.pic()
    }
}

// Emit one short progress marker for long grouped PocketIC scenario tests.
pub(super) fn progress(phase: &str) {
    eprintln!("[pic_role_attestation] fixture: {phase}");
    let _ = std::io::stderr().flush();
}

/// Restore or create the cached `root + signer` baseline.
#[must_use]
pub fn install_test_root_cached() -> CachedInstalledRoot {
    baseline::install_signer_only_cached_root_fixture()
}

/// Restore or create the cached `root + signer + verifier` baseline.
#[must_use]
pub fn install_test_root_with_verifier_cached() -> CachedInstalledRoot {
    baseline::install_signer_and_verifier_cached_root_fixture()
}

/// Restore or create the cached normal-build `root + signer` baseline.
#[must_use]
pub fn install_test_root_without_test_material_cached() -> CachedInstalledRoot {
    baseline::install_signer_only_without_test_material_cached_root_fixture()
}

// Resolve the signer canister from the root-managed subnet registry.
#[must_use]
pub fn signer_pid(pic: &pocket_ic::PocketIc, root_id: Principal) -> Principal {
    baseline::signer_pid(pic, root_id)
}

// Resolve the managed wasm_store canister from the root-managed subnet registry.
#[must_use]
pub fn wasm_store_pid(pic: &pocket_ic::PocketIc, root_id: Principal) -> Principal {
    baseline::wasm_store_pid(pic, root_id)
}
