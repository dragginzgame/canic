use super::super::build_snapshot::InstallBuildTarget;
use super::super::build_targets::run_canic_build_targets;
use super::super::root_canister::ensure_root_canister_id;
use crate::canister_build::{CurrentCanisterArtifactBuildOutput, WorkspaceBuildContext};
use crate::icp::LocalReplicaTarget;
use std::path::Path;

pub(in crate::install_root) struct ResolveRootCanisterOperation<'a> {
    icp_root: &'a Path,
    environment: &'a str,
    root_canister: &'a str,
    config_path: &'a Path,
    local_replica: Option<&'a LocalReplicaTarget>,
}

impl<'a> ResolveRootCanisterOperation<'a> {
    pub(in crate::install_root) const fn new(
        icp_root: &'a Path,
        environment: &'a str,
        root_canister: &'a str,
        config_path: &'a Path,
        local_replica: Option<&'a LocalReplicaTarget>,
    ) -> Self {
        Self {
            icp_root,
            environment,
            root_canister,
            config_path,
            local_replica,
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
            self.environment,
            self.root_canister,
            self.config_path,
            self.local_replica,
        )
    }
}

pub(in crate::install_root) struct BuildInstallTargetsOperation<'a> {
    context: &'a WorkspaceBuildContext,
    build_targets: &'a [InstallBuildTarget],
}

impl<'a> BuildInstallTargetsOperation<'a> {
    pub(in crate::install_root) const fn new(
        context: &'a WorkspaceBuildContext,
        build_targets: &'a [InstallBuildTarget],
    ) -> Self {
        Self {
            context,
            build_targets,
        }
    }

    pub(in crate::install_root) fn evidence(&self) -> Vec<String> {
        self.build_targets
            .iter()
            .map(|target| format!("build_target:{}", target.role))
            .collect()
    }

    pub(in crate::install_root) fn role_names(&self) -> Vec<String> {
        self.build_targets
            .iter()
            .map(|target| target.role.clone())
            .collect()
    }

    pub(in crate::install_root) fn execute(
        &self,
    ) -> Result<Vec<CurrentCanisterArtifactBuildOutput>, Box<dyn std::error::Error>> {
        run_canic_build_targets(self.context, self.build_targets)
    }
}
