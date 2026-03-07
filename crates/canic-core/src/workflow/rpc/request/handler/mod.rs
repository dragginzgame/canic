#[cfg(test)]
use crate::dto::auth::RoleAttestation;
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{DelegationCert, DelegationRequest, RoleAttestationRequest},
    dto::rpc::{
        CreateCanisterRequest, CyclesRequest, Request, Response, RootCapabilityRequest,
        RootRequestMetadata, UpgradeCanisterRequest,
    },
    ops::{
        ic::IcOps,
        runtime::env::EnvOps,
        runtime::metrics::root_capability::{
            RootCapabilityMetricEvent, RootCapabilityMetricKey, RootCapabilityMetrics,
        },
    },
    storage::stable::replay::ReplaySlotKey,
    workflow::rpc::RpcWorkflowError,
};

mod authorize;
mod execute;
mod replay;

const REPLAY_PURGE_SCAN_LIMIT: usize = 256;
const MAX_ROOT_REPLAY_ENTRIES: usize = 10_000;
const MAX_ROOT_TTL_SECONDS: u64 = 300;
const DEFAULT_MAX_ROLE_ATTESTATION_TTL_SECONDS: u64 = 900;
const LEGACY_REPLAY_SLOT_KEY_DOMAIN: &[u8] = b"root-replay-slot-key:v1";
const LEGACY_REPLAY_NONCE: [u8; 16] = [0u8; 16];
const REPLAY_PAYLOAD_HASH_DOMAIN: &[u8] = b"root-replay-payload-hash:v1";

///
/// RootResponseWorkflow
///

pub struct RootResponseWorkflow;

///
/// RootContext
///

#[derive(Clone, Copy, Debug)]
struct RootContext {
    caller: Principal,
    self_pid: Principal,
    is_root_env: bool,
    subnet_id: Principal,
    now: u64,
}

#[derive(Clone, Copy, Debug)]
struct ReplayPending {
    slot_key: ReplaySlotKey,
    payload_hash: [u8; 32],
    issued_at: u64,
    expires_at: u64,
}

#[derive(Debug)]
enum ReplayDecision {
    Cached(Response),
    Pending(ReplayPending),
}

#[derive(Clone, Copy, Debug)]
enum AuthorizationPipelineOrder {
    AuthorizeThenReplay,
    ReplayThenAuthorize,
}

///
/// RootCapability
///

#[derive(Clone, Debug)]
enum RootCapability {
    Provision(CreateCanisterRequest),
    Upgrade(UpgradeCanisterRequest),
    MintCycles(CyclesRequest),
    IssueDelegation(DelegationRequest),
    IssueRoleAttestation(RoleAttestationRequest),
}

impl RootCapability {
    const fn capability_name(&self) -> &'static str {
        match self {
            Self::Provision(_) => "Provision",
            Self::Upgrade(_) => "Upgrade",
            Self::MintCycles(_) => "MintCycles",
            Self::IssueDelegation(_) => "IssueDelegation",
            Self::IssueRoleAttestation(_) => "IssueRoleAttestation",
        }
    }
}

impl RootResponseWorkflow {
    /// Handle a root-bound orchestration request and produce a [`Response`].
    pub async fn response(req: Request) -> Result<Response, InternalError> {
        Self::response_with_pipeline(req, AuthorizationPipelineOrder::AuthorizeThenReplay).await
    }

    /// Handle a root-bound orchestration request using replay-before-policy
    /// ordering for capability-envelope execution.
    pub async fn response_replay_first(req: Request) -> Result<Response, InternalError> {
        Self::response_with_pipeline(req, AuthorizationPipelineOrder::ReplayThenAuthorize).await
    }

