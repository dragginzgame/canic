use super::super::commands::{
    add_icp_environment_target, icp_canister_command_in_network, root_init_args, run_command,
};
use super::super::readiness::wait_for_root_ready;
use super::super::root_cycles::ensure_local_root_min_cycles;
use super::phase::InstallPhaseOperation;
use crate::release_set::{LOCAL_ROOT_MIN_READY_CYCLES, resume_root_bootstrap};
use std::path::{Path, PathBuf};

pub(in crate::install_root) struct InstallRootWasmOperation<'a> {
    icp_root: &'a Path,
    network: &'a str,
    root_canister_id: &'a str,
    root_wasm: PathBuf,
}

impl<'a> InstallRootWasmOperation<'a> {
    pub(in crate::install_root) const fn new(
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

pub(in crate::install_root) struct EnsureRootCyclesOperation<'a> {
    icp_root: &'a Path,
    network: &'a str,
    root_canister_id: &'a str,
    phase: &'static str,
    attempted_action: &'static str,
    phase_label: &'a str,
}

impl<'a> EnsureRootCyclesOperation<'a> {
    pub(in crate::install_root) const fn new(
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

pub(in crate::install_root) struct ResumeBootstrapOperation<'a> {
    network: &'a str,
    root_canister_id: &'a str,
}

impl<'a> ResumeBootstrapOperation<'a> {
    pub(in crate::install_root) const fn new(network: &'a str, root_canister_id: &'a str) -> Self {
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

pub(in crate::install_root) struct WaitRootReadyOperation<'a> {
    network: &'a str,
    root_canister_id: &'a str,
    timeout_seconds: u64,
}

impl<'a> WaitRootReadyOperation<'a> {
    pub(in crate::install_root) const fn new(
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
