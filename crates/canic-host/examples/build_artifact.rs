use canic_host::canister_build::{
    CanisterArtifactBuildOutput, CanisterBuildProfile, build_current_workspace_canister_artifact,
    print_current_workspace_build_context_once,
};
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(canister_name) = std::env::args().nth(1) else {
        return Err(
            "usage: cargo run -p canic-host --example build_artifact -- <canister-name>".into(),
        );
    };

    let profile = CanisterBuildProfile::current();
    print_current_workspace_build_context_once(profile)?;
    let output = build_current_workspace_canister_artifact(&canister_name, profile)?;
    copy_icp_wasm_output(&canister_name, &output)?;
    println!("{}", output.wasm_gz_path.display());
    Ok(())
}

fn copy_icp_wasm_output(
    canister_name: &str,
    output: &CanisterArtifactBuildOutput,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::var_os("ICP_WASM_OUTPUT_PATH").map(PathBuf::from) else {
        return Ok(());
    };

    if !output.wasm_path.is_file() {
        return Err(format!(
            "missing ICP wasm output source for {canister_name}: {}",
            output.wasm_path.display()
        )
        .into());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(&output.wasm_path, Path::new(&path))?;
    Ok(())
}
