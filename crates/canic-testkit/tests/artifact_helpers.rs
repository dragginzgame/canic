use canic_testkit::artifacts::{
    INTERNAL_TEST_ENDPOINTS_ENV, WasmBuildProfile, test_target_dir, wasm_artifacts_ready,
    wasm_path, workspace_root_for,
};
use std::{fs, path::PathBuf};

// Verify build profiles map to the cargo arguments used by wasm builders.
#[test]
fn wasm_build_profiles_expose_cargo_args_and_target_dirs() {
    assert_eq!(WasmBuildProfile::Debug.cargo_args(), &[] as &[&str]);
    assert_eq!(WasmBuildProfile::Fast.cargo_args(), &["--profile", "fast"]);
    assert_eq!(WasmBuildProfile::Release.cargo_args(), &["--release"]);

    assert_eq!(WasmBuildProfile::Debug.target_dir_name(), "debug");
    assert_eq!(WasmBuildProfile::Fast.target_dir_name(), "fast");
    assert_eq!(WasmBuildProfile::Release.target_dir_name(), "release");

    assert_eq!(WasmBuildProfile::Fast.canic_wasm_profile_value(), "fast");
}

// Verify wasm artifact paths stay aligned with Cargo wasm target layout.
#[test]
fn wasm_path_uses_profile_target_directory() {
    let target_dir = PathBuf::from("/tmp/canic-target");

    assert_eq!(
        wasm_path(&target_dir, "runtime_probe", WasmBuildProfile::Fast),
        target_dir
            .join("wasm32-unknown-unknown")
            .join("fast")
            .join("runtime_probe.wasm")
    );
}

// Verify readiness checks require every requested canister artifact.
#[test]
fn wasm_artifacts_ready_requires_all_artifacts() {
    let root = unique_temp_dir("canic-testkit-artifacts");
    let target_dir = root.join("target");
    let first = wasm_path(&target_dir, "alpha", WasmBuildProfile::Debug);
    let second = wasm_path(&target_dir, "beta", WasmBuildProfile::Debug);

    fs::create_dir_all(first.parent().expect("wasm parent")).expect("create wasm dir");
    fs::write(&first, b"alpha").expect("write first wasm");

    assert!(!wasm_artifacts_ready(
        &target_dir,
        &["alpha", "beta"],
        WasmBuildProfile::Debug
    ));

    fs::write(&second, b"beta").expect("write second wasm");
    assert!(wasm_artifacts_ready(
        &target_dir,
        &["alpha", "beta"],
        WasmBuildProfile::Debug
    ));

    fs::remove_dir_all(root).expect("clean temp dir");
}

// Verify workspace and target helpers derive stable host-side paths.
#[test]
fn workspace_helpers_resolve_expected_paths() {
    let manifest_dir = "/workspace/crates/canic-testkit";
    let workspace_root = workspace_root_for(manifest_dir);

    assert_eq!(workspace_root, PathBuf::from("/workspace"));
    assert_eq!(
        test_target_dir(&workspace_root, "pic-wasm"),
        PathBuf::from("/workspace/target/pic-wasm")
    );
}

// Verify the internal-test endpoint env pair remains stable for builders.
#[test]
fn internal_test_endpoint_env_pair_is_stable() {
    assert_eq!(
        INTERNAL_TEST_ENDPOINTS_ENV,
        ("CANIC_INTERNAL_TEST_ENDPOINTS", "1")
    );
}

// Build a unique temp directory path for filesystem-only artifact tests.
fn unique_temp_dir(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).expect("remove stale temp dir");
    }
    root
}
