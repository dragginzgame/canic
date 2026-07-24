use super::super::commands::{
    add_icp_environment_target, icp_canister_command, root_init_args, run_command,
};
use super::super::readiness::wait_for_root_ready;
use super::super::root_cycles::ensure_local_root_min_cycles;
use super::phase::{InstallPhaseLabel, InstallPhaseOperation};
use crate::icp::{IcpCanisterStatusReport, IcpCli, LocalReplicaTarget};
use crate::release_set::{LOCAL_ROOT_MIN_READY_CYCLES, resume_root_bootstrap};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

/// Typed failure while proving the module installed on the root Canister.
#[derive(Debug, ThisError)]
pub enum InstallRootModuleVerificationError {
    /// ICP status returned no installed module identity.
    #[error("installed root status does not contain a module hash")]
    Missing,

    /// ICP status returned a value that is not one 32-byte hexadecimal digest.
    #[error("installed root status contains an invalid module hash: {observed}")]
    Invalid { observed: String },

    /// The installed module identity differs from the exact Wasm supplied to ICP.
    #[error("installed root module hash {observed} does not match expected Wasm hash {expected}")]
    Mismatch { expected: String, observed: String },
}

pub(in crate::install_root) struct InstallRootWasmOperation<'a> {
    icp_root: &'a Path,
    environment: &'a str,
    root_canister_id: &'a str,
    root_wasm: PathBuf,
    expected_module_hash: [u8; 32],
    local_replica: Option<&'a LocalReplicaTarget>,
}

impl<'a> InstallRootWasmOperation<'a> {
    pub(in crate::install_root) fn new(
        icp_root: &'a Path,
        environment: &'a str,
        root_canister_id: &'a str,
        root_wasm: PathBuf,
        local_replica: Option<&'a LocalReplicaTarget>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let expected_module_hash = Sha256::digest(fs::read(&root_wasm)?).into();
        Ok(Self {
            icp_root,
            environment,
            root_canister_id,
            root_wasm,
            expected_module_hash,
            local_replica,
        })
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
            format!(
                "expected_module_hash:{}",
                module_hash_text(self.expected_module_hash)
            ),
        ]
    }

    fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        reinstall_root_wasm(
            self.icp_root,
            self.environment,
            self.root_canister_id,
            &self.root_wasm,
            self.local_replica,
        )
    }

    fn verified_evidence(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let report = IcpCli::new("icp", Some(self.environment.to_string()))
            .with_cwd(self.icp_root)
            .with_local_replica(self.local_replica.cloned())
            .canister_status_report(self.root_canister_id)?;
        verified_root_module_evidence(
            self.root_canister_id,
            &self.root_wasm,
            self.expected_module_hash,
            &report,
        )
        .map_err(Into::into)
    }
}

fn verified_root_module_evidence(
    root_canister_id: &str,
    root_wasm: &Path,
    expected_module_hash: [u8; 32],
    report: &IcpCanisterStatusReport,
) -> Result<Vec<String>, InstallRootModuleVerificationError> {
    let observed_text = report
        .module_hash
        .as_deref()
        .ok_or(InstallRootModuleVerificationError::Missing)?;
    let observed_module_hash = parse_module_hash(observed_text).ok_or_else(|| {
        InstallRootModuleVerificationError::Invalid {
            observed: observed_text.to_string(),
        }
    })?;
    if observed_module_hash != expected_module_hash {
        return Err(InstallRootModuleVerificationError::Mismatch {
            expected: module_hash_text(expected_module_hash),
            observed: module_hash_text(observed_module_hash),
        });
    }

    Ok(vec![
        format!("root_canister:{root_canister_id}"),
        format!("root_wasm:{}", root_wasm.display()),
        format!(
            "expected_module_hash:{}",
            module_hash_text(expected_module_hash)
        ),
        format!(
            "observed_module_hash:{}",
            module_hash_text(observed_module_hash)
        ),
    ])
}

fn parse_module_hash(value: &str) -> Option<[u8; 32]> {
    let value = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    if value.len() != 64 {
        return None;
    }
    let mut bytes = [0; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
    }
    Some(bytes)
}

const fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn module_hash_text(bytes: [u8; 32]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(64), |mut text, byte| {
            use std::fmt::Write as _;
            let _ = write!(text, "{byte:02x}");
            text
        })
}

