use super::super::build_snapshot::InstallBuildTarget;
use super::super::build_targets::{CompletedConfiguredBuild, run_canic_build_targets};
use crate::canister_build::WorkspaceBuildContext;

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
        self.role_names()
            .into_iter()
            .map(|role| format!("build_target:{role}"))
            .collect()
    }

    pub(in crate::install_root) fn role_names(&self) -> Vec<String> {
        let mut roles = Vec::with_capacity(self.build_targets.len() + 2);
        roles.push("fleet_coordinator".to_string());
        for target in self.build_targets {
            if !roles.contains(&target.role) {
                roles.push(target.role.clone());
            }
        }
        if !roles.iter().any(|role| role == "wasm_store") {
            roles.push("wasm_store".to_string());
        }
        roles
    }

    pub(in crate::install_root) fn execute(
        &self,
    ) -> Result<CompletedConfiguredBuild, Box<dyn std::error::Error>> {
        run_canic_build_targets(self.context, self.build_targets)
    }
}
