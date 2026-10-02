//! Module: icp::management
//!
//! Responsibility: perform typed management updates with the target
//! canister as the HTTP effective canister ID.
//! Does not own: install policy, durable effect intent, or effect reconciliation.
//! Boundary: the selected ICP environment and identity are resolved through the
//! maintained ICP CLI, while `ic-agent` owns the correctly routed call.

use std::{sync::Arc, time::Duration};

use candid::{CandidType, Principal};
use ic_agent::{
    Agent, AgentError, Identity,
    agent::AgentBuilder,
    identity::{BasicIdentity, Prime256v1Identity, Secp256k1Identity},
};
use serde::{Deserialize, de::DeserializeOwned};
use thiserror::Error as ThisError;

use super::{
    model::IcpCli,
    run::{run_json, run_secret_output},
};

const MANAGEMENT_CANISTER_STATUS: &str = "canister_status";
const MANAGEMENT_INGRESS_EXPIRY: Duration = Duration::from_mins(4);
pub const SNAPSHOT_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

#[derive(CandidType, Deserialize)]
struct ManagementSnapshot {
    id: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct IcpNetworkStatus {
    api_url: String,
    root_key: String,
}

/// Typed failure while resolving or executing an effective-ID-correct
/// management-canister call.
#[derive(Debug, ThisError)]
pub enum IcpManagementCallError {
    #[error("management snapshot read for {canister} exceeded its deadline")]
    SnapshotDeadline { canister: Principal },

    #[error("management snapshot response has {actual} bytes; maximum is {maximum}")]
    SnapshotResponseTooLarge { actual: usize, maximum: usize },

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
    /// List exact snapshot IDs through the selected authenticated management boundary.
    pub(crate) fn management_snapshot_ids(
        &self,
        canister_id: Principal,
    ) -> Result<Vec<Vec<u8>>, IcpManagementCallError> {
        self.measure_request(
            crate::icp::IcpRequestKind::Update,
            Some(&canister_id.to_text()),
            Some("list_canister_snapshots"),
            || {
                let agent =
                    self.authenticated_agent_with_response_limit(SNAPSHOT_RESPONSE_BYTES)?;
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(IcpManagementCallError::AsyncRuntime)?;
                self.record_remote_call();
                runtime.block_on(read_snapshot_ids(&agent, canister_id))
            },
        )
    }

    /// Delete exactly one snapshot after the caller has retained its reviewed intent.
    pub(crate) fn delete_canister_snapshot(
        &self,
        canister_id: Principal,
        snapshot_id: Vec<u8>,
    ) -> Result<(), IcpManagementCallError> {
        #[derive(CandidType)]
        struct Request {
            canister_id: Principal,
            snapshot_id: Vec<u8>,
        }
        self.snapshot_update(
            canister_id,
            "delete_canister_snapshot",
            &Request {
                canister_id,
                snapshot_id,
            },
        )
    }

    fn snapshot_update<I, O>(
        &self,
        canister_id: Principal,
        method: &str,
        input: &I,
    ) -> Result<O, IcpManagementCallError>
    where
        I: CandidType,
        O: for<'de> candid::utils::ArgumentDecoder<'de>,
    {
        self.measure_request(
            crate::icp::IcpRequestKind::Update,
            Some(&canister_id.to_text()),
            Some(method),
            || {
                let argument =
                    candid::encode_one(input).map_err(IcpManagementCallError::CandidEncode)?;
                let agent = self.authenticated_agent()?;
                self.record_remote_call();
                let response = call_management_update(
                    &LiveAgentUpdateBoundary { agent: &agent },
                    canister_id,
                    method,
                    argument,
                )?;
                candid::decode_args(&response).map_err(IcpManagementCallError::CandidResponse)
            },
        )
    }

