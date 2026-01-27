use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    canic::build_root!("canic.toml");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let target_dir = out_dir.join("signer_wasm_target");
    fs::create_dir_all(&target_dir).expect("create signer wasm target dir");

    let mut cmd = Command::new("cargo");
    cmd.current_dir(workspace_root);
    cmd.env("CARGO_TARGET_DIR", &target_dir);
    cmd.args([
        "build",
        "--release",
        "--target",
        "wasm32-unknown-unknown",
        "-p",
        "delegation_signer_stub",
    ]);
    let output = cmd.output().expect("build delegation_signer_stub");
    assert!(
        output.status.success(),
        "delegation_signer_stub build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let wasm_path = target_dir
        .join("wasm32-unknown-unknown")
        .join("release")
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
