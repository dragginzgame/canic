use super::super::commands::{
    add_icp_network_target, icp_canister_command, root_init_args, run_command,
};
use super::super::readiness::wait_for_root_ready;
use super::super::root_cycles::ensure_local_root_min_cycles;
use super::phase::{InstallPhaseLabel, InstallPhaseOperation};
use crate::icp::LocalReplicaTarget;
use crate::release_set::{LOCAL_ROOT_MIN_READY_CYCLES, resume_root_bootstrap};
use std::path::{Path, PathBuf};

pub(in crate::install_root) struct InstallRootWasmOperation<'a> {
    icp_root: &'a Path,
    network: &'a str,
    root_canister_id: &'a str,
    root_wasm: PathBuf,
    local_replica: Option<&'a LocalReplicaTarget>,
}

impl<'a> InstallRootWasmOperation<'a> {
    pub(in crate::install_root) const fn new(
        icp_root: &'a Path,
        network: &'a str,
        root_canister_id: &'a str,
        root_wasm: PathBuf,
        local_replica: Option<&'a LocalReplicaTarget>,
    ) -> Self {
        Self {
            icp_root,
            network,
            root_canister_id,
            root_wasm,
            local_replica,
        }
    }
}

impl InstallPhaseOperation for InstallRootWasmOperation<'_> {
    fn phase(&self) -> InstallPhaseLabel {
        InstallPhaseLabel::INSTALL_ROOT
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
            self.local_replica,
        )
    }
}

pub(in crate::install_root) struct EnsureRootCyclesOperation<'a> {
    icp_root: &'a Path,
    network: &'a str,
    root_canister_id: &'a str,
    phase: InstallPhaseLabel,
    attempted_action: &'static str,
    phase_label: &'a str,
    local_replica: Option<&'a LocalReplicaTarget>,
}

impl<'a> EnsureRootCyclesOperation<'a> {
    pub(in crate::install_root) const fn new(
        icp_root: &'a Path,
        network: &'a str,
        root_canister_id: &'a str,
        phase: InstallPhaseLabel,
        attempted_action: &'static str,
        phase_label: &'a str,
        local_replica: Option<&'a LocalReplicaTarget>,
    ) -> Self {
        Self {
            icp_root,
            network,
            root_canister_id,
            phase,
            attempted_action,
            phase_label,
            local_replica,
        }
    }
}

impl InstallPhaseOperation for EnsureRootCyclesOperation<'_> {
    fn phase(&self) -> InstallPhaseLabel {
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
            self.local_replica,
        )
    }
}

pub(in crate::install_root) struct ResumeBootstrapOperation<'a> {
    icp_root: &'a Path,
    network: &'a str,
    root_canister_id: &'a str,
    local_replica: Option<&'a LocalReplicaTarget>,
}

impl<'a> ResumeBootstrapOperation<'a> {
    pub(in crate::install_root) const fn new(
        icp_root: &'a Path,
        network: &'a str,
        root_canister_id: &'a str,
        local_replica: Option<&'a LocalReplicaTarget>,
    ) -> Self {
        Self {
            icp_root,
            network,
            root_canister_id,
            local_replica,
        }
    }
}

impl InstallPhaseOperation for ResumeBootstrapOperation<'_> {
    fn phase(&self) -> InstallPhaseLabel {
        InstallPhaseLabel::RESUME_BOOTSTRAP
    }

    fn attempted_action(&self) -> &'static str {
        "resume root bootstrap"
    }

    fn evidence(&self) -> Vec<String> {
        vec![format!("root_canister:{}", self.root_canister_id)]
    }

    fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        resume_root_bootstrap(
            self.icp_root,
            self.network,
            self.local_replica,
            self.root_canister_id,
        )
    }
}

pub(in crate::install_root) struct WaitRootReadyOperation<'a> {
    icp_root: &'a Path,
    network: &'a str,
    root_canister_id: &'a str,
    timeout_seconds: u64,
    local_replica: Option<&'a LocalReplicaTarget>,
}

impl<'a> WaitRootReadyOperation<'a> {
    pub(in crate::install_root) const fn new(
        icp_root: &'a Path,
        network: &'a str,
        root_canister_id: &'a str,
        timeout_seconds: u64,
        local_replica: Option<&'a LocalReplicaTarget>,
    ) -> Self {
        Self {
            icp_root,
            network,
            root_canister_id,
            timeout_seconds,
            local_replica,
        }
    }
}

impl InstallPhaseOperation for WaitRootReadyOperation<'_> {
    fn phase(&self) -> InstallPhaseLabel {
        InstallPhaseLabel::WAIT_READY
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
        wait_for_root_ready(
            self.icp_root,
            self.network,
            self.root_canister_id,
            self.timeout_seconds,
            self.local_replica,
        )
    }
}

fn reinstall_root_wasm(
    icp_root: &Path,
    network: &str,
    root_canister: &str,
    root_wasm: &Path,
    local_replica: Option<&LocalReplicaTarget>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut install = icp_canister_command(icp_root);
    install.args(["install", root_canister, "--mode=reinstall", "-y", "--wasm"]);
    install.arg(root_wasm);
    install.args(["--args", &root_init_args(root_wasm)?]);
    add_icp_network_target(&mut install, network, local_replica);
    run_command(&mut install)
}
