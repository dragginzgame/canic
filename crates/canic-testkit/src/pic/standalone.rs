use candid::encode_args;
use canic::{
    dto::{
        abi::v1::CanisterInitPayload,
        env::EnvBootstrapArgs,
        topology::{AppDirectoryArgs, SubnetDirectoryArgs},
    },
    ids::{CanisterRole, SubnetRole},
};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use crate::{
    Fake,
    artifacts::{
        WasmBuildProfile, build_wasm_canisters, read_wasm, test_target_dir, wasm_artifacts_ready,
        workspace_root_for,
    },
};

use super::{Pic, PicSerialGuard, acquire_pic_serial_guard, pic};

const STANDALONE_INSTALL_CYCLES: u128 = 1_000_000_000_000;
const STANDALONE_READY_TICK_LIMIT: usize = 60;
static STANDALONE_BUILD_SERIAL: Mutex<()> = Mutex::new(());

///
/// StandaloneCanisterFixture
///

pub struct StandaloneCanisterFixture {
    pub pic: Pic,
    pub canister_id: canic::cdk::types::Principal,
    _serial_guard: PicSerialGuard,
}

// Install one already-built wasm module into a fresh PocketIC instance with
// caller-provided init args and no Canic-specific bootstrap assumptions.
#[must_use]
pub fn install_prebuilt_canister(wasm: Vec<u8>, init_bytes: Vec<u8>) -> StandaloneCanisterFixture {
    install_prebuilt_canister_with_cycles(wasm, init_bytes, STANDALONE_INSTALL_CYCLES)
}

// Install one already-built wasm module into a fresh PocketIC instance with
// caller-provided init args and explicit install cycles.
#[must_use]
pub fn install_prebuilt_canister_with_cycles(
    wasm: Vec<u8>,
    init_bytes: Vec<u8>,
    install_cycles: u128,
) -> StandaloneCanisterFixture {
    let serial_guard = acquire_pic_serial_guard();
    let pic = pic();
    let canister_id = pic.create_and_install_with_args(wasm, init_bytes, install_cycles);

    StandaloneCanisterFixture {
        pic,
        canister_id,
        _serial_guard: serial_guard,
    }
}

// Install one non-root Canic canister into a fresh PocketIC instance with
// explicit local env bootstrap fields and no hierarchy directories.
#[must_use]
pub fn install_standalone_canister(
    crate_name: &str,
    role: CanisterRole,
    profile: WasmBuildProfile,
) -> StandaloneCanisterFixture {
    assert!(
        !role.is_root(),
        "standalone helper is for non-root canisters"
    );

    let workspace_root = workspace_root();
    let target_name = format!("standalone-{crate_name}");
    let target_dir = test_target_dir(&workspace_root, &target_name);
    ensure_canister_wasm_ready(&workspace_root, &target_dir, crate_name, profile);

    let wasm = read_wasm(&target_dir, crate_name, profile);
    let fixture = install_prebuilt_canister(wasm, standalone_init_args(role));
    let canister_id = fixture.canister_id;
    let pic = &fixture.pic;
    pic.wait_for_ready(
        canister_id,
        STANDALONE_READY_TICK_LIMIT,
        "standalone canister bootstrap",
    );

    fixture
}

// Build the requested wasm artifact once when it is missing from the shared
// standalone target directory.
fn ensure_canister_wasm_ready(
    workspace_root: &Path,
    target_dir: &Path,
    crate_name: &str,
    profile: WasmBuildProfile,
) {
    let _build_guard = STANDALONE_BUILD_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    if wasm_artifacts_ready(target_dir, &[crate_name], profile) {
        return;
    }

    build_wasm_canisters(workspace_root, target_dir, &[crate_name], profile, &[]);
}

// Encode one explicit local non-root init payload without any hierarchy
// directory snapshots.
fn standalone_init_args(role: CanisterRole) -> Vec<u8> {
    let root_pid = Fake::principal(1);
    let payload = CanisterInitPayload {
        env: EnvBootstrapArgs {
            prime_root_pid: Some(root_pid),
            subnet_role: Some(SubnetRole::PRIME),
            subnet_pid: Some(Fake::principal(2)),
            root_pid: Some(root_pid),
            canister_role: Some(role),
            parent_pid: Some(root_pid),
        },
        app_directory: AppDirectoryArgs(Vec::new()),
        subnet_directory: SubnetDirectoryArgs(Vec::new()),
    };

    encode_args::<(CanisterInitPayload, Option<Vec<u8>>)>((payload, None))
        .expect("encode standalone init args")
}

fn workspace_root() -> PathBuf {
    workspace_root_for(env!("CARGO_MANIFEST_DIR"))
}
