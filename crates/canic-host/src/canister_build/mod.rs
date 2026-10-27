mod artifact;
mod batch;
pub(crate) mod cache;
mod candid;
mod candid_cache;
pub(crate) mod compiled;
mod context;
mod infrastructure;
mod metrics;
mod model;
mod output_roots;
pub mod release_binding;
mod reuse;
mod wire_schema;

pub(crate) use reuse::retain_cargo_include_exports;

pub use crate::{
    artifact_io::{
        WireArtifactError, WireBindings, deterministic_gzip_bytes, generate_wire_bindings,
        validate_wasm_endpoints, write_gzip_artifact,
    },
    build_profile::CanisterBuildProfile,
};
pub use artifact::{
    CanisterArtifactBuilder, build_workspace_canister_artifact,
    build_workspace_canister_artifact_with_options, build_workspace_configured_canister_artifacts,
    copy_icp_wasm_output, prepare_workspace_infrastructure_packages,
};
pub use cache::canister_build_target_root;
pub use candid::extract_candid_bytes;
pub use context::WorkspaceBuildContext;
pub use metrics::read_wasm_artifact_metrics;
pub use model::{
    AppCanisterArtifactBuildOutput, ArtifactTransformKind, ArtifactTransformOutcome,
    ArtifactTransformOutput, CanisterArtifactBuildOptions, CanisterArtifactBuildOutput,
    ConfiguredCanisterArtifactBuildOutput, NativeSchemaBuildRecipe,
    TimedCanisterArtifactBuildOutput, WasmArtifactMetrics, WasmTransformMetrics,
};
pub use wire_schema::{
    RetainedWireSchema, WireSchemaBuildError, build_native_wire_contract, build_native_wire_schema,
    build_workspace_wire_schema, produce_wire_schema,
};

#[cfg(test)]
use candid::remove_stale_icp_candid_sidecars;

#[cfg(test)]
mod tests;

pub use reuse::{
    BuildLockInspection, BuildLockOwner, BuildLockPhase, BuildLockWait, BuildProcessActivity,
    BuildProcessIdentity, BuildProcessKind, BuildProcessObservation, BuildProcessVisibility,
    BuildReuseError, BuildReuseProgress, CompleteBuildReuse, KernelBuildLock, ReusedCompleteBuild,
    inspect_build_lock,
};

/// Bounds for Cargo configuration observation in Canic artifact builds and proofs.
/// Admission is shared with framework fixtures; IC Build owns traversal and refusal.
pub const CARGO_CONFIGURATION_LIMITS: ic_build::cargo::environment::ConfigurationLimits =
    ic_build::cargo::environment::ConfigurationLimits {
        file_bytes: 1024 * 1024,
        total_bytes: 16 * 1024 * 1024,
        files: 128,
        depth: 32,
    };
