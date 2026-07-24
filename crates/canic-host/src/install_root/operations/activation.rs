use super::super::commands::{
    add_icp_environment_target, icp_canister_command, root_init_args, run_command,
};
use super::phase::{InstallPhaseLabel, InstallPhaseOperation};
use crate::icp::{
    IcpCanisterStatusReport, IcpCli, LocalReplicaTarget, decode_json_result_response,
};
use canic_core::{
    dto::fleet_activation::{
        CurrentRootInstallIdentity, FleetActivationIdentity, FleetActivationPhase,
        FleetActivationStatusResponse,
    },
    protocol,
};
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

#[derive(Debug, ThisError)]
#[error(
    "root install command failed and the exact Prepared postcondition could not be reconciled: operation={operation}; reconciliation={reconciliation}"
)]
pub struct InstallRootExecutionReconciliationError {
    #[source]
    operation: Box<dyn std::error::Error>,
    reconciliation: Box<dyn std::error::Error>,
}

impl InstallRootExecutionReconciliationError {
    #[must_use]
    pub fn operation_error(&self) -> &(dyn std::error::Error + 'static) {
        self.operation.as_ref()
    }

    #[must_use]
    pub fn reconciliation_error(&self) -> &(dyn std::error::Error + 'static) {
        self.reconciliation.as_ref()
    }
}

/// Typed rejection for a root status that cannot prove the exact initial Prepared state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ThisError)]
pub enum InstallRootActivationStatusError {
    #[error("installed root Fleet activation phase is {observed:?}, expected Prepared")]
    Phase { observed: FleetActivationPhase },

    #[error("installed root Fleet activation identity differs from the journalled identity")]
    Identity,

    #[error("installed root initial Prepared state already contains activation evidence")]
    Evidence,
}

pub(in crate::install_root) struct InstallRootWasmOperation<'a> {
    icp_root: &'a Path,
    environment: &'a str,
    root_canister_id: &'a str,
    root_wasm: PathBuf,
    expected_module_hash: [u8; 32],
    init_identity: CurrentRootInstallIdentity,
    local_replica: Option<&'a LocalReplicaTarget>,
}

impl<'a> InstallRootWasmOperation<'a> {
    pub(in crate::install_root) fn new(
        icp_root: &'a Path,
        environment: &'a str,
        root_canister_id: &'a str,
        root_wasm: PathBuf,
        activation_identity: &FleetActivationIdentity,
        local_replica: Option<&'a LocalReplicaTarget>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let expected_module_hash = Sha256::digest(fs::read(&root_wasm)?).into();
        let init_identity = CurrentRootInstallIdentity {
            fleet: activation_identity.fleet.clone(),
            install_id: activation_identity.operation_id,
            release_build_id: activation_identity.release_build_id,
            expected_module_hash: Some(expected_module_hash),
        };
        Ok(Self {
            icp_root,
            environment,
            root_canister_id,
            root_wasm,
            expected_module_hash,
            init_identity,
            local_replica,
        })
    }

    fn icp(&self) -> IcpCli {
        IcpCli::new("icp", Some(self.environment.to_string()))
            .with_cwd(self.icp_root)
            .with_local_replica(self.local_replica.cloned())
    }

    fn observed_module_hash(&self) -> Result<Option<[u8; 32]>, Box<dyn std::error::Error>> {
        let report = self.icp().canister_status_report(self.root_canister_id)?;
        report
            .module_hash
            .as_deref()
            .map(|observed| {
                parse_module_hash(observed).ok_or_else(|| {
                    InstallRootModuleVerificationError::Invalid {
                        observed: observed.to_string(),
                    }
                    .into()
                })
            })
            .transpose()
    }

    fn observe_exact_prepared(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let report = self.icp().canister_status_report(self.root_canister_id)?;
        let mut evidence = verified_root_module_evidence(
            self.root_canister_id,
            &self.root_wasm,
            self.expected_module_hash,
            &report,
        )?;
        let output = self.icp().canister_query_output_with_candid(
            self.root_canister_id,
            protocol::CANIC_FLEET_ACTIVATION_STATUS,
            Some("json"),
            None,
        )?;
        let status = decode_json_result_response::<FleetActivationStatusResponse>(&output)?;
        validate_initial_prepared_status(&self.init_identity, &status)?;
        evidence.extend([
            format!("fleet_id:{}", status.identity.fleet.fleet.fleet_id),
            format!(
                "activation_operation_id:{}",
                digest_text(status.identity.operation_id)
            ),
            format!("release_build_id:{}", status.identity.release_build_id),
            "fleet_activation_phase:prepared".to_string(),
        ]);
        Ok(evidence)
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
            &self.init_identity,
            self.local_replica,
        )
    }

    fn verified_evidence(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        self.observe_exact_prepared()
    }

    fn execute_and_verify(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        if self.observed_module_hash()? == Some(self.expected_module_hash) {
            return self.observe_exact_prepared();
        }

        match self.execute() {
            Ok(()) => self.observe_exact_prepared(),
            Err(operation) => match self.observe_exact_prepared() {
                Ok(evidence) => Ok(evidence),
                Err(reconciliation) => Err(Box::new(InstallRootExecutionReconciliationError {
                    operation,
                    reconciliation,
                })),
            },
        }
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
    digest_text(bytes)
}

