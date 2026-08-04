use crate::canister::{APP, SCALE_HUB};
use candid::Principal;
use ic_testkit::{
    artifacts::{build_wasm_canisters, read_wasm, test_target_dir, workspace_root_for},
    pic::{PocketIc, PocketIcBuilder, StandaloneCanisterFixture},
};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use super::{
    CanicPicExt, CanicWasmBuildProfile,
    artifacts::{INTERNAL_TEST_ENDPOINTS_ENV, INTERNAL_TEST_RELEASE_BUILD_ID},
    install_standalone_canister,
};

static AUDIT_BUILD_SERIAL: Mutex<()> = Mutex::new(());

pub struct RootAuditProbeFixture {
    pub pic: PocketIc,
    pub canister_id: Principal,
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
/// install the root probe canister, or its protected Prepared authority is
/// rejected.
#[must_use]
pub fn install_audit_root_probe(profile: CanicWasmBuildProfile) -> RootAuditProbeFixture {
    let workspace_root = workspace_root();
    let target_dir = test_target_dir(&workspace_root, "standalone-root-probe");
    ensure_probe_wasm_ready(&workspace_root, &target_dir, "root_probe", profile);

    let root_wasm = read_wasm(&target_dir, "root_probe", profile.target_dir_name());
    let wasm_store_wasm = read_wasm(
        &target_dir,
        "canister_wasm_store",
        profile.target_dir_name(),
    );
    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let canister_id = pic
        .create_and_install_root_canister(
            root_wasm,
            wasm_store_wasm,
            &workspace_root.join("canisters/audit/root_probe/canic.toml"),
        )
        .expect("install audit root probe canister");

    RootAuditProbeFixture { pic, canister_id }
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

    let config_path = workspace_root.join("canisters/audit/root_probe/canic.toml");
    let config_path = config_path.to_str().expect("audit root config UTF-8");
    let build_env = [
        ("ICP_ENVIRONMENT", "local"),
        (
            canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV,
            config_path,
        ),
        INTERNAL_TEST_ENDPOINTS_ENV,
        INTERNAL_TEST_RELEASE_BUILD_ID,
    ];
    build_wasm_canisters(
        workspace_root,
        target_dir,
        &[crate_name, "canic-wasm-store"],
        profile.cargo_profile_args(),
        &build_env,
    );
}

fn workspace_root() -> PathBuf {
    workspace_root_for(env!("CARGO_MANIFEST_DIR"))
}
