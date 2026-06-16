use super::super::build_targets::run_canic_build_targets;
use super::super::root_canister::ensure_root_canister_id;
use crate::canister_build::CanisterBuildProfile;
use std::path::Path;

pub(in crate::install_root) struct ResolveRootCanisterOperation<'a> {
    icp_root: &'a Path,
    network: &'a str,
    root_canister: &'a str,
    config_path: &'a Path,
}

impl<'a> ResolveRootCanisterOperation<'a> {
    pub(in crate::install_root) const fn new(
        icp_root: &'a Path,
        network: &'a str,
        root_canister: &'a str,
        config_path: &'a Path,
    ) -> Self {
        Self {
            icp_root,
            network,
            root_canister,
            config_path,
        }
    }

    pub(in crate::install_root) fn evidence(&self, root_canister_id: &str) -> Vec<String> {
        vec![
            format!("root_target:{}", self.root_canister),
            format!("root_canister:{root_canister_id}"),
        ]
    }

    pub(in crate::install_root) fn execute(&self) -> Result<String, Box<dyn std::error::Error>> {
        ensure_root_canister_id(
            self.icp_root,
            self.network,
            self.root_canister,
            self.config_path,
        )
    }
}

pub(in crate::install_root) struct BuildInstallTargetsOperation<'a> {
    network: &'a str,
    build_targets: Vec<String>,
    build_profile: Option<CanisterBuildProfile>,
    config_path: &'a Path,
    icp_root: &'a Path,
}

impl<'a> BuildInstallTargetsOperation<'a> {
    pub(in crate::install_root) const fn new(
        network: &'a str,
        build_targets: Vec<String>,
        build_profile: Option<CanisterBuildProfile>,
        config_path: &'a Path,
        icp_root: &'a Path,
    ) -> Self {
        Self {
            network,
            build_targets,
            build_profile,
            config_path,
            icp_root,
        }
    }

    pub(in crate::install_root) fn evidence(&self) -> Vec<String> {
        self.build_targets
            .iter()
            .map(|target| format!("build_target:{target}"))
            .collect()
    }

    pub(in crate::install_root) fn role_names(&self) -> Vec<String> {
        self.build_targets.clone()
    }

    pub(in crate::install_root) fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        run_canic_build_targets(
            self.network,
            &self.build_targets,
            self.build_profile,
            self.config_path,
            self.icp_root,
        )
    }
}
