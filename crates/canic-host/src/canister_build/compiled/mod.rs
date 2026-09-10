//! Module: canister_build::compiled
//!
//! Responsibility: retain an exact compiled input until artifact qualification finishes.
//! Does not own: Cargo execution, release identity, or publication of release manifests.
//! Boundary: captures Cargo bytes synchronously before another compilation starts.

use crate::{
    artifact_io::{CapturedWasmArtifact, WasmArtifactFinalization, finalize_wasm_artifact},
    build_toolchain::BuildToolchain,
    canister_build::{CanisterArtifactBuildOutput, CanisterBuildProfile, WorkspaceBuildContext},
    should_embed_candid_metadata,
};
use canic_core::ids::BuildNetwork;
use std::path::{Path, PathBuf};

/// Compiled input and private output metadata awaiting the existing finalization pipeline.
pub struct CompiledCanisterArtifact {
    source: CapturedWasmArtifact,
    output: CanisterArtifactBuildOutput,
    candid: Vec<u8>,
    profile: CanisterBuildProfile,
    build_network: BuildNetwork,
    profile_marker: Option<PathBuf>,
}

impl CompiledCanisterArtifact {
    pub(crate) fn capture(
        context: &WorkspaceBuildContext,
        source: &Path,
        candid: Vec<u8>,
        output: CanisterArtifactBuildOutput,
        profile_marker: Option<PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            source: CapturedWasmArtifact::capture(source, &output.wasm_path)?,
            output,
            candid,
            profile: context.profile,
            build_network: context.build_network,
            profile_marker,
        })
    }

    /// Return a build output only after all existing artifact checks and transforms pass.
    pub(crate) fn finish(
        mut self,
        toolchain: &BuildToolchain,
    ) -> Result<CanisterArtifactBuildOutput, Box<dyn std::error::Error>> {
        self.output.transforms = finalize_wasm_artifact(
            &WasmArtifactFinalization {
                profile: self.profile,
                build_network: self.build_network,
                embed_candid: should_embed_candid_metadata(self.build_network),
                validate_sidecar_only: false,
                source_wasm_path: self.source.path(),
                candid: &self.candid,
                wasm_path: &self.output.wasm_path,
                did_path: &self.output.did_path,
                wasm_gz_path: &self.output.wasm_gz_path,
            },
            toolchain,
        )?;
        if let Some(path) = &self.profile_marker {
            crate::durable_io::write_bytes(path, self.profile.target_dir_name().as_bytes())?;
        }
        Ok(self.output)
    }
}
