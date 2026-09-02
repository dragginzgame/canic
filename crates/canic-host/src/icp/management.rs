//! Module: icp::management
//!
//! Responsibility: perform one typed management-canister update with the target
//! canister as the HTTP effective canister ID.
//! Does not own: install policy, durable effect intent, or effect reconciliation.
//! Boundary: the selected ICP environment and identity are resolved through the
//! maintained ICP CLI, while `ic-agent` owns the correctly routed call.

use std::{sync::Arc, time::Duration};

use candid::{CandidType, Principal};
use ic_agent::{
    Agent, AgentError, Identity,
    identity::{BasicIdentity, Prime256v1Identity, Secp256k1Identity},
};
use serde::{Deserialize, de::DeserializeOwned};
use thiserror::Error as ThisError;

use super::{
    model::{IcpCli, LOCAL_ICP_TARGET},
    run::{run_json, run_output, run_secret_output},
};

const MANAGEMENT_CANISTER_STATUS: &str = "canister_status";
const MANAGEMENT_INGRESS_EXPIRY: Duration = Duration::from_mins(4);

#[derive(Debug, Deserialize)]
struct IcpNetworkStatus {
    api_url: String,
    root_key: String,
}

/// Typed failure while resolving or executing an effective-ID-correct
/// management-canister call.
#[derive(Debug, ThisError)]
pub enum IcpManagementCallError {
    #[error("failed to encode the management-canister argument: {0}")]
    CandidEncode(#[source] candid::Error),

    #[error("failed to decode the management-canister response: {0}")]
    CandidResponse(#[source] candid::Error),

    #[error("failed to build the management-canister agent: {0}")]
    AgentBuild(#[source] AgentError),

    #[error("the effective-ID-correct management-canister call failed: {0}")]
    AgentCall(#[source] AgentError),

    #[error("failed to create the management-canister async runtime: {0}")]
    AsyncRuntime(#[source] std::io::Error),

    #[error(transparent)]
    Icp(#[from] super::IcpCommandError),

    #[error("ICP CLI reported an invalid network root key: {0}")]
    NetworkRootKey(#[source] canic_core::cdk::utils::hash::DecodeHexError),

    #[error("the selected ICP identity cannot be represented by a supported exported PEM")]
    UnsupportedExportedIdentity,

    #[error("failed to resolve the Principal of the exported ICP identity: {message}")]
    ExportedIdentityPrincipal { message: String },

    #[error(
        "the exported ICP identity Principal {exported} conflicts with the active ICP identity {active}"
    )]
    ExportedIdentityConflict { active: String, exported: String },

    #[error("an ICP environment is required for the management-canister call")]
    MissingEnvironment,
}

impl IcpCli {
    /// Whether this command context is bound directly to one local replica.
    #[must_use]
    pub(crate) fn uses_direct_local_replica(&self) -> bool {
        self.environment.as_deref() == Some(LOCAL_ICP_TARGET) && self.local_replica.is_some()
    }

    /// Perform one typed management-canister update routed through the target
    /// canister's exact effective canister ID.
    pub(crate) fn management_canister_status_candid<I, O>(
        &self,
        effective_canister_id: Principal,
        input: &I,
    ) -> Result<O, IcpManagementCallError>
    where
        I: CandidType,
        O: CandidType + DeserializeOwned,
    {
        let argument = candid::encode_one(input).map_err(IcpManagementCallError::CandidEncode)?;
        let agent = self.management_agent()?;
        let response = call_management_update(
            &LiveAgentUpdateBoundary { agent: &agent },
            effective_canister_id,
            MANAGEMENT_CANISTER_STATUS,
            argument,
        )?;
        candid::decode_one(&response).map_err(IcpManagementCallError::CandidResponse)
    }

    fn management_agent(&self) -> Result<Agent, IcpManagementCallError> {
        let environment = self
            .environment
            .as_deref()
            .ok_or(IcpManagementCallError::MissingEnvironment)?;
        let network = self.network_status(environment)?;
        let identity = self.exported_active_identity()?;
        let agent = Agent::builder()
            .with_url(&network.api_url)
            .with_arc_identity(identity)
            .with_ingress_expiry(MANAGEMENT_INGRESS_EXPIRY)
            .build()
            .map_err(IcpManagementCallError::AgentBuild)?;
        let root_key = canic_core::cdk::utils::hash::decode_hex(&network.root_key)
            .map_err(IcpManagementCallError::NetworkRootKey)?;
        agent.set_root_key(root_key);
        Ok(agent)
    }

    fn network_status(
        &self,
        environment: &str,
    ) -> Result<IcpNetworkStatus, IcpManagementCallError> {
        if environment == LOCAL_ICP_TARGET
            && let Some(target) = &self.local_replica
        {
            return Ok(IcpNetworkStatus {
                api_url: target.url.clone(),
                root_key: target.root_key.clone(),
            });
        }
        let mut command = self.command();
        command.args(["network", "status", "--environment", environment, "--json"]);
        run_json(&mut command).map_err(Into::into)
    }

    fn exported_active_identity(&self) -> Result<Arc<dyn Identity>, IcpManagementCallError> {
        let mut default_command = self.command();
        default_command.args(["identity", "default"]);
        let identity_name = run_output(&mut default_command)?;

        let mut export_command = self.command();
        export_command.args(["identity", "export", &identity_name]);
        if let Some(password_file) = self.identity_password_file.as_deref() {
            export_command.arg("--password-file").arg(password_file);
        }
        let mut pem = run_secret_output(&mut export_command)?;
        let identity = parse_exported_identity(&pem);
        pem.fill(0);
        let identity = identity?;
        let exported = identity
            .sender()
            .map_err(|message| IcpManagementCallError::ExportedIdentityPrincipal { message })?;
        let active = self.identity_principal_text()?;
        if exported.to_text() != active {
            return Err(IcpManagementCallError::ExportedIdentityConflict {
                active,
                exported: exported.to_text(),
            });
        }
        Ok(identity)
    }
}

fn parse_exported_identity(pem: &[u8]) -> Result<Arc<dyn Identity>, IcpManagementCallError> {
    if let Ok(identity) = BasicIdentity::from_pem(pem) {
        return Ok(Arc::new(identity));
    }
    if let Ok(identity) = Secp256k1Identity::from_pem(pem) {
        return Ok(Arc::new(identity));
    }
    if let Ok(identity) = Prime256v1Identity::from_pem(pem) {
        return Ok(Arc::new(identity));
    }
    Err(IcpManagementCallError::UnsupportedExportedIdentity)
}

#[derive(Debug, Eq, PartialEq)]
struct ManagementUpdateRequest {
    argument: Vec<u8>,
    canister_id: Principal,
    effective_canister_id: Principal,
    method: String,
}

trait AgentUpdateBoundary {
    fn update(&self, request: ManagementUpdateRequest) -> Result<Vec<u8>, IcpManagementCallError>;
}

struct LiveAgentUpdateBoundary<'a> {
    agent: &'a Agent,
}

impl AgentUpdateBoundary for LiveAgentUpdateBoundary<'_> {
    fn update(&self, request: ManagementUpdateRequest) -> Result<Vec<u8>, IcpManagementCallError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(IcpManagementCallError::AsyncRuntime)?;
        runtime
            .block_on(
                self.agent
                    .update(&request.canister_id, request.method)
                    .with_effective_canister_id(request.effective_canister_id)
                    .with_arg(request.argument)
                    .call_and_wait(),
            )
            .map_err(IcpManagementCallError::AgentCall)
    }
}

fn call_management_update(
    boundary: &impl AgentUpdateBoundary,
    effective_canister_id: Principal,
    method: &str,
    argument: Vec<u8>,
) -> Result<Vec<u8>, IcpManagementCallError> {
    boundary.update(ManagementUpdateRequest {
        argument,
        canister_id: Principal::management_canister(),
        effective_canister_id,
        method: method.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icp::LocalReplicaTarget;
    use std::cell::RefCell;

    struct RecordingAgentBoundary {
        request: RefCell<Option<ManagementUpdateRequest>>,
    }

    impl AgentUpdateBoundary for RecordingAgentBoundary {
        fn update(
            &self,
            request: ManagementUpdateRequest,
        ) -> Result<Vec<u8>, IcpManagementCallError> {
            self.request.replace(Some(request));
            Ok(vec![1, 2, 3])
        }
    }

    #[test]
    fn management_status_routes_through_the_target_effective_canister_id() {
        let target =
            Principal::from_text("rrkah-fqaaa-aaaaa-aaaaq-cai").expect("effective canister ID");
        let boundary = RecordingAgentBoundary {
            request: RefCell::new(None),
        };
        let response =
            call_management_update(&boundary, target, MANAGEMENT_CANISTER_STATUS, vec![7, 8, 9])
                .expect("record routed management update");

        assert_eq!(response, vec![1, 2, 3]);
        assert_eq!(
            boundary.request.into_inner(),
            Some(ManagementUpdateRequest {
                argument: vec![7, 8, 9],
                canister_id: Principal::management_canister(),
                effective_canister_id: target,
                method: MANAGEMENT_CANISTER_STATUS.to_string(),
            })
        );
    }

    #[test]
    fn explicit_local_replica_owns_management_network_resolution() {
        let target = LocalReplicaTarget {
            root_key: "010203".to_string(),
            url: "http://127.0.0.1:4943/".to_string(),
        };
        let icp = IcpCli::new("missing-icp", Some(LOCAL_ICP_TARGET.to_string()))
            .with_local_replica(Some(target.clone()));

        let status = icp
            .network_status(LOCAL_ICP_TARGET)
            .expect("resolve explicit local replica without invoking ICP CLI");

        assert_eq!(status.api_url, target.url);
        assert_eq!(status.root_key, target.root_key);
    }
}
