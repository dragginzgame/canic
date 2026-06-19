use crate::canister::{APP, SCALE_HUB};
use canic::cdk::types::Principal;
use ic_testkit::{
    artifacts::{build_wasm_canisters, read_wasm, test_target_dir, workspace_root_for},
    pic::{Pic, PicSerialGuard, StandaloneCanisterFixture, acquire_pic_serial_guard, pic},
};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use super::{CanicPicExt, CanicWasmBuildProfile, install_standalone_canister};

const AUDIT_READY_TICK_LIMIT: usize = 60;
static AUDIT_BUILD_SERIAL: Mutex<()> = Mutex::new(());

pub struct RootAuditProbeFixture {
    pub pic: Pic,
    pub canister_id: Principal,
    _serial_guard: PicSerialGuard,
}

// Build one standalone internal leaf probe for shared query-floor audits.
#[must_use]
pub fn install_audit_leaf_probe(profile: CanicWasmBuildProfile) -> StandaloneCanisterFixture {
    install_standalone_canister("leaf_probe", APP, profile)
}

// Build one standalone internal scaling probe for dry-run placement audits.
#[must_use]
pub fn install_audit_scaling_probe(profile: CanicWasmBuildProfile) -> StandaloneCanisterFixture {
    install_standalone_canister("scaling_probe", SCALE_HUB, profile)
}

/// Build one standalone internal root probe for root-only query audits.
///
/// # Panics
///
/// Panics if the probe wasm cannot be built/read, the PocketIC instance cannot
/// install the root probe canister, or the probe does not become ready within
/// the configured tick limit.
#[must_use]
pub fn install_audit_root_probe(profile: CanicWasmBuildProfile) -> RootAuditProbeFixture {
    let workspace_root = workspace_root();
    let target_dir = test_target_dir(&workspace_root, "standalone-root-probe");
    ensure_probe_wasm_ready(&workspace_root, &target_dir, "root_probe", profile);

    let wasm = read_wasm(&target_dir, "root_probe", profile.target_dir_name());
    let serial_guard = acquire_pic_serial_guard();
    let pic = pic();
    let canister_id = pic
        .create_and_install_root_canister(wasm)
        .expect("install audit root probe canister");
    pic.wait_for_ready(
        canister_id,
        AUDIT_READY_TICK_LIMIT,
        "audit root probe bootstrap",
    );

    RootAuditProbeFixture {
        pic,
        canister_id,
        _serial_guard: serial_guard,
    }
}

fn ensure_probe_wasm_ready(
    workspace_root: &Path,
    target_dir: &Path,
    crate_name: &str,
    profile: CanicWasmBuildProfile,
) {
    let _build_guard = AUDIT_BUILD_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    build_wasm_canisters(
        workspace_root,
        target_dir,
        &[crate_name],
        profile.cargo_profile_args(),
        &[],
    );
}

fn workspace_root() -> PathBuf {
    workspace_root_for(env!("CARGO_MANIFEST_DIR"))
}