    async fn response_with_pipeline(
        req: Request,
        order: AuthorizationPipelineOrder,
    ) -> Result<Response, InternalError> {
        let ctx = Self::extract_root_context()?;
        let capability_req = RootCapabilityRequest::from(req);
        let capability = Self::map_request(capability_req);
        let capability_key = capability.metric_key();

        let replay = Self::preflight(&ctx, &capability, order)?;
        if let ReplayDecision::Cached(response) = replay {
            return Ok(response);
        }

        let response = match Self::execute_root_capability(&ctx, capability).await {
            Ok(response) => response,
            Err(err) => {
                RootCapabilityMetrics::record(
                    capability_key,
                    RootCapabilityMetricEvent::ExecutionError,
                );
                return Err(err);
            }
        };
        let pending = match replay {
            ReplayDecision::Pending(pending) => pending,
            ReplayDecision::Cached(_) => {
                return Err(InternalError::invariant(
                    crate::InternalErrorOrigin::Workflow,
                    "replay state inconsistency: cached response reached commit path".to_string(),
                ));
            }
        };
        if let Err(err) = Self::commit_replay(pending, &response) {
            RootCapabilityMetrics::record(
                capability_key,
                RootCapabilityMetricEvent::ExecutionError,
            );
            return Err(err);
        }
        RootCapabilityMetrics::record(capability_key, RootCapabilityMetricEvent::ExecutionSuccess);

        Ok(response)
    }

    fn preflight(
        ctx: &RootContext,
        capability: &RootCapability,
        order: AuthorizationPipelineOrder,
    ) -> Result<ReplayDecision, InternalError> {
        match order {
            AuthorizationPipelineOrder::AuthorizeThenReplay => {
                Self::authorize(ctx, capability)?;
                Self::check_replay(ctx, capability)
            }
            AuthorizationPipelineOrder::ReplayThenAuthorize => {
                let replay = Self::check_replay(ctx, capability)?;
                if let ReplayDecision::Cached(_) = &replay {
                    return Ok(replay);
                }
                Self::authorize(ctx, capability)?;
                Ok(replay)
            }
        }
    }

    fn extract_root_context() -> Result<RootContext, InternalError> {
        EnvOps::require_root()?;

        Ok(RootContext {
            caller: IcOps::msg_caller(),
            self_pid: IcOps::canister_self(),
            is_root_env: EnvOps::is_root(),
            subnet_id: EnvOps::subnet_pid()?,
            now: IcOps::now_secs(),
        })
    }

    fn map_request(req: RootCapabilityRequest) -> RootCapability {
        match req {
            RootCapabilityRequest::ProvisionCanister(req) => RootCapability::Provision(req),
            RootCapabilityRequest::UpgradeCanister(req) => RootCapability::Upgrade(req),
            RootCapabilityRequest::MintCycles(req) => RootCapability::MintCycles(req),
            RootCapabilityRequest::IssueDelegation(req) => RootCapability::IssueDelegation(req),
            RootCapabilityRequest::IssueRoleAttestation(req) => {
                RootCapability::IssueRoleAttestation(req)
            }
        }
    }

    fn authorize(ctx: &RootContext, capability: &RootCapability) -> Result<(), InternalError> {
        authorize::authorize(ctx, capability)
    }

    async fn execute_root_capability(
        ctx: &RootContext,
        capability: RootCapability,
    ) -> Result<Response, InternalError> {
        execute::execute_root_capability(ctx, capability).await
    }

    fn check_replay(
        ctx: &RootContext,
        capability: &RootCapability,
    ) -> Result<ReplayDecision, InternalError> {
        replay::check_replay(ctx, capability)
    }

    fn commit_replay(pending: ReplayPending, response: &Response) -> Result<(), InternalError> {
        replay::commit_replay(pending, response)
    }

    #[cfg(test)]
    fn build_role_attestation(
        ctx: &RootContext,
        req: &RoleAttestationRequest,
    ) -> Result<RoleAttestation, InternalError> {
        execute::build_role_attestation(ctx, req)
    }
}

impl RootCapability {
    const fn metadata(&self) -> Option<RootRequestMetadata> {
        match self {
            Self::Provision(req) => req.metadata,
            Self::Upgrade(req) => req.metadata,
            Self::MintCycles(req) => req.metadata,
            Self::IssueDelegation(req) => req.metadata,
            Self::IssueRoleAttestation(req) => req.metadata,
        }
    }

    const fn metric_key(&self) -> RootCapabilityMetricKey {
        match self {
            Self::Provision(_) => RootCapabilityMetricKey::Provision,
            Self::Upgrade(_) => RootCapabilityMetricKey::Upgrade,
            Self::MintCycles(_) => RootCapabilityMetricKey::MintCycles,
            Self::IssueDelegation(_) => RootCapabilityMetricKey::IssueDelegation,
            Self::IssueRoleAttestation(_) => RootCapabilityMetricKey::IssueRoleAttestation,
        }
    }

