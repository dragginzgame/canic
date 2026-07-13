mod artifact;
pub(crate) mod cache;
mod candid;
mod context;
mod model;
mod process;
mod wasm_store;

pub use crate::build_profile::CanisterBuildProfile;
pub use artifact::{build_workspace_canister_artifact, copy_icp_wasm_output};
pub(crate) use artifact::{
    build_workspace_canister_artifact_from_spec, resolve_canister_artifact_build_spec,
};
pub use context::{
    WorkspaceBuildContext, print_workspace_build_context_once, workspace_build_context_once,
};
pub use model::CanisterArtifactBuildOutput;
pub(crate) use model::{CanisterArtifactBuildSpec, CurrentCanisterArtifactBuildOutput};

#[cfg(test)]
use candid::remove_stale_icp_candid_sidecars;
#[cfg(test)]
use process::parse_parent_process_id;

#[cfg(test)]
mod tests;
