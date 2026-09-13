//! Passive operator input and browser handoff records.
//!
//! No credentials, controller lists or Fleet mutation authority belong in these records.

use candid::Principal;
use canic_core::ids::{CanisterRole, CanonicalNetworkId};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Explicit finite limits for an external asset payload review.
pub struct FrontendAssetCapacityInput {
    pub environment: String,
    pub canister_id: Principal,
    pub payload_directory: PathBuf,
    pub minimum_native_cycles: u128,
    pub maximum_payload_bytes: u64,
    pub maximum_files: u32,
}

/// Exact role instance explicitly selected for browser use.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrontendRoleInput {
    pub role: CanisterRole,
    pub canister_id: Principal,
}

/// Internet Identity provider and the caller namespace retained by the frontend.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrontendIdentityInput {
    pub canister_id: Principal,
    pub provider_origin: String,
    pub derivation_origin: String,
    /// Complete contents of the certified II alternative-origins asset.
    pub alternative_origins: Vec<String>,
}

/// External static-asset identity; Canic neither uploads assets nor controls this canister.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrontendAssetInput {
    pub canister_id: Principal,
    pub origin: String,
}

/// Selected-environment input to a bounded frontend export.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrontendEnvironmentInput {
    pub schema_version: u16,
    pub environment: String,
    pub canonical_network_id: CanonicalNetworkId,
    pub api_origin: String,
    pub identity: FrontendIdentityInput,
    pub asset: Option<FrontendAssetInput>,
    pub roles: Vec<FrontendRoleInput>,
}

/// File identity included in the browser bundle digest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrontendFileRecord {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

/// Public identity and exact generated binding artifacts for one selected instance.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrontendRoleRecord {
    pub role: CanisterRole,
    pub canister_id: Principal,
    pub release_identity: String,
    pub module_sha256: String,
    pub candid: FrontendFileRecord,
    pub javascript: FrontendFileRecord,
    pub typescript: FrontendFileRecord,
}

/// Integrity-covered browser data derived from one terminal Fleet review.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrontendManifestRecord {
    pub schema_version: u16,
    pub manifest_sha256: String,
    pub generator: String,
    pub app: String,
    pub fleet: String,
    pub fleet_id: String,
    pub source_plan_sha256: String,
    pub environment: String,
    pub canonical_network_id: CanonicalNetworkId,
    pub api_origin: String,
    /// Present only for an explicitly enrolled local network; never fetched by the browser.
    #[serde(deserialize_with = "Option::deserialize")]
    pub local_root_key_der_hex: Option<String>,
    pub identity: FrontendIdentityInput,
    pub alternative_origins: FrontendFileRecord,
    #[serde(deserialize_with = "Option::deserialize")]
    pub asset: Option<FrontendAssetInput>,
    pub roles: Vec<FrontendRoleRecord>,
}