    fn payload_hash(&self) -> Result<[u8; 32], InternalError> {
        let canonical = match self {
            Self::Provision(req) => {
                let mut canonical = req.clone();
                canonical.metadata = None;
                RootCapabilityRequest::ProvisionCanister(canonical)
            }
            Self::Upgrade(req) => {
                let mut canonical = req.clone();
                canonical.metadata = None;
                RootCapabilityRequest::UpgradeCanister(canonical)
            }
            Self::MintCycles(req) => {
                let mut canonical = req.clone();
                canonical.metadata = None;
                RootCapabilityRequest::MintCycles(canonical)
            }
            Self::IssueDelegation(req) => {
                let mut canonical = req.clone();
                canonical.metadata = None;
                RootCapabilityRequest::IssueDelegation(canonical)
            }
            Self::IssueRoleAttestation(req) => {
                let mut canonical = req.clone();
                canonical.metadata = None;
                RootCapabilityRequest::IssueRoleAttestation(canonical)
            }
        };

        hash_capability_payload(&canonical)
    }
}

fn hash_capability_payload(payload: &RootCapabilityRequest) -> Result<[u8; 32], InternalError> {
    replay::hash_capability_payload(payload)
}

#[cfg(test)]
fn replay_slot_key(
    caller: Principal,
    target_canister: Principal,
    request_id: [u8; 32],
) -> ReplaySlotKey {
    replay::replay_slot_key(caller, target_canister, request_id)
}

#[cfg(test)]
fn replay_slot_key_legacy(
    caller: Principal,
    subnet_id: Principal,
    request_id: [u8; 32],
) -> ReplaySlotKey {
    replay::replay_slot_key_legacy(caller, subnet_id, request_id)
}

#[cfg(test)]
fn hash_domain_separated(domain: &[u8], payload: &[u8]) -> [u8; 32] {
    replay::hash_domain_separated(domain, payload)
}

