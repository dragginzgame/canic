use crate::bootstrap_store::{BootstrapWasmStoreBuildOutput, build_bootstrap_wasm_store_artifact};

use super::{WorkspaceBuildContext, model::CanisterArtifactBuildOutput};

pub(super) fn build_hidden_wasm_store_artifact(
    context: &WorkspaceBuildContext,
) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
    let output = build_bootstrap_wasm_store_artifact(context)?;
    Ok(map_bootstrap_output(output))
}

// Normalize the bootstrap store builder output to the public canister-artifact shape.
fn map_bootstrap_output(output: BootstrapWasmStoreBuildOutput) -> CanisterArtifactBuildOutput {
    CanisterArtifactBuildOutput {
        artifact_root: output.artifact_root,
        wasm_path: output.wasm_path,
        wasm_gz_path: output.wasm_gz_path,
        did_path: output.did_path,
        transforms: output.transforms,
    }
}
