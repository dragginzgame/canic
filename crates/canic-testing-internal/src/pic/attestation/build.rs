use ic_testkit::artifacts::{read_wasm, test_target_dir as artifact_test_target_dir};
use ic_testkit::pic::{Pic, PicBuilder, PicSerialGuard, acquire_pic_serial_guard};
use std::{
    env,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, Once},
};

use super::super::artifacts::{
    CanicWasmBuildProfile, INTERNAL_TEST_ENDPOINTS_ENV, INTERNAL_TEST_RELEASE_BUILD_ID,
    build_internal_test_wasm_canisters, build_internal_test_wasm_canisters_with_env,
};
use super::fixture::progress;

const ROOT_CANISTER_PACKAGE: &str = "delegation_root_stub";
const EMBEDDED_CANISTER_PACKAGES: [&str; 3] = [
    "delegation_issuer_stub",
    "project_hub_stub",
    "project_instance_stub",
];
const REQUIRE_EMBEDDED_ARTIFACTS_ENV: (&str, &str) = (
    canic_core::role_contract::CANONICAL_BUILD_REQUIRE_EMBEDDED_ARTIFACTS_ENV,
    canic_core::role_contract::CANONICAL_BUILD_MARKER_VALUE,
);
static BUILD_ONCE: Once = Once::new();
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

// Build the test root wasm.
pub(super) fn build_test_root_wasm() -> Vec<u8> {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    read_built_wasm(&test_target_dir(&workspace_root), "delegation_root_stub")
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

// Build the test canisters once for the shared PocketIC attestation fixtures.
fn build_canisters_once(workspace_root: &Path) {
    let _serial_guard = CANISTER_BUILD_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    BUILD_ONCE.call_once_force(|_| {
        let target_dir = test_target_dir(workspace_root);
        progress("building embedded PIC wasm artifacts");
        build_internal_test_wasm_canisters(
            workspace_root,
            &target_dir,
            &EMBEDDED_CANISTER_PACKAGES,
            CanicWasmBuildProfile::Fast,
        );
        progress("building bootstrap wasm_store artifact");
        build_bootstrap_wasm_store(workspace_root, &target_dir);
        progress("building PIC root wasm artifact");
        build_internal_test_wasm_canisters_with_env(
            workspace_root,
            &target_dir,
            &[ROOT_CANISTER_PACKAGE],
            CanicWasmBuildProfile::Fast,
            &[REQUIRE_EMBEDDED_ARTIFACTS_ENV],
        );
        progress("finished PIC wasm build");
    });
}

// Build the root's implicit wasm_store before Cargo runs the root build script.
fn build_bootstrap_wasm_store(workspace_root: &Path, target_dir: &Path) {
    let config_path = workspace_root
        .join("canisters")
        .join("test")
        .join(ROOT_CANISTER_PACKAGE)
        .join("canic.toml");
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .current_dir(workspace_root)
        .env("CARGO_INCREMENTAL", "0")
        .env("CARGO_TARGET_DIR", target_dir)
        .env("ICP_ENVIRONMENT", "local")
        .env(INTERNAL_TEST_ENDPOINTS_ENV.0, INTERNAL_TEST_ENDPOINTS_ENV.1)
        .env(
            INTERNAL_TEST_RELEASE_BUILD_ID.0,
            INTERNAL_TEST_RELEASE_BUILD_ID.1,
        )
        .args([
            "run",
            "-q",
            "--profile",
            "fast",
            "-p",
            "canic-host",
            "--example",
            "build_artifact",
            "--locked",
            "--",
            "wasm_store",
            "fast",
            workspace_root.to_str().expect("workspace root UTF-8"),
            workspace_root.to_str().expect("ICP root UTF-8"),
            config_path.to_str().expect("config path UTF-8"),
        ])
        .output()
        .expect("run bootstrap wasm_store artifact builder");

    assert!(
        output.status.success(),
        "bootstrap wasm_store artifact build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// Read one built fast-profile wasm artifact from an explicit target directory.
fn read_built_wasm(target_dir: &Path, crate_name: &str) -> Vec<u8> {
    read_wasm(
        target_dir,
        crate_name,
        CanicWasmBuildProfile::Fast.target_dir_name(),
    )
}

// Resolve the shared PocketIC wasm target directory.
fn test_target_dir(workspace_root: &Path) -> PathBuf {
    artifact_test_target_dir(workspace_root, "pic-wasm")
}

// Resolve the canic workspace root from the internal test crate manifest dir.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}
