//! Read-only projections used while assembling a frontend handoff.

use crate::{frontend::model::FrontendManifestRecord, registry::RegistryEntry};
use canic_core::ids::CanonicalNetworkId;
use std::collections::BTreeMap;

///
/// FrontendAssetChunkView
///
/// First asset query response, before comparison with the exact handoff identity.
///

pub struct FrontendAssetChunkView {
    pub content: Vec<u8>,
    pub total_length: u64,
    pub encoding: String,
    pub sha256: Option<Vec<u8>>,
}

///
/// FrontendUploadedFileView
///
/// Expected remote identity of one verified handoff file.
///

#[derive(Debug, Eq, PartialEq, serde::Serialize)]
pub struct FrontendUploadedFileView {
    pub key: String,
    pub bytes: u64,
    pub sha256: String,
}

///
/// FrontendUploadView
///
/// Exact files observed during one read-only asset verification.
///

#[derive(Debug, Eq, PartialEq, serde::Serialize)]
pub struct FrontendUploadView {
    pub environment: String,
    pub canister_id: candid::Principal,
    pub manifest_sha256: String,
    pub files: Vec<FrontendUploadedFileView>,
}

/// Bounded byte inventory of exactly the static assets selected for upload.
#[derive(Debug, Eq, PartialEq, serde::Serialize)]
pub struct FrontendPayloadView {
    pub files: u32,
    pub bytes: u64,
    pub sha256: String,
}

/// External native-balance observation compared with an explicit operator floor.
#[derive(Debug, Eq, PartialEq, serde::Serialize)]
pub struct FrontendAssetCapacityView {
    pub environment: String,
    pub canister_id: String,
    pub observed_at_unix_secs: u64,
    pub native_cycles: String,
    pub minimum_native_cycles: String,
    pub payload: FrontendPayloadView,
    pub sufficient: bool,
}

/// Selected terminal authority without protected Registry or admission records.
pub struct FrontendAuthorityView {
    pub app: String,
    pub fleet: String,
    pub fleet_id: String,
    pub source_plan_sha256: String,
    pub network: CanonicalNetworkId,
    pub admission_nonempty: bool,
    pub admission_origin: Option<String>,
    pub entries: Vec<RegistryEntry>,
}

/// Completely prepared immutable browser bundle; manifest publication happens last.
pub struct FrontendBundleView {
    pub manifest: FrontendManifestRecord,
    pub files: BTreeMap<String, Vec<u8>>,
}

/// Generated browser and TypeScript modules for one checked Candid service.
pub struct FrontendBindingsView {
    pub javascript: Vec<u8>,
    pub typescript: Vec<u8>,
}
