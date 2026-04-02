use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    // Rebuild when test-material cfg flag changes to avoid stale cfg mismatches.
    println!("cargo:rerun-if-env-changed=CANIC_TEST_DELEGATION_MATERIAL");
    println!("cargo:rerun-if-env-changed=CARGO_TARGET_DIR");

    // Register and forward the test-only delegation-material cfg for this
    // canister and the nested signer build that this script triggers.
    println!("cargo:rustc-check-cfg=cfg(canic_test_delegation_material)");
    if env::var_os("CANIC_TEST_DELEGATION_MATERIAL").is_some() {
        println!("cargo:rustc-cfg=canic_test_delegation_material");
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let workspace_root = discover_workspace_root(&manifest_dir);
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let outer_target_dir =
        env::var("CARGO_TARGET_DIR").map_or_else(|_| workspace_root.join("target"), PathBuf::from);

    // Build the hidden bootstrap store artifact first so `build_root!` can
    // embed a registered bootstrap module even on plain cargo builds.
    let bootstrap_target_dir = outer_target_dir.join("delegation_root_stub_bootstrap_wasm_store");
    fs::create_dir_all(&bootstrap_target_dir).expect("create bootstrap wasm_store target dir");
    let mut bootstrap_cmd = Command::new("cargo");
    bootstrap_cmd.current_dir(&workspace_root);
    bootstrap_cmd.env("CANIC_WORKSPACE_ROOT", &workspace_root);
    bootstrap_cmd.env("CANIC_CONFIG_PATH", manifest_dir.join("canic.toml"));
    bootstrap_cmd.env("CARGO_TARGET_DIR", &bootstrap_target_dir);
    bootstrap_cmd.env("DFX_NETWORK", "local");
    bootstrap_cmd.env("RELEASE", "1");
    bootstrap_cmd.args([
        "run",
        "-q",
        "-p",
        "canic-installer",
        "--bin",
        "canic-build-wasm-store-artifact",
        "--",
    ]);
    let bootstrap_output = bootstrap_cmd
        .output()
        .expect("build bootstrap wasm_store artifact");
    assert!(
        bootstrap_output.status.success(),
        "bootstrap wasm_store artifact build failed: {}",
        String::from_utf8_lossy(&bootstrap_output.stderr)
    );

    canic::build_root!("canic.toml");

    let target_dir = outer_target_dir.join("delegation_root_stub_embedded_wasm");
    fs::create_dir_all(&target_dir).expect("create embedded wasm target dir");

    let mut cmd = Command::new("cargo");
    cmd.current_dir(&workspace_root);
    cmd.env("CARGO_TARGET_DIR", &target_dir);
    if let Some(flag) = env::var_os("CANIC_TEST_DELEGATION_MATERIAL") {
        cmd.env("CANIC_TEST_DELEGATION_MATERIAL", flag);
    }
    cmd.args([
        "build",
        "--profile",
        "wasm-release",
        "--target",
        "wasm32-unknown-unknown",
        "-p",
        "delegation_signer_stub",
    ]);
    let output = cmd.output().expect("build embedded root test canisters");
    assert!(
        output.status.success(),
        "embedded root test canister build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let wasm_path = target_dir
        .join("wasm32-unknown-unknown")
        .join("wasm-release")
        .join("delegation_signer_stub.wasm");

    let out_wasm = out_dir.join("delegation_signer_stub.wasm");
    fs::copy(&wasm_path, &out_wasm).expect("copy signer wasm");

    println!("cargo:rerun-if-changed=build.rs");
    println!(
        "cargo:rerun-if-changed={}",
        workspace_root
            .join("crates/canic-core/test-canisters/delegation_signer_stub/Cargo.toml")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace_root
            .join("crates/canic-core/test-canisters/delegation_signer_stub/src/lib.rs")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace_root
            .join("crates/canic-core/test-canisters/delegation_signer_stub/canic.toml")
            .display()
    );
}

fn discover_workspace_root(manifest_dir: &std::path::Path) -> PathBuf {
    for candidate in manifest_dir.ancestors() {
        let cargo_toml = candidate.join("Cargo.toml");
        if !cargo_toml.is_file() {
            continue;
        }

        let cargo_toml_text = fs::read_to_string(&cargo_toml)
            .unwrap_or_else(|err| panic!("read {} failed: {err}", cargo_toml.display()));

        if cargo_toml_text.contains("[workspace]") {
            return candidate.to_path_buf();
        }
    }

    panic!(
        "unable to discover workspace root from {}; expected an ancestor Cargo.toml with [workspace]",
        manifest_dir.display()
    );
}
