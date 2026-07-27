use super::super::build_snapshot::InstallBuildTarget;
use super::super::build_targets::run_canic_build_targets;
use crate::canister_build::{CurrentCanisterArtifactBuildOutput, WorkspaceBuildContext};

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
