use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{
        DelegationCert, DelegationProvisionRequest, DelegationProvisionResponse, DelegationRequest,
        RoleAttestation, RoleAttestationRequest,
    },
    dto::rpc::{
        CreateCanisterParent, CreateCanisterRequest, CreateCanisterResponse, CyclesRequest,
        CyclesResponse, Request, Response, RootCapabilityRequest, RootRequestMetadata,
        UpgradeCanisterRequest, UpgradeCanisterResponse,
    },
    ops::{
        auth::DelegatedTokenOps,
        config::ConfigOps,
        ic::{IcOps, mgmt::MgmtOps},
        runtime::env::EnvOps,
        runtime::metrics::root_capability::{
            RootCapabilityMetricEvent, RootCapabilityMetricKey, RootCapabilityMetrics,
        },
        storage::{
            auth::DelegationStateOps,
            directory::subnet::SubnetDirectoryOps,
            registry::subnet::SubnetRegistryOps,
            replay::{ReplayService, RootReplayOps},
        },
    },
    storage::stable::replay::{ReplaySlotKey, RootReplayRecord},
    workflow::{
        auth::DelegationWorkflow,
        canister_lifecycle::{CanisterLifecycleEvent, CanisterLifecycleWorkflow},
        prelude::*,
        rpc::RpcWorkflowError,
    },
};
use candid::{decode_one, encode_one};
use sha2::{Digest, Sha256};

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
        let ctx = Self::extract_root_context()?;
        let capability_req = RootCapabilityRequest::from(req);
        let capability = Self::map_request(capability_req);
        let capability_key = capability.metric_key();

        Self::authorize(&ctx, &capability)?;

        let replay = Self::check_replay(&ctx, &capability)?;
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
        if !ctx.is_root_env {
            RootCapabilityMetrics::record(
                capability.metric_key(),
                RootCapabilityMetricEvent::Denied,
            );
            return EnvOps::require_root();
        }

        let capability_key = capability.metric_key();
        let capability_name = capability.capability_name();
        let decision = match capability {
            RootCapability::Provision(_req) => Ok(()),
            RootCapability::Upgrade(req) => Self::authorize_upgrade(ctx, req),
            RootCapability::MintCycles(req) => Self::authorize_mint_cycles(ctx, req),
            RootCapability::IssueDelegation(req) => Self::authorize_issue_delegation(ctx, req),
            RootCapability::IssueRoleAttestation(req) => {
                Self::authorize_issue_role_attestation(ctx, req)
            }
        };

        match &decision {
            Ok(()) => {
                RootCapabilityMetrics::record(
                    capability_key,
                    RootCapabilityMetricEvent::Authorized,
                );
                log!(
                    Topic::Rpc,
                    Info,
                    "root capability authorized (capability={capability_name}, caller={}, subnet={}, now={})",
                    ctx.caller,
                    ctx.subnet_id,
                    ctx.now
                );
            }
            Err(err) => {
                RootCapabilityMetrics::record(capability_key, RootCapabilityMetricEvent::Denied);
                log!(
                    Topic::Rpc,
                    Warn,
                    "root capability denied (capability={capability_name}, caller={}, subnet={}, now={}): {err}",
                    ctx.caller,
                    ctx.subnet_id,
                    ctx.now
                );
            }
        }

        decision
    }

    fn authorize_upgrade(
        ctx: &RootContext,
        req: &UpgradeCanisterRequest,
    ) -> Result<(), InternalError> {
        let registry_entry = SubnetRegistryOps::get(req.canister_pid)
            .ok_or(RpcWorkflowError::ChildNotFound(req.canister_pid))?;

        if registry_entry.parent_pid != Some(ctx.caller) {
            return Err(RpcWorkflowError::NotChildOfCaller(req.canister_pid, ctx.caller).into());
        }

        Ok(())
    }

    fn authorize_mint_cycles(_ctx: &RootContext, req: &CyclesRequest) -> Result<(), InternalError> {
        let available = MgmtOps::canister_cycle_balance().to_u128();
        if req.cycles > available {
            return Err(RpcWorkflowError::InsufficientRootCycles {
                requested: req.cycles,
                available,
            }
            .into());
        }

        Ok(())
    }

    fn authorize_issue_delegation(
        ctx: &RootContext,
        req: &DelegationRequest,
    ) -> Result<(), InternalError> {
        let cfg = ConfigOps::delegated_tokens_config()?;
        if !cfg.enabled {
            return Err(RpcWorkflowError::DelegatedTokensDisabled.into());
        }

        let root_pid = EnvOps::root_pid()?;
        if root_pid != IcOps::canister_self() {
            return Err(RpcWorkflowError::DelegationMustTargetRoot.into());
        }

        if ctx.caller != req.shard_pid {
            return Err(
                RpcWorkflowError::DelegationCallerShardMismatch(ctx.caller, req.shard_pid).into(),
            );
        }

        if req.ttl_secs == 0 {
            return Err(RpcWorkflowError::DelegationInvalidTtl(req.ttl_secs).into());
        }

        if req.aud.is_empty() {
            return Err(RpcWorkflowError::DelegationAudienceEmpty.into());
        }

        if req.scopes.is_empty() {
            return Err(RpcWorkflowError::DelegationScopesEmpty.into());
        }

        if req.scopes.iter().any(String::is_empty) {
            return Err(RpcWorkflowError::DelegationScopeEmpty.into());
        }

        Ok(())
    }

    fn authorize_issue_role_attestation(
        ctx: &RootContext,
        req: &RoleAttestationRequest,
    ) -> Result<(), InternalError> {
        if req.subject != ctx.caller {
            return Err(RpcWorkflowError::RoleAttestationSubjectMismatch {
                caller: ctx.caller,
                subject: req.subject,
            }
            .into());
        }

        let registered = SubnetRegistryOps::get(req.subject).ok_or(
            RpcWorkflowError::RoleAttestationSubjectNotRegistered {
                subject: req.subject,
            },
        )?;

        if registered.role != req.role {
            return Err(RpcWorkflowError::RoleAttestationRoleMismatch {
                subject: req.subject,
                requested: req.role.clone(),
                registered: registered.role,
            }
            .into());
        }

        if let Some(requested_subnet) = req.subnet_id
            && requested_subnet != ctx.subnet_id
        {
            return Err(RpcWorkflowError::RoleAttestationSubnetMismatch {
                subject: req.subject,
                requested: requested_subnet,
                local: ctx.subnet_id,
            }
            .into());
        }

        if req.audience.is_none() {
            return Err(RpcWorkflowError::RoleAttestationAudienceRequired.into());
        }

        let max_ttl_secs = Self::max_role_attestation_ttl_seconds();
        if req.ttl_secs == 0 || req.ttl_secs > max_ttl_secs {
            return Err(RpcWorkflowError::RoleAttestationInvalidTtl {
                ttl_secs: req.ttl_secs,
                max_ttl_secs,
            }
            .into());
        }

        Ok(())
    }

    async fn execute_root_capability(
        ctx: &RootContext,
        capability: RootCapability,
    ) -> Result<Response, InternalError> {
        let capability_name = capability.capability_name();

        let result = match capability {
            RootCapability::Provision(req) => Self::execute_provision(ctx, &req).await,
            RootCapability::Upgrade(req) => Self::execute_upgrade(&req).await,
            RootCapability::MintCycles(req) => Self::execute_mint_cycles(ctx, &req).await,
            RootCapability::IssueDelegation(req) => Self::execute_issue_delegation(ctx, &req).await,
            RootCapability::IssueRoleAttestation(req) => {
                Self::execute_issue_role_attestation(ctx, &req).await
            }
        };

        if let Err(err) = &result {
            log!(
                Topic::Rpc,
                Warn,
                "execute_root_capability failed (capability={capability_name}, caller={}, subnet={}, now={}): {err}",
                ctx.caller,
                ctx.subnet_id,
                ctx.now
            );
        }

        result
    }

    fn check_replay(
        ctx: &RootContext,
        capability: &RootCapability,
    ) -> Result<ReplayDecision, InternalError> {
        let capability_key = capability.metric_key();

        let metadata = capability
            .metadata()
            .ok_or_else(|| RpcWorkflowError::MissingReplayMetadata(capability.capability_name()))?;

        if metadata.ttl_seconds == 0 || metadata.ttl_seconds > MAX_ROOT_TTL_SECONDS {
            RootCapabilityMetrics::record(
                capability_key,
                RootCapabilityMetricEvent::ReplayTtlExceeded,
            );
            return Err(RpcWorkflowError::InvalidReplayTtl {
                ttl_seconds: metadata.ttl_seconds,
                max_ttl_seconds: MAX_ROOT_TTL_SECONDS,
            }
            .into());
        }

        let payload_hash = capability.payload_hash()?;
        let slot_key = replay_slot_key(ctx.caller, ctx.self_pid, metadata.request_id);
        let legacy_slot_key =
            replay_slot_key_legacy(ctx.caller, ctx.subnet_id, metadata.request_id);

        if let Some(existing) = RootReplayOps::get(slot_key) {
            return Self::resolve_existing_replay(
                capability.capability_name(),
                capability_key,
                ctx.now,
                payload_hash,
                existing,
            );
        }

        // Compatibility path for 0.11-era keys during replay key migration.
        if legacy_slot_key != slot_key
            && let Some(existing) = RootReplayOps::get(legacy_slot_key)
        {
            return Self::resolve_existing_replay(
                capability.capability_name(),
                capability_key,
                ctx.now,
                payload_hash,
                existing,
            );
        }

        let _ = RootReplayOps::purge_expired(ctx.now, REPLAY_PURGE_SCAN_LIMIT);

        let issued_at = ctx.now;
        let expires_at = issued_at.saturating_add(metadata.ttl_seconds);
        RootCapabilityMetrics::record(capability_key, RootCapabilityMetricEvent::ReplayAccepted);

        Ok(ReplayDecision::Pending(ReplayPending {
            slot_key,
            payload_hash,
            issued_at,
            expires_at,
        }))
    }

    fn resolve_existing_replay(
        capability_name: &'static str,
        capability_key: RootCapabilityMetricKey,
        now: u64,
        payload_hash: [u8; 32],
        existing: RootReplayRecord,
    ) -> Result<ReplayDecision, InternalError> {
        if now > existing.expires_at {
            RootCapabilityMetrics::record(capability_key, RootCapabilityMetricEvent::ReplayExpired);
            // Do not resurrect expired entries.
            return Err(RpcWorkflowError::ReplayExpired(capability_name).into());
        }

        if existing.payload_hash != payload_hash {
            RootCapabilityMetrics::record(
                capability_key,
                RootCapabilityMetricEvent::ReplayDuplicateConflict,
            );
            return Err(RpcWorkflowError::ReplayConflict(capability_name).into());
        }

        let response = decode_one::<Response>(&existing.response_candid)
            .map_err(|err| RpcWorkflowError::ReplayDecodeFailed(err.to_string()))?;
        RootCapabilityMetrics::record(
            capability_key,
            RootCapabilityMetricEvent::ReplayDuplicateSame,
        );

        Ok(ReplayDecision::Cached(response))
    }

    fn commit_replay(pending: ReplayPending, response: &Response) -> Result<(), InternalError> {
        if RootReplayOps::len() >= MAX_ROOT_REPLAY_ENTRIES {
            return Err(
                RpcWorkflowError::ReplayStoreCapacityReached(MAX_ROOT_REPLAY_ENTRIES).into(),
            );
        }

        let response_candid = encode_one(response)
            .map_err(|err| RpcWorkflowError::ReplayEncodeFailed(err.to_string()))?;

        RootReplayOps::upsert(
            pending.slot_key,
            RootReplayRecord {
                payload_hash: pending.payload_hash,
                issued_at: pending.issued_at,
                expires_at: pending.expires_at,
                response_candid,
            },
        );

        Ok(())
    }

    async fn execute_provision(
        ctx: &RootContext,
        req: &CreateCanisterRequest,
    ) -> Result<Response, InternalError> {
        // Look up parent
        let parent_pid = match &req.parent {
            CreateCanisterParent::Canister(pid) => *pid,
            CreateCanisterParent::Root => IcOps::canister_self(),
            CreateCanisterParent::ThisCanister => ctx.caller,

            CreateCanisterParent::Parent => SubnetRegistryOps::get_parent(ctx.caller)
                .ok_or(RpcWorkflowError::ParentNotFound(ctx.caller))?,

            CreateCanisterParent::Directory(role) => SubnetDirectoryOps::get(role)
                .ok_or_else(|| RpcWorkflowError::CanisterRoleNotFound(role.clone()))?,
        };

        let event = CanisterLifecycleEvent::Create {
            role: req.canister_role.clone(),
            parent: parent_pid,
            extra_arg: req.extra_arg.clone(),
        };

        let lifecycle_result = CanisterLifecycleWorkflow::apply(event).await?;
        let new_canister_pid = lifecycle_result
            .new_canister_pid
            .ok_or(RpcWorkflowError::MissingNewCanisterPid)?;

        Ok(Response::CreateCanister(CreateCanisterResponse {
            new_canister_pid,
        }))
    }

    async fn execute_upgrade(req: &UpgradeCanisterRequest) -> Result<Response, InternalError> {
        let event = CanisterLifecycleEvent::Upgrade {
            pid: req.canister_pid,
        };

        CanisterLifecycleWorkflow::apply(event).await?;

        Ok(Response::UpgradeCanister(UpgradeCanisterResponse {}))
    }

    async fn execute_mint_cycles(
        ctx: &RootContext,
        req: &CyclesRequest,
    ) -> Result<Response, InternalError> {
        MgmtOps::deposit_cycles(ctx.caller, req.cycles).await?;

        let cycles_transferred = req.cycles;

        Ok(Response::Cycles(CyclesResponse { cycles_transferred }))
    }

    async fn execute_issue_delegation(
        ctx: &RootContext,
        req: &DelegationRequest,
    ) -> Result<Response, InternalError> {
        let root_pid = EnvOps::root_pid()?;
        let cert = DelegationCert {
            root_pid,
            shard_pid: req.shard_pid,
            issued_at: ctx.now,
            expires_at: ctx.now.saturating_add(req.ttl_secs),
            scopes: req.scopes.clone(),
            aud: req.aud.clone(),
        };

        validate_delegation_cert_policy(&cert)?;

        let response: DelegationProvisionResponse =
            DelegationWorkflow::provision(DelegationProvisionRequest {
                cert,
                signer_targets: vec![ctx.caller],
                verifier_targets: req.verifier_targets.clone(),
            })
            .await?;

        if req.include_root_verifier {
            DelegatedTokenOps::cache_public_keys_for_cert(&response.proof.cert).await?;
            DelegationStateOps::set_proof_from_dto(response.proof.clone());
        }

        Ok(Response::DelegationIssued(response))
    }

    async fn execute_issue_role_attestation(
        ctx: &RootContext,
        req: &RoleAttestationRequest,
    ) -> Result<Response, InternalError> {
        let payload = Self::build_role_attestation(ctx, req)?;
        let signed = DelegatedTokenOps::sign_role_attestation(payload).await?;
        log!(
            Topic::Auth,
            Info,
            "role attestation issued subject={} role={} audience={:?} subnet={:?} issued_at={} expires_at={} epoch={}",
            signed.payload.subject,
            signed.payload.role,
            signed.payload.audience,
            signed.payload.subnet_id,
            signed.payload.issued_at,
            signed.payload.expires_at,
            signed.payload.epoch
        );
        Ok(Response::RoleAttestationIssued(signed))
    }

    fn build_role_attestation(
        ctx: &RootContext,
        req: &RoleAttestationRequest,
    ) -> Result<RoleAttestation, InternalError> {
        let max_ttl_secs = Self::max_role_attestation_ttl_seconds();
        if req.ttl_secs == 0 || req.ttl_secs > max_ttl_secs {
            return Err(RpcWorkflowError::RoleAttestationInvalidTtl {
                ttl_secs: req.ttl_secs,
                max_ttl_secs,
            }
            .into());
        }

        let expires_at = ctx.now.checked_add(req.ttl_secs).ok_or({
            RpcWorkflowError::RoleAttestationInvalidTtl {
                ttl_secs: req.ttl_secs,
                max_ttl_secs,
            }
        })?;

        Ok(RoleAttestation {
            subject: req.subject,
            role: req.role.clone(),
            subnet_id: req.subnet_id,
            audience: req.audience,
            issued_at: ctx.now,
            expires_at,
            epoch: req.epoch,
        })
    }

    fn max_role_attestation_ttl_seconds() -> u64 {
        ConfigOps::role_attestation_config()
            .map(|cfg| cfg.max_ttl_secs)
            .unwrap_or(DEFAULT_MAX_ROLE_ATTESTATION_TTL_SECONDS)
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
    let bytes = encode_one(payload).map_err(|err| {
        RpcWorkflowError::ReplayEncodeFailed(format!("canonical payload encode failed: {err}"))
    })?;
    Ok(hash_domain_separated(REPLAY_PAYLOAD_HASH_DOMAIN, &bytes))
}

