use crate::release_set::emit_root_release_set_manifest_with_config;
use std::path::{Path, PathBuf};

pub(in crate::install_root) struct EmitRootManifestOperation<'a> {
    workspace_root: &'a Path,
    icp_root: &'a Path,
    network: &'a str,
    config_path: &'a Path,
}

impl<'a> EmitRootManifestOperation<'a> {
    pub(in crate::install_root) const fn new(
        workspace_root: &'a Path,
        icp_root: &'a Path,
        network: &'a str,
        config_path: &'a Path,
    ) -> Self {
        Self {
            workspace_root,
            icp_root,
            network,
            config_path,
        }
    }

    pub(in crate::install_root) fn evidence(manifest_path: &Path) -> Vec<String> {
        vec![format!("manifest_path:{}", manifest_path.display())]
    }

    pub(in crate::install_root) fn execute(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        emit_root_release_set_manifest_with_config(
            self.workspace_root,
            self.icp_root,
            self.network,
            self.config_path,
        )
    }
}
