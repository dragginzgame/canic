use std::path::Path;

use crate::bootstrap_store::{BootstrapWasmStoreBuildOutput, build_bootstrap_wasm_store_artifact};

use super::{CanisterBuildProfile, model::CanisterArtifactBuildOutput};

pub(super) fn build_hidden_wasm_store_artifact(
    workspace_root: &Path,
    icp_root: &Path,
    profile: CanisterBuildProfile,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    let output = build_bootstrap_wasm_store_artifact(workspace_root, icp_root, profile)?;
    Ok(map_bootstrap_output(output))
}

// Normalize the bootstrap store builder output to the public canister-artifact shape.
fn map_bootstrap_output(output: BootstrapWasmStoreBuildOutput) -> CanisterArtifactBuildOutput {
    CanisterArtifactBuildOutput {
        artifact_root: output.artifact_root,
        wasm_path: output.wasm_path,
        wasm_gz_path: output.wasm_gz_path,
        did_path: output.did_path,
        manifest_path: None,
    }
}
