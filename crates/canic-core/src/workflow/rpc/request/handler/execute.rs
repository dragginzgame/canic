use super::{
    RootCapability, RootContext, authorize, nonroot_cycles, nonroot_cycles::AuthorizedCyclesGrant,
};
use crate::{
    InternalError,
    dto::auth::{
        InternalInvocationProofPayloadV1, InternalInvocationProofRequest, RoleAttestation,
        RoleAttestationRequest,
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
        auth::AuthOps,
        ic::IcOps,
        storage::{index::subnet::SubnetIndexOps, registry::subnet::SubnetRegistryOps},
    },
    workflow::{
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
        RootCapability::IssueRoleAttestation(req) => execute_issue_role_attestation(ctx, req).await,
        RootCapability::IssueInternalInvocationProof(req) => {
            execute_issue_internal_invocation_proof(ctx, req).await
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

async fn execute_issue_role_attestation(
    ctx: &RootContext,
    req: RoleAttestationRequest,
) -> Result<Response, InternalError> {
    let payload = build_role_attestation(ctx, req)?;
    let signed = AuthOps::sign_role_attestation(payload).await?;
    log!(
        Topic::Auth,
        Info,
        "role attestation issued subject={} role={} audience={} subnet={} issued_at={} expires_at={} epoch={}",
        signed.payload.subject,
        signed.payload.role,
        signed.payload.audience,
        display_optional(signed.payload.subnet_id),
        signed.payload.issued_at,
        signed.payload.expires_at,
        signed.payload.epoch
    );
    Ok(Response::RoleAttestationIssued(signed))
}

async fn execute_issue_internal_invocation_proof(
    ctx: &RootContext,
    req: InternalInvocationProofRequest,
) -> Result<Response, InternalError> {
    let payload = build_internal_invocation_proof(ctx, req)?;
    let signed = AuthOps::sign_internal_invocation_proof(payload).await?;
    log!(
        Topic::Auth,
        Info,
        "internal invocation proof issued subject={} role={} audience={} method={} subnet={} issued_at={} expires_at={} epoch={}",
        signed.payload.subject,
        signed.payload.role,
        signed.payload.audience,
        signed.payload.audience_method,
        display_optional(signed.payload.subnet_id),
        signed.payload.issued_at,
        signed.payload.expires_at,
        signed.payload.epoch
    );
    Ok(Response::InternalInvocationProofIssued(signed))
}

pub(super) fn build_role_attestation(
    ctx: &RootContext,
    req: RoleAttestationRequest,
) -> Result<RoleAttestation, InternalError> {
    let expires_at = attestation_expires_at(ctx.now, req.ttl_secs)?;
    let epoch = AuthOps::current_role_epoch(&req.role)?;

    Ok(RoleAttestation {
        subject: req.subject,
        role: req.role,
        subnet_id: req.subnet_id,
        audience: req.audience,
        issued_at: ctx.now,
        expires_at,
        epoch,
    })
}

pub(super) fn build_internal_invocation_proof(
    ctx: &RootContext,
    req: InternalInvocationProofRequest,
) -> Result<InternalInvocationProofPayloadV1, InternalError> {
    let expires_at = attestation_expires_at(ctx.now, req.ttl_secs)?;
    let epoch = AuthOps::current_role_epoch(&req.role)?;

    Ok(InternalInvocationProofPayloadV1 {
        subject: req.subject,
        role: req.role,
        subnet_id: req.subnet_id,
        audience: req.audience,
        audience_method: req.audience_method,
        issued_at: ctx.now,
        expires_at,
        epoch,
    })
}

fn attestation_expires_at(issued_at: u64, ttl_secs: u64) -> Result<u64, InternalError> {
    let max_ttl_secs = authorize::max_role_attestation_ttl_seconds();
    if ttl_secs == 0 || ttl_secs > max_ttl_secs {
        return Err(RpcWorkflowError::RoleAttestationInvalidTtl {
            ttl_secs,
            max_ttl_secs,
        }
        .into());
    }

    issued_at.checked_add(ttl_secs).ok_or_else(|| {
        RpcWorkflowError::RoleAttestationInvalidTtl {
            ttl_secs,
            max_ttl_secs,
        }
        .into()
    })
}
