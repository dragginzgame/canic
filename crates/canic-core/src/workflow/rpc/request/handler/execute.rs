use super::{
    RootCapability, RootContext, authorize, delegation, nonroot_cycles,
    nonroot_cycles::AuthorizedCyclesGrant,
};
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{
        DelegationCert, DelegationProvisionResponse, DelegationProvisionStatus, DelegationRequest,
        RoleAttestation, RoleAttestationRequest,
    },
    dto::rpc::{
        CreateCanisterParent, CreateCanisterRequest, CreateCanisterResponse,
        RecycleCanisterRequest, RecycleCanisterResponse, Response, UpgradeCanisterRequest,
        UpgradeCanisterResponse,
    },
    format::display_optional,
    log,
    log::Topic,
    ops::{
        auth::DelegatedTokenOps,
        ic::IcOps,
        runtime::metrics::auth::{
            VerifierProofCacheEvictionClass, record_delegation_provision_complete,
            record_delegation_verifier_target_count, record_delegation_verifier_target_failed,
            record_delegation_verifier_target_missing, record_verifier_proof_cache_eviction,
            record_verifier_proof_cache_stats,
        },
        storage::{
            auth::DelegationStateOps, index::subnet::SubnetIndexOps,
            registry::subnet::SubnetRegistryOps,
        },
    },
    workflow::{
        auth::DelegationWorkflow,
        canister_lifecycle::{CanisterLifecycleEvent, CanisterLifecycleWorkflow},
        pool::PoolWorkflow,
        rpc::RpcWorkflowError,
    },
};

