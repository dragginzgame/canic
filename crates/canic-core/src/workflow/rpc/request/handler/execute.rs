use super::{RootCapability, RootContext, authorize, delegation, nonroot_cycles};
use crate::{
    InternalError,
    dto::auth::{
        DelegationCert, DelegationProvisionRequest, DelegationProvisionResponse, DelegationRequest,
        RoleAttestation, RoleAttestationRequest,
    },
    dto::rpc::{
        CreateCanisterParent, CreateCanisterRequest, CreateCanisterResponse, Response,
        UpgradeCanisterRequest, UpgradeCanisterResponse,
    },
    log,
    log::Topic,
    ops::{
        auth::DelegatedTokenOps,
        ic::IcOps,
        runtime::env::EnvOps,
        runtime::metrics::auth::{
            VerifierProofCacheEvictionClass, record_verifier_proof_cache_eviction,
            record_verifier_proof_cache_stats,
        },
        storage::{
            auth::DelegationStateOps, directory::subnet::SubnetDirectoryOps,
            registry::subnet::SubnetRegistryOps,
        },
    },
    workflow::{
        auth::DelegationWorkflow,
        canister_lifecycle::{CanisterLifecycleEvent, CanisterLifecycleWorkflow},
        rpc::RpcWorkflowError,
    },
};

pub(super) async fn execute_root_capability(
    ctx: &RootContext,
    capability: RootCapability,
) -> Result<Response, InternalError> {
    let capability_name = capability.capability_name();

    let result = match capability {
        RootCapability::Provision(req) => execute_provision(ctx, &req).await,
        RootCapability::Upgrade(req) => execute_upgrade(&req).await,
        RootCapability::RequestCycles(req) => {
            let response = if ctx.is_root_env {
                nonroot_cycles::execute_root_request_cycles(ctx, &req).await
            } else {
                nonroot_cycles::execute_request_cycles(ctx, &req).await
            }?;
            Ok(Response::Cycles(response))
        }
        RootCapability::IssueDelegation(req) => execute_issue_delegation(ctx, &req).await,
        RootCapability::IssueRoleAttestation(req) => {
            execute_issue_role_attestation(ctx, &req).await
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

    delegation::validate_delegation_cert_policy(&cert)?;

    let response: DelegationProvisionResponse =
        DelegationWorkflow::provision(DelegationProvisionRequest {
            cert,
            signer_targets: vec![ctx.caller],
            verifier_targets: req.verifier_targets.clone(),
        })
        .await?;

    if req.include_root_verifier {
        DelegatedTokenOps::cache_public_keys_for_cert(&response.proof.cert).await?;
        let outcome = DelegationStateOps::upsert_proof_from_dto(response.proof.clone(), ctx.now)?;
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
    }

    Ok(Response::DelegationIssued(response))
}

async fn execute_issue_role_attestation(
    ctx: &RootContext,
    req: &RoleAttestationRequest,
) -> Result<Response, InternalError> {
    let payload = build_role_attestation(ctx, req)?;
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

pub(super) fn build_role_attestation(
    ctx: &RootContext,
    req: &RoleAttestationRequest,
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
        role: req.role.clone(),
        subnet_id: req.subnet_id,
        audience: req.audience,
        issued_at: ctx.now,
        expires_at,
        epoch: req.epoch,
    })
}
