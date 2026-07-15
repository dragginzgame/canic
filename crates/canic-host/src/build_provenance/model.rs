use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    canister_build::{CanisterArtifactBuildOutput, CanisterBuildProfile},
    evidence_envelope::{
        CommandProvenanceV1, EvidenceMessageV1, InputFingerprintV1, InputPathDisplayV1,
    },
};

pub const BUILD_PROVENANCE_SCHEMA_ID: &str = "canic.build_provenance.v1";
pub(super) const WASM_TARGET: &str = "wasm32-unknown-unknown";
pub(super) const DIRTY_SUMMARY_ALGORITHM: &str = "git-status-porcelain-v1-z-sha256";

///
/// BuildProvenanceV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BuildProvenanceV1 {
    pub schema_version: u8,
    pub generated_at: String,
    pub canic_version: String,
    pub command: CommandProvenanceV1,
    pub build_status: BuildProvenanceStatusV1,
    pub source: SourceProvenanceV1,
    pub cargo: CargoProvenanceV1,
    pub artifacts: Vec<ArtifactProvenanceV1>,
    pub transforms: Vec<ArtifactTransformProvenanceV1>,
    pub warnings: Vec<EvidenceMessageV1>,
}

///
/// BuildProvenanceStatusV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildProvenanceStatusV1 {
    Success,
    Failed,
    NotRecorded,
}

///
/// SourceProvenanceV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceProvenanceV1 {
    pub schema_version: u8,
    pub vcs: SourceVcsV1,
    pub revision: Option<String>,
    pub branch: Option<String>,
    pub dirty: Option<bool>,
    pub dirty_policy: SourceDirtyPolicyV1,
    pub dirty_summary_digest: Option<String>,
    pub dirty_summary_algorithm: Option<String>,
}

///
/// SourceVcsV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceVcsV1 {
    Git,
    Unknown,
}

///
/// SourceDirtyPolicyV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceDirtyPolicyV1 {
    Clean,
    DirtyRecorded,
    Unknown,
}

///
/// CargoProvenanceV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CargoProvenanceV1 {
    pub cargo_lock_sha256: Option<String>,
    pub package_manifest_sha256: Option<String>,
    pub package_name: String,
    pub package_manifest: String,
    pub package_metadata_fleet: String,
    pub package_metadata_role: String,
    pub rustc_version: Option<String>,
    pub cargo_version: Option<String>,
    pub target: Option<String>,
    pub profile: String,
    pub features: Vec<String>,
    pub default_features: Option<bool>,
    pub rustflags_digest: Option<String>,
    pub rustflags_digest_algorithm: Option<String>,
    pub cargo_config_fingerprints: Vec<InputFingerprintV1>,
    pub build_script_inputs: BuildScriptInputStateV1,
}

///
/// BuildScriptInputStateV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildScriptInputStateV1 {
    NotRecorded,
    Recorded,
    Unknown,
}

///
/// ArtifactProvenanceV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactProvenanceV1 {
    pub role: String,
    pub fleet: String,
    pub artifact_kind: ArtifactProvenanceKindV1,
    pub path: Option<String>,
    pub path_display: InputPathDisplayV1,
    pub hash_algorithm: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub produced_by: String,
}

///
/// ArtifactProvenanceKindV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactProvenanceKindV1 {
    Wasm,
    WasmGzip,
    Candid,
    Metadata,
    Other,
}

///
/// ArtifactTransformProvenanceV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactTransformProvenanceV1 {
    pub role: String,
    pub transform: ArtifactTransformKindV1,
    pub mode: ArtifactTransformModeV1,
    pub tool: String,
    pub tool_version: Option<String>,
    pub outcome: ArtifactTransformOutcomeV1,
}

/// Artifact-changing operation selected by the host builder.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactTransformKindV1 {
    Shrink,
    CandidMetadata,
}

/// Admission mode for an artifact transform.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactTransformModeV1 {
    Optional,
}

/// Recorded outcome of one artifact transform decision.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactTransformOutcomeV1 {
    Applied,
    ToolUnavailable,
    NotRequested,
}

///
/// BuildProvenanceRequest
///
#[derive(Clone, Debug)]
pub struct BuildProvenanceRequest {
    pub fleet: String,
    pub role: String,
    pub network: String,
    pub build_network: String,
    pub profile: CanisterBuildProfile,
    pub workspace_root: PathBuf,
    pub config_path: PathBuf,
    pub output: CanisterArtifactBuildOutput,
    pub command: CommandProvenanceV1,
    pub generated_at: String,
    pub canic_version: String,
}
