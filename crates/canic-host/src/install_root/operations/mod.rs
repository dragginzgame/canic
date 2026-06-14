use super::build_targets::run_canic_build_targets;
use super::commands::{
    add_icp_environment_target, icp_canister_command_in_network, root_init_args, run_command,
};
use super::readiness::wait_for_root_ready;
use super::root_canister::ensure_root_canister_id;
use super::root_cycles::ensure_local_root_min_cycles;
use crate::canister_build::CanisterBuildProfile;
use crate::release_set::{
    LOCAL_ROOT_MIN_READY_CYCLES, emit_root_release_set_manifest_with_config, resume_root_bootstrap,
};
use std::path::{Path, PathBuf};

pub(super) trait InstallPhaseOperation {
    fn phase(&self) -> &'static str;
    fn attempted_action(&self) -> &'static str;
    fn evidence(&self) -> Vec<String>;
    fn execute(&self) -> Result<(), Box<dyn std::error::Error>>;
}

pub(super) struct ResolveRootCanisterOperation<'a> {
    icp_root: &'a Path,
    network: &'a str,
    root_canister: &'a str,
    config_path: &'a Path,
}

impl<'a> ResolveRootCanisterOperation<'a> {
    pub(super) const fn new(
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

    pub(super) fn evidence(&self, root_canister_id: &str) -> Vec<String> {
        vec![
            format!("root_target:{}", self.root_canister),
            format!("root_canister:{root_canister_id}"),
        ]
    }

    pub(super) fn execute(&self) -> Result<String, Box<dyn std::error::Error>> {
        ensure_root_canister_id(
            self.icp_root,
            self.network,
            self.root_canister,
            self.config_path,
        )
    }
}

pub(super) struct BuildInstallTargetsOperation<'a> {
    network: &'a str,
    build_targets: Vec<String>,
    build_profile: Option<CanisterBuildProfile>,
    config_path: &'a Path,
    icp_root: &'a Path,
}

impl<'a> BuildInstallTargetsOperation<'a> {
    pub(super) const fn new(
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

    pub(super) fn evidence(&self) -> Vec<String> {
        self.build_targets
            .iter()
            .map(|target| format!("build_target:{target}"))
            .collect()
    }

    pub(super) fn role_names(&self) -> Vec<String> {
        self.build_targets.clone()
    }

    pub(super) fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        run_canic_build_targets(
            self.network,
            &self.build_targets,
            self.build_profile,
            self.config_path,
            self.icp_root,
        )
    }
}

pub(super) struct EmitRootManifestOperation<'a> {
    workspace_root: &'a Path,
    icp_root: &'a Path,
    network: &'a str,
    config_path: &'a Path,
}

impl<'a> EmitRootManifestOperation<'a> {
    pub(super) const fn new(
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

    pub(super) fn evidence(manifest_path: &Path) -> Vec<String> {
        vec![format!("manifest_path:{}", manifest_path.display())]
    }

    pub(super) fn execute(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        emit_root_release_set_manifest_with_config(
            self.workspace_root,
            self.icp_root,
            self.network,
            self.config_path,
        )
    }
}

pub(super) struct InstallRootWasmOperation<'a> {
    icp_root: &'a Path,
    network: &'a str,
    root_canister_id: &'a str,
    root_wasm: PathBuf,
}

impl<'a> InstallRootWasmOperation<'a> {
    pub(super) const fn new(
        icp_root: &'a Path,
        network: &'a str,
        root_canister_id: &'a str,
        root_wasm: PathBuf,
    ) -> Self {
        Self {
            icp_root,
            network,
            root_canister_id,
            root_wasm,
        }
    }
}