    /// Whether this command context is bound directly to one local replica.
    #[must_use]
    pub(crate) fn uses_direct_local_replica(&self) -> bool {
        self.local_replica
            .as_ref()
            .is_some_and(|target| self.environment.as_deref() == Some(target.environment.as_str()))
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
        self.measure_request(
            crate::icp::IcpRequestKind::ManagementStatus,
            Some(&effective_canister_id.to_text()),
            Some(MANAGEMENT_CANISTER_STATUS),
            || {
                let argument =
                    candid::encode_one(input).map_err(IcpManagementCallError::CandidEncode)?;
                let agent = self.authenticated_agent()?;
                self.record_remote_call();
                let response = call_management_update(
                    &LiveAgentUpdateBoundary { agent: &agent },
                    effective_canister_id,
                    MANAGEMENT_CANISTER_STATUS,
                    argument,
                )?;
                candid::decode_one(&response).map_err(IcpManagementCallError::CandidResponse)
            },
        )
    }

    /// Clear code and stable state while preserving the exact canister ID and cycles.
    /// The caller must retain the reviewed uninstall intent before invoking this effect.
    pub(crate) fn uninstall_canister(
        &self,
        canister_id: Principal,
    ) -> Result<(), IcpManagementCallError> {
        #[derive(CandidType)]
        struct UninstallRequest {
            canister_id: Principal,
            sender_canister_version: Option<u64>,
        }
        self.measure_request(
            crate::icp::IcpRequestKind::Update,
            Some(&canister_id.to_text()),
            Some("uninstall_code"),
            || {
                let argument = candid::encode_one(UninstallRequest {
                    canister_id,
                    sender_canister_version: None,
                })
                .map_err(IcpManagementCallError::CandidEncode)?;
                let agent = self.authenticated_agent()?;
                self.record_remote_call();
                let response = call_management_update(
                    &LiveAgentUpdateBoundary { agent: &agent },
                    canister_id,
                    "uninstall_code",
                    argument,
                )?;
                candid::decode_args::<()>(&response).map_err(IcpManagementCallError::CandidResponse)
            },
        )
    }

    /// Resolve an agent bound to the selected ICP environment and verified active identity.
    ///
    /// The caller owns reviewed effect authority and durable intent before using it.
    pub fn authenticated_agent(&self) -> Result<Agent, IcpManagementCallError> {
        self.build_authenticated_agent(Agent::builder())
    }

    /// Resolve the same selected identity and network with bounded HTTP response bodies.
    pub(crate) fn authenticated_agent_with_response_limit(
        &self,
        maximum: usize,
    ) -> Result<Agent, IcpManagementCallError> {
        self.build_authenticated_agent(Agent::builder().with_max_response_body_size(maximum))
    }

    pub(super) fn build_authenticated_agent(
        &self,
        builder: AgentBuilder,
    ) -> Result<Agent, IcpManagementCallError> {
        let environment = self
            .environment
            .as_deref()
            .ok_or(IcpManagementCallError::MissingEnvironment)?;
        let network = self.network_status(environment)?;
        let identity = self.exported_active_identity()?;
        let builder = builder
            .with_url(&network.api_url)
            .with_arc_identity(identity)
            .with_ingress_expiry(MANAGEMENT_INGRESS_EXPIRY);
        let agent = builder
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
        if let Some(target) = &self.local_replica
            && environment == target.environment
        {
            return Ok(IcpNetworkStatus {
                api_url: target.url.clone(),
                root_key: target.root_key.clone(),
            });
        }
        let mut command = self.request_command();
        command.args(["network", "status", "--environment", environment, "--json"]);
        run_json(&mut command, self).map_err(Into::into)
    }

