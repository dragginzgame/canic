mod artifact;
mod batch;
pub(crate) mod cache;
mod candid;
mod candid_cache;
pub(crate) mod compiled;
mod compiler_cache;
mod context;
mod metrics;
mod model;
mod output_roots;
mod process;
mod reuse;

pub use crate::{artifact_io::validate_wasm_candid_endpoints, build_profile::CanisterBuildProfile};
pub use artifact::{
    CanisterArtifactBuilder, build_workspace_canister_artifact,
    build_workspace_canister_artifact_with_options, build_workspace_configured_canister_artifacts,
    copy_icp_wasm_output, prepare_workspace_infrastructure_packages,
};
pub use cache::canister_build_target_root;
pub use candid::extract_candid_bytes;
pub use context::{
    WorkspaceBuildContext, print_workspace_build_context_once, workspace_build_context_once,
};
pub use metrics::read_wasm_artifact_metrics;
pub use model::{
    AppCanisterArtifactBuildOutput, ArtifactTransformKind, ArtifactTransformOutcome,
    ArtifactTransformOutput, CanisterArtifactBuildOptions, CanisterArtifactBuildOutput,
    ConfiguredCanisterArtifactBuildOutput, TimedCanisterArtifactBuildOutput, WasmArtifactMetrics,
    WasmTransformMetrics,
};

#[cfg(test)]
use candid::remove_stale_icp_candid_sidecars;
#[cfg(test)]
use process::parse_parent_process_id;

#[cfg(test)]
mod tests;

pub use reuse::{
    BuildLockInspection, BuildLockOwner, BuildLockPhase, BuildLockWait, BuildProcessActivity,
    BuildProcessIdentity, BuildProcessKind, BuildProcessObservation, BuildProcessVisibility,
    BuildReuseError, BuildReuseProgress, CompleteBuildReuse, KernelBuildLock, ReusedCompleteBuild,
    inspect_build_lock,
};
