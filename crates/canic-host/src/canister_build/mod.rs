mod artifact;
mod candid;
mod context;
mod model;
mod process;
mod wasm_store;

pub use crate::build_profile::CanisterBuildProfile;
pub use artifact::{build_current_workspace_canister_artifact, copy_icp_wasm_output};
pub use context::{
    WorkspaceBuildContext, current_workspace_build_context_once,
    print_current_workspace_build_context_once,
};
pub use model::CanisterArtifactBuildOutput;

#[cfg(test)]
use candid::remove_stale_icp_candid_sidecars;
#[cfg(test)]
use process::parse_parent_process_id;

#[cfg(test)]
mod tests;