    fn exported_active_identity(&self) -> Result<Arc<dyn Identity>, IcpManagementCallError> {
        let identity_name = self.selected_identity_name()?;

        let mut export_command = self.request_command();
        export_command.args(["identity", "export", &identity_name]);
        if let Some(password_file) = self.identity_password_file.as_deref() {
            export_command.arg("--password-file").arg(password_file);
        }
        let mut pem = run_secret_output(&mut export_command, self)?;
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

/// One bounded snapshot inventory call on an already authenticated, network-bound agent.
pub async fn read_snapshot_ids(
    agent: &Agent,
    canister: Principal,
) -> Result<Vec<Vec<u8>>, IcpManagementCallError> {
    #[derive(CandidType)]
    struct Request {
        canister_id: Principal,
    }
    let argument = candid::encode_one(Request {
        canister_id: canister,
    })
    .map_err(IcpManagementCallError::CandidEncode)?;
    let bytes = tokio::time::timeout(
        Duration::from_secs(45),
        agent
            .update(&Principal::management_canister(), "list_canister_snapshots")
            .with_effective_canister_id(canister)
            .with_arg(argument)
            .call_and_wait(),
    )
    .await
    .map_err(|_| IcpManagementCallError::SnapshotDeadline { canister })?
    .map_err(IcpManagementCallError::AgentCall)?;
    decode_snapshot_ids(&bytes)
}

fn decode_snapshot_ids(bytes: &[u8]) -> Result<Vec<Vec<u8>>, IcpManagementCallError> {
    if bytes.len() > SNAPSHOT_RESPONSE_BYTES {
        return Err(IcpManagementCallError::SnapshotResponseTooLarge {
            actual: bytes.len(),
            maximum: SNAPSHOT_RESPONSE_BYTES,
        });
    }
    let mut config = candid::de::DecoderConfig::new();
    config
        .set_decoding_quota(SNAPSHOT_RESPONSE_BYTES * 64)
        .set_skipping_quota(SNAPSHOT_RESPONSE_BYTES * 64);
    let snapshots: Vec<ManagementSnapshot> = candid::utils::decode_one_with_config(bytes, &config)
        .map_err(IcpManagementCallError::CandidResponse)?;
    Ok(snapshots.into_iter().map(|snapshot| snapshot.id).collect())
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

    #[test]
    fn snapshot_inventory_decoder_preserves_ids_and_refuses_oversized_or_malformed_replies() {
        let bytes = candid::encode_one(vec![ManagementSnapshot { id: vec![1, 2, 3] }]).unwrap();
        assert_eq!(decode_snapshot_ids(&bytes).unwrap(), [vec![1, 2, 3]]);
        assert!(matches!(
            decode_snapshot_ids(&vec![0; SNAPSHOT_RESPONSE_BYTES + 1]),
            Err(IcpManagementCallError::SnapshotResponseTooLarge { .. })
        ));
        assert!(matches!(
            decode_snapshot_ids(b"DIDL"),
            Err(IcpManagementCallError::CandidResponse(_))
        ));
    }

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
            environment: "local-qualification".into(),
            root_key: "010203".to_string(),
            url: "http://127.0.0.1:4943/".to_string(),
        };
        let icp = IcpCli::new("missing-icp", Some(target.environment.clone()))
            .with_local_replica(Some(target.clone()));

        let status = icp
            .network_status(&target.environment)
            .expect("resolve explicit local replica without invoking ICP CLI");

        assert_eq!(status.api_url, target.url);
        assert_eq!(status.root_key, target.root_key);
        assert!(icp.uses_direct_local_replica());
        let mut direct = std::process::Command::new("icp");
        icp.add_target_args(&mut direct);
        assert_eq!(
            direct.get_args().collect::<Vec<_>>(),
            ["-n", &target.url, "-k", &target.root_key].map(std::ffi::OsStr::new)
        );
        let other = IcpCli::new("missing-icp", Some("ic".into())).with_local_replica(Some(target));
        assert!(!other.uses_direct_local_replica());
        let mut named = std::process::Command::new("icp");
        other.add_target_args(&mut named);
        assert_eq!(
            named.get_args().collect::<Vec<_>>(),
            ["-e", "ic"].map(std::ffi::OsStr::new)
        );
    }
}
