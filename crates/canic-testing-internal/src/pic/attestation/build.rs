use canic_testkit::artifacts::{
    WasmBuildProfile, build_internal_test_wasm_canisters,
    build_internal_test_wasm_canisters_with_env,
};
use canic_testkit::pic::{Pic, PicBuilder, PicSerialGuard, acquire_pic_serial_guard};
use std::{
    fs,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::{Mutex, Once},
};

use super::fixture::progress;

const CANISTER_PACKAGES: [&str; 1] = ["delegation_root_stub"];
static BUILD_ONCE: Once = Once::new();
static BUILD_WITHOUT_TEST_MATERIAL_ONCE: Once = Once::new();
static CANISTER_BUILD_SERIAL: Mutex<()> = Mutex::new(());

///
/// SerialPic
///

pub(super) struct SerialPic {
    pub(super) pic: Pic,
    _serial_guard: PicSerialGuard,
}

impl Deref for SerialPic {
    type Target = Pic;

    fn deref(&self) -> &Self::Target {
        &self.pic
    }
}

impl DerefMut for SerialPic {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.pic
    }
}

impl SerialPic {
    // Release the serialization guard and return the owned PocketIC instance.
    pub(super) fn into_pic(self) -> Pic {
        self.pic
    }
}

// Build the test root wasm with delegation-material test cfg enabled.
pub(super) fn build_test_root_wasm() -> Vec<u8> {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    read_wasm(&workspace_root, "delegation_root_stub")
}

// Build the normal root wasm without delegation-material test cfg enabled.
pub(super) fn build_normal_root_wasm() -> Vec<u8> {
    let workspace_root = workspace_root();
    build_canisters_without_test_material_once(&workspace_root);
    read_wasm_from_target(
        &test_target_dir_without_test_material(&workspace_root),
        "delegation_root_stub",
    )
}

// Serialize full PocketIC usage to avoid concurrent server races across tests.
pub(super) fn build_pic() -> SerialPic {
    progress("acquiring PocketIC serial guard");
    let serial_guard = acquire_pic_serial_guard();
    progress("starting serialized PocketIC instance");
    let pic = PicBuilder::new()
        .with_ii_subnet()
        .with_application_subnet()
        .build();
    progress("serialized PocketIC instance ready");

    SerialPic {
        pic,
        _serial_guard: serial_guard,
    }
}

// Build the test canisters with delegation-material test cfg enabled.
fn build_canisters_once(workspace_root: &Path) {
    let _serial_guard = CANISTER_BUILD_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    BUILD_ONCE.call_once_force(|_| {
        let target_dir = test_target_dir(workspace_root);
        progress("building PIC wasm artifacts with test delegation material");
        build_internal_test_wasm_canisters_with_env(
            workspace_root,
            &target_dir,
            &CANISTER_PACKAGES,
            WasmBuildProfile::Fast,
            &[("CANIC_TEST_DELEGATION_MATERIAL", "1")],
        );
        progress("finished PIC wasm build with test delegation material");
    });
}

// Build the same test canisters without delegation-material test cfg enabled.
fn build_canisters_without_test_material_once(workspace_root: &Path) {
    let _serial_guard = CANISTER_BUILD_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    BUILD_WITHOUT_TEST_MATERIAL_ONCE.call_once_force(|_| {
        let target_dir = test_target_dir_without_test_material(workspace_root);
        progress("building PIC wasm artifacts without test delegation material");
        build_internal_test_wasm_canisters(
            workspace_root,
            &target_dir,
            &CANISTER_PACKAGES,
            WasmBuildProfile::Fast,
        );
        progress("finished PIC wasm build without test delegation material");
    });
}

// Read one built wasm artifact from the shared test target directory.
fn read_wasm(workspace_root: &Path, crate_name: &str) -> Vec<u8> {
    let wasm_path = wasm_path(workspace_root, crate_name);
    fs::read(&wasm_path).unwrap_or_else(|err| panic!("failed to read {crate_name} wasm: {err}"))
}

// Read one built wasm artifact from an explicit target directory.
fn read_wasm_from_target(target_dir: &Path, crate_name: &str) -> Vec<u8> {
    let wasm_path = wasm_path_from_target(target_dir, crate_name);
    fs::read(&wasm_path).unwrap_or_else(|err| panic!("failed to read {crate_name} wasm: {err}"))
}

// Resolve one crate wasm path under the shared fast wasm target layout.
fn wasm_path(workspace_root: &Path, crate_name: &str) -> PathBuf {
    let target_dir = test_target_dir(workspace_root);

    wasm_path_from_target(&target_dir, crate_name)
}

// Resolve one crate wasm path under a caller-provided target directory.
fn wasm_path_from_target(target_dir: &Path, crate_name: &str) -> PathBuf {
    target_dir
        .join("wasm32-unknown-unknown")
        .join("fast")
        .join(format!("{crate_name}.wasm"))
}

// Resolve the shared test-material PocketIC wasm target directory.
fn test_target_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join("target").join("pic-wasm")
}

// Resolve the normal-build PocketIC wasm target directory.
fn test_target_dir_without_test_material(workspace_root: &Path) -> PathBuf {
    workspace_root
        .join("target")
        .join("pic-wasm-no-test-material")
}

// Resolve the canic workspace root from the internal test crate manifest dir.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}