pub(in crate::install_root) struct EnsureRootCyclesOperation<'a> {
    icp_root: &'a Path,
    environment: &'a str,
    root_canister_id: &'a str,
    phase: InstallPhaseLabel,
    attempted_action: &'static str,
    phase_label: &'a str,
    local_replica: Option<&'a LocalReplicaTarget>,
}

impl<'a> EnsureRootCyclesOperation<'a> {
    pub(in crate::install_root) const fn new(
        icp_root: &'a Path,
        environment: &'a str,
        root_canister_id: &'a str,
        phase: InstallPhaseLabel,
        attempted_action: &'static str,
        phase_label: &'a str,
        local_replica: Option<&'a LocalReplicaTarget>,
    ) -> Self {
        Self {
            icp_root,
            environment,
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
            self.environment,
            self.root_canister_id,
            self.phase_label,
            self.local_replica,
        )
    }
}

pub(in crate::install_root) struct ResumeBootstrapOperation<'a> {
    icp_root: &'a Path,
    environment: &'a str,
    root_canister_id: &'a str,
    local_replica: Option<&'a LocalReplicaTarget>,
}

impl<'a> ResumeBootstrapOperation<'a> {
    pub(in crate::install_root) const fn new(
        icp_root: &'a Path,
        environment: &'a str,
        root_canister_id: &'a str,
        local_replica: Option<&'a LocalReplicaTarget>,
    ) -> Self {
        Self {
            icp_root,
            environment,
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
            self.environment,
            self.local_replica,
            self.root_canister_id,
        )
    }
}

pub(in crate::install_root) struct WaitRootReadyOperation<'a> {
    icp_root: &'a Path,
    environment: &'a str,
    root_canister_id: &'a str,
    timeout_seconds: u64,
    local_replica: Option<&'a LocalReplicaTarget>,
}

impl<'a> WaitRootReadyOperation<'a> {
    pub(in crate::install_root) const fn new(
        icp_root: &'a Path,
        environment: &'a str,
        root_canister_id: &'a str,
        timeout_seconds: u64,
        local_replica: Option<&'a LocalReplicaTarget>,
    ) -> Self {
        Self {
            icp_root,
            environment,
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
            self.environment,
            self.root_canister_id,
            self.timeout_seconds,
            self.local_replica,
        )
    }
}

fn reinstall_root_wasm(
    icp_root: &Path,
    environment: &str,
    root_canister: &str,
    root_wasm: &Path,
    local_replica: Option<&LocalReplicaTarget>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut install = icp_canister_command(icp_root);
    install.args(["install", root_canister, "--mode=reinstall", "-y", "--wasm"]);
    install.arg(root_wasm);
    install.args(["--args", &root_init_args(root_wasm)?]);
    add_icp_environment_target(&mut install, environment, local_replica);
    run_command(&mut install)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(module_hash: Option<String>) -> IcpCanisterStatusReport {
        IcpCanisterStatusReport {
            id: "aaaaa-aa".to_string(),
            name: Some("root".to_string()),
            status: "running".to_string(),
            settings: None,
            module_hash,
            memory_size: None,
            cycles: None,
            reserved_cycles: None,
            idle_cycles_burned_per_day: None,
        }
    }

    #[test]
    fn root_module_postcondition_records_only_an_exact_observed_hash() {
        let expected = [0xab; 32];
        let evidence = verified_root_module_evidence(
            "aaaaa-aa",
            Path::new("/tmp/root.wasm"),
            expected,
            &status(Some(format!("0x{}", module_hash_text(expected)))),
        )
        .expect("exact installed module");

        assert_eq!(
            evidence,
            [
                "root_canister:aaaaa-aa".to_string(),
                "root_wasm:/tmp/root.wasm".to_string(),
                format!("expected_module_hash:{}", module_hash_text(expected)),
                format!("observed_module_hash:{}", module_hash_text(expected)),
            ]
        );
        assert!(matches!(
            verified_root_module_evidence(
                "aaaaa-aa",
                Path::new("/tmp/root.wasm"),
                expected,
                &status(None),
            ),
            Err(InstallRootModuleVerificationError::Missing)
        ));
        assert!(matches!(
            verified_root_module_evidence(
                "aaaaa-aa",
                Path::new("/tmp/root.wasm"),
                expected,
                &status(Some(module_hash_text([0xcd; 32]))),
            ),
            Err(InstallRootModuleVerificationError::Mismatch { .. })
        ));
    }
}