pub(super) async fn execute_root_capability(
    ctx: &RootContext,
    capability: RootCapability,
    authorized_cycles: Option<AuthorizedCyclesGrant>,
) -> Result<Response, InternalError> {
    let capability_name = capability.capability_name();

    let result = match capability {
        RootCapability::Provision(req) => execute_provision(ctx, &req).await,
        RootCapability::Upgrade(req) => execute_upgrade(&req).await,
        RootCapability::RecycleCanister(req) => execute_recycle(&req).await,
        RootCapability::RequestCycles(req) => {
            let response = if let Some(grant) = authorized_cycles {
                nonroot_cycles::execute_authorized_request_cycles(ctx, grant).await
            } else if ctx.is_root_env {
                nonroot_cycles::execute_root_request_cycles(ctx, &req).await
            } else {
                nonroot_cycles::execute_request_cycles(ctx, &req).await
            }?;
            Ok(Response::Cycles(response))
        }
        RootCapability::IssueDelegation(req) => execute_issue_delegation(ctx, req).await,
        RootCapability::IssueRoleAttestation(req) => execute_issue_role_attestation(ctx, req).await,
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

async fn execute_provision(
    ctx: &RootContext,
    req: &CreateCanisterRequest,
) -> Result<Response, InternalError> {
    let parent_pid = match &req.parent {
        CreateCanisterParent::Canister(pid) => *pid,
        CreateCanisterParent::Root => IcOps::canister_self(),
        CreateCanisterParent::ThisCanister => ctx.caller,
        CreateCanisterParent::Parent => SubnetRegistryOps::get_parent(ctx.caller)
            .ok_or(RpcWorkflowError::ParentNotFound(ctx.caller))?,
        CreateCanisterParent::Index(role) => SubnetIndexOps::get(role)
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

async fn execute_recycle(req: &RecycleCanisterRequest) -> Result<Response, InternalError> {
    PoolWorkflow::pool_recycle_canister(req.canister_pid).await?;

    Ok(Response::RecycleCanister(RecycleCanisterResponse {}))
}

async fn execute_issue_delegation(
    ctx: &RootContext,
    req: DelegationRequest,
) -> Result<Response, InternalError> {
    let DelegationRequest {
        shard_pid,
        scopes,
        aud,
        ttl_secs,
        shard_public_key_sec1,
        metadata: _,
    } = req;
    let root_pid = ctx.self_pid;
    let verifier_targets =
        DelegatedTokenOps::required_verifier_targets_from_audience(&aud, shard_pid, root_pid)
            .map_err(delegation::map_verifier_target_derivation_error)?;
    let required_verifier_targets = verifier_targets.clone();
    let cert = DelegationCert {
        root_pid,
        shard_pid,
        issued_at: ctx.now,
        expires_at: ctx.now.saturating_add(ttl_secs),
        scopes,
        aud,
    };

    delegation::validate_delegation_cert_policy(&cert, root_pid)?;
    let root_public_key_sec1 = DelegatedTokenOps::local_root_public_key_sec1(root_pid).await?;

    let (response, cert_hash): (DelegationProvisionResponse, [u8; 32]) =
        DelegationWorkflow::provision(
            cert,
            verifier_targets,
            root_public_key_sec1.as_slice(),
            shard_public_key_sec1.as_slice(),
        )
        .await?;

    ensure_required_verifier_targets_provisioned(&required_verifier_targets, &response)?;

    cache_root_verifier_proof(ctx, &response, cert_hash, shard_public_key_sec1)?;

    Ok(Response::DelegationIssued(response))
}

// Fail canonical issuance when any root-derived verifier fanout target failed.
pub(super) fn ensure_required_verifier_targets_provisioned(
    verifier_targets: &[Principal],
    response: &DelegationProvisionResponse,
) -> Result<(), InternalError> {
    record_delegation_verifier_target_count(verifier_targets.len());

    for target in verifier_targets {
        let Some(result) = response
            .results
            .iter()
            .find(|entry| entry.target == *target)
        else {
            record_delegation_verifier_target_missing();
            return Err(RpcWorkflowError::DelegationVerifierTargetResultMissing {
                target: *target,
            }
            .into());
        };

        if result.status != DelegationProvisionStatus::Ok {
            record_delegation_verifier_target_failed();
            let detail = result
                .error
                .as_ref()
                .map_or_else(|| "unknown error".to_string(), ToString::to_string);
            return Err(RpcWorkflowError::DelegationVerifierTargetProvisionFailed {
                target: *target,
                detail,
            }
            .into());
        }
    }

    record_delegation_provision_complete();
    Ok(())
}

// Store root's verifier proof copy for the issued delegation response.
fn cache_root_verifier_proof(
    ctx: &RootContext,
    response: &DelegationProvisionResponse,
    cert_hash: [u8; 32],
    shard_public_key_sec1: Vec<u8>,
) -> Result<(), InternalError> {
    crate::perf!("cache_root_verifier_keys");
    let outcome =
        DelegationStateOps::upsert_proof_from_dto_ref_with_cert_hash_and_shard_public_key(
            &response.proof,
            cert_hash,
            ctx.now,
            Some(shard_public_key_sec1),
        )?;
    crate::perf!("cache_root_verifier_upsert");
    record_verifier_proof_cache_stats(
        outcome.stats.size,
        outcome.stats.active_count,
        outcome.stats.capacity,
        outcome.stats.profile,
        outcome.stats.active_window_secs,
    );
    if let Some(class) = outcome.evicted {
        let class = match class {
            crate::ops::storage::auth::DelegationProofEvictionClass::Cold => {
                VerifierProofCacheEvictionClass::Cold
            }
            crate::ops::storage::auth::DelegationProofEvictionClass::Active => {
                VerifierProofCacheEvictionClass::Active
            }
        };
        record_verifier_proof_cache_eviction(class);
    }
    crate::perf!("cache_root_verifier_metrics");
    Ok(())
}

async fn execute_issue_role_attestation(
    ctx: &RootContext,
    req: RoleAttestationRequest,
) -> Result<Response, InternalError> {
    let payload = build_role_attestation(ctx, req)?;
    let signed = DelegatedTokenOps::sign_role_attestation(payload).await?;
    log!(
        Topic::Auth,
        Info,
        "role attestation issued subject={} role={} audience={} subnet={} issued_at={} expires_at={} epoch={}",
        signed.payload.subject,
        signed.payload.role,
        display_optional(signed.payload.audience),
        display_optional(signed.payload.subnet_id),
        signed.payload.issued_at,
        signed.payload.expires_at,
        signed.payload.epoch
    );
    Ok(Response::RoleAttestationIssued(signed))
}

pub(super) fn build_role_attestation(
    ctx: &RootContext,
    req: RoleAttestationRequest,
) -> Result<RoleAttestation, InternalError> {
    let max_ttl_secs = authorize::max_role_attestation_ttl_seconds();
    if req.ttl_secs == 0 || req.ttl_secs > max_ttl_secs {
        return Err(RpcWorkflowError::RoleAttestationInvalidTtl {
            ttl_secs: req.ttl_secs,
            max_ttl_secs,
        }
        .into());
    }

    let expires_at =
        ctx.now
            .checked_add(req.ttl_secs)
            .ok_or(RpcWorkflowError::RoleAttestationInvalidTtl {
                ttl_secs: req.ttl_secs,
                max_ttl_secs,
            })?;

    Ok(RoleAttestation {
        subject: req.subject,
        role: req.role,
        subnet_id: req.subnet_id,
        audience: req.audience,
        issued_at: ctx.now,
        expires_at,
        epoch: req.epoch,
    })
}