fn replay_slot_key(
    caller: Principal,
    target_canister: Principal,
    request_id: [u8; 32],
) -> ReplaySlotKey {
    RootReplayOps::slot_key(
        caller,
        target_canister,
        ReplayService::Root,
        &request_id,
        LEGACY_REPLAY_NONCE,
    )
}

fn replay_slot_key_legacy(
    caller: Principal,
    subnet_id: Principal,
    request_id: [u8; 32],
) -> ReplaySlotKey {
    let mut hasher = Sha256::new();
    hasher.update((LEGACY_REPLAY_SLOT_KEY_DOMAIN.len() as u64).to_be_bytes());
    hasher.update(LEGACY_REPLAY_SLOT_KEY_DOMAIN);
    hasher.update(caller.as_slice());
    hasher.update(subnet_id.as_slice());
    hasher.update(request_id);
    ReplaySlotKey(hasher.finalize().into())
}

fn hash_domain_separated(domain: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update((domain.len() as u64).to_be_bytes());
    hasher.update(domain);
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    hasher.finalize().into()
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
                CreateCanisterParent, CreateCanisterRequest, CyclesRequest, RootRequestMetadata,
                UpgradeCanisterRequest,
            },
        },
        ids::CanisterRole,
        ops::storage::replay::RootReplayOps,
    };

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
