mod artifact;
pub(crate) mod cache;
mod candid;
mod context;
mod model;
mod process;

pub use crate::build_profile::CanisterBuildProfile;
pub use artifact::{
    build_workspace_canister_artifact, build_workspace_canister_artifact_with_options,
    build_workspace_configured_canister_artifacts, copy_icp_wasm_output,
};
pub(crate) use candid::extract_candid_bytes;
pub use context::{
    WorkspaceBuildContext, print_workspace_build_context_once, workspace_build_context_once,
};
pub use model::{
    ArtifactTransformKind, ArtifactTransformOutcome, ArtifactTransformOutput,
    CanisterArtifactBuildOptions, CanisterArtifactBuildOutput,
    ConfiguredCanisterArtifactBuildOutput, WasmArtifactMetrics, WasmTransformMetrics,
};

#[cfg(test)]
use candid::remove_stale_icp_candid_sidecars;
#[cfg(test)]
use process::parse_parent_process_id;

#[cfg(test)]
mod tests;
