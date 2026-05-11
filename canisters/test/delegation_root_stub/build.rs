use std::{env, fs, path::PathBuf, process::Command};

// Build the root stub, its bootstrap wasm_store artifact, and the embedded test canisters.
fn main() {
    configure_cfg();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let workspace_root = discover_workspace_root(&manifest_dir);
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let outer_target_dir =
        env::var("CARGO_TARGET_DIR").map_or_else(|_| workspace_root.join("target"), PathBuf::from);

    build_bootstrap_wasm_store(&workspace_root, &manifest_dir, &outer_target_dir);
    canic::build_root!("canic.toml");
    build_embedded_test_canisters(&workspace_root, &outer_target_dir, &out_dir);
    emit_rerun_inputs(&workspace_root);
}

// Register the test-only cfg passthroughs used by this build script and nested builds.
fn configure_cfg() {
    println!("cargo:rerun-if-env-changed=CANIC_TEST_DELEGATION_MATERIAL");
    println!("cargo:rerun-if-env-changed=CARGO_TARGET_DIR");
    println!("cargo:rustc-check-cfg=cfg(canic_test_delegation_material)");
    if env::var_os("CANIC_TEST_DELEGATION_MATERIAL").is_some() {
        println!("cargo:rustc-cfg=canic_test_delegation_material");
    }
}

// Build the implicit bootstrap wasm_store artifact so `build_root!` can embed it.
fn build_bootstrap_wasm_store(
    workspace_root: &std::path::Path,
    manifest_dir: &std::path::Path,
    outer_target_dir: &std::path::Path,
) {
    let bootstrap_target_dir = outer_target_dir.join("delegation_root_stub_bootstrap_wasm_store");
    fs::create_dir_all(&bootstrap_target_dir).expect("create bootstrap wasm_store target dir");

    let mut bootstrap_cmd = cargo_command();
    bootstrap_cmd.current_dir(workspace_root);
    bootstrap_cmd.env("CANIC_WORKSPACE_ROOT", workspace_root);
    bootstrap_cmd.env("CANIC_CONFIG_PATH", manifest_dir.join("canic.toml"));
    bootstrap_cmd.env("CARGO_TARGET_DIR", &bootstrap_target_dir);
    bootstrap_cmd.env("ICP_ENVIRONMENT", "local");
    bootstrap_cmd.env("CANIC_WASM_PROFILE", "fast");
    bootstrap_cmd.args([
        "run",
        "-q",
        "-p",
        "canic-host",
        "--example",
        "build_artifact",
        "--",
        "wasm_store",
    ]);

    let bootstrap_output = bootstrap_cmd
        .output()
        .expect("build bootstrap wasm_store artifact");
    assert!(
        bootstrap_output.status.success(),
        "bootstrap wasm_store artifact build failed: {}",
        String::from_utf8_lossy(&bootstrap_output.stderr)
    );
}

// Build the nested wasm canisters that the root stub embeds into its release set.
fn build_embedded_test_canisters(
    workspace_root: &std::path::Path,
    outer_target_dir: &std::path::Path,
    out_dir: &std::path::Path,
) {
    let target_dir = outer_target_dir.join("delegation_root_stub_embedded_wasm");
    fs::create_dir_all(&target_dir).expect("create embedded wasm target dir");

    let mut cmd = cargo_command();
    cmd.current_dir(workspace_root);
    cmd.env("CARGO_TARGET_DIR", &target_dir);
    if let Some(flag) = env::var_os("CANIC_TEST_DELEGATION_MATERIAL") {
        cmd.env("CANIC_TEST_DELEGATION_MATERIAL", flag);
    }
    cmd.args([
        "build",
        "--profile",
        "fast",
        "--target",
        "wasm32-unknown-unknown",
        "-p",
        "delegation_signer_stub",
        "-p",
        "project_hub_stub",
        "-p",
        "project_instance_stub",
    ]);

    let output = cmd.output().expect("build embedded root test canisters");
    assert!(
        output.status.success(),
        "embedded root test canister build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    copy_embedded_wasm(&target_dir, out_dir, "delegation_signer_stub");
    copy_embedded_wasm(&target_dir, out_dir, "project_hub_stub");
    copy_embedded_wasm(&target_dir, out_dir, "project_instance_stub");
}

// Copy one compiled wasm artifact into this build script's OUT_DIR for embedding.
fn copy_embedded_wasm(target_dir: &std::path::Path, out_dir: &std::path::Path, crate_name: &str) {
    let wasm_path = target_dir
        .join("wasm32-unknown-unknown")
        .join("fast")
        .join(format!("{crate_name}.wasm"));
    let out_wasm = out_dir.join(format!("{crate_name}.wasm"));
    fs::copy(&wasm_path, &out_wasm).unwrap_or_else(|err| {
        panic!(
            "copy {crate_name} wasm from {} failed: {err}",
            wasm_path.display()
        )
    });
}

// Emit rerun markers for the nested test canister sources that feed this release set.
fn emit_rerun_inputs(workspace_root: &std::path::Path) {
    println!("cargo:rerun-if-changed=build.rs");
    emit_canister_rerun_inputs(workspace_root, "delegation_signer_stub");
    emit_canister_rerun_inputs(workspace_root, "project_hub_stub");
    emit_canister_rerun_inputs(workspace_root, "project_instance_stub");
}

// Emit rerun markers for one nested test canister package.
fn emit_canister_rerun_inputs(workspace_root: &std::path::Path, package: &str) {
    let package_root = workspace_root.join("fleets/test").join(package);

    for relative in ["Cargo.toml", "src/lib.rs", "canic.toml"] {
        println!(
            "cargo:rerun-if-changed={}",
            package_root.join(relative).display()
        );
    }
}

// Walk up from the current manifest until the workspace Cargo.toml is found.
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

fn cargo_command() -> Command {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);

    if let Some(toolchain) = env::var_os("RUSTUP_TOOLCHAIN") {
        command.env("RUSTUP_TOOLCHAIN", toolchain);
    }

    command
}
