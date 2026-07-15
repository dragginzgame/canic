use std::path::PathBuf;

pub(super) const ROOT_ROLE: &str = "root";
pub(super) const WASM_STORE_ROLE: &str = "wasm_store";
pub(super) const LOCAL_ARTIFACT_ROOT_RELATIVE: &str = ".icp/local/canisters";
pub(super) const WASM_TARGET: &str = "wasm32-unknown-unknown";

/// Exact package and output paths admitted before one role build starts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanisterArtifactBuildSpec {
    pub(crate) role: String,
    pub(crate) package_name: String,
    pub(crate) package_manifest_path: PathBuf,
    pub(crate) artifact_root: PathBuf,
    pub(crate) wasm_path: PathBuf,
    pub(crate) wasm_gz_path: PathBuf,
    pub(crate) did_path: PathBuf,
}

///
/// CanisterArtifactBuildOutput
///

#[derive(Clone, Debug)]
pub struct CanisterArtifactBuildOutput {
    pub artifact_root: PathBuf,
    pub wasm_path: PathBuf,
    pub wasm_gz_path: PathBuf,
    pub did_path: PathBuf,
    pub transforms: Vec<ArtifactTransformOutput>,
}

/// One optional artifact-changing tool invocation owned by the host builder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactTransformOutput {
    pub role: String,
    pub transform: ArtifactTransformKind,
    pub mode: ArtifactTransformMode,
    pub tool: String,
    pub tool_version: Option<String>,
    pub outcome: ArtifactTransformOutcome,
}

impl ArtifactTransformOutput {
    pub(crate) fn not_requested(role: &str, transform: ArtifactTransformKind) -> Self {
        Self {
            role: role.to_string(),
            transform,
            mode: ArtifactTransformMode::Optional,
            tool: "ic-wasm".to_string(),
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

/// Admission mode for an artifact transform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactTransformMode {
    Optional,
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
