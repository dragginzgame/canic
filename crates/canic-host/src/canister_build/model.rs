use std::path::PathBuf;

pub(super) const FLEET_COORDINATOR_ROLE: &str = "fleet_coordinator";
pub(super) const WASM_STORE_ROLE: &str = "wasm_store";
pub(super) const WASM_TARGET: &str = "wasm32-unknown-unknown";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CanisterArtifactSource {
    DeclaredRole,
    FleetCoordinator,
    WasmStore,
}

impl CanisterArtifactSource {
    #[must_use]
    pub(super) fn for_role(role: &str) -> Self {
        match role {
            FLEET_COORDINATOR_ROLE => Self::FleetCoordinator,
            WASM_STORE_ROLE => Self::WasmStore,
            _ => Self::DeclaredRole,
        }
    }
}

/// Exact package and output paths admitted before one role build starts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanisterArtifactBuildSpec {
    pub(crate) role: String,
    pub(crate) package_name: String,
    pub(crate) package_version: String,
    pub(crate) package_manifest_path: PathBuf,
    pub(crate) cargo_workspace_root: PathBuf,
    pub(crate) artifact_root: PathBuf,
    pub(crate) wasm_path: PathBuf,
    pub(crate) wasm_gz_path: PathBuf,
    pub(crate) did_path: PathBuf,
}

///
/// CanisterArtifactBuildOutput
///
/// Canonical package identity and artifact outputs produced by one admitted build.
/// Owned by the host build boundary and consumed by provenance and install planning.
///

#[derive(Clone, Debug)]
pub struct CanisterArtifactBuildOutput {
    pub package_name: String,
    pub package_version: String,
    pub artifact_root: PathBuf,
    pub wasm_path: PathBuf,
    pub wasm_gz_path: PathBuf,
    pub did_path: PathBuf,
    pub transforms: Vec<ArtifactTransformOutput>,
}

/// One configured role and its successfully materialized artifact outputs.
#[derive(Clone, Debug)]
pub struct ConfiguredCanisterArtifactBuildOutput {
    pub role: String,
    pub output: CanisterArtifactBuildOutput,
}

/// One optional artifact-changing tool invocation owned by the host builder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactTransformOutput {
    pub transform: ArtifactTransformKind,
    pub tool_version: Option<String>,
    pub outcome: ArtifactTransformOutcome,
}

impl ArtifactTransformOutput {
    pub(crate) const fn not_requested(transform: ArtifactTransformKind) -> Self {
        Self {
            transform,
            tool_version: None,
            outcome: ArtifactTransformOutcome::NotRequested,
        }
    }
}

/// Optional `ic-wasm` operation that can change one emitted Wasm artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactTransformKind {
    Shrink,
    CandidMetadata,
}

/// Result of one optional artifact transform decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactTransformOutcome {
    Applied,
    ToolUnavailable,
    NotRequested,
}

/// One successful role output from the current complete-build invocation.
#[derive(Clone, Debug)]
pub struct CurrentCanisterArtifactBuildOutput {
    pub(crate) role: String,
    pub(crate) output: CanisterArtifactBuildOutput,
}