fn validate_delegation_cert_policy(cert: &DelegationCert) -> Result<(), InternalError> {
    if cert.expires_at <= cert.issued_at {
        return Err(RpcWorkflowError::DelegationInvalidWindow {
            issued_at: cert.issued_at,
            expires_at: cert.expires_at,
        }
        .into());
    }

    if cert.aud.is_empty() {
        return Err(RpcWorkflowError::DelegationAudienceEmpty.into());
    }

    if cert.scopes.is_empty() {
        return Err(RpcWorkflowError::DelegationScopesEmpty.into());
    }

    if cert.scopes.iter().any(String::is_empty) {
        return Err(RpcWorkflowError::DelegationScopeEmpty.into());
    }

    let root_pid = EnvOps::root_pid()?;
    if cert.root_pid != root_pid {
        return Err(RpcWorkflowError::DelegationRootPidMismatch(cert.root_pid, root_pid).into());
    }

    if cert.shard_pid == root_pid {
        return Err(RpcWorkflowError::DelegationShardCannotBeRoot.into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal,
        dto::{
            auth::{DelegationRequest, RoleAttestationRequest},
            rpc::{
                CreateCanisterParent, CreateCanisterRequest, CyclesRequest, CyclesResponse,
                RootRequestMetadata, UpgradeCanisterRequest, UpgradeCanisterResponse,
            },
        },
        ids::CanisterRole,
        ops::storage::{registry::subnet::SubnetRegistryOps, replay::RootReplayOps},
        storage::stable::replay::RootReplayRecord,
    };
    use candid::encode_one;
    use sha2::{Digest, Sha256};

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn meta(id: u8, ttl_seconds: u64) -> RootRequestMetadata {
        RootRequestMetadata {
            request_id: [id; 32],
            ttl_seconds,
        }
    }

    #[test]
    fn map_request_maps_provision() {
        let req = RootCapabilityRequest::ProvisionCanister(CreateCanisterRequest {
            canister_role: CanisterRole::new("app"),
            parent: CreateCanisterParent::Root,
            extra_arg: None,
            metadata: None,
        });

        let mapped = RootResponseWorkflow::map_request(req);
        assert_eq!(mapped.capability_name(), "Provision");
    }

    #[test]
    fn map_request_maps_upgrade() {
        let req = RootCapabilityRequest::UpgradeCanister(UpgradeCanisterRequest {
            canister_pid: p(1),
            metadata: None,
        });

        let mapped = RootResponseWorkflow::map_request(req);
        assert_eq!(mapped.capability_name(), "Upgrade");
    }

    #[test]
    fn map_request_maps_cycles() {
        let req = RootCapabilityRequest::MintCycles(CyclesRequest {
            cycles: 42,
            metadata: None,
        });

        let mapped = RootResponseWorkflow::map_request(req);
        assert_eq!(mapped.capability_name(), "MintCycles");
    }

    #[test]
    fn map_request_maps_issue_delegation() {
        let req = RootCapabilityRequest::IssueDelegation(DelegationRequest {
            shard_pid: p(2),
            scopes: vec!["rpc:call".to_string()],
            aud: vec![p(3)],
            ttl_secs: 60,
            verifier_targets: vec![p(4)],
            include_root_verifier: true,
            metadata: None,
        });

        let mapped = RootResponseWorkflow::map_request(req);
        assert_eq!(mapped.capability_name(), "IssueDelegation");
    }

    #[test]
    fn map_request_maps_issue_role_attestation() {
        let req = RootCapabilityRequest::IssueRoleAttestation(RoleAttestationRequest {
            subject: p(2),
            role: CanisterRole::new("test"),
            subnet_id: Some(p(7)),
            audience: Some(p(8)),
            ttl_secs: 120,
            epoch: 1,
            metadata: None,
        });

        let mapped = RootResponseWorkflow::map_request(req);
        assert_eq!(mapped.capability_name(), "IssueRoleAttestation");
    }

    #[test]
    fn authorize_denies_non_root_context() {
        let ctx = RootContext {
            caller: p(1),
            self_pid: p(9),
            is_root_env: false,
            subnet_id: p(2),
            now: 5,
        };
        let capability = RootCapability::Provision(CreateCanisterRequest {
            canister_role: CanisterRole::new("app"),
            parent: CreateCanisterParent::Root,
            extra_arg: None,
            metadata: None,
        });

        let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
        assert!(
            err.to_string().contains("root"),
            "expected root-env denial, got: {err}"
        );
    }

    #[test]
    fn authorize_allows_provision_in_root_context() {
        let ctx = RootContext {
            caller: p(1),
            self_pid: p(9),
            is_root_env: true,
            subnet_id: p(2),
            now: 5,
        };
        let capability = RootCapability::Provision(CreateCanisterRequest {
            canister_role: CanisterRole::new("app"),
            parent: CreateCanisterParent::Root,
            extra_arg: None,
            metadata: None,
        });

        RootResponseWorkflow::authorize(&ctx, &capability).expect("must authorize");
    }

    #[test]
    fn preflight_authorize_then_replay_denies_before_replay_validation() {
        RootReplayOps::reset_for_tests();

        let ctx = RootContext {
            caller: p(1),
            self_pid: p(9),
            is_root_env: false,
            subnet_id: p(2),
            now: 5,
        };
        let capability = RootCapability::MintCycles(CyclesRequest {
            cycles: 42,
            metadata: None,
        });

        let err = RootResponseWorkflow::preflight(
            &ctx,
            &capability,
            AuthorizationPipelineOrder::AuthorizeThenReplay,
        )
        .expect_err("authorize-then-replay should deny before replay validation");
        assert!(
            err.to_string().contains("root"),
            "expected root denial first, got: {err}"
        );
    }

    #[test]
    fn preflight_replay_then_authorize_validates_replay_before_policy() {
        RootReplayOps::reset_for_tests();

        let ctx = RootContext {
            caller: p(1),
            self_pid: p(9),
            is_root_env: false,
            subnet_id: p(2),
            now: 5,
        };
        let capability = RootCapability::MintCycles(CyclesRequest {
            cycles: 42,
            metadata: None,
        });

        let err = RootResponseWorkflow::preflight(
            &ctx,
            &capability,
            AuthorizationPipelineOrder::ReplayThenAuthorize,
        )
        .expect_err("replay-then-authorize should validate replay first");
        assert!(
            err.to_string().contains("missing replay metadata"),
            "expected replay metadata error first, got: {err}"
        );
    }

    #[test]
    fn authorize_rejects_role_attestation_when_subject_mismatches_caller() {
        let ctx = RootContext {
            caller: p(1),
            self_pid: p(9),
            is_root_env: true,
            subnet_id: p(2),
            now: 5,
        };
        let capability = RootCapability::IssueRoleAttestation(RoleAttestationRequest {
            subject: p(3),
            role: CanisterRole::new("test"),
            subnet_id: None,
            audience: None,
            ttl_secs: 60,
            epoch: 0,
            metadata: None,
        });

        let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
        assert!(
            err.to_string().contains("must match caller"),
            "expected subject/caller mismatch error, got: {err}"
        );
    }

    #[test]
    fn authorize_rejects_role_attestation_when_subject_not_registered() {
        let subject = p(41);
        let ctx = RootContext {
            caller: subject,
            self_pid: p(9),
            is_root_env: true,
            subnet_id: p(2),
            now: 5,
        };
        let capability = RootCapability::IssueRoleAttestation(RoleAttestationRequest {
            subject,
            role: CanisterRole::new("test"),
            subnet_id: None,
            audience: Some(p(8)),
            ttl_secs: 60,
            epoch: 0,
            metadata: None,
        });

        let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
        assert!(
            err.to_string().contains("not registered"),
            "expected subject not registered error, got: {err}"
        );
    }

    #[test]
    fn authorize_rejects_role_attestation_when_requested_role_differs_from_registry() {
        let subject = p(42);
        SubnetRegistryOps::register_root(subject, 1);

        let ctx = RootContext {
            caller: subject,
            self_pid: p(9),
            is_root_env: true,
            subnet_id: p(2),
            now: 5,
        };
        let capability = RootCapability::IssueRoleAttestation(RoleAttestationRequest {
            subject,
            role: CanisterRole::new("test"),
            subnet_id: None,
            audience: Some(p(8)),
            ttl_secs: 60,
            epoch: 0,
            metadata: None,
        });

        let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
        assert!(
            err.to_string().contains("role mismatch"),
            "expected role mismatch error, got: {err}"
        );
    }

    #[test]
    fn authorize_rejects_role_attestation_when_audience_missing() {
        let subject = p(43);
        SubnetRegistryOps::register_root(subject, 1);

        let ctx = RootContext {
            caller: subject,
            self_pid: p(9),
            is_root_env: true,
            subnet_id: p(2),
            now: 5,
        };
        let capability = RootCapability::IssueRoleAttestation(RoleAttestationRequest {
            subject,
            role: CanisterRole::ROOT,
            subnet_id: None,
            audience: None,
            ttl_secs: 60,
            epoch: 0,
            metadata: None,
        });

        let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
        assert!(
            err.to_string().contains("audience is required"),
            "expected audience-required error, got: {err}"
        );
    }

    #[test]
    fn authorize_rejects_role_attestation_when_subnet_mismatch() {
        let subject = p(44);
        SubnetRegistryOps::register_root(subject, 1);

        let ctx = RootContext {
            caller: subject,
            self_pid: p(9),
            is_root_env: true,
            subnet_id: p(2),
            now: 5,
        };
        let capability = RootCapability::IssueRoleAttestation(RoleAttestationRequest {
            subject,
            role: CanisterRole::ROOT,
            subnet_id: Some(p(7)),
            audience: Some(p(8)),
            ttl_secs: 60,
            epoch: 0,
            metadata: None,
        });

        let err = RootResponseWorkflow::authorize(&ctx, &capability).expect_err("must deny");
        assert!(
            err.to_string().contains("subnet mismatch"),
            "expected subnet mismatch error, got: {err}"
        );
    }

    #[test]
    fn build_role_attestation_uses_root_generated_time_window() {
        let ctx = RootContext {
            caller: p(1),
            self_pid: p(9),
            is_root_env: true,
            subnet_id: p(2),
            now: 1_000,
        };
        let req = RoleAttestationRequest {
            subject: p(1),
            role: CanisterRole::new("test"),
            subnet_id: Some(p(7)),
            audience: Some(p(8)),
            ttl_secs: 120,
            epoch: 5,
            metadata: None,
        };

        let payload = RootResponseWorkflow::build_role_attestation(&ctx, &req).expect("payload");
        assert_eq!(payload.subject, req.subject);
        assert_eq!(payload.role, req.role);
        assert_eq!(payload.subnet_id, req.subnet_id);
        assert_eq!(payload.audience, req.audience);
        assert_eq!(payload.issued_at, 1_000);
        assert_eq!(payload.expires_at, 1_120);
        assert_eq!(payload.epoch, 5);
    }

    #[test]
    fn build_role_attestation_rejects_invalid_ttl() {
        let ctx = RootContext {
            caller: p(1),
            self_pid: p(9),
            is_root_env: true,
            subnet_id: p(2),
            now: 1_000,
        };
        let mut req = RoleAttestationRequest {
            subject: p(1),
            role: CanisterRole::new("test"),
            subnet_id: Some(p(7)),
            audience: Some(p(8)),
            ttl_secs: 0,
            epoch: 5,
            metadata: None,
        };

        let zero_ttl =
            RootResponseWorkflow::build_role_attestation(&ctx, &req).expect_err("must reject");
        assert!(
            zero_ttl.to_string().contains("ttl_secs"),
            "expected ttl error for zero ttl, got: {zero_ttl}"
        );

        req.ttl_secs = DEFAULT_MAX_ROLE_ATTESTATION_TTL_SECONDS + 1;
        let too_large =
            RootResponseWorkflow::build_role_attestation(&ctx, &req).expect_err("must reject");
        assert!(
            too_large.to_string().contains("ttl_secs"),
            "expected ttl error for too-large ttl, got: {too_large}"
        );
    }

    #[test]
    fn payload_hash_ignores_metadata() {
        let hash_a = RootCapability::MintCycles(CyclesRequest {
            cycles: 42,
            metadata: Some(meta(1, 60)),
        })
        .payload_hash()
        .expect("hash");
        let hash_b = RootCapability::MintCycles(CyclesRequest {
            cycles: 42,
            metadata: Some(meta(9, 120)),
        })
        .payload_hash()
        .expect("hash");

        assert_eq!(hash_a, hash_b, "metadata must not affect payload hash");
    }

    #[test]
    fn payload_hash_includes_capability_variant_discriminant() {
        let capability_hash = RootCapability::MintCycles(CyclesRequest {
            cycles: 42,
            metadata: None,
        })
        .payload_hash()
        .expect("hash");

        let legacy_struct_hash = {
            let bytes = encode_one(&CyclesRequest {
                cycles: 42,
                metadata: None,
            })
            .expect("encode");
            hash_domain_separated(REPLAY_PAYLOAD_HASH_DOMAIN, &bytes)
        };

        assert_ne!(
            capability_hash, legacy_struct_hash,
            "capability payload hash must include variant discriminant"
        );
    }

    #[test]
    fn replay_slot_key_binds_caller_target_and_request_id() {
        let request_id = [9u8; 32];
        let key = replay_slot_key(p(1), p(2), request_id);

        assert_ne!(
            key,
            replay_slot_key(p(3), p(2), request_id),
            "caller must affect replay key"
        );
        assert_ne!(
            key,
            replay_slot_key(p(1), p(4), request_id),
            "target must affect replay key"
        );
        assert_ne!(
            key,
            replay_slot_key(p(1), p(2), [8u8; 32]),
            "request_id must affect replay key"
        );
    }

    #[test]
    fn check_replay_reads_legacy_slot_key_for_compatibility() {
        RootReplayOps::reset_for_tests();

        let ctx = RootContext {
            caller: p(1),
            self_pid: p(42),
            is_root_env: true,
            subnet_id: p(2),
            now: 1_000,
        };
        let capability = RootCapability::MintCycles(CyclesRequest {
            cycles: 77,
            metadata: Some(meta(7, 60)),
        });
        let payload_hash = capability.payload_hash().expect("hash");
        let request_id = capability.metadata().expect("metadata").request_id;
        let legacy_key = replay_slot_key_legacy(ctx.caller, ctx.subnet_id, request_id);

        let response = Response::Cycles(CyclesResponse {
            cycles_transferred: 77,
        });
        let response_candid = encode_one(&response).expect("encode");

        RootReplayOps::upsert(
            legacy_key,
            RootReplayRecord {
                payload_hash,
                issued_at: 900,
                expires_at: 1_200,
                response_candid,
            },
        );

        let replay =
            RootResponseWorkflow::check_replay(&ctx, &capability).expect("legacy replay hit");
        match replay {
            ReplayDecision::Cached(Response::Cycles(cached)) => {
                assert_eq!(cached.cycles_transferred, 77);
            }
            _ => panic!("expected cached cycles response"),
        }
    }

    #[test]
    fn check_replay_rejects_invalid_ttl() {
        RootReplayOps::reset_for_tests();

        let ctx = RootContext {
            caller: p(1),
            self_pid: p(42),
            is_root_env: true,
            subnet_id: p(2),
            now: 1_000,
        };

        let too_small = RootCapability::MintCycles(CyclesRequest {
            cycles: 77,
            metadata: Some(meta(7, 0)),
        });
        let err = RootResponseWorkflow::check_replay(&ctx, &too_small).expect_err("must reject");
        assert!(
            err.to_string().contains("invalid replay ttl"),
            "expected ttl validation error, got: {err}"
        );

        let too_large = RootCapability::MintCycles(CyclesRequest {
            cycles: 77,
            metadata: Some(meta(7, MAX_ROOT_TTL_SECONDS + 1)),
        });
        let err = RootResponseWorkflow::check_replay(&ctx, &too_large).expect_err("must reject");
        assert!(
            err.to_string().contains("invalid replay ttl"),
            "expected ttl validation error, got: {err}"
        );
    }

    #[test]
    fn check_replay_rejects_expired_entry_when_purge_limit_exceeded() {
        RootReplayOps::reset_for_tests();

        let ctx = RootContext {
            caller: p(7),
            self_pid: p(55),
            is_root_env: true,
            subnet_id: p(8),
            now: 10_000,
        };
        let capability = RootCapability::MintCycles(CyclesRequest {
            cycles: 500,
            metadata: Some(meta(11, 60)),
        });
        let payload_hash = capability.payload_hash().expect("hash");
        let request_id = capability.metadata().expect("metadata").request_id;
        let target_key = replay_slot_key(ctx.caller, ctx.self_pid, request_id);
        let response_candid = encode_one(Response::Cycles(CyclesResponse {
            cycles_transferred: 500,
        }))
        .expect("encode");

        RootReplayOps::upsert(
            target_key,
            RootReplayRecord {
                payload_hash,
                issued_at: 9_900,
                expires_at: 9_999,
                response_candid: response_candid.clone(),
            },
        );

        // Force purge limit exhaustion before reaching target_key by seeding
        // 256 lexicographically smaller expired entries.
        let mut seeded = 0usize;
        let mut nonce = 0u64;
        while seeded < REPLAY_PURGE_SCAN_LIMIT {
            let mut hasher = Sha256::new();
            hasher.update(nonce.to_be_bytes());
            let candidate: [u8; 32] = hasher.finalize().into();
            nonce = nonce.saturating_add(1);
            assert!(
                nonce < 1_000_000,
                "failed to seed replay filler keys before nonce overflow"
            );

            if candidate >= target_key.0 {
                continue;
            }

            RootReplayOps::upsert(
                ReplaySlotKey(candidate),
                RootReplayRecord {
                    payload_hash: [0u8; 32],
                    issued_at: 9_000,
                    expires_at: 9_100,
                    response_candid: response_candid.clone(),
                },
            );
            seeded += 1;
        }

        let err = RootResponseWorkflow::check_replay(&ctx, &capability).expect_err("must expire");
        assert!(
            err.to_string().contains("replay request expired"),
            "expected replay expiration error, got: {err}"
        );
    }

    #[test]
    fn check_replay_returns_cached_for_identical_payload() {
        RootReplayOps::reset_for_tests();

        let ctx = RootContext {
            caller: p(1),
            self_pid: p(42),
            is_root_env: true,
            subnet_id: p(2),
            now: 1_000,
        };
        let capability = RootCapability::MintCycles(CyclesRequest {
            cycles: 77,
            metadata: Some(meta(7, 60)),
        });

        let first = RootResponseWorkflow::check_replay(&ctx, &capability).expect("first replay");
        let pending = match first {
            ReplayDecision::Pending(pending) => pending,
            ReplayDecision::Cached(_) => panic!("first request must not be cached"),
        };

        let response = Response::Cycles(CyclesResponse {
            cycles_transferred: 77,
        });
        RootResponseWorkflow::commit_replay(pending, &response).expect("commit");

        let second = RootResponseWorkflow::check_replay(&ctx, &capability).expect("second replay");
        match second {
            ReplayDecision::Cached(Response::Cycles(cached)) => {
                assert_eq!(cached.cycles_transferred, 77);
            }
            _ => panic!("expected cached cycles response"),
        }
    }

    #[test]
    fn check_replay_rejects_conflicting_payload_for_same_request_id() {
        RootReplayOps::reset_for_tests();

        let ctx = RootContext {
            caller: p(3),
            self_pid: p(42),
            is_root_env: true,
            subnet_id: p(4),
            now: 2_000,
        };
        let base = RootCapability::MintCycles(CyclesRequest {
            cycles: 10,
            metadata: Some(meta(8, 60)),
        });
        let conflict = RootCapability::MintCycles(CyclesRequest {
            cycles: 11,
            metadata: Some(meta(8, 60)),
        });

        let first = RootResponseWorkflow::check_replay(&ctx, &base).expect("first replay");
        let pending = match first {
            ReplayDecision::Pending(pending) => pending,
            ReplayDecision::Cached(_) => panic!("first request must not be cached"),
        };
        RootResponseWorkflow::commit_replay(
            pending,
            &Response::Cycles(CyclesResponse {
                cycles_transferred: 10,
            }),
        )
        .expect("commit");

        let err = RootResponseWorkflow::check_replay(&ctx, &conflict).expect_err("must conflict");
        assert!(
            err.to_string().contains("replay conflict"),
            "expected replay conflict error, got: {err}"
        );
    }

    #[test]
    fn replay_purge_respects_limit_and_keeps_unexpired_entries() {
        RootReplayOps::reset_for_tests();

        let ok = encode_one(Response::UpgradeCanister(UpgradeCanisterResponse {})).expect("encode");

        for i in 0..5u8 {
            RootReplayOps::upsert(
                ReplaySlotKey([i; 32]),
                RootReplayRecord {
                    payload_hash: [i; 32],
                    issued_at: 0,
                    expires_at: 10,
                    response_candid: ok.clone(),
                },
            );
        }

        for i in 200..202u8 {
            RootReplayOps::upsert(
                ReplaySlotKey([i; 32]),
                RootReplayRecord {
                    payload_hash: [i; 32],
                    issued_at: 0,
                    expires_at: 999,
                    response_candid: ok.clone(),
                },
            );
        }

        let purged = RootReplayOps::purge_expired(100, 3);
        assert_eq!(purged, 3, "purge must stop at the configured limit");
        assert_eq!(
            RootReplayOps::len(),
            4,
            "expected 4 entries after first purge"
        );

        let purged = RootReplayOps::purge_expired(100, 10);
        assert_eq!(purged, 2, "remaining expired entries must be purged");
        assert_eq!(
            RootReplayOps::len(),
            2,
            "only unexpired entries should remain"
        );
    }

    #[test]
    fn commit_replay_rejects_when_capacity_reached() {
        RootReplayOps::reset_for_tests();

        let response_candid = encode_one(Response::Cycles(CyclesResponse {
            cycles_transferred: 1,
        }))
        .expect("encode");

        for i in 0..MAX_ROOT_REPLAY_ENTRIES {
            let mut key = [0u8; 32];
            key[..8].copy_from_slice(&(i as u64).to_be_bytes());

            RootReplayOps::upsert(
                ReplaySlotKey(key),
                RootReplayRecord {
                    payload_hash: [0u8; 32],
                    issued_at: 0,
                    expires_at: 100,
                    response_candid: response_candid.clone(),
                },
            );
        }

        let err = RootResponseWorkflow::commit_replay(
            ReplayPending {
                slot_key: ReplaySlotKey([255u8; 32]),
                payload_hash: [1u8; 32],
                issued_at: 1,
                expires_at: 2,
            },
            &Response::Cycles(CyclesResponse {
                cycles_transferred: 1,
            }),
        )
        .expect_err("commit must fail when store is at capacity");

        assert!(
            err.to_string().contains("replay store capacity reached"),
            "expected capacity error, got: {err}"
        );
    }
}
