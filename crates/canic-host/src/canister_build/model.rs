use std::{collections::BTreeSet, path::PathBuf};

use canic_core::{
    ids::CanisterRole,
    role_contract::{ProtocolProfileDigest, RoleCapabilityKey},
};

pub(super) const FLEET_COORDINATOR_ROLE: &str = "fleet_coordinator";
pub(super) const WASM_STORE_ROLE: &str = "wasm_store";
pub(super) const WASM_TARGET: &str = "wasm32-unknown-unknown";

/// Caller-selected Cargo and Candid policy for one focused canister build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanisterArtifactBuildOptions {
    /// Cargo features applied identically to the declaration and runtime passes.
    pub cargo_features: BTreeSet<String>,
    /// Whether Cargo's package default features remain enabled.
    pub default_features: bool,
    /// Whether the final runtime must retain Candid only as an adjacent sidecar.
    pub sidecar_only_candid: bool,
}

impl Default for CanisterArtifactBuildOptions {
    fn default() -> Self {
        Self {
            cargo_features: BTreeSet::new(),
            default_features: true,
            sidecar_only_candid: false,
        }
    }
}

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
    pub(crate) canic_version: String,
    pub(crate) capabilities:
        std::collections::BTreeSet<canic_core::role_contract::RoleCapabilityKey>,
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
    pub protocol_release_identity: String,
    pub protocol_role: CanisterRole,
    pub protocol_capabilities: BTreeSet<RoleCapabilityKey>,
    pub artifact_root: PathBuf,
    pub wasm_path: PathBuf,
    pub wasm_gz_path: PathBuf,
    pub did_path: PathBuf,
    pub candid_sha256: [u8; 32],
    pub protocol_profile_digest: ProtocolProfileDigest,
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
    pub tool_sha256: Option<String>,
    pub outcome: ArtifactTransformOutcome,
    pub metrics: Option<WasmTransformMetrics>,
}

impl ArtifactTransformOutput {
    pub(crate) const fn not_requested(transform: ArtifactTransformKind) -> Self {
        Self {
            transform,
            tool_version: None,
            tool_sha256: None,
            outcome: ArtifactTransformOutcome::NotRequested,
            metrics: None,
        }
    }
}

/// Tool-owned operation that can change one emitted Wasm artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactTransformKind {
    Shrink,
    CandidMetadata,
    Optimize,
}

/// Result of one optional artifact transform decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactTransformOutcome {
    Applied,
    ToolUnavailable,
    NotRequested,
}

/// Exact structural sizes around one Wasm transform.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmTransformMetrics {
    pub before: WasmArtifactMetrics,
    pub after: WasmArtifactMetrics,
}

/// Install-relevant and transport sizes for one Wasm module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmArtifactMetrics {
    pub raw_bytes: u64,
    pub gzip_bytes: u64,
    pub code_section_bytes: u64,
    pub data_section_bytes: u64,
    pub defined_functions: u32,
}