fn digest_text(bytes: [u8; 32]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(64), |mut text, byte| {
            use std::fmt::Write as _;
            let _ = write!(text, "{byte:02x}");
            text
        })
}

fn reinstall_root_wasm(
    icp_root: &Path,
    environment: &str,
    root_canister: &str,
    root_wasm: &Path,
    init_identity: &CurrentRootInstallIdentity,
    local_replica: Option<&LocalReplicaTarget>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut install = icp_canister_command(icp_root);
    install.args(["install", root_canister, "--mode=reinstall", "-y", "--wasm"]);
    install.arg(root_wasm);
    install.args(["--args", &root_init_args(init_identity)?]);
    add_icp_environment_target(&mut install, environment, local_replica);
    run_command(&mut install)
}

fn validate_initial_prepared_status(
    expected: &CurrentRootInstallIdentity,
    status: &FleetActivationStatusResponse,
) -> Result<(), InstallRootActivationStatusError> {
    if status.phase != FleetActivationPhase::Prepared {
        return Err(InstallRootActivationStatusError::Phase {
            observed: status.phase,
        });
    }
    if status.identity.fleet != expected.fleet
        || status.identity.operation_id != expected.install_id
        || status.identity.release_build_id != expected.release_build_id
    {
        return Err(InstallRootActivationStatusError::Identity);
    }
    if status.cascade.is_some()
        || status.cascade_manifest.is_some()
        || status.credential.is_some()
        || status.credential_manifest.is_some()
        || status.activated_at_ns.is_some()
    {
        return Err(InstallRootActivationStatusError::Evidence);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::ids::{
        AppId, CanonicalNetworkId, FleetBinding, FleetId, FleetKey, ReleaseBuildId,
        ReleaseBuildNonce,
    };

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

    fn install_identity() -> CurrentRootInstallIdentity {
        CurrentRootInstallIdentity {
            fleet: FleetBinding {
                fleet: FleetKey {
                    network: CanonicalNetworkId::public_ic(),
                    fleet_id: FleetId::from_generated_bytes([1; 32]),
                },
                app: AppId::from("demo"),
            },
            install_id: [2; 32],
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [3; 32],
            )),
            expected_module_hash: Some([4; 32]),
        }
    }

    fn prepared_status(identity: &CurrentRootInstallIdentity) -> FleetActivationStatusResponse {
        FleetActivationStatusResponse {
            phase: FleetActivationPhase::Prepared,
            identity: FleetActivationIdentity {
                fleet: identity.fleet.clone(),
                operation_id: identity.install_id,
                release_build_id: identity.release_build_id,
            },
            cascade: None,
            cascade_manifest: None,
            credential: None,
            credential_manifest: None,
            activated_at_ns: None,
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

    #[test]
    fn root_install_reconciliation_accepts_only_exact_empty_prepared_state() {
        let identity = install_identity();
        validate_initial_prepared_status(&identity, &prepared_status(&identity))
            .expect("exact initial Prepared state");

        let mut active = prepared_status(&identity);
        active.phase = FleetActivationPhase::Active;
        assert_eq!(
            validate_initial_prepared_status(&identity, &active),
            Err(InstallRootActivationStatusError::Phase {
                observed: FleetActivationPhase::Active,
            })
        );

        let mut different_operation = prepared_status(&identity);
        different_operation.identity.operation_id = [5; 32];
        assert_eq!(
            validate_initial_prepared_status(&identity, &different_operation),
            Err(InstallRootActivationStatusError::Identity)
        );

        let mut advanced = prepared_status(&identity);
        advanced.activated_at_ns = Some(6);
        assert_eq!(
            validate_initial_prepared_status(&identity, &advanced),
            Err(InstallRootActivationStatusError::Evidence)
        );
    }
}
