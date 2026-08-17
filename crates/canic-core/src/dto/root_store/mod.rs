//! Passive contracts for one Fleet Subnet Root's initial local Wasm Store bootstrap.

use crate::ids::{
    CanisterRole, ComponentSpecId, ComponentTopologyDigest, FleetSubnetRootReleaseSet,
    ReleaseBuildId,
};
use crate::role_contract::ProtocolProfileDigest;
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

/// Maximum canonical root release-set manifest bytes admitted at the runtime boundary.
pub const ROOT_STORE_RELEASE_SET_MANIFEST_MAX_BYTES: u64 = 16 * 1_024 * 1_024;
/// Stable template prefix for a chunk-staged canonical root release-set manifest.
pub const ROOT_STORE_RELEASE_SET_TEMPLATE_PREFIX: &str = "root-release-set:";
/// Stable template prefix for one chunk-staged application artifact.
pub const ROOT_STORE_ARTIFACT_TEMPLATE_PREFIX: &str = "component:";

///
/// RootStoreArtifact
///
/// Qualified immutable application artifact carried by one root release-set entry.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootStoreArtifact {
    pub role: CanisterRole,
    pub package: String,
    pub release_build_id: ReleaseBuildId,
    pub wasm_relative_path: String,
    pub wasm_size_bytes: u64,
    pub wasm_sha256_hex: String,
    pub wasm_gz_relative_path: String,
    pub wasm_gz_size_bytes: u64,
    pub wasm_gz_sha256_hex: String,
    pub candid_sha256: [u8; 32],
    pub protocol_profile_digest: ProtocolProfileDigest,
}

///
/// RootStoreReleaseSetEntryKind
///
/// Structural use of one artifact within one exact Component Spec closure.
///

#[derive(
    CandidType, Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
pub enum RootStoreReleaseSetEntryKind {
    #[serde(rename = "component")]
    Component,

    #[serde(rename = "component_child")]
    ComponentChild,
}

///
/// RootStoreReleaseSetEntry
///
/// One Spec-scoped artifact authorization from the canonical root release-set manifest.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootStoreReleaseSetEntry {
    pub component_spec: ComponentSpecId,
    pub kind: RootStoreReleaseSetEntryKind,
    pub artifact: RootStoreArtifact,
}

///
/// RootStoreReleaseSetManifest
///
/// Runtime decoding shape for the host's exact canonical root release-set JSON bytes.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RootStoreReleaseSetManifest {
    pub release_build_id: ReleaseBuildId,
    pub component_topology_digest: ComponentTopologyDigest,
    pub entries: Vec<RootStoreReleaseSetEntry>,
}

///
/// RootStoreBootstrapRequest
///
/// Small bootstrap request referencing an already chunk-staged canonical manifest.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootStoreBootstrapRequest {
    pub operation_id: [u8; 32],
    pub manifest_payload_size_bytes: u64,
}

///
/// RootStoreCatalogEntry
///
/// Exact release-set identity retained after live root-local Store verification.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RootStoreCatalogEntry {
    pub role: CanisterRole,
    /// SHA-256 of the raw Wasm qualified by the release set.
    pub raw_module_hash: [u8; 32],
    /// SHA-256 of the exact canonical Candid qualified by the release set.
    pub candid_sha256: [u8; 32],
    /// Exact profile identity used to select the role binding before the first call.
    pub protocol_profile_digest: ProtocolProfileDigest,
    /// SHA-256 of the exact gzip payload retained by the Store.
    pub payload_hash: [u8; 32],
    pub payload_size_bytes: u64,
}

///
/// RootStoreBootstrapResponse
///
/// Exact live Store evidence returned after root-side publication and verification.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootStoreBootstrapResponse {
    pub fleet_subnet_root: Principal,
    pub wasm_store: Principal,
    pub release_set: FleetSubnetRootReleaseSet,
    pub catalog: Vec<RootStoreCatalogEntry>,
}