impl InstallPhaseOperation for InstallRootWasmOperation<'_> {
    fn phase(&self) -> &'static str {
        "install_root"
    }

    fn attempted_action(&self) -> &'static str {
        "install root wasm"
    }

    fn evidence(&self) -> Vec<String> {
        vec![
            format!("root_canister:{}", self.root_canister_id),
            format!("root_wasm:{}", self.root_wasm.display()),
        ]
    }

    fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        reinstall_root_wasm(
            self.icp_root,
            self.network,
            self.root_canister_id,
            &self.root_wasm,
        )
    }
}

pub(super) struct EnsureRootCyclesOperation<'a> {
    icp_root: &'a Path,
    network: &'a str,
    root_canister_id: &'a str,
    phase: &'static str,
    attempted_action: &'static str,
    phase_label: &'a str,
}

impl<'a> EnsureRootCyclesOperation<'a> {
    pub(super) const fn new(
        icp_root: &'a Path,
        network: &'a str,
        root_canister_id: &'a str,
        phase: &'static str,
        attempted_action: &'static str,
        phase_label: &'a str,
    ) -> Self {
        Self {
            icp_root,
            network,
            root_canister_id,
            phase,
            attempted_action,
            phase_label,
        }
    }
}

impl InstallPhaseOperation for EnsureRootCyclesOperation<'_> {
    fn phase(&self) -> &'static str {
        self.phase
    }

    fn attempted_action(&self) -> &'static str {
        self.attempted_action
    }

    fn evidence(&self) -> Vec<String> {
        vec![
            format!("root_canister:{}", self.root_canister_id),
            format!("minimum_cycles:{LOCAL_ROOT_MIN_READY_CYCLES}"),
            format!("funding_phase:{}", self.phase_label),
        ]
    }

    fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        ensure_local_root_min_cycles(
            self.icp_root,
            self.network,
            self.root_canister_id,
            self.phase_label,
        )
    }
}

pub(super) struct ResumeBootstrapOperation<'a> {
    network: &'a str,
    root_canister_id: &'a str,
}

impl<'a> ResumeBootstrapOperation<'a> {
    pub(super) const fn new(network: &'a str, root_canister_id: &'a str) -> Self {
        Self {
            network,
            root_canister_id,
        }
    }
}

impl InstallPhaseOperation for ResumeBootstrapOperation<'_> {
    fn phase(&self) -> &'static str {
        "resume_bootstrap"
    }

    fn attempted_action(&self) -> &'static str {
        "resume root bootstrap"
    }

    fn evidence(&self) -> Vec<String> {
        vec![format!("root_canister:{}", self.root_canister_id)]
    }

    fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        resume_root_bootstrap(self.network, self.root_canister_id)
    }
}

pub(super) struct WaitRootReadyOperation<'a> {
    network: &'a str,
    root_canister_id: &'a str,
    timeout_seconds: u64,
}

impl<'a> WaitRootReadyOperation<'a> {
    pub(super) const fn new(
        network: &'a str,
        root_canister_id: &'a str,
        timeout_seconds: u64,
    ) -> Self {
        Self {
            network,
            root_canister_id,
            timeout_seconds,
        }
    }
}

impl InstallPhaseOperation for WaitRootReadyOperation<'_> {
    fn phase(&self) -> &'static str {
        "wait_ready"
    }

    fn attempted_action(&self) -> &'static str {
        "wait for root bootstrap readiness"
    }

    fn evidence(&self) -> Vec<String> {
        vec![
            format!("root_canister:{}", self.root_canister_id),
            format!("timeout_seconds:{}", self.timeout_seconds),
        ]
    }

    fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        wait_for_root_ready(self.network, self.root_canister_id, self.timeout_seconds)
    }
}

fn reinstall_root_wasm(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
    root_wasm: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut install = icp_canister_command_in_network(icp_root);
    install.args(["install", root_canister, "--mode=reinstall", "-y", "--wasm"]);
    install.arg(root_wasm);
    install.args(["--args", &root_init_args(root_wasm)?]);
    add_icp_environment_target(&mut install, network);
    run_command(&mut install)
}
