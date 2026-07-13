use crate::{
    canister_build::CurrentCanisterArtifactBuildOutput,
    release_set::{RootReleaseSetBuildSnapshot, emit_root_release_set_manifest_from_build},
};
use std::path::{Path, PathBuf};

pub(in crate::install_root) struct EmitRootManifestOperation<'a> {
    snapshot: &'a RootReleaseSetBuildSnapshot,
    outputs: &'a [CurrentCanisterArtifactBuildOutput],
}

impl<'a> EmitRootManifestOperation<'a> {
    pub(in crate::install_root) const fn new(
        snapshot: &'a RootReleaseSetBuildSnapshot,
        outputs: &'a [CurrentCanisterArtifactBuildOutput],
    ) -> Self {
        Self { snapshot, outputs }
    }

    pub(in crate::install_root) fn evidence(manifest_path: &Path) -> Vec<String> {
        vec![format!("manifest_path:{}", manifest_path.display())]
    }

    pub(in crate::install_root) fn execute(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        emit_root_release_set_manifest_from_build(self.snapshot, self.outputs)
    }
}
