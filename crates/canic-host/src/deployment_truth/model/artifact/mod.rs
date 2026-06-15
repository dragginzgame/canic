use super::inventory::DeploymentObservationGapV1;
use serde::{Deserialize, Serialize};

///
/// RoleArtifactManifestV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleArtifactManifestV1 {
    pub schema_version: u32,
    pub manifest_id: String,
    pub network: String,
    pub artifact_root: Option<String>,
    pub role_artifacts: Vec<RoleArtifactV1>,
    pub unresolved_artifacts: Vec<DeploymentObservationGapV1>,
}

///
/// RoleArtifactV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleArtifactV1 {
    pub role: String,
    pub source: ArtifactSourceV1,
    pub build_profile: String,
    pub wasm_path: Option<String>,
    pub wasm_gz_path: Option<String>,
    pub wasm_gz_size_bytes: Option<u64>,
    pub wasm_sha256: Option<String>,
    pub wasm_gz_sha256: Option<String>,
    pub wasm_gz_sha256_source: Option<ArtifactDigestSourceV1>,
    pub observed_wasm_gz_file_sha256: Option<String>,
    pub observed_wasm_gz_file_sha256_source: Option<ArtifactDigestSourceV1>,
    pub installed_module_hash: Option<String>,
    pub candid_path: Option<String>,
    pub candid_sha256: Option<String>,
    pub raw_config_sha256: Option<String>,
    pub canonical_embedded_config_sha256: Option<String>,
    pub embedded_topology_sha256: Option<String>,
    pub builder_version: Option<String>,
    pub rust_toolchain: Option<String>,
    pub package_version: Option<String>,
}

///
/// ArtifactDigestSourceV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ArtifactDigestSourceV1 {
    ReleaseSetManifest,
    ObservedFileDigest,
    InstalledModuleHash,
}

///
/// ArtifactSourceV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ArtifactSourceV1 {
    LocalBuild,
    ReleaseSet,
    WasmStore,
    External,
    Unknown,
}

///
/// ObservedArtifactV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObservedArtifactV1 {
    pub role: String,
    pub artifact_path: String,
    pub file_sha256: Option<String>,
    pub file_sha256_source: Option<ArtifactDigestSourceV1>,
    pub payload_sha256: Option<String>,
    pub payload_size_bytes: Option<u64>,
    pub source: ArtifactSourceV1,
}
